//! Desktop updater: the state machine, the launch/daily cadence, the gates and
//! the German copy projection (design D5/D7; REQ-UPD-07, 10-13, 15, 16).
//!
//! The model never does HTTP itself: it calls `snackpilot_core::update` with
//! injected services and spawns the work on the model runtime, never
//! `block_on` from the caller. Every failure ends in a state or a German line;
//! nothing the user owns is touched and other flows keep running.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use snackpilot_core::http::Transport;
use snackpilot_core::storage::Kv;
use snackpilot_core::update::{
    discover, download_verified, Candidate, Discovery, Downloader, HostTarget, Manifest,
    UpdateError, UPDATE_PUBLIC_KEY,
};

use crate::platform::{HandoffOutcome, PlatformError, UpdateHostInfo, VerifiedUpdate};
use crate::runtime::handle;
use crate::session::AppModel;
use crate::state::StateHub;

/// Stored switch, `"true"`/`"false"`; absent means on.
pub const AUTO_CHECK_KEY: &str = "update_auto_check_enabled";
/// Epoch millis of the last check attempt, written even when it failed.
pub const LAST_CHECK_KEY: &str = "update_last_check_ms";
/// The version the user skipped; automatic checks hide it, a manual clears it.
pub const SKIPPED_VERSION_KEY: &str = "update_skipped_version";

/// Runtime between two automatic checks (REQ-UPD-11).
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

/// The signed manifest the host verifies against (REQ-UPD-05/06).
pub type VerifiedManifest = Manifest;

/// Who asked for a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateTrigger {
    Launch,
    Periodic,
    Manual,
}

/// Which step failed, so the copy can name it (design D7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateFailure {
    Check,
    Download,
    Verify,
    Handoff,
}

/// The single state machine (design D5, REQ-UPD-10). `Verifying` is the step
/// between a finished download and `Ready`, so the host's verification runs off
/// the UI thread like every other step.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate {
        checked_ms: i64,
    },
    Available {
        candidate: Candidate,
    },
    Skipped {
        version: String,
    },
    Downloading {
        candidate: Candidate,
        done: u64,
        total: Option<u64>,
    },
    Verifying {
        candidate: Candidate,
    },
    /// Verified and ready for the platform handoff, which `handoff` names.
    Ready {
        update: VerifiedUpdate,
        handoff: HandoffOutcome,
    },
    HandedOff {
        version: String,
        outcome: HandoffOutcome,
        /// The verified files stay, so a retry needs no new download.
        update: VerifiedUpdate,
    },
    Failed {
        stage: UpdateFailure,
        /// German, from [`UpdateError::user_message`] or a fixed platform text.
        reason: String,
        /// What this state's own action retries; `None` retries with a check.
        retry: Option<Retry>,
    },
}

/// What the action of a failed state repeats.
#[derive(Debug, Clone, PartialEq)]
pub enum Retry {
    /// Boxed: a candidate is the manifest plus its URL, far larger than the
    /// handoff case below.
    Download(Box<Candidate>),
    Handoff(VerifiedUpdate, HandoffOutcome),
}

/// The services the updater needs besides the platform, injected so a host
/// without them never gets closer to the network than an `Unsupported` state.
pub trait UpdateServices: Send + Sync {
    /// One anonymous transport for release discovery.
    fn discovery_transport(&self) -> Arc<dyn Transport>;
    /// The artifact downloader.
    fn downloader(&self) -> Arc<dyn Downloader>;
}

/// Production services: core's own reqwest transport (cookie store off) and its
/// streaming `ReqwestDownloader`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProductionUpdateServices;

impl UpdateServices for ProductionUpdateServices {
    fn discovery_transport(&self) -> Arc<dyn Transport> {
        Arc::new(
            snackpilot_core::http::reqwest_transport::ReqwestTransport::new(false, true)
                .expect("static reqwest configuration cannot fail"),
        )
    }

    fn downloader(&self) -> Arc<dyn Downloader> {
        Arc::new(snackpilot_core::update::ReqwestDownloader::new())
    }
}

/// One button of the action row; rendered in this order, primary first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    Check,
    Download,
    Skip,
    InstallAndQuit,
    RevealInFinder,
}

impl UpdateAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Check => "Jetzt prüfen",
            Self::Download => "Herunterladen",
            Self::Skip => "Diese Version überspringen",
            Self::InstallAndQuit => "Installieren und beenden",
            Self::RevealInFinder => "Im Finder zeigen",
        }
    }
}

/// Everything the Updates detail renders, copy included (design D7).
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateView {
    /// `Version 2.2.0 (41)`.
    pub installed: String,
    pub state_line: String,
    /// Release notes of an offered update, shown before any download.
    pub notes: Option<String>,
    pub notes_url: Option<String>,
    /// The notes heading, `Neu in Version 2.3.0`.
    pub notes_heading: Option<String>,
    pub switch_on: bool,
    pub actions: Vec<UpdateAction>,
    /// Download progress, 0.0–100.0.
    pub progress: Option<f32>,
    /// The Einstellungen row's hint.
    pub row_hint: String,
}

impl UpdateView {
    /// Every action is disabled while an operation runs (REQ-UPD-16).
    pub fn actions_disabled(&self) -> bool {
        self.progress.is_some() || self.state_line == "Download wird überprüft …"
    }
}

impl AppModel {
    /// Whether the updater is live here (REQ-UPD-13): a supported desktop host
    /// and not evidence mode. Else every entry point is a no-op.
    pub fn update_supported(&self) -> bool {
        !self.evidence_mode() && self.platform.update_target().is_some()
    }

    /// The state machine's snapshot (`State.update`).
    pub fn update_state(&self) -> UpdateState {
        self.hub.get().update
    }

    /// The switch; absent means on (REQ-UPD-11).
    pub fn auto_update_check(&self) -> bool {
        self.kv
            .get(AUTO_CHECK_KEY)
            .ok()
            .flatten()
            .map(|v| v != "false")
            .unwrap_or(true)
    }

    /// Persists the switch.
    pub fn set_auto_update_check(&self, enabled: bool) {
        if !self.update_supported() {
            return;
        }
        let _ = self
            .kv
            .set(AUTO_CHECK_KEY, if enabled { "true" } else { "false" });
        self.hub.update("update_switch", |_| {});
    }

    /// The cadence rule of REQ-UPD-11: `Launch` checks while the switch is on,
    /// `Periodic` only 24 h after the last attempt, `Manual` always and it
    /// clears the skip. Gates and the guard come first, so a refused request
    /// makes no core call and changes no state.
    pub fn check_for_update(&self, trigger: UpdateTrigger, now_ms: i64) {
        // A verified update outranks a new check (design D5): `Ready` keeps the
        // files and its handoff through launch, periodic and manual checks; the
        // handoff's own action or a fresh download is the way out of it.
        if !self.update_supported()
            || self.update_op_running()
            || matches!(self.update_state(), UpdateState::Ready { .. })
        {
            return;
        }
        if trigger == UpdateTrigger::Manual {
            let _ = self.kv.remove(SKIPPED_VERSION_KEY);
        }
        if trigger != UpdateTrigger::Manual && !self.auto_update_check() {
            return;
        }
        if trigger == UpdateTrigger::Periodic && !self.update_due(now_ms) {
            return;
        }
        let Some(host) = self.platform.update_target() else {
            return;
        };
        let Some(target) = update_target(&host) else {
            return;
        };
        if !self.update_begin() {
            return;
        }
        let skipped = self.update_skipped();
        let _ = self.kv.set(LAST_CHECK_KEY, &now_ms.to_string());
        let transport = self.update_services.discovery_transport();
        let installed = host.installed_version;
        let hub = self.hub.clone();
        let guard = self.update_op.clone();
        // The state flips only once the request is on its way, so `Checking`
        // always means a request was actually made.
        handle().spawn(async move {
            hub.update("update_checking", |s| {
                s.update = UpdateState::Checking;
            });
            let result =
                discover(transport.as_ref(), &target, &installed, &UPDATE_PUBLIC_KEY).await;
            hub.update("update_checked", |s| {
                s.update = checked_state(result, now_ms, skipped.as_deref(), trigger);
            });
            guard.store(false, Ordering::SeqCst);
        });
    }

    /// Downloads the offered version; only an explicit user action gets here
    /// (REQ-UPD-12).
    pub fn start_update_download(&self) {
        if !self.update_supported() || self.update_op_running() {
            return;
        }
        let candidate = match self.update_state() {
            UpdateState::Available { candidate } => Some(candidate),
            UpdateState::Failed {
                stage: UpdateFailure::Download,
                retry: Some(Retry::Download(candidate)),
                ..
            } => Some(*candidate),
            _ => None,
        };
        let Some(candidate) = candidate else {
            return;
        };
        let staging = match self
            .platform
            .update_staging_dir(&candidate.manifest.version)
        {
            Ok(dir) => dir,
            Err(_) => {
                self.hub.update("update_staging_failed", |s| {
                    s.update = UpdateState::Failed {
                        stage: UpdateFailure::Download,
                        reason: UpdateError::Io.user_message().to_string(),
                        retry: Some(Retry::Download(Box::new(candidate))),
                    };
                });
                return;
            }
        };
        if !self.update_begin() {
            return;
        }
        let downloader = self.update_services.downloader();
        let platform = self.platform.clone();
        let hub = self.hub.clone();
        let guard = self.update_op.clone();
        handle().spawn(async move {
            hub.update("update_downloading", |s| {
                s.update = UpdateState::Downloading {
                    candidate: candidate.clone(),
                    done: 0,
                    total: None,
                };
            });
            let progress = progress_reporter(hub.clone(), candidate.clone());
            let staged =
                download_verified(downloader.as_ref(), &candidate, &staging, &progress).await;
            let artifact = match staged {
                Ok(artifact) => artifact,
                Err(e) => {
                    hub.update("update_download_failed", |s| {
                        s.update = UpdateState::Failed {
                            stage: UpdateFailure::Download,
                            reason: e.user_message().to_string(),
                            retry: Some(Retry::Download(Box::new(candidate))),
                        };
                    });
                    guard.store(false, Ordering::SeqCst);
                    return;
                }
            };
            hub.update("update_verifying", |s| {
                s.update = UpdateState::Verifying {
                    candidate: candidate.clone(),
                };
            });
            let manifest = candidate.manifest.clone();
            let verified = tokio::task::spawn_blocking(move || {
                platform.verify_update_artifact(&artifact, &manifest)
            })
            .await
            .unwrap_or_else(|_| Err(PlatformError::System("verify".into())));
            hub.update("update_verified", |s| {
                s.update = verified_state(verified, candidate);
            });
            guard.store(false, Ordering::SeqCst);
        });
    }

    /// Persists the skipped version; only automatic checks hide it (REQ-UPD-12).
    pub fn skip_update(&self) {
        if !self.update_supported() {
            return;
        }
        let UpdateState::Available { candidate } = self.update_state() else {
            return;
        };
        let version = candidate.manifest.version;
        let _ = self.kv.set(SKIPPED_VERSION_KEY, &version);
        self.hub.update("update_skipped", |s| {
            s.update = UpdateState::Skipped { version };
        });
    }

    /// Hands a verified update to the platform (REQ-UPD-07). A failure keeps the
    /// verified files and a retry; no copy ever claims an installation.
    pub fn hand_off_update(&self) {
        if !self.update_supported() || self.update_op_running() {
            return;
        }
        let (update, outcome) = match self.update_state() {
            UpdateState::Ready { update, handoff } => (update, handoff),
            UpdateState::HandedOff {
                update, outcome, ..
            } => (update, outcome),
            UpdateState::Failed {
                stage: UpdateFailure::Handoff,
                retry: Some(Retry::Handoff(update, outcome)),
                ..
            } => (update, outcome),
            _ => return,
        };
        if !self.update_begin() {
            return;
        }
        let platform = self.platform.clone();
        let hub = self.hub.clone();
        let guard = self.update_op.clone();
        handle().spawn(async move {
            let task = tokio::task::spawn_blocking({
                let update = update.clone();
                move || platform.hand_off_update(&update)
            });
            let result = task
                .await
                .unwrap_or_else(|_| Err(PlatformError::System("handoff".into())));
            hub.update("update_handoff", |s| {
                s.update = match result {
                    Ok(outcome) => UpdateState::HandedOff {
                        version: update.version.clone(),
                        outcome,
                        update,
                    },
                    Err(_) => UpdateState::Failed {
                        stage: UpdateFailure::Handoff,
                        reason: HANDOFF_FAILED.to_string(),
                        retry: Some(Retry::Handoff(update, outcome)),
                    },
                };
            });
            guard.store(false, Ordering::SeqCst);
        });
    }

    /// The Updates detail projection (design D7), at `now_ms` so the de-AT date
    /// and time are testable.
    pub fn update_view_at(&self, _now_ms: i64) -> UpdateView {
        let state = self.update_state();
        let host = self
            .platform
            .update_target()
            .filter(|_| self.update_supported());

        let (state_line, actions) = match &state {
            UpdateState::Idle => ("Noch nicht geprüft.".to_string(), vec![UpdateAction::Check]),
            UpdateState::Checking => (
                "Suche nach Updates …".to_string(),
                vec![UpdateAction::Check],
            ),
            UpdateState::UpToDate { checked_ms } => (
                format!(
                    "SnackPilot ist auf dem neuesten Stand. Zuletzt geprüft: {}, {}.",
                    crate::format::format_date_long(*checked_ms),
                    crate::format::format_time(*checked_ms)
                ),
                vec![UpdateAction::Check],
            ),
            UpdateState::Available { candidate } => (
                format!("Version {} ist verfügbar.", candidate.manifest.version),
                vec![
                    UpdateAction::Download,
                    UpdateAction::Skip,
                    UpdateAction::Check,
                ],
            ),
            UpdateState::Skipped { version } => (
                format!("Version {version} wird übersprungen. „Jetzt prüfen“ zeigt sie wieder an."),
                vec![UpdateAction::Check],
            ),
            UpdateState::Downloading {
                candidate,
                done,
                total,
            } => {
                let version = &candidate.manifest.version;
                match total {
                    Some(total) => (
                        format!(
                            "Version {version} wird geladen … {} %",
                            percent(*done, *total)
                        ),
                        Vec::new(),
                    ),
                    None => (format!("Version {version} wird geladen …"), Vec::new()),
                }
            }
            UpdateState::Verifying { .. } => ("Download wird überprüft …".to_string(), Vec::new()),
            UpdateState::Ready { update, handoff } => {
                (ready_line(update, *handoff), vec![handoff_action(*handoff)])
            }
            UpdateState::HandedOff {
                version, outcome, ..
            } => (
                handed_off_line(version, *outcome),
                match outcome {
                    HandoffOutcome::QuitRequested => vec![UpdateAction::InstallAndQuit],
                    HandoffOutcome::Revealed => {
                        vec![UpdateAction::RevealInFinder, UpdateAction::Check]
                    }
                },
            ),
            UpdateState::Failed {
                stage,
                reason,
                retry,
            } => failed_view(
                *stage,
                reason,
                retry.as_ref(),
                host.as_ref().map(|h| h.platform),
            ),
        };

        let (notes, notes_url, notes_heading) = match &state {
            UpdateState::Available { candidate } => (
                Some(candidate.manifest.notes.clone()),
                Some(candidate.manifest.notes_url.clone()),
                Some(format!("Neu in Version {}", candidate.manifest.version)),
            ),
            _ => (None, None, None),
        };
        let progress = match &state {
            UpdateState::Downloading { done, total, .. } => {
                total.map(|total| percent(*done, total) as f32)
            }
            _ => None,
        };
        let (installed, row_hint) = match host {
            Some(info) => (
                installed_line(&info),
                match &state {
                    UpdateState::Available { candidate } => {
                        format!("Version {} verfügbar", candidate.manifest.version)
                    }
                    UpdateState::Skipped { version } => format!("Version {version} verfügbar"),
                    _ => installed_line(&info),
                },
            ),
            None => (String::new(), String::new()),
        };

        UpdateView {
            installed,
            state_line,
            notes,
            notes_url,
            notes_heading,
            switch_on: self.auto_update_check(),
            actions,
            progress,
            row_hint,
        }
    }

    /// [`Self::update_view_at`] at the wall clock.
    pub fn update_view(&self) -> UpdateView {
        self.update_view_at(now_ms())
    }

    /// Replaces the snapshot; the entry point of offline detail scenarios
    /// (evidence mode and an unsupported host never reach the network anyway).
    pub fn set_update_state(&self, state: UpdateState) {
        self.hub.update("update_test_state", |s| s.update = state);
    }

    // ---- internals ----

    /// Epoch millis of the last check attempt, `None` before the first one.
    /// Written on every attempt, even a failed one (REQ-UPD-11).
    pub fn update_last_check(&self) -> Option<i64> {
        self.update_last_check_ms()
    }

    /// The version the user skipped, if any (REQ-UPD-12).
    pub fn skipped_version(&self) -> Option<String> {
        self.update_skipped()
    }

    fn update_begin(&self) -> bool {
        self.update_op
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Whether an operation is in flight for this model.
    fn update_op_running(&self) -> bool {
        self.update_op.load(Ordering::SeqCst) || update_op_running(&self.update_state())
    }

    /// The 24-hour rule; no recorded attempt means due.
    fn update_due(&self, now_ms: i64) -> bool {
        match self.update_last_check_ms() {
            Some(last) => now_ms.saturating_sub(last) >= CHECK_INTERVAL_MS,
            None => true,
        }
    }

    fn update_last_check_ms(&self) -> Option<i64> {
        self.kv
            .get(LAST_CHECK_KEY)
            .ok()
            .flatten()?
            .trim()
            .parse()
            .ok()
    }

    fn update_skipped(&self) -> Option<String> {
        self.kv.get(SKIPPED_VERSION_KEY).ok().flatten()
    }
}

/// German text of a failed handoff; carries no OS detail.
const HANDOFF_FAILED: &str = "Der Installer konnte nicht gestartet werden.";

/// One operation at a time per model (REQ-UPD-10): a check, download,
/// verification or handoff owns `update_op` from its start to its last state
/// change, so a second request is refused before any core call.
fn update_op_running(state: &UpdateState) -> bool {
    matches!(
        state,
        UpdateState::Checking | UpdateState::Downloading { .. } | UpdateState::Verifying { .. }
    )
}

/// The core host target; `None` when the host names no supported platform.
fn update_target(info: &UpdateHostInfo) -> Option<HostTarget> {
    match (info.platform, info.architecture) {
        ("macos", "arm64") | ("windows", "x64") => Some(HostTarget {
            platform: info.platform,
            architecture: info.architecture,
            os_version: info.os_version.clone(),
        }),
        _ => None,
    }
}

/// The state a finished check lands in (REQ-UPD-11/12); a skipped version is
/// hidden from automatic checks only.
fn checked_state(
    result: Result<Discovery, UpdateError>,
    checked_ms: i64,
    skipped: Option<&str>,
    trigger: UpdateTrigger,
) -> UpdateState {
    match result {
        Ok(Discovery::UpToDate { .. }) => UpdateState::UpToDate { checked_ms },
        Ok(Discovery::Available(candidate)) => {
            let hidden = trigger != UpdateTrigger::Manual
                && skipped == Some(candidate.manifest.version.as_str());
            if hidden {
                UpdateState::Skipped {
                    version: candidate.manifest.version,
                }
            } else {
                UpdateState::Available { candidate }
            }
        }
        Err(e) => UpdateState::Failed {
            stage: UpdateFailure::Check,
            reason: e.user_message().to_string(),
            retry: None,
        },
    }
}

/// The state a finished platform verification lands in (REQ-UPD-05/06).
fn verified_state(
    verified: Result<VerifiedUpdate, PlatformError>,
    candidate: Candidate,
) -> UpdateState {
    let platform = candidate.manifest.platform.as_str();
    match verified {
        Ok(update) => UpdateState::Ready {
            handoff: ready_handoff(platform),
            update,
        },
        Err(_) => UpdateState::Failed {
            stage: UpdateFailure::Verify,
            reason: String::new(),
            retry: None,
        },
    }
}

/// The handoff a platform offers once an update is verified (REQ-UPD-08/09).
fn ready_handoff(platform: &str) -> HandoffOutcome {
    match platform {
        "windows" => HandoffOutcome::QuitRequested,
        _ => HandoffOutcome::Revealed,
    }
}

/// Publishes download progress; one state change per 64 KiB keeps the UI calm.
fn progress_reporter(
    hub: Arc<StateHub>,
    candidate: Candidate,
) -> impl Fn(u64, Option<u64>) + Send + Sync {
    let last = std::sync::atomic::AtomicU64::new(0);
    move |done: u64, total: Option<u64>| {
        let previous = last.load(Ordering::SeqCst);
        let finished = total.is_some_and(|t| done >= t);
        if done / 65536 == previous / 65536 && !finished {
            return;
        }
        last.store(done, Ordering::SeqCst);
        let candidate = candidate.clone();
        hub.update("update_progress", |s| {
            s.update = UpdateState::Downloading {
                candidate: candidate.clone(),
                done,
                total,
            };
        });
    }
}

/// Wall clock in epoch millis.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn percent(done: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    (done.min(total) * 100) / total
}

fn installed_line(info: &UpdateHostInfo) -> String {
    format!(
        "Version {} ({})",
        info.installed_version, info.installed_build
    )
}

fn ready_line(update: &VerifiedUpdate, handoff: HandoffOutcome) -> String {
    match handoff {
        HandoffOutcome::QuitRequested => format!(
            "Version {} ist bereit zur Installation. SnackPilot wird dafür beendet.",
            update.version
        ),
        HandoffOutcome::Revealed => format!(
            "Version {} ist geprüft. Zum Installieren ziehst du die App in den Ordner „Programme“.",
            update.version
        ),
    }
}

fn handed_off_line(version: &str, outcome: HandoffOutcome) -> String {
    match outcome {
        HandoffOutcome::QuitRequested => format!(
            "Die Installation von Version {version} wurde gestartet. SnackPilot beendet sich dafür."
        ),
        HandoffOutcome::Revealed => "Im Finder angezeigt. Ziehe SnackPilot in „Programme“ und \
                                     starte die App neu. Das Update ist noch nicht installiert."
            .to_string(),
    }
}

fn handoff_action(outcome: HandoffOutcome) -> UpdateAction {
    match outcome {
        HandoffOutcome::QuitRequested => UpdateAction::InstallAndQuit,
        HandoffOutcome::Revealed => UpdateAction::RevealInFinder,
    }
}

fn failed_view(
    stage: UpdateFailure,
    reason: &str,
    retry: Option<&Retry>,
    platform: Option<&str>,
) -> (String, Vec<UpdateAction>) {
    let reason = reason.trim_end_matches('.');
    match stage {
        UpdateFailure::Check => (
            format!("Die Suche nach Updates ist fehlgeschlagen. {reason}."),
            vec![UpdateAction::Check],
        ),
        UpdateFailure::Download => (
            format!("Der Download ist fehlgeschlagen. {reason}."),
            vec![UpdateAction::Download],
        ),
        UpdateFailure::Verify => (
            "Das Update konnte nicht überprüft werden und wurde verworfen. Es wurde nichts \
             installiert."
                .to_string(),
            vec![UpdateAction::Check],
        ),
        UpdateFailure::Handoff => {
            let retry_action = match retry {
                Some(Retry::Handoff(_, outcome)) => handoff_action(*outcome),
                _ if platform == Some("windows") => UpdateAction::InstallAndQuit,
                _ => UpdateAction::RevealInFinder,
            };
            (
                format!("Die Installation konnte nicht gestartet werden. {reason}."),
                vec![retry_action],
            )
        }
    }
}
