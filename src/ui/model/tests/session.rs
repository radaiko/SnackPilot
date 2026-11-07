//! Session flows against the real core with FakePlatform: demo credentials
//! only, so no test ever reaches a real host.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use snackpilot_model::{AppModel, FakePlatform, Platform, PlatformError};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-session-{tag}-{}-{}",
        std::process::id(),
        uuid_like()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn uuid_like() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

const DEMO_USER: &str = "demo";
const DEMO_PASS: &str = "demo1234!";

/// How long a gated worker may block before the test declares a deadlock.
const GATE_TIMEOUT: Duration = Duration::from_secs(60);

/// "YYYY-MM" of the current month, device-local (same key the core caches
/// billing under, `month_key_from_offset(clock, 0)`).
fn current_month_key() -> String {
    chrono::Local::now().format("%Y-%m").to_string()
}

/// Seed one credential slot directly in the fake's store.
fn seed(fake: &FakePlatform, key: &str, value: &str) {
    fake.save_credential(key, value).unwrap();
}

/// Platform wrapper that blocks the first call of one named method until the
/// test releases it, so the test can observe state mid-flow.
struct GatePlatform {
    inner: Arc<FakePlatform>,
    gate_method: String,
    entered: Mutex<Option<mpsc::Sender<()>>>,
    release: Mutex<Option<mpsc::Receiver<()>>>,
    hit: AtomicBool,
}

impl GatePlatform {
    fn new(
        inner: Arc<FakePlatform>,
        gate_method: &str,
    ) -> (Arc<Self>, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let gate = Arc::new(Self {
            inner,
            gate_method: gate_method.to_string(),
            entered: Mutex::new(Some(entered_tx)),
            release: Mutex::new(Some(release_rx)),
            hit: AtomicBool::new(false),
        });
        (gate, entered_rx, release_tx)
    }
}

impl Platform for GatePlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        self.inner.get_credential(key)
    }
    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        if self.gate_method == format!("save_credential({key})")
            && !self.hit.swap(true, Ordering::SeqCst)
        {
            // Block BEFORE the inner save so the test sees the state and the
            // core exactly as they are when the save starts.
            if let Some(tx) = self.entered.lock().unwrap().take() {
                tx.send(()).unwrap();
            }
            if let Some(rx) = self.release.lock().unwrap().take() {
                rx.recv_timeout(GATE_TIMEOUT).unwrap();
            }
        }
        self.inner.save_credential(key, value)
    }
    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        self.inner.clear_credential(key)
    }
    fn notify(
        &self,
        command: snackpilot_core::notify::NotificationCommand,
    ) -> Result<(), PlatformError> {
        self.inner.notify(command)
    }
    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        self.inner.request_notification_permission()
    }
    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.inner.request_location_authorisation()
    }
    fn current_location(&self) -> Result<Option<snackpilot_model::LocationFix>, PlatformError> {
        self.inner.current_location()
    }
    fn start_geofence(&self) -> Result<(), PlatformError> {
        self.inner.start_geofence()
    }
    fn stop_geofence(&self) -> Result<(), PlatformError> {
        self.inner.stop_geofence()
    }
    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        self.inner.is_geofence_monitoring()
    }
    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError> {
        self.inner.track(event, props)
    }
    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError> {
        self.inner.set_app_icon(accent)
    }
    fn open_app_settings(&self) -> Result<(), PlatformError> {
        self.inner.open_app_settings()
    }
    fn compose_email(&self, email: snackpilot_model::SupportEmail) -> Result<(), PlatformError> {
        self.inner.compose_email(email)
    }
    fn capabilities(&self) -> snackpilot_model::Capabilities {
        self.inner.capabilities()
    }
}

/// REQ-PLT-01: the core saves credentials before validating them — while the
/// credential save is still running, the core is not authenticated yet.
/// Demo credentials only: no network. @req REQ-PLT-01
#[test]
fn login_saves_credentials_before_validating() {
    let fake = Arc::new(FakePlatform::new());
    let (gate, entered, release_tx) =
        GatePlatform::new(fake.clone(), "save_credential(gourmet_username)");
    let model = AppModel::new(temp_dir("plt01"), gate).unwrap();

    let handle = std::thread::scope(|s| {
        let handle = s.spawn(|| model.login(DEMO_USER, DEMO_PASS));
        entered
            .recv_timeout(GATE_TIMEOUT)
            .expect("login never reached the credential save");
        // Under catch_unwind so a failure still releases the gate instead of
        // deadlocking the scope on the blocked worker.
        // The save is in flight: the core has not validated (not even called)
        // the login yet, so it cannot be authenticated.
        let mid = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert!(
                !model.core().gourmet_is_authenticated(),
                "core validated before the save finished"
            );
        }));
        release_tx.send(()).unwrap();
        let result = handle.join().unwrap();
        mid.unwrap();
        result
    });
    assert!(handle.is_ok(), "login: {:?}", handle.err());
    assert!(model.core().gourmet_is_authenticated());
    // v1 saves both keys of the pair unconditionally.
    assert_eq!(
        fake.credential("gourmet_username").as_deref(),
        Some(DEMO_USER)
    );
    assert_eq!(
        fake.credential("gourmet_password").as_deref(),
        Some(DEMO_PASS)
    );
}

/// REQ-SCR-09: auto-login restores Ventopay first without its own billing
/// fetch, then Gourmet; the single billing fetch belongs to the Gourmet
/// session-load. Proof: mid-flow the core has no billing data at all, and
/// across the whole flow `billing_loading` rises exactly once.
/// Demo credentials only: no network. @req REQ-SCR-09
#[test]
fn auto_login_ventopay_first_without_own_billing_fetch() {
    let fake = Arc::new(FakePlatform::new());
    seed(&fake, "gourmet_username", DEMO_USER);
    seed(&fake, "gourmet_password", DEMO_PASS);
    seed(&fake, "ventopay_username", DEMO_USER);
    seed(&fake, "ventopay_password", DEMO_PASS);

    let (gate, entered, release_tx) =
        GatePlatform::new(fake.clone(), "save_credential(gourmet_username)");
    let model = AppModel::new(temp_dir("scr09"), gate).unwrap();

    // Record every published billing_loading value; false→true transitions
    // count the billing fetch windows of the whole flow.
    let loading_log: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
    let log = loading_log.clone();
    model.subscribe(Box::new(move |s: &snackpilot_model::State| {
        log.lock().unwrap().push(s.billing_loading)
    }));

    std::thread::scope(|s| {
        let handle = s.spawn(|| model.attempt_auto_login());
        entered
            .recv_timeout(GATE_TIMEOUT)
            .expect("auto-login never reached the gourmet credential save");
        // Assertions run under catch_unwind so a failure still releases the
        // gate instead of deadlocking the scope on the blocked worker.
        let mid = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Ventopay was restored first...
            assert!(
                model.state().ventopay_authenticated,
                "ventopay not authenticated before gourmet login"
            );
            // ...and no billing load ran with it: neither the state nor the
            // core has any billing data while the gourmet save is starting.
            let state = model.state();
            assert!(!state.billing_loading, "billing load already running");
            assert!(
                state.month_options.is_empty(),
                "month options set before the gourmet session-load: {:?}",
                state.month_options
            );
            assert!(
                state.ventopay_month.is_none(),
                "ventopay billing loaded by the ventopay restore"
            );
            assert!(
                model
                    .core()
                    .ventopay_billing_month(current_month_key())
                    .is_none(),
                "core has ventopay billing before the gourmet session-load"
            );
            let calls = fake.calls();
            assert!(
                calls.contains(&"save_credential(ventopay_username)".to_string()),
                "ventopay not restored first: {calls:?}"
            );
            assert!(
                calls.contains(&"save_credential(ventopay_password)".to_string()),
                "ventopay pair incomplete: {calls:?}"
            );
            // The gourmet save is the one in flight: no gourmet save recorded
            // after the ventopay login finished (the seeded ones precede it).
            let last_ventopay_save = calls
                .iter()
                .rposition(|c| c.starts_with("save_credential(ventopay"))
                .unwrap();
            assert!(
                !calls[last_ventopay_save..]
                    .iter()
                    .any(|c| c.starts_with("save_credential(gourmet")),
                "gourmet saved before ventopay finished: {calls:?}"
            );
        }));
        release_tx.send(()).unwrap();
        handle.join().unwrap().unwrap();
        mid.unwrap();
    });

    let state = model.state();
    assert!(state.gourmet_authenticated, "gourmet login failed");
    assert!(state.ventopay_authenticated, "ventopay login failed");
    assert_eq!(
        state.user_info.as_ref().map(|u| u.username.as_str()),
        Some("Demo User")
    );
    assert!(
        !state.month_options.is_empty(),
        "billing month options not loaded"
    );
    assert!(!model.state().billing_loading);
    // Exactly one billing fetch window across the whole auto-login.
    let log = loading_log.lock().unwrap();
    let windows = log
        .iter()
        .zip(log.iter().skip(1))
        .filter(|(from, to)| !**from && **to)
        .count();
    assert_eq!(
        windows, 1,
        "expected exactly one billing fetch, saw {windows} from {log:?}"
    );
    // The flow ends with the billing data of the selected month loaded.
    let key = state
        .month_options
        .iter()
        .find(|m| m.offset == state.selected_offset)
        .map(|m| m.key.clone())
        .unwrap();
    assert!(
        model.core().gourmet_billing_month(key.clone()).is_some(),
        "gourmet billing not loaded"
    );
    assert!(
        model.core().ventopay_billing_month(key).is_some(),
        "ventopay billing not loaded"
    );
}

/// REQ-SCR-05/REQ-SCR-06: logout clears session state but keeps the saved
/// credentials (v1 parity), for both accounts.
/// Demo credentials only: no network.
/// @req REQ-SCR-05
/// @req REQ-SCR-06
#[test]
fn logout_keeps_credentials() {
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("logout"), fake.clone()).unwrap();
    model.login(DEMO_USER, DEMO_PASS).unwrap();
    assert!(model.state().gourmet_authenticated);
    assert!(model.state().demo_mode);
    // Load the Automaten month so the ventopay logout has something to clear.
    model
        .ventopay_login(DEMO_USER, DEMO_PASS, true)
        .expect("ventopay login");
    assert!(
        model.state().ventopay_month.is_some(),
        "ventopay billing not loaded before logout"
    );

    model.gourmet_logout();
    let state = model.state();
    assert!(!state.gourmet_authenticated);
    assert!(state.user_info.is_none());
    assert!(state.menu_snapshot.is_none());
    assert!(state.orders_split.is_none());
    assert!(state.gourmet_month.is_none());
    assert!(!state.demo_mode, "demo flag must clear with the session");
    // Credentials stay in the store (v1: logout keeps credentials).
    assert_eq!(
        fake.credential("gourmet_username").as_deref(),
        Some(DEMO_USER)
    );
    assert_eq!(
        fake.credential("gourmet_password").as_deref(),
        Some(DEMO_PASS)
    );

    assert!(
        model.state().ventopay_authenticated,
        "ventopay must be authenticated before the logout under test"
    );
    model.ventopay_logout();
    let state = model.state();
    assert!(!state.ventopay_authenticated);
    assert!(
        state.ventopay_month.is_none(),
        "ventopay logout must clear the selected month"
    );
    // Neither logout may clear a saved credential (v1 parity).
    assert!(
        !fake
            .calls()
            .iter()
            .any(|c| c.starts_with("clear_credential")),
        "logout cleared a saved credential: {:?}",
        fake.calls()
    );
    // Both credential pairs survive both logouts (v1 parity).
    assert_eq!(
        fake.credential("ventopay_username").as_deref(),
        Some(DEMO_USER)
    );
    assert_eq!(
        fake.credential("ventopay_password").as_deref(),
        Some(DEMO_PASS)
    );
    assert_eq!(
        fake.credential("gourmet_username").as_deref(),
        Some(DEMO_USER)
    );
    assert_eq!(
        fake.credential("gourmet_password").as_deref(),
        Some(DEMO_PASS)
    );
}

/// REQ-PLT-06: a failing Platform call surfaces as a typed error and never
/// panics. @req REQ-PLT-06
#[test]
fn failing_platform_returns_typed_error_without_panicking() {
    let fake = Arc::new(FakePlatform::new());
    fake.fail("notify");
    fake.fail("save_credential");
    let model = AppModel::new(temp_dir("plt06"), fake.clone()).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        model.login(DEMO_USER, DEMO_PASS)
    }));
    let result = result.expect("model.login must not panic");
    match result {
        Err(PlatformError::System(msg)) => {
            assert!(msg.contains("save_credential"), "unexpected: {msg}")
        }
        other => panic!("expected typed PlatformError, got {other:?}"),
    }
    // The core was never reached, so nothing got authenticated.
    assert!(!model.core().gourmet_is_authenticated());
    assert!(
        model.state().error_text.is_none(),
        "platform failure must not set core error text"
    );
}

/// REQ-PLT-06: a failing notification bridge during login's orders load
/// surfaces as a typed error and must not leave the model stuck busy. The
/// demo orders fetch deterministically produces the cancel-reminder command,
/// so `fail("notify")` always hits the bridge. Demo credentials only.
/// @req REQ-PLT-06
#[test]
fn notify_failure_does_not_leave_busy() {
    let fake = Arc::new(FakePlatform::new());
    fake.fail("notify");
    let model = AppModel::new(temp_dir("busy"), fake.clone()).unwrap();

    let result = model.login(DEMO_USER, DEMO_PASS);
    match result {
        Err(PlatformError::System(msg)) => assert!(msg.contains("notify"), "unexpected: {msg}"),
        other => panic!("expected typed notify error, got {other:?}"),
    }
    assert!(
        fake.calls().iter().any(|c| c == "notify"),
        "bridge was never reached: {:?}",
        fake.calls()
    );
    let state = model.state();
    assert!(!state.busy, "login left busy set after a notify failure");
    assert!(
        !state.ventopay_busy,
        "login left ventopay_busy set after a notify failure"
    );
    // The orders split is published even though the bridge failed.
    assert!(
        state.orders_split.is_some(),
        "orders split must be published despite the bridge failure"
    );
    // The notify failure must not cost the rest of the session: the month
    // options and the billing of the selected month still loaded.
    assert!(
        !state.month_options.is_empty(),
        "notify failure skipped the billing load"
    );
    assert!(
        state.gourmet_month.is_some() && state.ventopay_month.is_some(),
        "notify failure lost the billing data: {:?}",
        (
            state.gourmet_month.is_some(),
            state.ventopay_month.is_some()
        )
    );
    assert!(
        state.error_text.is_none(),
        "bridge failure must surface as the typed error, not core error text"
    );
}

/// A subscriber may register another subscriber from inside its callback —
/// GPUI views mount during state updates. Regression test for the hub
/// deadlock: the hub must not hold any lock while callbacks run.
#[test]
fn subscriber_can_subscribe_from_callback() {
    let model =
        Arc::new(AppModel::new(temp_dir("nested-sub"), Arc::new(FakePlatform::new())).unwrap());
    let inner_saw_busy = Arc::new(AtomicBool::new(false));

    let (done_tx, done_rx) = mpsc::channel();
    let m = model.clone();
    let flag = inner_saw_busy.clone();
    // Run the flows on a detached worker so a hub deadlock fails the test
    // after GATE_TIMEOUT instead of hanging the suite.
    std::thread::spawn(move || {
        let outer_model = m.clone();
        let outer_flag = flag.clone();
        m.subscribe(Box::new(move |_s: &snackpilot_model::State| {
            // Fires on the initial delivery: subscribe from inside the
            // callback exactly once.
            let flag = outer_flag.clone();
            outer_model.subscribe(Box::new(move |s| {
                if s.ventopay_busy {
                    flag.store(true, Ordering::SeqCst);
                }
            }));
        }));
        m.ventopay_login(DEMO_USER, DEMO_PASS, false)
            .expect("ventopay login");
        let _ = done_tx.send(());
    });
    done_rx
        .recv_timeout(GATE_TIMEOUT)
        .expect("nested subscribe deadlocked the state hub");

    assert!(
        inner_saw_busy.load(Ordering::SeqCst),
        "subscriber registered from a callback never fired"
    );
}

/// The Automaten login screen's busy state is observable through the state
/// hub: the subscriber fires after each of the login's distinct state
/// changes, not just the first one.
/// @req REQ-SCR-06
#[test]
fn subscribe_fires_after_multiple_changes() {
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("sub"), fake).unwrap();
    // Every state the hub hands the subscriber, in delivery order.
    let seen: Arc<Mutex<Vec<(bool, bool, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    model.subscribe(Box::new(move |s: &snackpilot_model::State| {
        log.lock()
            .unwrap()
            .push((s.busy, s.ventopay_busy, s.ventopay_authenticated));
    }));
    model.ventopay_login(DEMO_USER, DEMO_PASS, false).unwrap();

    let seen = seen.lock().unwrap();
    let distinct: std::collections::HashSet<_> = seen.iter().collect();
    assert!(
        distinct.len() > 1,
        "subscriber must fire after multiple distinct changes, saw only {seen:?}"
    );
    assert!(
        seen.iter().any(|&(_, busy, _)| busy),
        "subscriber never saw ventopay_busy: {seen:?}"
    );
    assert!(
        seen.iter().any(|&(_, _, auth)| auth),
        "subscriber never saw the finished login: {seen:?}"
    );
    assert!(
        seen.last().is_some_and(|&(_, busy, _)| !busy),
        "last delivered state must have ventopay_busy cleared: {seen:?}"
    );
}

/// REQ-PLT-04: core analytics events reach the host through the
/// `PlatformAnalytics` sink wired into the core at construction. Demo
/// credentials only: no network. @req REQ-PLT-04
#[tokio::test]
async fn analytics_forwards_core_events_to_platform() {
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(temp_dir("plt04"), fake.clone()).unwrap();
    let core = model.core().clone();

    core.gourmet_login(snackpilot_core::domain::Credentials {
        username: DEMO_USER.into(),
        password: DEMO_PASS.into(),
    })
    .await
    .unwrap();
    core.fetch_menus(false).await.unwrap();

    // Order a menu on a date past the ordering cutoff so the submit is not
    // early-returned before the analytics track.
    let snap = core.menu_snapshot();
    let item = snap
        .items
        .iter()
        .find(|i| i.available && !i.ordered && !core.is_ordering_cutoff(i.day.clone()))
        .expect("demo menus must contain an orderable item past the cutoff");
    core.toggle_pending(item.id.clone(), item.day.clone());
    core.submit_orders(None).await.unwrap();

    assert!(
        fake.tracks()
            .iter()
            .any(|(event, _)| event == "order.submitted"),
        "order.submitted never reached the platform: {:?}",
        fake.tracks()
    );
}

/// REQ-PLT-04: analytics is best effort — a failing `track` bridge must not
/// fail the order submit. Demo credentials only: no network.
/// @req REQ-PLT-04
#[tokio::test]
async fn analytics_failure_does_not_break_submit() {
    let fake = Arc::new(FakePlatform::new());
    fake.fail("track");
    let model = AppModel::new(temp_dir("plt04f"), fake.clone()).unwrap();
    let core = model.core().clone();

    core.gourmet_login(snackpilot_core::domain::Credentials {
        username: DEMO_USER.into(),
        password: DEMO_PASS.into(),
    })
    .await
    .unwrap();
    core.fetch_menus(false).await.unwrap();

    let snap = core.menu_snapshot();
    let item = snap
        .items
        .iter()
        .find(|i| i.available && !i.ordered && !core.is_ordering_cutoff(i.day.clone()))
        .expect("demo menus must contain an orderable item past the cutoff");
    core.toggle_pending(item.id.clone(), item.day.clone());
    core.submit_orders(None)
        .await
        .expect("a failing analytics bridge must not fail submit");
    // The track really was attempted through the wired sink, so an
    // unwired sink cannot pass this test silently.
    assert!(
        fake.calls().iter().any(|c| c == "track(order.submitted)"),
        "track(order.submitted) was never attempted: {:?}",
        fake.calls()
    );
}

/// The model runtime's timer driver must actually run: the session flows
/// block on it from the UI thread, and every core network call would hang
/// if the drivers were never driven. The sleep is awaited on the same
/// `block_on` path the flows use; a watchdog fails the test instead of
/// hanging the suite if the driver stalls. Loopback only: no real host.
#[test]
fn model_runtime_drives_the_timer_driver() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        snackpilot_model::block_on_model(async {
            tokio::time::sleep(Duration::from_millis(50)).await;
        });
        let _ = tx.send(start.elapsed());
    });
    let elapsed = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("block_on never returned: the model runtime drivers are stalled");
    assert!(
        elapsed >= Duration::from_millis(40),
        "the sleep returned without waiting: {elapsed:?}"
    );
}
