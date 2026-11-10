//! Orders tab (REQ-SCR-02): confirm, single-flight cancel, multi-select batch
//! cancel with the single `order.cancelled` track. Demo mode only — no test
//! reaches a real host.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use snackpilot_core::domain::{MenuCategory, MenuItem, OrderedMenu};
use snackpilot_model::navigation::ListKind;
use snackpilot_model::{unconfirmed_banner_text, AppModel, FakePlatform, Platform, PlatformError};

const DEMO_USER: &str = "demo";
const DEMO_PASS: &str = "demo1234!";

/// How long a gated worker may block before the test declares a deadlock.
const GATE_TIMEOUT: Duration = Duration::from_secs(60);

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-orders-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Platform wrapper that blocks the first call of one named method (a
/// `calls()`-style name such as `track(order.cancelled)`) until the test
/// releases it, so the test can observe state mid-flow.
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

    /// Blocks once on `name` if it is the gated method.
    fn gate(&self, name: &str) {
        if self.gate_method == name && !self.hit.swap(true, Ordering::SeqCst) {
            if let Some(tx) = self.entered.lock().unwrap().take() {
                tx.send(()).unwrap();
            }
            if let Some(rx) = self.release.lock().unwrap().take() {
                rx.recv_timeout(GATE_TIMEOUT).unwrap();
            }
        }
    }
}

impl Platform for GatePlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        self.inner.get_credential(key)
    }
    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        self.gate(&format!("save_credential({key})"));
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
        self.gate(&format!("track({event})"));
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

/// Platform wrapper that panics on the FIRST `track(order.cancelled)`, to
/// crash the core's cancel path after the cancel itself succeeded.
struct PanicOncePlatform {
    inner: Arc<FakePlatform>,
    fired: AtomicBool,
}

impl PanicOncePlatform {
    fn new(inner: Arc<FakePlatform>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            fired: AtomicBool::new(false),
        })
    }
}

impl Platform for PanicOncePlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        self.inner.get_credential(key)
    }
    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
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
        if event == "order.cancelled" && !self.fired.swap(true, Ordering::SeqCst) {
            panic!("platform crashed while tracking {event}");
        }
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

fn demo_model_in(dir: PathBuf, platform: Arc<dyn Platform>) -> AppModel {
    let model = AppModel::new(dir, platform).unwrap();
    model.login(DEMO_USER, DEMO_PASS).unwrap();
    model
}

fn demo_model(tag: &str) -> (AppModel, Arc<FakePlatform>) {
    let fake = Arc::new(FakePlatform::new());
    let model = demo_model_in(temp_dir(tag), fake.clone());
    (model, fake)
}

fn item_in(model: &AppModel, day: &str, category: MenuCategory) -> MenuItem {
    let snap = model.state().menu_snapshot.clone().expect("menus loaded");
    snap.items
        .into_iter()
        .filter(|i| i.day == day)
        .find(|i| i.category == category)
        .unwrap_or_else(|| panic!("no {} item on {day}", category.display()))
}

/// The last offered day: always in the future, so the 09:00 Vienna ordering
/// cutoff (docs/requirements/06-testing.md §6.3) can never block the submit.
fn future_day(model: &AppModel) -> String {
    model
        .state()
        .menu_snapshot
        .as_ref()
        .expect("menus loaded")
        .available_dates
        .last()
        .cloned()
        .expect("demo offers days")
}

/// Order one item of the last offered day through the real submit pipeline.
fn order_one(model: &AppModel, category: MenuCategory) -> MenuItem {
    let day = future_day(model);
    let item = item_in(model, &day, category);
    model.toggle(&item);
    model.submit_orders();
    item
}

fn upcoming(model: &AppModel) -> Vec<OrderedMenu> {
    model
        .state()
        .orders_split
        .clone()
        .expect("orders loaded")
        .upcoming
}

/// The `order.cancelled` tracks recorded by the fake, with their count props.
fn cancelled_tracks(fake: &FakePlatform) -> Vec<String> {
    fake.tracks()
        .into_iter()
        .filter(|(event, _)| event == "order.cancelled")
        .map(|(_, props)| props.get("count").cloned().unwrap_or_default())
        .collect()
}

#[test]
/// @req REQ-SCR-02
fn confirm_orders_approves_and_never_leaves_busy() {
    let (model, _fake) = demo_model("confirm");
    order_one(&model, MenuCategory::Menu1);
    // The submit pipeline confirms inline (core menus §6.5 step 6-8:
    // add_to_cart → confirm_orders), so demo orders are already approved;
    // the demo has no failure hook to leave one unapproved. The confirm
    // action must still run its busy lifecycle and keep them approved.
    assert!(!upcoming(&model).is_empty(), "order placed");

    let busy_log: Arc<Mutex<Vec<(bool, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let log = busy_log.clone();
    model.subscribe(Box::new(move |s: &snackpilot_model::State| {
        log.lock().unwrap().push((s.busy, s.orders_split.is_some()))
    }));
    let before = busy_log.lock().unwrap().len();

    model.confirm_orders();

    let log = busy_log.lock().unwrap();
    let updates = &log[before..];
    // Full lifecycle: busy raised, an orders refresh while busy, busy lowered.
    assert!(
        updates.iter().any(|&(busy, _)| busy),
        "confirm never raised busy: {updates:?}"
    );
    assert!(
        updates.last().is_some_and(|&(busy, _)| !busy),
        "confirm left busy set: {updates:?}"
    );
    assert!(
        updates.iter().any(|&(busy, orders)| busy && orders),
        "confirm never re-loaded the orders while busy: {updates:?}"
    );
    assert!(
        updates.len() >= 3,
        "confirm must emit busy-on, orders refresh and busy-off updates: {updates:?}"
    );
    drop(log);

    let orders = upcoming(&model);
    assert!(!orders.is_empty(), "orders still loaded after confirm");
    assert!(
        orders.iter().all(|o| o.approved),
        "confirm did not approve the orders: {orders:?}"
    );
    assert!(
        model.state().orders_error.is_none(),
        "successful confirm must not set orders_error"
    );
}

#[test]
/// @req REQ-SCR-02
fn cancel_order_removes_it_and_tracks_once() {
    let (model, fake) = demo_model("cancel-one");
    order_one(&model, MenuCategory::Menu1);
    let pid = upcoming(&model)[0].position_id.clone();

    model.cancel_order(&pid);

    assert!(
        upcoming(&model).iter().all(|o| o.position_id != pid),
        "cancelled order still listed"
    );
    assert_eq!(
        cancelled_tracks(&fake),
        vec!["1".to_string()],
        "one cancel emits exactly one order.cancelled with count=1"
    );
    assert!(
        model.state().cancelling_id.is_none(),
        "cancelling marker must clear after the cancel"
    );
}

#[test]
/// @req REQ-SCR-02
fn cancel_is_single_flight() {
    let fake = Arc::new(FakePlatform::new());
    let (gate, entered, release_tx) = GatePlatform::new(fake.clone(), "track(order.cancelled)");
    let model = demo_model_in(temp_dir("single-flight"), gate);
    order_one(&model, MenuCategory::Menu1);
    order_one(&model, MenuCategory::Menu2);

    let orders = upcoming(&model);
    assert_eq!(orders.len(), 2, "two orders needed for the guard test");
    let (a, b) = (orders[0].position_id.clone(), orders[1].position_id.clone());

    std::thread::scope(|s| {
        let worker = s.spawn(|| model.cancel_order(&a));
        entered
            .recv_timeout(GATE_TIMEOUT)
            .expect("cancel never reached the analytics track");
        // Assertions under catch_unwind so a failure still releases the gate.
        let mid = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(
                model.state().cancelling_id.as_deref(),
                Some(a.as_str()),
                "cancelling marker not set while the cancel is in flight"
            );
            // Second tap during the first: dropped by the guard, so it must
            // return at once instead of queueing a second cancel.
            model.cancel_order(&b);
        }));
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        mid.unwrap();
    });

    let orders = upcoming(&model);
    assert!(
        orders.iter().all(|o| o.position_id != a),
        "first cancel lost"
    );
    assert!(
        orders.iter().any(|o| o.position_id == b),
        "the guarded second cancel went through anyway"
    );
    assert_eq!(
        cancelled_tracks(&fake),
        vec!["1".to_string()],
        "the dropped second tap must not emit a second order.cancelled"
    );
    assert!(model.state().cancelling_id.is_none());
}

#[test]
/// @req REQ-SCR-02
fn batch_cancel_emits_one_track_with_the_selection_count() {
    let (model, fake) = demo_model("batch");
    order_one(&model, MenuCategory::Menu1);
    order_one(&model, MenuCategory::Menu2);
    let orders = upcoming(&model);
    assert_eq!(orders.len(), 2);

    model.begin_order_selection();
    assert!(model.state().order_selection_mode);
    model.toggle_order_selection(&orders[0].position_id);
    model.toggle_order_selection(&orders[1].position_id);
    assert_eq!(
        model.state().selected_order_ids,
        vec![orders[0].position_id.clone(), orders[1].position_id.clone()],
        "both orders selected"
    );

    model.cancel_selected_orders();

    assert!(upcoming(&model).is_empty(), "both orders cancelled");
    assert_eq!(
        cancelled_tracks(&fake),
        vec!["2".to_string()],
        "the batch is ONE order.cancelled with count=2"
    );
    let state = model.state();
    assert!(
        !state.order_selection_mode && state.selected_order_ids.is_empty(),
        "selection must end with the batch cancel"
    );
    assert!(!state.busy, "batch cancel left busy set");
}

#[test]
/// @req REQ-SCR-02
fn selection_toggles_add_and_remove() {
    let (model, _fake) = demo_model("selection");
    model.begin_order_selection();
    model.toggle_order_selection("pos-a");
    model.toggle_order_selection("pos-b");
    assert_eq!(model.state().selected_order_ids, ["pos-a", "pos-b"]);
    model.toggle_order_selection("pos-a");
    assert_eq!(model.state().selected_order_ids, ["pos-b"]);

    model.end_order_selection();
    assert!(!model.state().order_selection_mode);
    assert!(model.state().selected_order_ids.is_empty());
}

#[test]
/// @req REQ-SCR-02
fn cancel_selected_with_empty_selection_is_a_noop() {
    let (model, fake) = demo_model("batch-empty");
    model.begin_order_selection();
    model.cancel_selected_orders();
    assert!(
        !fake.calls().iter().any(|c| c == "track(order.cancelled)"),
        "empty selection must not cancel or track anything: {:?}",
        fake.calls()
    );
    assert!(!model.state().busy, "no-op must not raise busy");
}

#[test]
/// @req REQ-SCR-02
fn cancellation_cutoff_follows_the_order_day() {
    let (model, _fake) = demo_model("cutoff");
    let past = OrderedMenu {
        position_id: "p1".into(),
        eating_cycle_id: "c1".into(),
        date_epoch_ms: chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis(),
        title: "MENÜ I".into(),
        subtitle: String::new(),
        approved: false,
    };
    assert!(
        model.is_cancellation_cutoff(&past),
        "an order on a past day is past the cutoff"
    );

    order_one(&model, MenuCategory::Menu1);
    let future = upcoming(&model)[0].clone();
    assert!(
        !model.is_cancellation_cutoff(&future),
        "a future demo order is before the cutoff"
    );
}

fn order_on(pid: &str, approved: bool) -> OrderedMenu {
    OrderedMenu {
        position_id: pid.into(),
        eating_cycle_id: "c1".into(),
        date_epoch_ms: 1_700_000_000_000,
        title: "MENÜ I".into(),
        subtitle: String::new(),
        approved,
    }
}

#[test]
/// @req REQ-SCR-02
fn banner_text_counts_unconfirmed_orders() {
    assert_eq!(unconfirmed_banner_text(&[]), None);
    assert_eq!(unconfirmed_banner_text(&[order_on("a", true)]), None);
    assert_eq!(
        unconfirmed_banner_text(&[order_on("a", false), order_on("b", true)]).as_deref(),
        Some("1 unbestätigte Bestellung"),
        "exactly one unconfirmed order uses the singular"
    );
    assert_eq!(
        unconfirmed_banner_text(&[
            order_on("a", false),
            order_on("b", false),
            order_on("c", true)
        ])
        .as_deref(),
        Some("2 unbestätigte Bestellungen"),
        "spec scenario: two unconfirmed orders read '2 unbestätigte Bestellungen'"
    );

    // The model method derives the banner from the live state; demo orders are
    // confirmed inline by submit, so the banner is hidden there.
    let (model, _fake) = demo_model("banner");
    order_one(&model, MenuCategory::Menu1);
    assert_eq!(model.unconfirmed_banner_text(), None);
}

#[test]
/// @req REQ-SCR-02
fn cancel_rejects_concurrent_tap_then_allows_retry() {
    let fake = Arc::new(FakePlatform::new());
    let (gate, entered, release_tx) = GatePlatform::new(fake.clone(), "track(order.cancelled)");
    let model = demo_model_in(temp_dir("cancel-retry"), gate);
    order_one(&model, MenuCategory::Menu1);
    order_one(&model, MenuCategory::Menu2);
    let orders = upcoming(&model);
    assert_eq!(orders.len(), 2, "two orders needed for the guard test");
    let (a, b) = (orders[0].position_id.clone(), orders[1].position_id.clone());

    std::thread::scope(|s| {
        let worker = s.spawn(|| model.cancel_order(&a));
        entered
            .recv_timeout(GATE_TIMEOUT)
            .expect("cancel never reached the analytics track");
        // Concurrent second tap while the first holds the slot: rejected by
        // the atomic check-and-set, so it returns without cancelling.
        model.cancel_order(&b);
        release_tx.send(()).unwrap();
        worker.join().unwrap();
    });

    assert!(
        upcoming(&model).iter().all(|o| o.position_id != a),
        "first cancel lost"
    );
    assert!(
        upcoming(&model).iter().any(|o| o.position_id == b),
        "the rejected concurrent tap cancelled anyway"
    );
    assert_eq!(
        cancelled_tracks(&fake),
        vec!["1".to_string()],
        "the rejected tap must not emit a second order.cancelled"
    );

    // Retry after the first cancel completed: the slot is free again.
    model.cancel_order(&b);
    assert!(
        upcoming(&model).iter().all(|o| o.position_id != b),
        "the retry of the rejected tap must go through"
    );
    assert_eq!(
        cancelled_tracks(&fake),
        vec!["1".to_string(), "1".to_string()],
        "retry emits its own order.cancelled"
    );
    assert!(model.state().cancelling_id.is_none());
}

#[test]
/// @req REQ-SCR-02
fn cancel_clears_single_flight_marker_when_the_flow_panics() {
    let fake = Arc::new(FakePlatform::new());
    let model = demo_model_in(
        temp_dir("cancel-panic"),
        PanicOncePlatform::new(fake.clone()),
    );
    order_one(&model, MenuCategory::Menu1);
    let pid = upcoming(&model)[0].position_id.clone();

    let result = std::thread::scope(|s| s.spawn(|| model.cancel_order(&pid)).join());
    assert!(
        result.is_err(),
        "the platform panic must propagate out of cancel_order"
    );
    assert!(
        model.state().cancelling_id.is_none(),
        "a panic must not leave the single-flight marker stuck"
    );

    // The slot is free again: the retry completes the cancel.
    model.cancel_order(&pid);
    assert!(
        upcoming(&model).iter().all(|o| o.position_id != pid),
        "cancel after the panic did not complete"
    );
    assert!(model.state().cancelling_id.is_none());
}

#[test]
/// @req REQ-SCR-02
fn past_segment_exits_batch_without_cancel() {
    let (model, fake) = demo_model("segment-exit");
    order_one(&model, MenuCategory::Menu1);
    let pid = upcoming(&model)[0].position_id.clone();

    model.begin_order_selection();
    model.toggle_order_selection(&pid);
    assert_eq!(
        model.state().selected_order_ids.len(),
        1,
        "the order must be checked"
    );
    model.open_preview(ListKind::Orders, &pid);

    model.set_orders_segment(true);

    assert!(model.state().orders_past, "segment must switch to past");
    assert!(
        !model.state().order_selection_mode,
        "entering past must exit batch selection"
    );
    assert!(
        model.state().selected_order_ids.is_empty(),
        "entering past must clear the checks"
    );
    assert_eq!(
        model.preview(ListKind::Orders),
        None,
        "the segment change must clear the orders preview"
    );
    assert!(
        upcoming(&model).iter().any(|o| o.position_id == pid),
        "the order must still be in the upcoming list"
    );
    assert!(
        cancelled_tracks(&fake).is_empty(),
        "a segment switch must not cancel anything"
    );
}
