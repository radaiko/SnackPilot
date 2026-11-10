//! Desktop workspace identities (desktop-ui-redesign D1/D2): host-gated
//! breakpoints, identity-stable previews/cursors with reconciliation, settings
//! projection across resizes, Esc precedence and duplicate-safe commands.
//! Demo mode only — no test reaches a real host.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use snackpilot_core::domain::MenuCategory;
use snackpilot_model::desktop::{
    command_allowed, layout_mode, nearest_surviving, DesktopCommand, EscapeAction, HostKind,
    KeyScope, LayoutMode,
};
use snackpilot_model::navigation::{ListKind, Screen, SettingsDetail, Tab};
use snackpilot_model::{
    menu_key, AppModel, BillingSource, FakePlatform, Platform, PlatformError, State,
};

const DEMO_USER: &str = "demo";
const DEMO_PASS: &str = "demo1234!";

/// How long a gated worker may block before the test declares a deadlock.
const GATE_TIMEOUT: Duration = Duration::from_secs(60);

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-desktop-{tag}-{}-{}",
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

/// Platform wrapper whose `notify` panics once armed: lets a test crash a
/// flow mid-way and prove the command claim is still released on the unwind.
struct PanicPlatform {
    inner: Arc<FakePlatform>,
    armed: AtomicBool,
}

impl PanicPlatform {
    fn new(inner: Arc<FakePlatform>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            armed: AtomicBool::new(false),
        })
    }

    /// Make the next `notify` panic.
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }
}

impl Platform for PanicPlatform {
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
        if self.armed.load(Ordering::SeqCst) {
            panic!("armed platform crashed on {command:?}");
        }
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

fn item_in(model: &AppModel, day: &str, category: MenuCategory) -> MenuItemLike {
    let snap = model.state().menu_snapshot.clone().expect("menus loaded");
    snap.items
        .into_iter()
        .filter(|i| i.day == day)
        .find(|i| i.category == category)
        .unwrap_or_else(|| panic!("no {} item on {day}", category.display()))
}

type MenuItemLike = snackpilot_core::domain::MenuItem;

/// Stable ids of the selected day's menu rows, in list order.
fn menu_ids(model: &AppModel) -> Vec<String> {
    let day = model.state().selected_day.clone().expect("day selected");
    model
        .state()
        .menu_snapshot
        .clone()
        .expect("menus loaded")
        .items
        .into_iter()
        .filter(|i| i.day == day)
        .map(|i| menu_key(&i))
        .collect()
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
fn order_one(model: &AppModel, category: MenuCategory) -> MenuItemLike {
    let day = future_day(model);
    let item = item_in(model, &day, category);
    model.toggle(&item);
    model.submit_orders();
    item
}

fn upcoming(model: &AppModel) -> Vec<snackpilot_core::domain::OrderedMenu> {
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

/// A day other than the selected one, for context-change tests.
fn other_day(model: &AppModel) -> String {
    let state = model.state();
    let selected = state.selected_day.clone().expect("day selected");
    state
        .menu_snapshot
        .clone()
        .expect("menus loaded")
        .available_dates
        .into_iter()
        .find(|d| *d != selected)
        .expect("demo offers more than one day")
}

#[test]
/// @req REQ-UI-04
/// @req REQ-UI-07
fn desktop_breakpoint_is_host_gated() {
    // Pure rule: only a Desktop host may ever leave Phone, whatever it reports.
    assert_eq!(layout_mode(HostKind::Phone, 1920.0), LayoutMode::Phone);
    assert_eq!(layout_mode(HostKind::Tablet, 1920.0), LayoutMode::Phone);
    assert_eq!(layout_mode(HostKind::Desktop, 719.0), LayoutMode::Phone);
    assert_eq!(
        layout_mode(HostKind::Desktop, 720.0),
        LayoutMode::DesktopCompact
    );
    assert_eq!(
        layout_mode(HostKind::Desktop, 1023.0),
        LayoutMode::DesktopCompact
    );
    assert_eq!(
        layout_mode(HostKind::Desktop, 1024.0),
        LayoutMode::DesktopExpanded
    );

    // Through the model: a tablet reporting a desktop width stays Phone.
    let (model, _fake) = demo_model("breakpoint");
    model.set_host_kind(HostKind::Tablet);
    model.set_content_width(1440.0);
    assert_eq!(model.layout_mode(), LayoutMode::Phone);
    assert!(!model.is_wide());
    model.set_host_kind(HostKind::Desktop);
    assert_eq!(model.layout_mode(), LayoutMode::DesktopExpanded);
    assert!(model.is_wide());
    model.set_content_width(800.0);
    assert_eq!(model.layout_mode(), LayoutMode::DesktopCompact);
    assert!(
        !model.is_wide(),
        "compact counts as narrow for the projection"
    );
}

#[test]
/// @req REQ-UI-10
fn preview_does_not_mutate() {
    let (model, fake) = demo_model("preview-readonly");
    let day = model.state().selected_day.clone().unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu1);
    let key = menu_key(&item);
    let calls_before = fake.calls().len();
    let tracks_before = fake.tracks().len();

    model.begin_order_selection();
    model.open_preview(ListKind::Menus, &key);
    model.open_preview(ListKind::Orders, "pos-1");

    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str())
    );
    assert_eq!(
        model.preview(ListKind::Orders).as_deref(),
        Some("pos-1"),
        "orders preview is keyed by position_id, independent of the menus preview"
    );
    assert!(
        !model.has_pending_changes(),
        "opening a preview must never toggle a pending order"
    );
    assert!(
        model.state().selected_order_ids.is_empty(),
        "opening a preview must never check a batch order"
    );
    assert!(
        model.state().order_selection_mode,
        "opening a preview must not exit batch mode"
    );
    assert_eq!(
        fake.calls().len(),
        calls_before,
        "opening a preview must not fetch anything"
    );
    assert_eq!(
        fake.tracks().len(),
        tracks_before,
        "opening a preview must not track anything"
    );
}

#[test]
/// @req REQ-UI-10
fn refresh_keeps_stable_identity() {
    let (model, _fake) = demo_model("stable-id");
    let day = model.state().selected_day.clone().unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu1);
    let key = menu_key(&item);
    assert_eq!(
        key,
        format!("{}|{}", item.id, item.day),
        "the menu identity is the core composite key id|day, never a row index"
    );

    model.open_preview(ListKind::Menus, &key);
    model.set_cursor(ListKind::Menus, Some(key.clone()));
    let previous = menu_ids(&model);
    assert!(previous.contains(&key));

    model.refresh_menus();

    let surviving = menu_ids(&model);
    assert_eq!(
        previous, surviving,
        "a refresh republishes the same rows with the same identities"
    );
    assert!(
        !model.reconcile_list(ListKind::Menus, &previous, &surviving),
        "a surviving identity keeps its preview"
    );
    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str())
    );
    assert_eq!(model.cursor(ListKind::Menus).as_deref(), Some(key.as_str()));
}

#[test]
/// @req REQ-UI-10
fn removed_identity_clears_preview() {
    let (model, _fake) = demo_model("removed-id");
    let previous: Vec<String> = ["r1", "r2", "r3", "r4"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let surviving: Vec<String> = ["r1", "r3", "r4"].iter().map(|s| s.to_string()).collect();

    assert_eq!(
        nearest_surviving(&previous, &surviving, "r2"),
        Some("r1".to_string()),
        "equidistant neighbours fall back to the one earlier in the surviving list"
    );
    assert_eq!(
        nearest_surviving(&previous, &surviving, "gone"),
        None,
        "an anchor that was never in the previous list has no fallback"
    );

    model.open_preview(ListKind::Menus, "r2");
    model.set_cursor(ListKind::Menus, Some("r2".to_string()));
    assert!(
        model.reconcile_list(ListKind::Menus, &previous, &surviving),
        "a removed identity must report that it cleared the preview"
    );
    assert_eq!(model.preview(ListKind::Menus), None);
    assert_eq!(
        model.cursor(ListKind::Menus).as_deref(),
        Some("r1"),
        "the cursor moves to the nearest surviving row without activating it"
    );

    // No preview open: reconciliation is a no-op and reports nothing cleared.
    assert!(!model.reconcile_list(ListKind::Menus, &previous, &surviving));
}

#[test]
/// @req REQ-UI-10
fn context_change_clears_only_preview() {
    let (model, _fake) = demo_model("ctx");
    let day = model.state().selected_day.clone().unwrap();
    let key = menu_key(&item_in(&model, &day, MenuCategory::Menu1));

    model.open_preview(ListKind::Menus, &key);
    model.set_cursor(ListKind::Menus, Some(key.clone()));
    model.open_preview(ListKind::Orders, "pos-x");
    model.set_cursor(ListKind::Orders, Some("pos-x".to_string()));
    model.open_preview(ListKind::Receipts, "g-1");
    model.set_cursor(ListKind::Receipts, Some("g-1".to_string()));

    // Day change: the menus rows are a different list; cursor stays.
    model.select_day(&other_day(&model));
    assert_eq!(model.preview(ListKind::Menus), None);
    assert_eq!(model.cursor(ListKind::Menus).as_deref(), Some(key.as_str()));
    assert_eq!(model.preview(ListKind::Orders).as_deref(), Some("pos-x"));
    assert_eq!(model.preview(ListKind::Receipts).as_deref(), Some("g-1"));

    // Billing source filter: only the receipts list is a new context.
    model.open_preview(ListKind::Menus, &key);
    model.set_billing_source(BillingSource::Gourmet);
    assert_eq!(model.preview(ListKind::Receipts), None);
    assert_eq!(
        model.cursor(ListKind::Receipts).as_deref(),
        Some("g-1"),
        "a context change clears the preview but keeps the cursor"
    );
    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str())
    );
    assert_eq!(model.preview(ListKind::Orders).as_deref(), Some("pos-x"));

    // Orders segment switch: only the orders preview goes.
    model.set_orders_segment(true);
    assert_eq!(model.preview(ListKind::Orders), None);
    assert_eq!(
        model.preview(ListKind::Menus).as_deref(),
        Some(key.as_str())
    );
}

#[test]
/// @req REQ-UI-04
/// @req REQ-UI-07
fn resize_preserves_settings_projection() {
    let (model, _fake) = demo_model("resize");
    model.set_host_kind(HostKind::Desktop);
    model.set_content_width(800.0);
    assert_eq!(model.layout_mode(), LayoutMode::DesktopCompact);

    model.select_tab(Tab::Settings);
    model.open_settings_detail(SettingsDetail::Security);
    assert_eq!(model.settings_detail(), Some(SettingsDetail::Security));
    assert_eq!(
        model.navigation().top(),
        Screen::SettingsDetail,
        "narrow layouts project the open detail as a pushed screen"
    );
    assert_eq!(
        model.navigation().depth(),
        2,
        "the detail is pushed exactly once"
    );

    model.set_content_width(1280.0);
    assert!(model.is_wide());
    assert_eq!(
        model.settings_detail(),
        Some(SettingsDetail::Security),
        "the detail identity survives the resize"
    );
    assert_eq!(
        model.navigation().top(),
        Screen::Settings,
        "wide layouts drop the push and show containment instead"
    );
    assert_eq!(model.navigation().depth(), 1);

    model.set_content_width(719.0);
    assert_eq!(model.navigation().top(), Screen::SettingsDetail);
    assert_eq!(
        model.navigation().depth(),
        2,
        "narrow again: pushed, never duplicated"
    );
    model.set_content_width(760.0);
    assert_eq!(
        model.navigation().depth(),
        2,
        "further resizes must not push a second detail screen"
    );

    model.close_settings_detail();
    assert_eq!(model.settings_detail(), None);
    assert_eq!(model.navigation().top(), Screen::Settings);
    assert_eq!(model.navigation().depth(), 1);
}

#[test]
/// @req REQ-UI-11
fn escape_precedence_preserves_pending() {
    let (model, fake) = demo_model("escape");
    let day = model.state().selected_day.clone().unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu1);
    let key = menu_key(&item);
    model.toggle(&item);
    assert!(model.has_pending_changes());

    // 1. A modal wins and the model only reports it; the app owns closing it.
    model.set_modal_open(true);
    assert_eq!(model.escape(), EscapeAction::ClosedModal);
    assert!(
        model.state().modal_open,
        "the model must not close the app's modal"
    );
    model.set_modal_open(false);

    // 2. Batch selection exits next, without cancelling anything.
    model.begin_order_selection();
    model.toggle_order_selection("pos-1");
    assert_eq!(model.escape(), EscapeAction::EndedBatch);
    assert!(!model.state().order_selection_mode);
    assert!(model.state().selected_order_ids.is_empty());
    assert!(
        !fake.calls().iter().any(|c| c == "track(order.cancelled)"),
        "leaving batch mode must not cancel anything"
    );

    // 3. Then the active tab's preview; the cursor stays for re-activation.
    model.open_preview(ListKind::Menus, &key);
    model.set_cursor(ListKind::Menus, Some(key.clone()));
    assert_eq!(model.escape(), EscapeAction::ClosedPreview);
    assert_eq!(model.preview(ListKind::Menus), None);
    assert_eq!(model.cursor(ListKind::Menus).as_deref(), Some(key.as_str()));

    // 4. Then the settings detail.
    model.open_settings_detail(SettingsDetail::Notifications);
    assert_eq!(model.escape(), EscapeAction::ClosedSettingsDetail);
    assert_eq!(model.settings_detail(), None);

    // 5. Nothing left to unwind.
    assert_eq!(model.escape(), EscapeAction::Nothing);

    // Through the whole chain, pending menu changes were never touched.
    assert!(
        model.has_pending_changes(),
        "Esc must never drop pending menu changes"
    );
    assert_eq!(
        model
            .state()
            .menu_snapshot
            .clone()
            .unwrap()
            .pending_orders
            .len(),
        1
    );
}

#[test]
/// @req REQ-UI-11
fn scoped_keys_do_not_capture_inputs() {
    // Per-command scoping (REQ-UI-11): refresh only on the tabs that own a
    // reload, navigation on every tab, nothing outside the Workspace scope.
    assert!(command_allowed(
        DesktopCommand::Refresh,
        KeyScope::Workspace,
        Tab::Menus
    ));
    assert!(command_allowed(
        DesktopCommand::Refresh,
        KeyScope::Workspace,
        Tab::Orders
    ));
    assert!(command_allowed(
        DesktopCommand::Refresh,
        KeyScope::Workspace,
        Tab::Billing
    ));
    assert!(command_allowed(
        DesktopCommand::Retry,
        KeyScope::Workspace,
        Tab::Billing
    ));
    assert!(
        !command_allowed(DesktopCommand::Refresh, KeyScope::Workspace, Tab::Settings),
        "Cmd+R has no flow on Einstellungen and must not be captured"
    );
    assert!(
        !command_allowed(DesktopCommand::Retry, KeyScope::Workspace, Tab::Settings),
        "Retry is the Refresh twin and shares its gate"
    );
    assert!(
        command_allowed(
            DesktopCommand::OpenSettings,
            KeyScope::Workspace,
            Tab::Menus
        ),
        "Cmd+1–4 work on every tab"
    );
    assert!(command_allowed(
        DesktopCommand::OpenMenus,
        KeyScope::Workspace,
        Tab::Settings
    ));
    assert!(command_allowed(
        DesktopCommand::OrderToday,
        KeyScope::Workspace,
        Tab::Billing
    ));
    assert!(
        !command_allowed(DesktopCommand::OrderToday, KeyScope::TextInput, Tab::Menus),
        "workspace shortcuts must be inert while a text field has focus"
    );
    assert!(
        !command_allowed(DesktopCommand::Refresh, KeyScope::TextInput, Tab::Menus),
        "no shortcut may steal keys from a focused text field"
    );
    assert!(
        !command_allowed(DesktopCommand::OpenMenus, KeyScope::Modal, Tab::Menus),
        "a modal captures keys before the workspace"
    );
    assert!(
        !command_allowed(DesktopCommand::Refresh, KeyScope::Modal, Tab::Billing),
        "a modal captures keys before the workspace, even on a reloadable tab"
    );

    let (model, _fake) = demo_model("scope");
    assert_eq!(model.key_scope(), KeyScope::Workspace);

    model.set_text_input_focused(true);
    assert_eq!(model.key_scope(), KeyScope::TextInput);
    assert!(!model.handle_command(DesktopCommand::Refresh));
    assert_eq!(
        model.claimed_command(),
        None,
        "a rejected command claims nothing"
    );
    assert!(
        !model.state().busy,
        "a rejected command must not start a flow"
    );

    model.set_text_input_focused(false);
    model.set_modal_open(true);
    assert_eq!(model.key_scope(), KeyScope::Modal);
    assert!(!model.handle_command(DesktopCommand::OrderToday));
    assert_eq!(model.claimed_command(), None);

    model.set_modal_open(false);
    assert!(model.handle_command(DesktopCommand::OpenOrders));
    assert_eq!(model.navigation().active_tab(), Tab::Orders);
    assert_eq!(
        model.claimed_command(),
        None,
        "synchronous commands release their claim before returning"
    );
}

#[test]
/// @req REQ-UI-11
fn repeat_or_busy_command_is_rejected() {
    let fake = Arc::new(FakePlatform::new());
    let (gate, entered, release_tx) = GatePlatform::new(fake.clone(), "track(order.cancelled)");
    let model = demo_model_in(temp_dir("repeat"), gate);
    order_one(&model, MenuCategory::Menu1);
    order_one(&model, MenuCategory::Menu2);
    let orders = upcoming(&model);
    assert_eq!(orders.len(), 2, "two orders needed for the duplicate test");

    model.begin_order_selection();
    model.toggle_order_selection(&orders[0].position_id);
    model.toggle_order_selection(&orders[1].position_id);
    model.open_preview(ListKind::Orders, &orders[0].position_id);

    let accepted = std::thread::scope(|s| {
        let worker = s.spawn(|| model.handle_command(DesktopCommand::CancelSelected));
        entered
            .recv_timeout(GATE_TIMEOUT)
            .expect("batch cancel never reached the analytics track");
        // Assertions under catch_unwind so a failure still releases the gate.
        let mid = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(
                model.claimed_command().as_deref(),
                Some("order.cancel_selected"),
                "the running flow must hold its command claim"
            );
            assert!(
                !model.handle_command(DesktopCommand::CancelSelected),
                "a duplicate tap while the batch runs must be rejected, not queued"
            );
            assert!(
                !model.handle_command(DesktopCommand::Refresh),
                "no command may start while another is claimed"
            );
        }));
        release_tx.send(()).unwrap();
        let accepted = worker.join().unwrap();
        mid.unwrap();
        accepted
    });
    assert!(accepted, "the first command was accepted and dispatched");

    assert_eq!(
        cancelled_tracks(&fake),
        vec!["2".to_string()],
        "the double tap ran exactly ONE batch operation with the full selection"
    );
    assert!(upcoming(&model).is_empty(), "both orders cancelled");
    assert_eq!(
        model.claimed_command(),
        None,
        "the flow released its claim when it finished"
    );

    // The claim is free again; a fresh command with an empty selection is
    // rejected by the flow itself and must not leave a claim behind.
    assert!(!model.handle_command(DesktopCommand::CancelSelected));
    assert_eq!(model.claimed_command(), None);
    assert!(!model.state().busy);
}

#[test]
/// @req REQ-UI-11
fn refresh_dispatches_the_flow_of_the_active_tab() {
    use chrono::{Local, Timelike};
    use chrono_tz::Europe::Vienna;

    // The Bestellungen phase proves "the orders flow ran" through the
    // post-fetch notification decisions; the core suppresses them at/after
    // the reminder target minute, so that one minute is skipped explicitly.
    let vienna_now = Local::now().with_timezone(&Vienna);
    if (vienna_now.hour(), vienna_now.minute()) == (23, 59) {
        eprintln!("skipped: Vienna is at 23:59, no future reminder minute exists");
        return;
    }

    let (model, fake) = demo_model("refresh-tab");
    // Only the enabled daily reminder makes an orders fetch reach notify; the
    // toggle itself delivers one command, so the baseline is taken after it.
    model.set_daily_reminder_time(23, 59);
    model.set_daily_reminder_enabled(true);

    // Every refresh flow is synchronous, so a state subscriber is the only
    // way to see WHICH pipeline ran: busy belongs to the menus reload,
    // billing_loading to the billing reload, notifications to the orders load.
    let saw_busy = Arc::new(AtomicBool::new(false));
    let saw_billing = Arc::new(AtomicBool::new(false));
    {
        let (busy, billing) = (Arc::clone(&saw_busy), Arc::clone(&saw_billing));
        model.subscribe(Box::new(move |s: &State| {
            if s.busy {
                busy.store(true, Ordering::SeqCst);
            }
            if s.billing_loading {
                billing.store(true, Ordering::SeqCst);
            }
        }));
    }
    let reset = || {
        saw_busy.store(false, Ordering::SeqCst);
        saw_billing.store(false, Ordering::SeqCst);
    };

    // Menüs → the menus reload: the only refresh flow that toggles busy.
    model.select_tab(Tab::Menus);
    let notes = fake.notifications().len();
    reset();
    assert!(
        model.handle_command(DesktopCommand::Refresh),
        "Cmd+R on Menüs is accepted"
    );
    assert!(
        saw_busy.load(Ordering::SeqCst),
        "Cmd+R on Menüs must run the menus reload (busy)"
    );
    assert!(
        !saw_billing.load(Ordering::SeqCst),
        "the billing reload must not run on Menüs"
    );
    assert_eq!(
        fake.notifications().len(),
        notes,
        "the menus reload must not notify"
    );
    assert_eq!(
        model.claimed_command(),
        None,
        "the menus flow released its claim"
    );

    // Bestellungen → the orders reload: the only flow delivering the
    // post-fetch notification decisions.
    model.select_tab(Tab::Orders);
    reset();
    assert!(
        model.handle_command(DesktopCommand::Refresh),
        "Cmd+R on Bestellungen is accepted"
    );
    assert!(
        fake.notifications().len() > notes,
        "Cmd+R on Bestellungen must run the orders load (post-fetch notifications)"
    );
    assert!(
        !saw_busy.load(Ordering::SeqCst),
        "the menus reload must not run on Bestellungen"
    );
    assert!(
        !saw_billing.load(Ordering::SeqCst),
        "the billing reload must not run on Bestellungen"
    );
    assert_eq!(
        model.claimed_command(),
        None,
        "the orders flow released its claim"
    );

    // Abrechnung → the billing reload: the only flow toggling billing_loading.
    let notes = fake.notifications().len();
    model.select_tab(Tab::Billing);
    reset();
    assert!(
        model.handle_command(DesktopCommand::Refresh),
        "Cmd+R on Abrechnung is accepted"
    );
    assert!(
        saw_billing.load(Ordering::SeqCst),
        "Cmd+R on Abrechnung must run the billing reload (billing_loading)"
    );
    assert!(
        !saw_busy.load(Ordering::SeqCst),
        "the menus reload must not run on Abrechnung"
    );
    assert_eq!(
        fake.notifications().len(),
        notes,
        "the billing reload must not notify"
    );
    assert_eq!(
        model.claimed_command(),
        None,
        "the billing flow released its claim"
    );

    // Einstellungen → no reload exists: rejected, claimed nothing, ran nothing.
    model.select_tab(Tab::Settings);
    reset();
    assert!(
        !model.handle_command(DesktopCommand::Refresh),
        "Cmd+R must be inert on Einstellungen"
    );
    assert!(
        !saw_busy.load(Ordering::SeqCst) && !saw_billing.load(Ordering::SeqCst),
        "no flow may start from Einstellungen"
    );
    assert_eq!(
        fake.notifications().len(),
        notes,
        "the rejected command must not run any flow"
    );
    assert_eq!(
        model.claimed_command(),
        None,
        "a rejected command claims nothing"
    );
}

#[test]
/// @req REQ-UI-11
fn failing_flow_leaves_no_claim_behind() {
    use chrono::{Local, Timelike};
    use chrono_tz::Europe::Vienna;

    // The crash rides the daily-reminder decision delivered by the post-submit
    // orders load; the core suppresses it at/after the target minute.
    let vienna_now = Local::now().with_timezone(&Vienna);
    if (vienna_now.hour(), vienna_now.minute()) == (23, 59) {
        eprintln!("skipped: Vienna is at 23:59, no future reminder minute exists");
        return;
    }

    let fake = Arc::new(FakePlatform::new());
    let crash = PanicPlatform::new(Arc::clone(&fake));
    let platform: Arc<dyn Platform> = crash.clone();
    let model = demo_model_in(temp_dir("claim-crash"), platform);

    // Everything that legitimately notifies runs while disarmed; enabling the
    // reminder delivers its ScheduleAt through the live platform.
    model.set_daily_reminder_time(23, 59);
    model.set_daily_reminder_enabled(true);
    crash.arm();

    let day = future_day(&model);
    let item = item_in(&model, &day, MenuCategory::Menu1);
    model.toggle(&item);

    // The submit flow reaches platform.notify inside its orders reload and
    // dies there; the drop guard must release the claim on the unwind.
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        model.handle_command(DesktopCommand::OrderToday)
    }));
    assert!(
        ran.is_err(),
        "the armed notify must crash the submit flow mid-way"
    );
    assert_eq!(
        model.claimed_command(),
        None,
        "a crashed flow must not wedge the duplicate-command guard"
    );
    // busy stays true by design here: the crashed flow never reached its
    // busy_off update, which is exactly why the claim guard must not stick.
}
