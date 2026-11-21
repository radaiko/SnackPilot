//! Updater state machine, cadence, gates and German copy (design D5/D7) against
//! scripted services: no test reaches a real host.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use snackpilot_core::error::{CoreError, CoreResult};
use snackpilot_core::http::{HttpResponse, Request, Transport};
use snackpilot_core::update::{sha256_hex, Candidate, Downloader, FakeDownloadCall, Manifest};
use snackpilot_model::{format_date_long, format_time};
use snackpilot_model::{
    AppModel, FakePlatform, HandoffOutcome, UpdateAction, UpdateFailure, UpdateHostInfo,
    UpdateServices, UpdateState, UpdateTrigger, UpdateView, VerifiedUpdate,
};

fn unique_token(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{tag}-{}-{nanos}", std::process::id())
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("snackpilot-model-update-{tag}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// A fixed instant for the de-AT date/time of the copy table.
const CHECKED_AT: i64 = 1_770_000_000_000;

fn windows_host(installed: &str, build: &str) -> UpdateHostInfo {
    UpdateHostInfo {
        platform: "windows",
        architecture: "x64",
        os_version: Some("10.0.26100".into()),
        installed_version: installed.into(),
        installed_build: build.into(),
    }
}

fn macos_host(installed: &str, build: &str) -> UpdateHostInfo {
    UpdateHostInfo {
        platform: "macos",
        architecture: "arm64",
        os_version: Some("26.1".into()),
        installed_version: installed.into(),
        installed_build: build.into(),
    }
}

/// A pending discovery request: it stays in flight (holding the model's single
/// operation guard) until the test signals it, without blocking the model
/// runtime, which is a single-thread scheduler.
struct Signal(Arc<(Mutex<bool>, Condvar)>);

impl Signal {
    fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    fn send(&self) {
        let (flag, condvar) = &*self.0;
        *flag.lock().unwrap() = true;
        condvar.notify_all();
    }

    async fn wait(&self) {
        let (flag, _) = &*self.0;
        while !*flag.lock().unwrap() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

enum Reply {
    Body(String),
    /// Answers only after `Signal::send`; served when it answers.
    Pending(Signal),
    Fail,
}

/// Answers the discovery sequence from scripted replies and counts the
/// answered requests: the tests' "core calls".
#[derive(Default)]
struct CapturingTransport {
    requests: Mutex<Vec<Request>>,
    replies: Mutex<VecDeque<Reply>>,
    /// Requests that got an answer, so a pending one is not counted yet.
    answered: AtomicUsize,
}

impl CapturingTransport {
    fn new() -> Self {
        Self::default()
    }

    fn queue(&self, body: impl Into<String>) {
        self.replies
            .lock()
            .unwrap()
            .push_back(Reply::Body(body.into()));
    }

    fn queue_pending(&self) -> Signal {
        let signal = Signal::new();
        self.replies
            .lock()
            .unwrap()
            .push_back(Reply::Pending(Signal(signal.0.clone())));
        signal
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }

    fn answered(&self) -> usize {
        self.answered.load(Ordering::SeqCst)
    }

    /// The URLs asked for, since the last reset.
    fn urls(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.url.clone())
            .collect()
    }
}

impl Transport for CapturingTransport {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CoreResult<HttpResponse>> + Send + 'a>>
    {
        Box::pin(async move {
            // Recorded first: a pending answer still shows the request leaving.
            self.requests.lock().unwrap().push(req);
            let reply = self
                .replies
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(CoreError::Http {
                    detail: "no queued reply".into(),
                    status: None,
                })?;
            self.answered.fetch_add(1, Ordering::SeqCst);
            let body = match reply {
                Reply::Body(body) => body,
                Reply::Pending(signal) => {
                    signal.wait().await;
                    "[]".to_string()
                }
                Reply::Fail => {
                    return Err(CoreError::Http {
                        detail: "offline".into(),
                        status: None,
                    })
                }
            };
            Ok(HttpResponse {
                status: 200,
                headers: vec![],
                body,
            })
        })
    }
}

/// Discovery over the scripted transport, downloads over core's
/// [`FakeDownloader`] (design D1).
struct FakeServices {
    transport: Arc<CapturingTransport>,
    /// Scripted here and read through the model: the same queue and call log.
    downloader: Arc<QueuedDownloader>,
}

/// Core's [`FakeDownloader`] behind interior mutability, so a test scripts the
/// next download however many handles exist (a queued reply holds one).
#[derive(Default)]
struct QueuedDownloader {
    bodies: Mutex<VecDeque<Vec<u8>>>,
    calls: Mutex<Vec<FakeDownloadCall>>,
}

impl QueuedDownloader {
    fn push(&self, body: &[u8]) {
        self.bodies.lock().unwrap().push_back(body.to_vec());
    }

    fn calls(&self) -> Vec<FakeDownloadCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl Downloader for QueuedDownloader {
    /// Same contract as core's [`FakeDownloader`]: streams the queued body in
    /// 4 KiB chunks, writes `dest`, fails with `SizeMismatch` above `max_bytes`.
    fn download<'a>(
        &'a self,
        url: &'a str,
        dest: &'a std::path::Path,
        max_bytes: u64,
        progress: &'a (dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), snackpilot_core::update::UpdateError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.calls.lock().unwrap().push(FakeDownloadCall {
                url: url.to_string(),
                dest: dest.to_path_buf(),
                max_bytes,
            });
            let body = self
                .bodies
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(snackpilot_core::update::UpdateError::Network)?;
            let total = body.len() as u64;
            let mut file = Vec::new();
            for chunk in body.chunks(4096) {
                file.extend_from_slice(chunk);
                if file.len() as u64 > max_bytes {
                    return Err(snackpilot_core::update::UpdateError::SizeMismatch);
                }
                std::fs::write(dest, &file)
                    .map_err(|_| snackpilot_core::update::UpdateError::Io)?;
                progress(file.len() as u64, Some(total));
            }
            Ok(())
        })
    }
}

impl FakeServices {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            transport: Arc::new(CapturingTransport::new()),
            downloader: Arc::new(QueuedDownloader::default()),
        })
    }

    fn downloads(&self) -> Vec<FakeDownloadCall> {
        self.downloader.calls()
    }

    /// Scripts the bytes the next download writes.
    fn push_download(&self, body: &[u8]) {
        self.downloader.push(body);
    }
}

impl UpdateServices for FakeServices {
    fn discovery_transport(&self) -> Arc<dyn Transport> {
        self.transport.clone()
    }

    fn downloader(&self) -> Arc<dyn Downloader> {
        self.downloader.clone()
    }
}

fn model_for(
    tag: &str,
    host: Option<UpdateHostInfo>,
    services: &Arc<FakeServices>,
) -> (AppModel, Arc<FakePlatform>) {
    let platform = Arc::new(FakePlatform::new());
    platform.set_update_host(host);
    let model = AppModel::with_updates(
        std::env::temp_dir().join(unique_token(tag)),
        platform.clone(),
        services.clone(),
    )
    .unwrap();
    (model, platform)
}

/// Discovery verifies with the compiled production key, whose private half is
/// not in this repository, so no scripted manifest can be genuinely signed here.
/// The model tests therefore drive every scripted discovery into
/// `Failed(Check)` — exactly the "network down / GitHub unreachable" failure of
/// REQ-UPD-13 — and check the containment; the signature itself is
/// `src/core/tests/update.rs`'s job.
const UNVERIFIABLE_SIGNATURE: &str = "not-a-signature";

const BASE: &str = "https://github.com/radaiko/SnackPilot/releases/download/desktop%2Fv2.3.0";

fn manifest_bytes(platform: &str, arch: &str, bytes: &[u8]) -> (String, String, String) {
    let artifact = format!("SnackPilot-2.3.0-{platform}-{arch}-setup");
    let json = format!(
        "{{\"architecture\":\"{arch}\",\"artifact\":\"{artifact}\",\"build\":\"42\",\
         \"minimum_os\":\"1.0.0\",\"notes\":\"Fehler behoben.\",\
         \"notes_url\":\"https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0\",\
         \"platform\":\"{platform}\",\"schema\":1,\"sha256\":\"{}\",\"size\":{},\
         \"tag\":\"desktop/v2.3.0\",\"version\":\"2.3.0\"}}",
        sha256_hex(bytes),
        bytes.len()
    );
    let name = format!("SnackPilot-manifest-{platform}-{arch}.json");
    (json, name, artifact)
}

/// Queues the three answers of a discovery that offers 2.3.0 for `platform`.
/// Returns the artifact URL the model must download.
fn queue_available(
    transport: &CapturingTransport,
    platform: &str,
    arch: &str,
    bytes: &[u8],
) -> String {
    let (manifest, name, artifact) = manifest_bytes(platform, arch, bytes);
    let sig = format!("{name}.sig");
    let release = format!(
        "[{{\"tag_name\":\"desktop/v2.3.0\",\"draft\":false,\"prerelease\":false,\"assets\":[\
          {{\"name\":\"{name}\",\"browser_download_url\":\"{BASE}/{name}\"}},\
          {{\"name\":\"{sig}\",\"browser_download_url\":\"{BASE}/{sig}\"}},\
          {{\"name\":\"{artifact}\",\"browser_download_url\":\"{BASE}/{artifact}\"}}]}}]"
    );
    transport.queue(release);
    transport.queue(manifest.clone());
    transport.queue(UNVERIFIABLE_SIGNATURE);
    format!("{BASE}/{artifact}")
}

/// Queues the three answers of a discovery that is up to date.
fn queue_up_to_date(transport: &CapturingTransport, platform: &str, arch: &str) {
    queue_available(transport, platform, arch, b"artifact");
}

fn wait_until(what: &str, predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}");
}

/// A verified update the fake platform answers with, inside `staging`.
fn verified_in(staging: &std::path::Path) -> VerifiedUpdate {
    VerifiedUpdate {
        version: "2.3.0".into(),
        artifact: staging.join("artifact"),
        prepared: None,
    }
}

/// The model runtime is one thread for the whole test binary, so the tests that
/// keep a request in flight for longer than a turn take this lock: otherwise a
/// pending request of one test would hold the guard while another waits.
static RUNTIME_TURN: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    RUNTIME_TURN.lock().unwrap_or_else(|e| e.into_inner())
}

/// What every scripted download writes, so its size and hash match.
const ARTIFACT_BYTES: &[u8] = b"artifact";

fn candidate(platform: &str) -> Candidate {
    let artifact = format!("SnackPilot-2.3.0-{platform}-x64-setup");
    Candidate {
        manifest: Manifest {
            schema: 1,
            platform: platform.into(),
            architecture: "x64".into(),
            version: "2.3.0".into(),
            build: "42".into(),
            minimum_os: "1.0.0".into(),
            artifact,
            size: ARTIFACT_BYTES.len() as u64,
            sha256: sha256_hex(ARTIFACT_BYTES),
            notes: "Fehler behoben.".into(),
            notes_url: "https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0".into(),
            tag: "desktop/v2.3.0".into(),
        },
        artifact_url: format!("{BASE}/artifact"),
    }
}

// ---- cadence (REQ-UPD-11) ----

// @req REQ-UPD-11
#[test]
fn launch_checks_once_when_switch_on() {
    let _serial = serial();
    let services = FakeServices::new();
    queue_up_to_date(&services.transport, "windows", "x64");
    let (model, _) = model_for(
        &unique_token("launch-on"),
        Some(windows_host("2.3.0", "41")),
        &services,
    );

    model.check_for_update(UpdateTrigger::Launch, now());
    wait_until("the check", || services.transport.answered() == 3);
    wait_until("the finished check", || {
        matches!(model.update_state(), UpdateState::Failed { .. })
    });
    assert_eq!(
        services.transport.request_count(),
        3,
        "exactly one discovery: {:?}",
        services.transport.urls()
    );
    let state = model.state();
    assert_eq!(state.error_text, None, "a check touches only State.update");
    assert!(state.location_alert.is_none());
    assert!(!state.busy);
}

// @req REQ-UPD-11
#[test]
fn launch_does_nothing_when_switch_off() {
    let _serial = serial();
    let services = FakeServices::new();
    queue_up_to_date(&services.transport, "windows", "x64");
    let (model, _) = model_for(
        &unique_token("launch-off"),
        Some(windows_host("2.3.0", "41")),
        &services,
    );
    model.set_auto_update_check(false);

    model.check_for_update(UpdateTrigger::Launch, now());
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(
        services.transport.request_count(),
        0,
        "switch off: no request"
    );
    assert_eq!(model.update_state(), UpdateState::Idle);
    assert_eq!(
        model.update_last_check(),
        None,
        "a refused request records nothing"
    );

    model.check_for_update(UpdateTrigger::Manual, now());
    wait_until("the manual check", || services.transport.answered() == 3);
    assert_eq!(services.transport.request_count(), 3);
}

// @req REQ-UPD-11
#[test]
fn periodic_tick_waits_24_hours() {
    let _serial = serial();
    let services = FakeServices::new();
    queue_up_to_date(&services.transport, "windows", "x64");
    let (model, _) = model_for(
        &unique_token("periodic"),
        Some(windows_host("2.3.0", "41")),
        &services,
    );

    let start = now();
    model.check_for_update(UpdateTrigger::Launch, start);
    wait_until("the launch check", || services.transport.answered() == 3);
    assert_eq!(model.update_last_check(), Some(start));

    model.check_for_update(
        UpdateTrigger::Periodic,
        start + 23 * 60 * 60 * 1000 + 59 * 60 * 1000,
    );
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(
        services.transport.request_count(),
        3,
        "23 h 59 min is too early"
    );

    queue_up_to_date(&services.transport, "windows", "x64");
    model.check_for_update(UpdateTrigger::Periodic, start + 24 * 60 * 60 * 1000);
    wait_until("the daily check", || {
        services.transport.request_count() == 6
    });
    assert_eq!(model.update_last_check(), Some(start + 24 * 60 * 60 * 1000));
}

// @req REQ-UPD-12
#[test]
fn manual_check_ignores_switch_and_clears_skip() {
    let _serial = serial();
    use snackpilot_core::storage::{FileKv, Kv};
    let services = FakeServices::new();
    let dir = std::env::temp_dir().join(unique_token("manual"));
    let platform = Arc::new(FakePlatform::new());
    platform.set_update_host(Some(windows_host("2.2.0", "41")));
    let model = AppModel::with_updates(dir.clone(), platform, services.clone()).unwrap();
    model.set_auto_update_check(false);
    FileKv::new(dir.clone())
        .set("update_skipped_version", "2.3.0")
        .unwrap();

    queue_up_to_date(&services.transport, "windows", "x64");
    model.check_for_update(UpdateTrigger::Launch, now());
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(services.transport.request_count(), 0, "switch off");

    model.check_for_update(UpdateTrigger::Manual, now());
    wait_until("the manual check", || services.transport.answered() == 3);
    assert_eq!(
        FileKv::new(dir).get("update_skipped_version").unwrap(),
        None,
        "a manual check clears the skip"
    );
    assert_eq!(model.skipped_version(), None);
}

// @req REQ-UPD-10
#[test]
fn periodic_check_while_ready_makes_no_core_call() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, fake) = model_for(
        &unique_token("ready-gate"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    let staging = temp_dir(&unique_token("ready-gate-staging"));
    fake.set_update_staging(Some(staging.clone()));
    let verified = verified_in(&staging);
    fake.set_update_verified(Some(verified.clone()));
    services.push_download(ARTIFACT_BYTES);
    model.set_update_state(UpdateState::Available {
        candidate: candidate("windows"),
    });
    model.start_update_download();
    wait_until("the ready update", || {
        matches!(model.update_state(), UpdateState::Ready { .. })
    });
    let requests = services.transport.request_count();

    model.check_for_update(UpdateTrigger::Periodic, now() + 48 * 60 * 60 * 1000);
    model.check_for_update(UpdateTrigger::Manual, now() + 48 * 60 * 60 * 1000);
    model.check_for_update(UpdateTrigger::Launch, now() + 48 * 60 * 60 * 1000);
    std::thread::sleep(Duration::from_millis(150));

    assert_eq!(
        services.transport.request_count(),
        requests,
        "no check runs while a verified update waits for its handoff"
    );
    assert!(
        matches!(model.update_state(), UpdateState::Ready { .. }),
        "the verified update and its files stay: {:?}",
        model.update_state()
    );
    let view = model.update_view();
    assert_eq!(
        view.state_line,
        "Version 2.3.0 ist bereit zur Installation. SnackPilot wird dafür beendet."
    );
    assert_eq!(view.actions, vec![UpdateAction::InstallAndQuit]);
    assert_eq!(services.downloads().len(), 1, "no second download either");

    // The handoff is still the way out, over the files verified before.
    fake.set_update_handoff(HandoffOutcome::QuitRequested);
    model.hand_off_update();
    wait_until("the handoff", || {
        matches!(model.update_state(), UpdateState::HandedOff { .. })
    });
    assert_eq!(fake.handed_off(), vec![verified]);
}

// ---- one operation at a time (REQ-UPD-10) ----

// @req REQ-UPD-10
#[test]
fn second_request_while_running_makes_no_core_call() {
    let _serial = serial();
    let services = FakeServices::new();
    let signal = services.transport.queue_pending();
    let (model, _) = model_for(
        &unique_token("guard"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    let model = Arc::new(model);

    model.check_for_update(UpdateTrigger::Launch, now());
    wait_until("the check to start", || {
        matches!(model.update_state(), UpdateState::Checking)
    });
    assert_eq!(services.transport.request_count(), 1);

    std::thread::scope(|s| {
        for _ in 0..4 {
            let model = model.clone();
            s.spawn(move || {
                model.check_for_update(UpdateTrigger::Manual, now());
                model.check_for_update(UpdateTrigger::Periodic, now());
                model.start_update_download();
                model.hand_off_update();
            });
        }
    });
    assert_eq!(
        services.transport.request_count(),
        1,
        "a second request while running makes no core call"
    );
    assert!(services.downloads().is_empty());

    signal.send();
    wait_until("the finished check", || {
        matches!(model.update_state(), UpdateState::Failed { .. })
    });
    assert_eq!(
        services.transport.answered(),
        1,
        "only the one check answered"
    );
}

// ---- download and verify (REQ-UPD-12, REQ-UPD-16) ----

// @req REQ-UPD-12
#[test]
fn available_does_not_download() {
    let _serial = serial();
    let services = FakeServices::new();
    queue_available(&services.transport, "windows", "x64", b"installer");
    let (model, platform) = model_for(
        &unique_token("no-download"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );

    model.check_for_update(UpdateTrigger::Launch, now());
    wait_until("the finished check", || {
        matches!(model.update_state(), UpdateState::Failed { .. })
    });
    assert!(services.downloads().is_empty(), "a check never downloads");
    assert!(platform.staged_versions().is_empty());
    assert!(
        model.update_view().notes.is_none(),
        "no notes without an offer"
    );

    // The offer's shape itself, from the state the machine reaches on a real
    // download: Available is what the detail shows before "Herunterladen".
    let offered = view_with(
        UpdateState::Available {
            candidate: candidate("windows"),
        },
        windows_host("2.2.0", "41"),
    );
    assert_eq!(offered.state_line, "Version 2.3.0 ist verfügbar.");
    assert_eq!(offered.notes.as_deref(), Some("Fehler behoben."));
    assert_eq!(
        offered.notes_heading.as_deref(),
        Some("Neu in Version 2.3.0")
    );
    assert_eq!(
        offered.notes_url.as_deref(),
        Some("https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0")
    );
    assert_eq!(
        offered.actions,
        vec![
            UpdateAction::Download,
            UpdateAction::Skip,
            UpdateAction::Check
        ]
    );
}

// @req REQ-UPD-16
#[test]
fn available_then_download_then_ready_on_windows_target() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, platform) = model_for(
        &unique_token("win-flow"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    model.set_update_state(UpdateState::Available {
        candidate: candidate("windows"),
    });
    let staging = temp_dir(&unique_token("win-staging"));
    platform.set_update_staging(Some(staging.clone()));
    platform.set_update_verified(Some(verified_in(&staging)));
    services.push_download(ARTIFACT_BYTES);

    model.start_update_download();
    wait_until("the ready update", || {
        matches!(model.update_state(), UpdateState::Ready { .. })
    });
    assert_eq!(platform.staged_versions(), vec!["2.3.0".to_string()]);
    assert_eq!(services.downloads().len(), 1);
    assert_eq!(
        services.downloads()[0].url,
        candidate("windows").artifact_url
    );
    let view = model.update_view();
    assert_eq!(
        view.state_line,
        "Version 2.3.0 ist bereit zur Installation. SnackPilot wird dafür beendet."
    );
    assert_eq!(view.actions, vec![UpdateAction::InstallAndQuit]);
    assert_eq!(view.progress, None);
}

// @req REQ-UPD-16
#[test]
fn macos_target_verifies_before_ready() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, platform) = model_for(
        &unique_token("mac-flow"),
        Some(macos_host("2.2.0", "41")),
        &services,
    );
    model.set_update_state(UpdateState::Available {
        candidate: candidate("macos"),
    });
    let staging = temp_dir(&unique_token("mac-staging"));
    platform.set_update_staging(Some(staging.clone()));
    platform.set_update_verified(Some(VerifiedUpdate {
        version: "2.3.0".into(),
        artifact: staging.join("SnackPilot-2.3.0-macos-arm64.zip"),
        prepared: Some(staging.join("extracted/SnackPilot.app")),
    }));
    services.push_download(ARTIFACT_BYTES);

    model.start_update_download();
    wait_until("the ready update", || {
        matches!(model.update_state(), UpdateState::Ready { .. })
    });
    let calls = platform.calls();
    let download = calls
        .iter()
        .position(|c| c.starts_with("update_staging_dir("))
        .expect("a staging directory was asked for");
    let verify = calls
        .iter()
        .position(|c| c.starts_with("verify_update_artifact("))
        .expect("macOS verifies between hash success and Ready");
    assert!(
        download < verify,
        "verify runs after the download: {calls:?}"
    );
    let view = model.update_view();
    assert_eq!(
        view.state_line,
        "Version 2.3.0 ist geprüft. Zum Installieren ziehst du die App in den Ordner „Programme“."
    );
    assert_eq!(view.actions, vec![UpdateAction::RevealInFinder]);
}

// @req REQ-UPD-07
#[test]
fn verify_failure_discards_and_reports_nothing_installed() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, platform) = model_for(
        &unique_token("verify-fail"),
        Some(macos_host("2.2.0", "41")),
        &services,
    );
    model.set_update_state(UpdateState::Available {
        candidate: candidate("macos"),
    });
    platform.set_update_staging(Some(temp_dir(&unique_token("verify-fail-staging"))));
    platform.set_update_verified(None);
    services.push_download(ARTIFACT_BYTES);

    model.start_update_download();
    wait_until("the failed verification", || {
        matches!(
            model.update_state(),
            UpdateState::Failed {
                stage: UpdateFailure::Verify,
                ..
            }
        )
    });
    assert!(
        platform.handed_off().is_empty(),
        "nothing becomes installable"
    );
    let view = model.update_view();
    assert_eq!(
        view.state_line,
        "Das Update konnte nicht überprüft werden und wurde verworfen. Es wurde nichts installiert."
    );
    assert_eq!(view.actions, vec![UpdateAction::Check]);
}

// ---- skip (REQ-UPD-12) ----

// @req REQ-UPD-12
#[test]
fn skip_hides_version_from_automatic_checks_only() {
    let _serial = serial();
    use snackpilot_core::storage::{FileKv, Kv};
    let services = FakeServices::new();
    let dir = std::env::temp_dir().join(unique_token("skip"));
    let platform = Arc::new(FakePlatform::new());
    platform.set_update_host(Some(windows_host("2.2.0", "41")));
    let model = AppModel::with_updates(dir.clone(), platform, services).unwrap();
    model.set_update_state(UpdateState::Available {
        candidate: candidate("windows"),
    });
    model.skip_update();
    assert_eq!(model.skipped_version().as_deref(), Some("2.3.0"));
    assert_eq!(
        FileKv::new(dir.clone())
            .get("update_skipped_version")
            .unwrap()
            .as_deref(),
        Some("2.3.0")
    );
    let view = model.update_view();
    assert_eq!(
        view.state_line,
        "Version 2.3.0 wird übersprungen. „Jetzt prüfen“ zeigt sie wieder an."
    );
    assert_eq!(view.actions, vec![UpdateAction::Check]);
    assert_eq!(view.notes, None, "a skipped version is not offered");

    // Only a manual check clears it; an automatic check keeps it.
    model.check_for_update(UpdateTrigger::Launch, now());
    assert_eq!(model.skipped_version().as_deref(), Some("2.3.0"));
}

// ---- gates (REQ-UPD-13) ----

// @req REQ-UPD-13
#[test]
fn unsupported_host_and_evidence_mode_make_no_request() {
    let _serial = serial();
    let services = FakeServices::new();
    let (mobile, mobile_platform) = model_for(&unique_token("mobile"), None, &services);
    assert!(!mobile.update_supported());
    mobile.check_for_update(UpdateTrigger::Launch, now());
    mobile.check_for_update(UpdateTrigger::Manual, now());
    mobile.start_update_download();
    mobile.skip_update();
    mobile.hand_off_update();
    mobile.set_auto_update_check(false);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(services.transport.request_count(), 0);
    assert!(services.downloads().is_empty());
    assert!(mobile_platform.staged_versions().is_empty());
    assert_eq!(mobile.update_state(), UpdateState::Idle);
    assert_eq!(mobile.update_view().state_line, "Noch nicht geprüft.");
    assert!(
        mobile.update_view().installed.is_empty(),
        "no Updates entry"
    );

    let (evidence, _) = model_for(
        &unique_token("evidence"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    evidence.set_evidence_mode(true);
    assert!(!evidence.update_supported());
    evidence.check_for_update(UpdateTrigger::Manual, now());
    evidence.start_update_download();
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(services.transport.request_count(), 0);
    assert_eq!(evidence.update_state(), UpdateState::Idle);
}

// @req REQ-UPD-13
#[test]
fn failed_automatic_check_is_state_only() {
    let _serial = serial();
    let services = FakeServices::new();
    services
        .transport
        .replies
        .lock()
        .unwrap()
        .push_back(Reply::Fail);
    let (model, _) = model_for(
        &unique_token("offline"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );

    model.check_for_update(UpdateTrigger::Launch, now());
    wait_until("the failed check", || {
        matches!(
            model.update_state(),
            UpdateState::Failed {
                stage: UpdateFailure::Check,
                ..
            }
        )
    });
    let state = model.state();
    assert_eq!(state.error_text, None, "no banner for an automatic failure");
    assert!(state.location_alert.is_none(), "no alert");
    assert!(!state.busy);
    assert_eq!(
        model.update_view().state_line,
        "Die Suche nach Updates ist fehlgeschlagen. Keine Verbindung zu GitHub."
    );
    // The attempt is recorded, so a failing network is not retried in a loop.
    assert!(model.update_last_check().is_some());
    // Other model flows are untouched by the update failure.
    assert_eq!(model.state().launch_stage, state.launch_stage);
}

// ---- handoff (REQ-UPD-07) ----

// @req REQ-UPD-07
#[test]
fn handoff_quit_and_reveal_never_claim_installed() {
    let _serial = serial();
    for (name, host, outcome) in [
        (
            "win",
            windows_host("2.2.0", "41"),
            HandoffOutcome::QuitRequested,
        ),
        ("mac", macos_host("2.2.0", "41"), HandoffOutcome::Revealed),
    ] {
        let services = FakeServices::new();
        let (model, fake) = model_for(&unique_token(name), Some(host), &services);
        let staging = temp_dir(&unique_token(&format!("{name}-handoff")));
        fake.set_update_staging(Some(staging.clone()));
        fake.set_update_verified(Some(verified_in(&staging)));
        fake.set_update_handoff(outcome);
        services.push_download(ARTIFACT_BYTES);
        model.set_update_state(UpdateState::Available {
            candidate: candidate(if name == "win" { "windows" } else { "macos" }),
        });
        model.start_update_download();
        wait_until("the ready update", || {
            matches!(model.update_state(), UpdateState::Ready { .. })
        });

        model.hand_off_update();
        wait_until("the handoff", || {
            matches!(model.update_state(), UpdateState::HandedOff { .. })
        });
        assert_eq!(fake.handed_off().len(), 1);
        let view = model.update_view();
        println!("{name} -> {}", view.state_line);
        assert!(
            !view.state_line.contains("wurde installiert")
                && !view.state_line.contains("ist installiert"),
            "{name} claims an installation: {}",
            view.state_line
        );
        match outcome {
            HandoffOutcome::Revealed => {
                assert_eq!(
                    view.state_line,
                    "Im Finder angezeigt. Ziehe SnackPilot in „Programme“ und starte die App neu. \
                     Das Update ist noch nicht installiert."
                );
                assert_eq!(
                    view.actions,
                    vec![UpdateAction::RevealInFinder, UpdateAction::Check]
                );
            }
            HandoffOutcome::QuitRequested => {
                assert!(view.state_line.contains("beendet sich"));
                assert_eq!(view.actions, vec![UpdateAction::InstallAndQuit]);
            }
        }
    }
}

// @req REQ-UPD-07
#[test]
fn handoff_failure_keeps_verified_files() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, fake) = model_for(
        &unique_token("handoff-fail"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    let staging = temp_dir(&unique_token("handoff-fail-staging"));
    fake.set_update_staging(Some(staging.clone()));
    let verified = verified_in(&staging);
    services.push_download(ARTIFACT_BYTES);
    fake.set_update_verified(Some(verified.clone()));
    model.set_update_state(UpdateState::Available {
        candidate: candidate("windows"),
    });
    model.start_update_download();
    wait_until("the ready update", || {
        matches!(model.update_state(), UpdateState::Ready { .. })
    });

    fake.fail_next("hand_off_update", 1);
    model.hand_off_update();
    wait_until("the failed handoff", || {
        matches!(
            model.update_state(),
            UpdateState::Failed {
                stage: UpdateFailure::Handoff,
                ..
            }
        )
    });
    let view = model.update_view();
    assert!(view
        .state_line
        .starts_with("Die Installation konnte nicht gestartet werden."));
    assert_eq!(view.actions, vec![UpdateAction::InstallAndQuit]);

    // The retry hands off the same verified files: no second download.
    fake.set_update_handoff(HandoffOutcome::QuitRequested);
    model.hand_off_update();
    wait_until("the retried handoff", || {
        matches!(model.update_state(), UpdateState::HandedOff { .. })
    });
    assert_eq!(fake.handed_off(), vec![verified]);
    assert_eq!(services.downloads().len(), 1);
}

// ---- persistence (REQ-UPD-11) ----

// @req REQ-UPD-11
#[test]
fn switch_and_last_check_persist_across_models() {
    let _serial = serial();
    use snackpilot_core::storage::{FileKv, Kv};
    let services = FakeServices::new();
    let dir = std::env::temp_dir().join(unique_token("persist"));
    let platform = Arc::new(FakePlatform::new());
    platform.set_update_host(Some(windows_host("2.3.0", "41")));
    let model = AppModel::with_updates(dir.clone(), platform.clone(), services.clone()).unwrap();
    model.set_auto_update_check(false);
    assert!(!model.auto_update_check());
    queue_up_to_date(&services.transport, "windows", "x64");
    model.check_for_update(UpdateTrigger::Launch, CHECKED_AT);
    assert_eq!(services.transport.request_count(), 0, "switch off");

    model.set_auto_update_check(true);
    queue_up_to_date(&services.transport, "windows", "x64");
    model.check_for_update(UpdateTrigger::Launch, CHECKED_AT);
    wait_until("the check", || services.transport.request_count() == 3);

    let kv = FileKv::new(dir.clone());
    assert_eq!(
        kv.get("update_auto_check_enabled").unwrap().as_deref(),
        Some("true")
    );
    assert_eq!(
        kv.get("update_last_check_ms").unwrap().as_deref(),
        Some(CHECKED_AT.to_string().as_str())
    );

    drop(model);
    let again = AppModel::with_updates(dir, platform, services).unwrap();
    assert!(again.auto_update_check(), "the stored switch");
    assert_eq!(again.update_last_check(), Some(CHECKED_AT));
    assert_eq!(
        again.update_state(),
        UpdateState::Idle,
        "nothing ran or restored"
    );
}

// ---- copy table (REQ-UPD-15) ----

fn view_with(state: UpdateState, host: UpdateHostInfo) -> UpdateView {
    let services = FakeServices::new();
    let (model, _) = model_for(&unique_token("copy"), Some(host), &services);
    model.set_update_state(state);
    model.update_view_at(CHECKED_AT)
}

// @req REQ-UPD-15
#[test]
fn update_view_copy_matches_design_table() {
    let date = format_date_long(CHECKED_AT);
    let time = format_time(CHECKED_AT);
    assert_eq!(time.len(), 5, "HH:mm");
    let up_to_date =
        format!("SnackPilot ist auf dem neuesten Stand. Zuletzt geprüft: {date}, {time}.");

    let rows: Vec<(UpdateState, String, Vec<UpdateAction>, UpdateHostInfo)> = vec![
        (
            UpdateState::Idle,
            "Noch nicht geprüft.".into(),
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Checking,
            "Suche nach Updates …".into(),
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::UpToDate {
                checked_ms: CHECKED_AT,
            },
            up_to_date,
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Available {
                candidate: candidate("windows"),
            },
            "Version 2.3.0 ist verfügbar.".into(),
            vec![
                UpdateAction::Download,
                UpdateAction::Skip,
                UpdateAction::Check,
            ],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Skipped {
                version: "2.3.0".into(),
            },
            "Version 2.3.0 wird übersprungen. „Jetzt prüfen“ zeigt sie wieder an.".into(),
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Downloading {
                candidate: candidate("windows"),
                done: 41,
                total: Some(100),
            },
            "Version 2.3.0 wird geladen … 41 %".into(),
            vec![],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Downloading {
                candidate: candidate("windows"),
                done: 41,
                total: None,
            },
            "Version 2.3.0 wird geladen …".into(),
            vec![],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Verifying {
                candidate: candidate("windows"),
            },
            "Download wird überprüft …".into(),
            vec![],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Ready {
                update: verified_in(std::path::Path::new("/tmp/x")),
                handoff: HandoffOutcome::QuitRequested,
            },
            "Version 2.3.0 ist bereit zur Installation. SnackPilot wird dafür beendet.".into(),
            vec![UpdateAction::InstallAndQuit],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Ready {
                update: verified_in(std::path::Path::new("/tmp/x")),
                handoff: HandoffOutcome::Revealed,
            },
            "Version 2.3.0 ist geprüft. Zum Installieren ziehst du die App in den Ordner \
             „Programme“."
                .into(),
            vec![UpdateAction::RevealInFinder],
            macos_host("2.2.0", "41"),
        ),
        (
            UpdateState::HandedOff {
                version: "2.3.0".into(),
                outcome: HandoffOutcome::Revealed,
                update: verified_in(std::path::Path::new("/tmp/x")),
            },
            "Im Finder angezeigt. Ziehe SnackPilot in „Programme“ und starte die App neu. \
             Das Update ist noch nicht installiert."
                .into(),
            vec![UpdateAction::RevealInFinder, UpdateAction::Check],
            macos_host("2.2.0", "41"),
        ),
        (
            UpdateState::Failed {
                stage: UpdateFailure::Check,
                reason: "Keine Verbindung zu GitHub.".into(),
                retry: None,
            },
            "Die Suche nach Updates ist fehlgeschlagen. Keine Verbindung zu GitHub.".into(),
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Failed {
                stage: UpdateFailure::Download,
                reason: "Das Update hat eine unerwartete Größe.".into(),
                retry: None,
            },
            "Der Download ist fehlgeschlagen. Das Update hat eine unerwartete Größe.".into(),
            vec![UpdateAction::Download],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Failed {
                stage: UpdateFailure::Verify,
                reason: String::new(),
                retry: None,
            },
            "Das Update konnte nicht überprüft werden und wurde verworfen. Es wurde nichts \
             installiert."
                .into(),
            vec![UpdateAction::Check],
            windows_host("2.2.0", "41"),
        ),
        (
            UpdateState::Failed {
                stage: UpdateFailure::Handoff,
                reason: "Der Installer konnte nicht gestartet werden.".into(),
                retry: Some(snackpilot_model::Retry::Handoff(
                    verified_in(std::path::Path::new("/tmp/x")),
                    HandoffOutcome::QuitRequested,
                )),
            },
            "Die Installation konnte nicht gestartet werden. Der Installer konnte nicht \
             gestartet werden."
                .into(),
            vec![UpdateAction::InstallAndQuit],
            windows_host("2.2.0", "41"),
        ),
    ];

    for (state, line, actions, host) in rows {
        let view = view_with(state.clone(), host);
        println!("update_view {:?} -> {:?}", state, view.state_line);
        assert_eq!(view.state_line, line, "state line of {state:?}");
        assert!(
            !view.state_line.trim().is_empty(),
            "empty line for {state:?}"
        );
        assert_eq!(view.actions, actions, "actions of {state:?}");
        assert_eq!(view.installed, "Version 2.2.0 (41)");
    }

    // Every reachable state has a line and the switch is part of the view.
    let idle = view_with(UpdateState::Idle, windows_host("2.2.0", "41"));
    assert!(idle.switch_on, "the switch defaults on");
}

// @req REQ-UPD-15
#[test]
fn row_hint_reports_available_version() {
    let services = FakeServices::new();
    let (model, _) = model_for(
        &unique_token("row-hint"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    assert_eq!(model.update_view().row_hint, "Version 2.2.0 (41)");
    model.set_update_state(UpdateState::Available {
        candidate: candidate("windows"),
    });
    assert_eq!(model.update_view().row_hint, "Version 2.3.0 verfügbar");
}

// @req REQ-UPD-13
#[test]
fn new_model_makes_no_update_request() {
    let _serial = serial();
    let services = FakeServices::new();
    let (model, platform) = model_for(
        &unique_token("new-model"),
        Some(windows_host("2.2.0", "41")),
        &services,
    );
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(services.transport.request_count(), 0);
    assert!(services.downloads().is_empty());
    assert_eq!(model.update_state(), UpdateState::Idle);
    assert!(
        !platform.calls().iter().any(|c| c.starts_with("update_")),
        "no update hook runs at construction: {:?}",
        platform.calls()
    );
}
