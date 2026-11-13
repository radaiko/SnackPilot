//! App shell — docs/requirements/04-ui-ux.md §2 (tab bar), §3.4 (settings),
//! §3.5 (back affordance), §4.1 (dialogs), §4.2 (loading), §4.8 (toast).
//!
//! One `Root` (mounted by `crate::mount`) hosts this view: the four-tab shell
//! over a push/pop stack from [`snackpilot_model::navigation`], with the
//! overlays painted last (gpui has no `z_index`). Safe-area insets come from
//! the host (gpui-mobile), the desktop has none.
//! @req REQ-UI-03 @req REQ-UI-04 @req REQ-UI-07

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::*;
use gpui_kit::component::input::InputState;
#[cfg(target_os = "ios")]
use gpui_kit::component::WindowExt;
use snackpilot_model::desktop::{HostKind, LayoutMode};
use snackpilot_model::navigation::{
    ListKind, Navigation, Screen, SettingsDetail as ModelSettingsDetail, Tab,
};
use snackpilot_model::{format_day_short, AppModel, State};

use crate::components::broadcast_banner::broadcast_banner;
use crate::components::day_navigator::day_navigator;
use crate::components::desktop_navigation::desktop_rail;
use crate::components::dialog::{
    dialog_key, dialog_move, dialog_overlay, Dialog, DialogKey, DialogResolution,
};
use crate::components::loading_overlay::{desktop_loading_overlay, loading_overlay};
use crate::components::record_table::RecordListFocus;
use crate::components::tab_bar::{
    reserved_height, tab_bar, TabSelect, BAR_HEIGHT, IOS_PILL_HEIGHT,
};
use crate::components::toast::{
    desktop_toast, toast_overlay_for, Toast, ToastPhase, HOLD_DURATION, SLIDE_DURATION,
};
use crate::components::workspace::workspace_header;
use crate::navigation::ClickHandler;
use crate::screens::appearance::appearance_screen;
use crate::screens::billing::{billing_tab, desktop_billing_tab};
use crate::screens::login::{login_screen, LoginKind};
use crate::screens::menus::{desktop_menus_tab, menus_dock, menus_dock_height, menus_tab};
use crate::screens::notifications::notifications_screen;
use crate::screens::orders::{cutoff_repaint_due, desktop_orders_tab, orders_tab};
use crate::screens::settings::{desktop_settings_tab, placeholder_detail, settings_tab};
use crate::theme::{self, AppTheme};

/// The pushed settings screens (one detail at a time under
/// `Screen::SettingsDetail`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Detail {
    Settings,
    LoginKantine,
    LoginAutomaten,
    Appearance,
    Notifications,
}

/// App screen → the model-owned settings-detail kind. The model is
/// authoritative (REQ-UI-10); this is the presentation name of the same
/// section, not a second source of truth.
fn model_settings_detail(detail: Detail) -> Option<ModelSettingsDetail> {
    match detail {
        Detail::Settings => None,
        Detail::LoginKantine => Some(ModelSettingsDetail::Profile),
        Detail::LoginAutomaten => Some(ModelSettingsDetail::Security),
        Detail::Appearance => Some(ModelSettingsDetail::Data),
        Detail::Notifications => Some(ModelSettingsDetail::Notifications),
    }
}

/// Model-owned settings-detail kind → the app screen that renders it.
fn detail_from_model(detail: Option<ModelSettingsDetail>) -> Detail {
    match detail {
        Some(ModelSettingsDetail::Profile) => Detail::LoginKantine,
        Some(ModelSettingsDetail::Security) => Detail::LoginAutomaten,
        Some(ModelSettingsDetail::Data) => Detail::Appearance,
        Some(ModelSettingsDetail::Notifications) => Detail::Notifications,
        Some(ModelSettingsDetail::Advanced) | None => Detail::Settings,
    }
}

/// iOS keyboard driver decision: `Some(true)` show, `Some(false)` hide,
/// `None` no change (§3.6 typing proof, task 5.4).
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn keyboard_action(up: bool, focused: bool) -> Option<bool> {
    (focused != up).then_some(focused)
}

/// Debug knob (`--log-kb`): appends keyboard-driver transitions to the app's
/// temp dir so the UI test can tell whether the driver or UIKit dropped the
/// keyboard (§3.6 typing proof, task 5.4). Read on the host via
/// `xcrun simctl get_app_container <udid> <bundle> data`/tmp/sp54-kb.log.
pub(crate) fn kb_log(line: &str) {
    #[cfg(target_os = "ios")]
    if std::env::args().any(|a| a == "--log-kb") {
        use std::io::Write;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("sp54-kb.log"))
        {
            let _ = writeln!(f, "{stamp} {line}");
        }
    }
    #[cfg(not(target_os = "ios"))]
    let _ = line;
}

/// The workspace header's refresh action, one per tab (brief "Workspace
/// header": Menüs, Bestellungen and Abrechnung have one, Einstellungen none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefreshIntent {
    Menus,
    Orders,
    Billing,
}

/// Which refresh the active tab dispatches; `None` means no header action.
pub(crate) fn refresh_intent(tab: Tab) -> Option<RefreshIntent> {
    match tab {
        Tab::Menus => Some(RefreshIntent::Menus),
        Tab::Orders => Some(RefreshIntent::Orders),
        Tab::Billing => Some(RefreshIntent::Billing),
        Tab::Settings => None,
    }
}

/// Run one existing model refresh. These APIs block on the model's own
/// runtime, so the desktop shell calls them off the render thread.
fn run_refresh(model: &AppModel, intent: RefreshIntent) {
    match intent {
        RefreshIntent::Menus => model.refresh_menus(),
        RefreshIntent::Orders => model.refresh_orders(),
        RefreshIntent::Billing => model.reload_billing(),
    }
}

/// Whether entering `requested` must create fresh credential inputs. Re-entering
/// the same open form, or crossing 719↔720 while it is open, keeps the one
/// instance and its unsaved text (REQ-UI-10); leaving and returning creates a
/// fresh prefill.
pub(crate) fn should_create_inputs(current: Detail, requested: Detail, has_inputs: bool) -> bool {
    matches!(requested, Detail::LoginKantine | Detail::LoginAutomaten)
        && (current != requested || !has_inputs)
}

/// Where a closing dialog hands focus back (REQ-UI-14). The captured control is
/// exact while the record it belonged to still exists. A confirmed flow that
/// deleted that record also unmounts the control, so the wide workspace hands
/// focus to the nearest surviving row of the same list — the identity the
/// model reconciled its cursor to — and only falls back to the screen heading
/// when no row survives at all. The narrow shell keeps the incumbent restore
/// because it renders no persistent heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusRestore {
    Origin,
    Nearest,
    Heading,
}

/// Which row a closing dialog may hand focus to: the model reconciled cursor,
/// but only while this render shows that row. A refresh, filter or segment
/// change can leave the cursor naming an unmounted record whose handle the
/// list has pruned; the restore then falls back instead of focusing a dead
/// handle (REQ-UI-14).
pub(crate) fn nearest_restore_cursor<'a>(
    cursor: Option<&'a str>,
    rendered: &[String],
) -> Option<&'a str> {
    cursor.filter(|id| rendered.iter().any(|visible| visible == id))
}

pub(crate) fn focus_restore(
    desktop: bool,
    origin_record_survives: bool,
    nearest_row_survives: bool,
) -> FocusRestore {
    if !desktop || origin_record_survives {
        FocusRestore::Origin
    } else if nearest_row_survives {
        FocusRestore::Nearest
    } else {
        FocusRestore::Heading
    }
}

/// Whether the shared intent boundary dispatches its blocking operation off
/// the render thread (task 1.2 acceptance, completed in 1.5). This follows the
/// **host**, not the rail: every desktop process counts, including a narrow
/// desktop window that projects as `LayoutMode::Phone`; only a phone or tablet
/// host keeps the incumbent synchronous path.
pub(crate) fn dispatch_off_thread(host: HostKind) -> bool {
    host == HostKind::Desktop
}

/// Run one blocking model flow on the host's thread: off-thread on a desktop
/// host so cached content keeps painting, inline on a phone/tablet host
/// (incumbent mobile behaviour). No extra request, no second flow.
pub(crate) fn run_blocking(host: HostKind, cx: &mut App, action: impl FnOnce() + Send + 'static) {
    if dispatch_off_thread(host) {
        cx.background_executor()
            .spawn(async move { action() })
            .detach();
    } else {
        action();
    }
}

/// How one Menüs row activation behaves per host (task 1.3): the phone shell
/// keeps the incumbent card/FAB toggle, while the desktop shell only previews,
/// so selecting a row can never add or remove a pending change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuRowAction {
    Toggle,
    Preview,
}

/// The Menüs workspace the shell renders: the desktop split (preview only) on
/// every desktop layout, the incumbent mobile tab on the phone shell.
fn menu_row_action(layout: LayoutMode) -> MenuRowAction {
    if layout == LayoutMode::Phone {
        MenuRowAction::Toggle
    } else {
        MenuRowAction::Preview
    }
}

/// How one Bestellungen row activation behaves on the desktop shell (task
/// 1.4): the 44 px batch-check column toggles that one ID, while the row body
/// only previews, so activating a row can never approve, cancel or check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderRowEffect {
    Preview,
    ToggleCheck,
}

pub(crate) fn order_row_effect(on_checkbox: bool, selection_mode: bool) -> OrderRowEffect {
    if on_checkbox && selection_mode {
        OrderRowEffect::ToggleCheck
    } else {
        OrderRowEffect::Preview
    }
}

/// The batch-check column's handler decision, extracted so the click wiring
/// and its test share one rule: a check toggles its one ID only while the
/// selection mode is active; everywhere else it previews.
pub(crate) fn desktop_check_effect(selection_mode: bool) -> OrderRowEffect {
    order_row_effect(true, selection_mode)
}

/// The desktop detail's explicit cancel action (task 1.4): past orders offer
/// none, the affected order shows `Wird storniert`, the cutoff keeps the
/// action visible but disabled with its own notice, and any other blocking
/// dispatch leaves it inert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrderCancelState {
    Hidden,
    InFlight,
    Cutoff,
    Busy,
    Ready,
}

pub(crate) fn order_cancel_state(
    past: bool,
    cancelling_this: bool,
    cutoff: bool,
    blocked: bool,
) -> OrderCancelState {
    if past {
        OrderCancelState::Hidden
    } else if cancelling_this {
        OrderCancelState::InFlight
    } else if cutoff {
        OrderCancelState::Cutoff
    } else if blocked {
        OrderCancelState::Busy
    } else {
        OrderCancelState::Ready
    }
}

/// Whether a desktop mutation entry point must no-op: a submit or another
/// blocking operation is already dispatched, so a double activation cannot
/// toggle again or order twice (task 1.3 acceptance).
pub(crate) fn desktop_mutation_blocked(busy: bool, order_progress: bool) -> bool {
    busy || order_progress
}

/// The shell's in-flight claim for a dispatched menu mutation (task 1.3):
/// the model marks `busy` only after `submit_orders` is spawned, so the shell
/// claims synchronously before the spawn. The claim lives on the shell so the
/// blocked visual state covers the whole workspace, and it is released by the
/// ticket when the dispatch ends.
#[derive(Clone, Default)]
pub(crate) struct MutationGate(Arc<AtomicBool>);

impl MutationGate {
    /// Whether a dispatched mutation still holds the gate.
    pub(crate) fn is_claimed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    /// Claim the gate for one dispatch. `None` while a blocking operation
    /// runs or another dispatch still holds the claim, so a double activation
    /// cannot reach the core twice.
    pub(crate) fn claim(&self, busy: bool, order_progress: bool) -> Option<MutationTicket> {
        if desktop_mutation_blocked(busy, order_progress) {
            return None;
        }
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| MutationTicket(Arc::clone(&self.0)))
    }
}

/// Releases its [`MutationGate`] when the dispatched mutation ends, including
/// on an unwind (the model's `ClearClaim` has the same shape).
pub(crate) struct MutationTicket(Arc<AtomicBool>);

impl Drop for MutationTicket {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Dispatch a refresh at the shared intent boundary (task 1.2 acceptance):
/// on every desktop host, narrow windows included, the blocking model call
/// moves to the background executor, so the cached content keeps painting
/// while the existing flow runs. Nothing extra is requested — the same model
/// methods run, and their state notifications repaint the shell. A phone or
/// tablet host keeps the incumbent synchronous path.
fn dispatch_refresh(model: &Arc<AppModel>, intent: RefreshIntent, host: HostKind, cx: &mut App) {
    let model = Arc::clone(model);
    run_blocking(host, cx, move || run_refresh(&model, intent));
}

/// Safe-area insets (top, bottom, left, right) in logical px, read from the
/// host platform. Android asks the live window; iOS reads the FFI window list
/// with the v1 notch/home-indicator fallback (59/34) before the host reports
/// real insets; desktop has none.
fn safe_area() -> (f32, f32, f32, f32) {
    #[cfg(target_os = "android")]
    {
        if let Some(platform) = gpui_mobile::android::jni::platform() {
            if let Some(window) = platform.primary_window() {
                let insets = window.safe_area_insets_logical();
                return (insets.top, insets.bottom, insets.left, insets.right);
            }
        }
    }
    let (top, bottom, left, right) = gpui_mobile::safe_area_insets();
    if top == 0. && bottom == 0. && cfg!(target_os = "ios") {
        return (59., 34., 0., 0.);
    }
    (top, bottom, left, right)
}

/// Distance from the safe-area top to the compact heading, and from the
/// heading to the screen's own controls (design "Common mobile contract":
/// 16 below safe top, 16 above screen controls).
pub(crate) const HEADING_TOP: f32 = 16.;
pub(crate) const HEADING_GAP: f32 = 16.;
/// Scroll-end breathing room below the measured obstruction.
pub(crate) const SCROLL_END_GAP: f32 = 16.;

/// The compact shell's heading: the active tab's German title in the headline
/// role, supplied by the shell instead of every screen. A pushed settings
/// screen draws its own 48 px back row and title, so the tab heading is not
/// repeated there (design "Common mobile contract").
pub(crate) fn shell_heading(tab: Tab, pushed: bool) -> Option<&'static str> {
    (!pushed).then(|| tab.title())
}

/// The measured bottom obstruction a scrolled compact surface must clear
/// (REQ-UI-15/REQ-UI-16): the navigation height only when it is painted over the
/// content (the iOS floating pill), plus the dock's measured height when a dock
/// is present, plus the 16 px scroll-end gap. A bar in normal flow already owns
/// its height, so it is never counted twice, and no blind 100 px padding is
/// added anywhere in the shell.
pub(crate) fn obstruction_reservation(
    flat: bool,
    safe_bottom: f32,
    in_flow_navigation: bool,
    dock_height: f32,
) -> f32 {
    let navigation = if in_flow_navigation {
        0.
    } else {
        reserved_height(flat, safe_bottom)
    };
    navigation + dock_height.max(0.) + SCROLL_END_GAP
}

/// The top edge of the platform navigation above the window bottom: the
/// in-flow bar's own height plus its safe-area strip on Android, the floating
/// iOS pill's bottom inset plus its 56 px height. A compact surface's dock sits
/// [`crate::components::fab::DOCK_GAP`] above this line.
pub(crate) fn navigation_top(flat: bool, safe_bottom: f32) -> f32 {
    if flat {
        BAR_HEIGHT + safe_bottom
    } else {
        safe_bottom.max(8.) + IOS_PILL_HEIGHT
    }
}

/// A compact Menüs cached refresh keeps its cards, checks and day
/// (REQ-SCR-01): the neutral `Aktualisiere...` row is the only progress
/// feedback, so the blocking loading overlay is not painted over nonempty
/// cached content. An initial load without data still shows it.
pub(crate) fn cached_menu_refresh(
    top: Screen,
    snapshot: Option<&snackpilot_core::domain::MenuSnapshot>,
) -> bool {
    top == Screen::Menus
        && snapshot.is_some_and(|snapshot| snapshot.refreshing && !snapshot.items.is_empty())
}

/// Legacy screens that still paint their own bottom inset (`p(16)` + `pb(100)`
/// on the list/detail roots). Tasks 1.3–1.6 move them onto the shell; until
/// then the shell must not stack its measured reservation on top. This is the
/// tracked contract gap, not a second layout owner: once a screen drops its
/// padding it also drops out of this list and the measured value applies.
pub(crate) fn legacy_screen_owns_bottom_inset(top: Screen) -> bool {
    matches!(
        top,
        Screen::Orders | Screen::Billing | Screen::Settings | Screen::SettingsDetail
    )
}

/// The inset a legacy screen already paints (`.pb(px(100.))`). The shell tops
/// it up only when the measured clearance is larger, so the two never sum.
pub(crate) const LEGACY_SCREEN_BOTTOM_INSET: f32 = 100.;

/// The compact column's one bottom inset. A legacy screen already owns most of
/// it, so the shell contributes only the shortfall up to the measured target;
/// a migrated screen gets the measured reservation when the bar floats over it,
/// or only the 16 px scroll-end gap when the bar is in normal flow and already
/// owns its height.
pub(crate) fn compact_content_bottom(
    plan: &CompactShellPlan,
    in_flow: bool,
    screen_owns_inset: bool,
) -> f32 {
    let target = if in_flow {
        SCROLL_END_GAP
    } else {
        plan.reservation
    };
    if screen_owns_inset {
        (target - LEGACY_SCREEN_BOTTOM_INSET).max(0.)
    } else {
        target
    }
}

/// One compact-shell presentation plan: exactly one heading, one centred
/// content frame, one navigation surface and the incumbent active-tab identity,
/// with the safe-area top applied once and a measured obstruction reservation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CompactShellPlan {
    pub heading: Option<&'static str>,
    /// Total top offset of the heading, including the one safe-area top.
    pub heading_top: f32,
    pub heading_gap: f32,
    /// `min(viewport − 32, 560)`, or the 440 px form cap on a credential form.
    pub content_width: f32,
    /// Measured clearance the scrolled content keeps free at its end.
    pub reservation: f32,
    /// The platform navigation stays visible on pushed screens too.
    pub bar_visible: bool,
    /// A push never changes which destination is active.
    pub active: Tab,
    /// The shell applies the safe-area top once, outside the column.
    pub safe_top: f32,
}

/// Build the compact plan for one frame (REQ-UI-03/REQ-UI-04/REQ-UI-15). The
/// model still owns navigation, selection and identity; this only projects the
/// compact geometry the design fixes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compact_shell_plan(
    host: HostKind,
    reported_width: f32,
    viewport_width: f32,
    tab: Tab,
    top: Screen,
    form: bool,
    flat: bool,
    safe_top: f32,
    safe_bottom: f32,
    dock_height: f32,
) -> CompactShellPlan {
    let width = crate::compact_width(reported_width, viewport_width);
    let pushed = top == Screen::SettingsDetail;
    let geometry = crate::compact_column(host, width);
    CompactShellPlan {
        heading: shell_heading(tab, pushed),
        heading_top: HEADING_TOP,
        heading_gap: HEADING_GAP,
        // A credential form keeps the 440 px cap inside the same column; the
        // shell owns the cap so no screen derives its own.
        content_width: geometry
            .map(|g| if form { g.form } else { g.content })
            .unwrap_or(width),
        reservation: obstruction_reservation(
            flat,
            safe_bottom,
            /* in_flow_navigation: */ flat,
            dock_height,
        ),
        bar_visible: true,
        active: tab,
        safe_top,
    }
}

/// The compact shell heading row: headline role (24/32 Bold), on the pinned
/// primary ink, wrapping rather than ellipsizing.
fn shell_heading_row(title: &'static str, theme: &AppTheme) -> Div {
    div()
        .w_full()
        .pt(px(HEADING_TOP))
        .pb(px(HEADING_GAP))
        .child(
            div()
                .text_size(px(theme.mobile.headline))
                .line_height(px(theme.mobile.headline_line))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
}

/// The single root view: four tabs, one push/pop stack, dialogs, loading
/// overlay and the new-menu toast.
/// @req REQ-UI-04
pub struct AppShell {
    /// Fallback navigation for a host that installed no model. With a model
    /// present the model's `Navigation` is the only authority and this mirror
    /// is unused.
    nav: Navigation,
    dialog: Option<Dialog>,
    toast: Option<Toast>,
    pub(crate) model: Option<Arc<AppModel>>,
    /// In-flight claim for a dispatched menu mutation (task 1.3): taken
    /// synchronously before the spawn, so a double activation is inert even
    /// before the model reports `busy`.
    pub(crate) mutation_gate: MutationGate,
    /// Transient swipe-between-days gesture state (§3.1): pointer origin and
    /// the current (damped) horizontal offset.
    pub(crate) swipe_start: Option<f32>,
    pub(crate) swipe_start_y: Option<f32>,
    pub(crate) swipe_claimed: bool,
    pub(crate) swipe_dx: f32,
    /// Which pushed settings screen is shown.
    pub(crate) detail: Detail,
    /// Login form fields, created on push so typing survives re-renders.
    login_user: Option<Entity<InputState>>,
    login_pass: Option<Entity<InputState>>,
    /// Window activation watcher: a false→true transition is a foreground
    /// return and drives `AppModel::on_foreground` on every host (5.7).
    was_active: bool,
    seen_frame: bool,
    /// Debug knob (`--detail=login|automaten`): opens that credential screen
    /// on the first frame so a UI test can reach it without navigation.
    pending_detail: Option<Detail>,
    /// iOS: whether the soft keyboard is currently shown (§3.6, task 5.4).
    #[cfg_attr(not(target_os = "ios"), allow(dead_code))]
    keyboard_up: bool,
    /// The action row of the open dialog owns keyboard focus; the index moves
    /// only inside the row (REQ-UI-14).
    pub(crate) dialog_focus: usize,
    pub(crate) dialog_handles: [FocusHandle; 2],
    /// One resolution per open dialog, so a click, Return and backdrop never
    /// run the destructive closure twice.
    pub(crate) dialog_resolution: DialogResolution,
    /// Focus restored after the dialog closes, plus the record whose control
    /// held it: a flow that deletes that record also unmounts its handle
    /// (REQ-UI-14).
    dialog_origin: Option<FocusHandle>,
    dialog_origin_record: Option<(ListKind, String)>,
    dialog_open: bool,
    /// Roving focus inside the desktop settings master and the two appearance
    /// option groups (REQ-UI-11).
    pub(crate) settings_rows: [FocusHandle; 4],
    /// One tab stop and per-row handles for each wide record list (REQ-UI-11):
    /// Up/Down/Home/End move the list cursor, and a deleted origin row hands
    /// focus to its nearest surviving neighbour.
    pub(crate) menu_list: RecordListFocus,
    pub(crate) order_list: RecordListFocus,
    pub(crate) receipt_list: RecordListFocus,
    pub(crate) rail_rows: [FocusHandle; 4],
    /// Screen heading focus target for a changed destination (REQ-UI-11).
    heading_focus: FocusHandle,
    pub(crate) settings_master: FocusHandle,
    /// One tab stop for the Bestellungen segment group: Left/Right select the
    /// neighbouring segment without wrapping (REQ-UI-11).
    pub(crate) segment_group: FocusHandle,
    pub(crate) appearance_design: FocusHandle,
    pub(crate) appearance_accent: FocusHandle,
}

impl AppShell {
    /// Wrap the model (or nothing, when a host installed none) and subscribe
    /// to state changes: each notification wakes the 50 ms poll loop, which
    /// repaints the shell.
    pub fn new(model: Option<Arc<AppModel>>, cx: &mut Context<Self>) -> Self {
        if let Some(model) = model.as_ref() {
            // The subscriber must be Send + Sync, so it only flips a flag;
            // a 50 ms foreground poll repaints when it is set.
            let dirty = Arc::new(AtomicBool::new(false));
            model.subscribe(Box::new({
                let dirty = Arc::clone(&dirty);
                move |_| dirty.store(true, Ordering::Release)
            }));
            cx.spawn(
                async move |this: WeakEntity<AppShell>, cx: &mut AsyncApp| loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                    if dirty.swap(false, Ordering::AcqRel)
                        && this.update(cx, |_, cx| cx.notify()).is_err()
                    {
                        break;
                    }
                },
            )
            .detach();
            // Cutoff cadence (task 1.4): the desktop workspace re-evaluates
            // order eligibility every 30 s, so a disabled `Stornieren` flips
            // without waiting for pointer movement. Phone never repaints.
            cx.spawn(async move |this: WeakEntity<AppShell>, cx: &mut AsyncApp| {
                let mut last_check = Instant::now();
                loop {
                    cx.background_executor()
                        .timer(Duration::from_secs(30))
                        .await;
                    let elapsed = last_check.elapsed();
                    last_check = Instant::now();
                    if this
                        .update(cx, |shell, cx| {
                            let layout = shell
                                .model
                                .as_ref()
                                .map(|model| model.layout_mode())
                                .unwrap_or(LayoutMode::Phone);
                            if cutoff_repaint_due(layout, elapsed) {
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        // `mut` is only used by the debug-only evidence knobs below.
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut shell = AppShell {
            nav: Navigation::new(),
            dialog: None,
            toast: None,
            model,
            mutation_gate: MutationGate::default(),
            swipe_start: None,
            swipe_start_y: None,
            swipe_claimed: false,
            swipe_dx: 0.,
            detail: Detail::Settings,
            login_user: None,
            login_pass: None,
            was_active: true,
            seen_frame: false,
            pending_detail: None,
            keyboard_up: false,
            dialog_focus: 0,
            dialog_handles: [cx.focus_handle(), cx.focus_handle()],
            dialog_resolution: DialogResolution::default(),
            dialog_origin: None,
            dialog_origin_record: None,
            dialog_open: false,
            settings_rows: [
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
            ],
            rail_rows: [
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
            ],
            heading_focus: cx.focus_handle(),
            settings_master: cx.focus_handle(),
            segment_group: cx.focus_handle(),
            appearance_design: cx.focus_handle(),
            appearance_accent: cx.focus_handle(),
            menu_list: RecordListFocus::new(cx.focus_handle(), cx.focus_handle()),
            order_list: RecordListFocus::new(cx.focus_handle(), cx.focus_handle()),
            receipt_list: RecordListFocus::new(cx.focus_handle(), cx.focus_handle()),
        };
        // Evidence knobs (debug builds only, same entry as 4.1's --gallery):
        // `--dialog` opens a §4.1 confirm, `--toast` shows the §4.8 toast,
        // `--tab=N` selects a tab (simulator captures cannot tap).
        #[cfg(debug_assertions)]
        {
            // `--demo` signs in with the product's magic demo credentials so
            // simulator captures show the real screens offline (task 5.1
            // evidence; the login UI itself is 5.4).
            if std::env::args().any(|a| a == "--demo") {
                if let Some(model) = &shell.model {
                    let result = model.login("demo", "demo1234!");
                    log::info!("[5.1] demo login: {result:?}");
                }
            }
            let tab = std::env::args().find_map(|a| {
                let rest = a.strip_prefix("--tab=")?;
                rest.parse::<usize>().ok()
            });
            if let Some(n) = tab.filter(|n| *n < Tab::ALL.len()) {
                shell.select_tab(Tab::ALL[n]);
            }
            // `--detail=login|automaten` opens a credential screen directly.
            shell.pending_detail = std::env::args().find_map(|a| {
                let rest = a.strip_prefix("--detail=")?;
                Some(match rest {
                    "login" => Detail::LoginKantine,
                    "automaten" => Detail::LoginAutomaten,
                    "appearance" => Detail::Appearance,
                    "notifications" => Detail::Notifications,
                    _ => return None,
                })
            });
            if std::env::args().any(|a| a == "--dialog") {
                shell.set_dialog(
                    Dialog::Confirm {
                        title: "Abmelden?".into(),
                        message: "Möchtest du dich wirklich abmelden?".into(),
                        confirm_label: None,
                        cancel_label: None,
                        on_confirm: Rc::new(|_, _| {}),
                    },
                    cx,
                );
            }
            if std::env::args().any(|a| a == "--toast") {
                shell.show_toast(cx);
            }
        }
        // Evidence fixture selection (debug only, desktop entry): the host
        // bootstrap already ran, so the chosen synthetic scenario is applied
        // here on an isolated store. A phone capture selects and applies it
        // in `mobile.rs` before/after bootstrap respectively; a release build
        // has neither the knob nor the model entry.
        #[cfg(all(debug_assertions, not(any(target_os = "ios", target_os = "android"))))]
        if let Some(model) = shell.model.as_ref() {
            if let Some(scenario) = crate::evidence::scenario() {
                crate::evidence::apply_scenario(model, &scenario);
            }
        }
        shell
    }

    /// Show the §4.8 new-menu toast; the phase state machine
    /// (In → Hold → Out → gone) runs on chained background timers.
    /// The menus task wires the trigger once new-menu detection lands.
    #[allow(dead_code)]
    pub fn show_toast(&mut self, cx: &mut Context<Self>) {
        self.toast = Some(Toast {
            message: crate::components::toast::NEW_MENU_MESSAGE.into(),
            phase: ToastPhase::In,
        });
        cx.notify();
        cx.spawn(async move |this: WeakEntity<AppShell>, cx: &mut AsyncApp| {
            for phase in [ToastPhase::Hold, ToastPhase::Out] {
                let duration = match phase {
                    ToastPhase::Hold => SLIDE_DURATION,
                    _ => HOLD_DURATION,
                };
                cx.background_executor().timer(duration).await;
                let _ = this.update(cx, |this, cx| {
                    if let Some(toast) = this.toast.as_mut() {
                        toast.phase = phase;
                    }
                    cx.notify();
                });
            }
            cx.background_executor().timer(SLIDE_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            });
        })
        .detach();
    }

    /// The navigation the shell renders: the model's when a model is
    /// installed (the shared Root never keeps a second authoritative stack),
    /// otherwise the host's local mirror.
    fn navigation(&self) -> Navigation {
        self.model
            .as_ref()
            .map(|model| model.navigation())
            .unwrap_or_else(|| self.nav.clone())
    }

    /// Switch tabs through the model so the active tab, stack reset and
    /// preview cleanup are one model update (REQ-UI-04).
    fn select_tab(&mut self, tab: Tab) {
        if let Some(model) = &self.model {
            model.select_tab(tab);
        } else {
            self.nav.select_tab(tab);
        }
    }

    /// The pushed settings screen shown: derived from the model's
    /// `settings_detail` so the model is the only authority. The local
    /// fallback only serves a host that installed no model.
    fn current_detail(&self) -> Detail {
        match &self.model {
            Some(model) => detail_from_model(model.settings_detail()),
            None => self.detail,
        }
    }

    /// Push a settings detail screen; login details create fresh inputs.
    pub(crate) fn open_detail(
        &mut self,
        detail: Detail,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // One input instance per entry (REQ-UI-10/REQ-SCR-04): re-selecting
        // the open detail, or the 719→720→719 reprojection, must not recreate
        // the fields or re-prefill them from secure storage.
        let current = self.current_detail();
        let create_inputs = should_create_inputs(
            current,
            detail,
            self.login_user.is_some() && self.login_pass.is_some(),
        );
        if !create_inputs && current == detail {
            return;
        }
        self.detail = detail;
        if detail == Detail::Notifications {
            if let Some(model) = &self.model {
                model.refresh_diagnostic_log();
            }
        }
        if create_inputs {
            // §3.5: both fields pre-fill from the stored credentials.
            let (username, password) = self
                .model
                .as_ref()
                .map(|m| m.saved_credentials(detail == Detail::LoginAutomaten))
                .unwrap_or_default();
            self.login_user = Some(cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Benutzername eingeben")
                    .default_value(username.unwrap_or_default())
            }));
            self.login_pass = Some(cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Passwort eingeben")
                    // §3.6 secure entry: the field renders dots, never the text.
                    .masked(true)
                    .default_value(password.unwrap_or_default())
            }));
        }
        if let Some(model) = &self.model {
            // One model update owns both the detail identity and the narrow
            // projection's push (REQ-UI-10), so the view cannot disagree.
            match model_settings_detail(detail) {
                Some(model_detail) => model.open_settings_detail(model_detail),
                None => model.push_screen(Screen::SettingsDetail),
            }
        } else {
            self.nav.push(Screen::SettingsDetail);
        }
        cx.notify();
    }

    /// Pop back to the settings root.
    pub(crate) fn pop_detail(&mut self, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            model.close_settings_detail();
        } else {
            self.nav.pop();
        }
        self.detail = Detail::Settings;
        cx.notify();
    }

    pub(crate) fn set_dialog(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.dialog_focus = dialog.initial_focus();
        self.dialog_resolution.reset();
        self.dialog = Some(dialog);
        // iOS: an alert must not sit behind the soft keyboard (§4.1).
        #[cfg(target_os = "ios")]
        {
            kb_log("set_dialog hides keyboard");
            gpui_mobile::hide_keyboard();
            self.keyboard_up = false;
        }
        cx.notify();
    }

    /// The roving list that owns `kind`'s rows, so a closing dialog can hand
    /// focus back to a surviving row of the same list (REQ-UI-14).
    pub(crate) fn list_focus(&self, kind: ListKind) -> &RecordListFocus {
        match kind {
            ListKind::Menus => &self.menu_list,
            ListKind::Orders => &self.order_list,
            ListKind::Receipts => &self.receipt_list,
        }
    }

    /// Resolve the open dialog at `index` exactly once. Pointer, Return,
    /// Space and backdrop all come through here, so the destructive closure
    /// runs at most once even if the click lands twice before the repaint
    /// (REQ-UI-14).
    pub(crate) fn resolve_dialog(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let count = dialog.button_count();
        if self.dialog_resolution.begin(index, count).is_none() {
            self.dialog = Some(dialog);
            return;
        }
        if let Dialog::Confirm { on_confirm, .. } = &dialog {
            if index == 1 {
                on_confirm(window, cx);
            }
        }
        cx.notify();
    }

    /// Run one guarded desktop command from a shortcut or a native menu item
    /// (REQ-UI-11/14). Rejections are silent: the same eligibility the
    /// on-screen control uses decides.
    pub(crate) fn run_command(
        &mut self,
        command: snackpilot_model::desktop::DesktopCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let before = self
            .model
            .as_ref()
            .map(|model| model.state().navigation.active_tab());
        // One policy, and it lives in the model: the navigation chords are app
        // chords that work while a text field has focus and only a modal takes
        // them away, every other command keeps the guarded scope/claim path,
        // and native editing keys are never captured. The shell adds no
        // eligibility of its own; the only thing it adds is the thread: the
        // refresh flow blocks, so it moves off the render thread.
        let dispatched = match command {
            snackpilot_model::desktop::DesktopCommand::Refresh
            | snackpilot_model::desktop::DesktopCommand::Retry => {
                self.dispatch_refresh_command(command, cx)
            }
            _ => self
                .model
                .as_ref()
                .is_some_and(|model| model.handle_command(command)),
        };
        let after = self
            .model
            .as_ref()
            .map(|model| model.state().navigation.active_tab());
        // A changed destination moves focus to the new screen heading, never
        // to a destructive action; an active-tab no-op keeps focus (REQ-UI-11).
        if dispatched
            && before != after
            && matches!(
                command,
                snackpilot_model::desktop::DesktopCommand::OpenMenus
                    | snackpilot_model::desktop::DesktopCommand::OpenOrders
                    | snackpilot_model::desktop::DesktopCommand::OpenBilling
                    | snackpilot_model::desktop::DesktopCommand::OpenSettings
            )
        {
            let heading = self.heading_focus.clone();
            window.focus(&heading, cx);
        }
        cx.notify();
    }

    /// Cmd+R and the native Aktualisieren item: the model keeps the scope
    /// eligibility, the busy guard and the command claim, the shell takes its
    /// mutation gate for the pre-spawn window and moves the blocking flow to
    /// the host's thread (REQ-UI-07/REQ-UI-11). A phone or tablet host runs it
    /// inline like every other mobile flow.
    fn dispatch_refresh_command(
        &mut self,
        command: snackpilot_model::desktop::DesktopCommand,
        cx: &mut App,
    ) -> bool {
        let Some(model) = self.model.clone() else {
            return false;
        };
        let state = model.state();
        let Some(ticket) = self
            .mutation_gate
            .claim(state.busy, state.order_progress.is_some())
        else {
            return false;
        };
        let Some(claim) = model.begin_command(command) else {
            return false;
        };
        let Some(intent) = refresh_intent(state.navigation.active_tab()) else {
            return false;
        };
        let host = state.host_kind;
        run_blocking(host, cx, move || {
            let _claim = claim;
            let _ticket = ticket;
            run_refresh(&model, intent);
        });
        true
    }

    /// Capture focus on dialog open and restore the originating control when
    /// it closes. A confirmed flow can delete the record whose control held
    /// focus, unmounting that handle: the wide workspace then restores the
    /// nearest surviving row (the model's reconciled cursor) and only falls
    /// back to the screen heading when no row survives; the narrow shell keeps
    /// the incumbent restore (REQ-UI-14).
    fn sync_dialog_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() && !self.dialog_open {
            self.dialog_origin = window.focused(cx);
            self.dialog_origin_record = self.model.as_ref().and_then(|model| {
                ListKind::for_tab(model.state().navigation.active_tab())
                    .and_then(|kind| model.preview(kind).map(|id| (kind, id)))
            });
            self.dialog_open = true;
            let index = self.dialog_focus.min(1);
            let handle = self.dialog_handles[index].clone();
            window.focus(&handle, cx);
        } else if self.dialog.is_none() && self.dialog_open {
            self.dialog_open = false;
            let origin_record = self.dialog_origin_record.take();
            let Some(origin) = self.dialog_origin.take() else {
                return;
            };
            // The preview of a deleted record is cleared by the model's
            // reconciliation, so its presence is the survival check.
            let survives = match &origin_record {
                None => true,
                Some((kind, id)) => self
                    .model
                    .as_ref()
                    .is_some_and(|model| model.preview(*kind).as_deref() == Some(id.as_str())),
            };
            // The reconciliation also moved the list cursor to the nearest
            // surviving row; that row can take the focus the deleted control
            // gave up, and the heading is only the last resort.
            let nearest = origin_record.as_ref().and_then(|(kind, _)| {
                let cursor = self.model.as_ref().and_then(|model| model.cursor(*kind));
                let rendered = self.list_focus(*kind).rendered_ids();
                nearest_restore_cursor(cursor.as_deref(), &rendered)
                    .map(|id| (*kind, id.to_string()))
            });
            let nearest_available = nearest.is_some();
            let desktop = self
                .model
                .as_ref()
                .is_some_and(|model| model.layout_mode() != LayoutMode::Phone);
            match focus_restore(desktop, survives, nearest_available) {
                FocusRestore::Origin => window.focus(&origin, cx),
                FocusRestore::Nearest => {
                    let (kind, id) = nearest.expect("a nearest row was found");
                    if !self.list_focus(kind).focus(&id, window, cx) {
                        let heading = self.heading_focus.clone();
                        window.focus(&heading, cx);
                    }
                }
                FocusRestore::Heading => {
                    let heading = self.heading_focus.clone();
                    window.focus(&heading, cx);
                }
            }
        }
    }

    /// Back/Esc: close the dialog first, otherwise let the model unwind one
    /// layer (batch, preview, settings detail), never pending menu changes
    /// (REQ-UI-11).
    pub(crate) fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dialog.is_some() {
            self.resolve_dialog(0, window, cx);
            return;
        }
        if let Some(model) = &self.model {
            model.escape();
        }
        cx.notify();
    }

    /// Whether one of the credential fields currently owns native text focus.
    /// Workspace shortcuts stay inert while it does (REQ-UI-11).
    fn text_input_focused(&self, window: &Window, cx: &App) -> bool {
        [self.login_user.as_ref(), self.login_pass.as_ref()]
            .into_iter()
            .flatten()
            .any(|input| input.read(cx).focus_handle(cx).is_focused(window))
    }

    /// The dialog's keyboard traversal: arrows and Tab cycle inside the action
    /// row, Enter/Space use the focused action, Esc resolves index 0 and
    /// repeats are ignored (REQ-UI-14).
    fn dialog_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = &self.dialog else {
            return;
        };
        let count = dialog.button_count();
        let action = dialog_key(
            event.keystroke.key.as_str(),
            event.keystroke.modifiers.shift,
            event.is_held,
            self.dialog_focus,
            count,
        );
        match action {
            DialogKey::Ignore => {}
            DialogKey::Previous | DialogKey::Next => {
                self.dialog_focus =
                    dialog_move(self.dialog_focus, count, action == DialogKey::Next);
                if let Some(handle) = self.dialog_handles.get(self.dialog_focus).cloned() {
                    window.focus(&handle, cx);
                }
            }
            DialogKey::Activate(index) => self.resolve_dialog(index, window, cx),
            DialogKey::Dismiss => self.resolve_dialog(0, window, cx),
        }
        cx.stop_propagation();
    }

    /// iOS soft-keyboard driver (§3.6, task 5.4): gpui-mobile creates the
    /// native text input view hidden, so nothing raises the keyboard on its
    /// own. gpui re-runs `render` on every repaint, so polling the focused
    /// input here reacts to a tap-to-focus (focus change ⇒ repaint) and keeps
    /// the keyboard in sync without patching the library.
    fn sync_keyboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(target_os = "ios")]
        if self.dialog.is_none() {
            let focused = window.has_focused_input(cx);
            if let Some(show) = keyboard_action(self.keyboard_up, focused) {
                kb_log(&format!(
                    "driver {} (focused={focused} was_up={})",
                    if show { "show" } else { "hide" },
                    self.keyboard_up
                ));
                self.keyboard_up = show;
                if show {
                    gpui_mobile::show_keyboard();
                } else {
                    gpui_mobile::hide_keyboard();
                }
            }
        }
        #[cfg(not(target_os = "ios"))]
        {
            let _ = (&self, window, cx);
        }
    }
    fn placeholder(
        &self,
        nav: &Navigation,
        state: &State,
        theme: &AppTheme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (title, hint): (&str, String) = match nav.top() {
            Screen::Menus => match &state.menu_snapshot {
                Some(snapshot) if !snapshot.available_dates.is_empty() => {
                    let day = state
                        .selected_day
                        .as_deref()
                        .or(snapshot.available_dates.first().map(String::as_str))
                        .map(format_day_short)
                        .unwrap_or_default();
                    (
                        "Menüs",
                        format!("{} Menüs am {}", snapshot.items.len(), day),
                    )
                }
                _ => ("Menüs", "Keine Menüs verfügbar".to_string()),
            },
            Screen::Orders => match &state.orders_split {
                Some(split) => (
                    "Bestellungen",
                    format!(
                        "{} bevorstehende, {} vergangene Bestellungen",
                        split.upcoming.len(),
                        split.past.len()
                    ),
                ),
                None => ("Bestellungen", "Keine Bestellungen".to_string()),
            },
            Screen::Billing => {
                if state.billing_loading {
                    ("Abrechnung", "Wird geladen…".to_string())
                } else if state.gourmet_month.is_some() || state.ventopay_month.is_some() {
                    ("Abrechnung", "Daten verfügbar".to_string())
                } else {
                    ("Abrechnung", "Keine Abrechnungsdaten".to_string())
                }
            }
            Screen::Settings => return settings_tab(state, theme, cx),
            Screen::SettingsDetail => unreachable!("handled before placeholders"),
        };
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(theme.text_secondary)
                    .child(hint),
            )
            .into_any_element()
    }

    /// Content of the active screen, including the §4.4 day navigator on the
    /// Menus tab and the §3.5 back affordance on pushed screens.
    fn screen(
        &self,
        nav: &Navigation,
        state: &State,
        theme: &AppTheme,
        layout: LayoutMode,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        // Desktop containment (REQ-SCR-04): the model owns the projection, so
        // compact and expanded desktop render the settings master beside the
        // selected form, and only the phone projection is the incumbent
        // pushed screen with its back affordance.
        if matches!(nav.top(), Screen::Settings | Screen::SettingsDetail)
            && self
                .model
                .as_ref()
                .is_some_and(|model| model.settings_contained())
        {
            return desktop_settings_tab(
                state,
                self.model.as_ref(),
                &self.login_user,
                &self.login_pass,
                &self.settings_rows,
                &self.settings_master,
                &self.appearance_design,
                &self.appearance_accent,
                theme,
                cx,
            );
        }
        if nav.top() == Screen::SettingsDetail {
            return match (self.current_detail(), &self.login_user, &self.login_pass) {
                (Detail::LoginKantine, Some(user), Some(pass)) => login_screen(
                    LoginKind::Kantine,
                    state,
                    self.model.as_ref(),
                    user,
                    pass,
                    theme,
                    None,
                    cx,
                ),
                (Detail::LoginAutomaten, Some(user), Some(pass)) => login_screen(
                    LoginKind::Automaten,
                    state,
                    self.model.as_ref(),
                    user,
                    pass,
                    theme,
                    None,
                    cx,
                ),
                (Detail::Appearance, _, _) => appearance_screen(
                    state,
                    self.model.as_ref(),
                    theme,
                    None,
                    &self.appearance_design,
                    &self.appearance_accent,
                    cx,
                ),
                (Detail::Notifications, _, _) => {
                    notifications_screen(state, self.model.as_ref(), theme, None, cx)
                }
                (detail, _, _) => placeholder_detail(detail, theme, cx),
            };
        }

        let mut column = div().flex_1().flex().flex_col().overflow_hidden();
        // The compact Day Tray draws its own 48 px navigator inside the Menüs
        // screen (task 1.3); the wide Ledger keeps this incumbent toolbar.
        if nav.top() == Screen::Menus && menu_row_action(layout) == MenuRowAction::Preview {
            if let (Some(model), Some(snapshot)) = (&self.model, &state.menu_snapshot) {
                if !snapshot.available_dates.is_empty() {
                    let (prev, next, today) =
                        (Arc::clone(model), Arc::clone(model), Arc::clone(model));
                    let (entity_p, entity_n, entity_t) = (
                        cx.entity().clone(),
                        cx.entity().clone(),
                        cx.entity().clone(),
                    );
                    let on_prev = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        prev.prev_day();
                        entity_p.update(cx, |_, cx| cx.notify());
                    });
                    let on_next = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        next.next_day();
                        entity_n.update(cx, |_, cx| cx.notify());
                    });
                    let on_today = Rc::new(move |_w: &mut Window, cx: &mut App| {
                        today.go_to_today();
                        entity_t.update(cx, |_, cx| cx.notify());
                    });
                    column = column.child(day_navigator(
                        &snapshot.available_dates,
                        state.selected_day.as_deref(),
                        theme,
                        on_prev,
                        on_next,
                        on_today,
                    ));
                }
            }
        }
        column
            .child(match nav.top() {
                Screen::Menus => match menu_row_action(layout) {
                    MenuRowAction::Preview => desktop_menus_tab(
                        state,
                        self.model.as_ref(),
                        self.mutation_gate.is_claimed(),
                        &self.menu_list,
                        theme,
                        cx,
                    ),
                    MenuRowAction::Toggle => {
                        menus_tab(state, self.model.as_ref(), theme, self.swipe_dx, cx)
                    }
                },
                Screen::Orders => {
                    if layout == LayoutMode::Phone {
                        orders_tab(state, self.model.as_ref(), state.orders_past, theme, cx)
                    } else {
                        desktop_orders_tab(
                            state,
                            self.model.as_ref(),
                            layout,
                            self.mutation_gate.is_claimed(),
                            &self.order_list,
                            &self.segment_group,
                            theme,
                            cx,
                        )
                    }
                }
                Screen::Billing => {
                    if layout == LayoutMode::Phone {
                        billing_tab(state, self.model.as_ref(), theme, cx)
                    } else {
                        desktop_billing_tab(
                            state,
                            self.model.as_ref(),
                            layout,
                            &self.receipt_list,
                            theme,
                            cx,
                        )
                    }
                }
                _ => self.placeholder(nav, state, theme, cx),
            })
            .into_any_element()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(detail) = self.pending_detail.take() {
            self.open_detail(detail, window, cx);
        }
        let active = window.is_window_active();
        if self.seen_frame && active && !self.was_active {
            if let Some(model) = self.model.as_ref() {
                // Host-gated dispatch (task 1.5): the foreground refresh blocks
                // on the model's runtime, so a desktop host, narrow windows
                // included, runs it on the worker thread and keeps painting.
                let host = model.state().host_kind;
                let model = Arc::clone(model);
                run_blocking(host, cx, move || model.on_foreground());
            }
        }
        self.was_active = active;
        self.seen_frame = true;
        self.sync_keyboard(window, cx);
        if self.dialog.is_none() {
            if let Some(alert) = self.model.as_ref().and_then(|m| m.take_location_alert()) {
                self.dialog = Some(Dialog::Alert {
                    title: alert.title.into(),
                    message: Some(alert.message.into()),
                });
            }
        }
        let state = self
            .model
            .as_ref()
            .map(|model| model.state())
            .unwrap_or_default();
        let os_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let theme = AppTheme::resolve(state.theme, state.accent, os_dark);
        let (safe_top, safe_bottom, _, _) = safe_area();
        // Host-gated layout (task 1.2, REQ-UI-07): only a Desktop host past
        // the 720 px breakpoint leaves the phone shell, so iOS and Android
        // keep their exact appearance at every width.
        let layout = self
            .model
            .as_ref()
            .map(|model| model.layout_mode())
            .unwrap_or(LayoutMode::Phone);
        let desktop = layout != LayoutMode::Phone;
        let nav = self.navigation();
        // Host focus facts and the native menu enablement follow the same
        // model state every repaint (REQ-UI-11/14).
        let text_focused = self.text_input_focused(window, cx);
        crate::desktop::report_scope(self.model.as_deref(), self.dialog.is_some(), text_focused);
        crate::desktop::sync_menus(cx, self.model.as_deref());
        self.sync_dialog_focus(window, cx);

        let entity = cx.entity().clone();
        let on_select: TabSelect = Rc::new(move |tab: Tab, _window: &mut Window, cx: &mut App| {
            log::info!("[4.2] tab selected: {tab:?}");
            entity.update(cx, |this, cx| {
                this.select_tab(tab);
                cx.notify();
            });
        });
        let rail_select = Rc::clone(&on_select);
        let bar = tab_bar(&theme, nav.active_tab(), safe_bottom, on_select);

        let mut banner = None;
        if let Some(message) = state
            .broadcast
            .as_ref()
            .filter(|_| !state.broadcast_dismissed)
        {
            let entity = cx.entity().clone();
            let dismiss = Rc::new(move |_w: &mut Window, cx: &mut App| {
                let _ = entity.update(cx, |this, cx| {
                    if let Some(model) = &this.model {
                        model.dismiss_broadcast();
                    }
                    cx.notify();
                });
            });
            banner = Some(broadcast_banner(message, dismiss, &theme));
        }
        let screen_content = self.screen(&nav, &state, &theme, layout, cx);
        // Compact Day Tray shell plan (REQ-UI-03/04/15/16): one heading, one
        // centred column of min(viewport − 32, 560), the safe-area top applied
        // exactly once outside the column and a measured obstruction below the
        // scrolled content. The wide Ledger never reads this plan.
        let viewport_width = f32::from(window.viewport_size().width);
        let credential_form = nav.top() == Screen::SettingsDetail
            && matches!(
                self.current_detail(),
                Detail::LoginKantine | Detail::LoginAutomaten
            );
        // The compact Menüs dock's measured height feeds the one obstruction
        // reservation below (task 1.3); every other surface has none.
        let column_width = crate::compact_width(state.content_width, viewport_width);
        let menu_dock_height = if nav.top() == Screen::Menus {
            let width = crate::compact_column(state.host_kind, column_width)
                .map(|geometry| geometry.content)
                .unwrap_or(column_width);
            menus_dock_height(&state, self.model.as_ref(), width, &theme)
        } else {
            0.
        };
        let plan = compact_shell_plan(
            state.host_kind,
            state.content_width,
            viewport_width,
            nav.active_tab(),
            nav.top(),
            credential_form,
            theme.flat,
            safe_top,
            safe_bottom,
            menu_dock_height,
        );
        let action_shell = cx.entity().clone();
        let mut root = div().size_full().flex().flex_col().bg(theme.background);
        // The compact Menüs dock, painted above the busy scrim once built; the
        // wide Ledger keeps its own pending strip and never sets this.
        let mut compact_dock: Option<AnyElement> = None;
        // Every native menu command lands here; on-screen buttons call the
        // same model intents directly.
        {
            use crate::desktop::macos::*;
            use snackpilot_model::desktop::DesktopCommand;
            macro_rules! menu_command {
                ($action:ty, $cmd:expr) => {{
                    let shell = action_shell.clone();
                    root = root.on_action(move |_: &$action, window: &mut Window, cx: &mut App| {
                        let shell = shell.clone();
                        let _ = shell.update(cx, |this, cx| {
                            this.run_command($cmd, window, cx);
                        });
                    });
                }};
            }
            menu_command!(MenuOpenMenus, DesktopCommand::OpenMenus);
            menu_command!(MenuOpenOrders, DesktopCommand::OpenOrders);
            menu_command!(MenuOpenBilling, DesktopCommand::OpenBilling);
            menu_command!(MenuOpenSettings, DesktopCommand::OpenSettings);
            menu_command!(MenuRefresh, DesktopCommand::Refresh);
            {
                let shell = action_shell.clone();
                root = root.on_action(move |_: &MenuBack, window: &mut Window, cx: &mut App| {
                    let shell = shell.clone();
                    let _ = shell.update(cx, |this, cx| this.back(window, cx));
                });
            }
            {
                let shell = action_shell.clone();
                root = root.on_action(move |_: &MenuAbout, _window, cx: &mut App| {
                    let shell = shell.clone();
                    let message = crate::desktop::about_message(
                        &crate::desktop::product_version(),
                        &crate::desktop::product_build(),
                    );
                    let _ = shell.update(cx, |this, cx| {
                        this.set_dialog(
                            Dialog::Alert {
                                title: "Über SnackPilot".into(),
                                message: Some(message.into()),
                            },
                            cx,
                        );
                    });
                });
            }
        }
        {
            let shell = action_shell.clone();
            root = root.on_key_down(move |event: &KeyDownEvent, window, cx| {
                let _ = shell.update(cx, |this, cx| this.dialog_key(event, window, cx));
            });
        }
        if layout != LayoutMode::Phone {
            // SnackPilot Desktop Ledger shell: the navigation rail, then the
            // centred workspace panel. Every width comes from the model's
            // geometry, so no screen derives a tile itself.
            let geometry = self
                .model
                .as_ref()
                .map(|model| model.desktop_geometry())
                .unwrap_or_else(|| {
                    snackpilot_model::desktop::desktop_geometry(HostKind::Desktop, 0.)
                });
            let tab = nav.active_tab();
            // Menüs/Bestellungen/Abrechnung carry the refresh action;
            // Einstellungen has none (brief "Workspace header").
            let on_refresh: Option<ClickHandler> = match (refresh_intent(tab), self.model.as_ref())
            {
                (Some(intent), Some(model)) => {
                    let model = Arc::clone(model);
                    let host = state.host_kind;
                    Some(Rc::new(move |_w: &mut Window, cx: &mut App| {
                        dispatch_refresh(&model, intent, host, cx);
                    }) as ClickHandler)
                }
                _ => None,
            };
            let panel = theme::apply(
                div()
                    .flex()
                    .flex_col()
                    .w(px(geometry.workspace_width))
                    .h_full()
                    .overflow_hidden(),
                &theme.workspace_panel(),
            )
            .child(
                div()
                    .tab_index(0)
                    .track_focus(&self.heading_focus)
                    .focus(theme::desktop_focus(theme.desktop.focus))
                    .child(workspace_header(tab.title(), on_refresh, &theme)),
            )
            .child(
                banner
                    .map(|banner| {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .pt(px(plan.safe_top))
                            .child(banner)
                    })
                    .unwrap_or_else(|| {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .pt(px(plan.safe_top))
                    })
                    .child(screen_content),
            );
            // The desktop toast paints over the workspace only, never the
            // rail (brief "Overlays"), so it lives inside the main region.
            let mut main_region = div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .relative()
                .justify_center()
                .child(panel);
            if let Some(toast) = &self.toast {
                main_region = main_region.child(desktop_toast(toast, safe_top + 8., &theme));
            }
            root = root
                .flex_row()
                .child(desktop_rail(
                    tab,
                    geometry.rail_width,
                    Some(&self.rail_rows),
                    &theme,
                    rail_select,
                ))
                .child(main_region);
        } else {
            // Compact Day Tray: the shell's one heading, then the centred
            // column, with the safe-area top applied once outside it.
            let mut column = div()
                .w(px(plan.content_width))
                .flex_1()
                .flex()
                .flex_col()
                .min_h(px(0.));
            if let Some(heading) = plan.heading {
                column = column.child(shell_heading_row(heading, &theme));
            }
            if let Some(banner) = banner {
                column = column.child(banner);
            }
            let bottom = compact_content_bottom(
                &plan,
                theme.flat,
                legacy_screen_owns_bottom_inset(nav.top()),
            );
            let content = div()
                .w_full()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .overflow_hidden()
                .pt(px(plan.safe_top))
                .pb(px(bottom))
                .child(column.child(screen_content));
            // The measured Menüs dock occupies the band the reservation above
            // includes, so the last card is scrollable fully above it.
            compact_dock = if nav.top() == Screen::Menus {
                menus_dock(
                    &state,
                    self.model.as_ref(),
                    plan.content_width,
                    navigation_top(theme.flat, safe_bottom),
                    &theme,
                    cx,
                )
            } else {
                None
            };
            if theme.flat {
                // §2.2: in-flow bottom bar; the safe-area strip under it keeps
                // the bar's surface colour (v1 paddingBottom insets.bottom). The
                // bar owns its height in flow, so only the scroll-end gap is
                // added above it, never the bar's own height a second time.
                root = root.child(content).child(
                    div()
                        .w_full()
                        .bg(theme.surface)
                        .pb(px(safe_bottom))
                        .child(bar),
                );
            } else {
                // §2.1: floating pill, painted over the content it reserves,
                // with the measured obstruction + 16 px scroll-end gap.
                root = root.child(content).child(bar);
            }
        }

        // Overlays stack by paint order (gpui has no z-index): loading, then
        // toast, then dialog on top. Desktop uses the opaque panel variants
        // whenever the desktop shell is shown; the phone shell keeps the
        // incumbent mobile recipes on every host (REQ-UI-07/13).
        // The Menüs cached refresh keeps its cards: the neutral row is the only
        // progress feedback on nonempty content, so the blocking overlay only
        // covers an initial load (compact Day Tray; the wide Ledger keeps its
        // incumbent overlay).
        let keep_cached_menus =
            !desktop && cached_menu_refresh(nav.top(), state.menu_snapshot.as_ref());
        if (state.busy || state.ventopay_busy || state.billing_loading) && !keep_cached_menus {
            root = root.child(if desktop {
                desktop_loading_overlay(&theme)
            } else {
                loading_overlay(&theme)
            });
        }
        // The dock paints above the busy scrim: its own exact phase stays
        // readable in the dock region while an order operation runs.
        if let Some(dock) = compact_dock {
            root = root.child(dock);
        }
        if let Some(toast) = &self.toast {
            if !desktop {
                root = root.child(toast_overlay_for(
                    toast,
                    safe_top,
                    cx.reduce_motion(),
                    &theme,
                ));
            }
        }
        if let Some(dialog) = &self.dialog {
            let entity = cx.entity().clone();
            let on_activate: crate::components::dialog::DialogActivate =
                Rc::new(move |index, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |this, cx| this.resolve_dialog(index, window, cx));
                });
            root = root.child(dialog_overlay(
                dialog,
                !desktop,
                &theme,
                self.dialog_focus,
                &self.dialog_handles,
                on_activate,
            ));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cached_menu_refresh, compact_content_bottom, compact_shell_plan, desktop_check_effect,
        desktop_mutation_blocked, detail_from_model, dispatch_off_thread, focus_restore,
        keyboard_action, legacy_screen_owns_bottom_inset, menu_row_action, model_settings_detail,
        navigation_top, nearest_restore_cursor, obstruction_reservation, order_cancel_state,
        order_row_effect, refresh_intent, shell_heading, should_create_inputs, Detail,
        FocusRestore, MenuRowAction, MutationGate, OrderCancelState, OrderRowEffect, RefreshIntent,
        HEADING_GAP, HEADING_TOP, LEGACY_SCREEN_BOTTOM_INSET, SCROLL_END_GAP,
    };
    use crate::components::fab::{dock_bottom, dock_height, dock_state, DOCK_GAP};
    use crate::screens::menus::menus_dock_height;
    use snackpilot_model::desktop::HostKind;
    use snackpilot_model::desktop::LayoutMode;
    use snackpilot_model::navigation::{ListKind, Screen, SettingsDetail, Tab};
    use snackpilot_model::{AppModel, FakePlatform};
    use std::sync::Arc;

    use crate::screens::orders::{
        desktop_cancel_basis, desktop_cancel_dispatch, desktop_cancel_selected_dispatch,
        desktop_confirm_dispatch,
    };

    /// A signed-in demo model over a fake platform: no network, no live host.
    fn claim_demo_model(tag: &str) -> (Arc<AppModel>, Arc<FakePlatform>) {
        let dir = std::env::temp_dir().join(format!("snackpilot-app-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let platform = Arc::new(FakePlatform::new());
        let model = AppModel::new(dir, platform.clone()).unwrap();
        model.login("demo", "demo1234!").unwrap();
        (Arc::new(model), platform)
    }

    /// Place two demo orders in one submit so the batch paths have a real
    /// two-row selection.
    fn place_two_orders(model: &AppModel) -> Vec<snackpilot_core::domain::OrderedMenu> {
        let items: Vec<_> = model
            .state()
            .menu_snapshot
            .expect("demo menus load at login")
            .items
            .into_iter()
            .filter(|item| {
                item.available
                    && !item.ordered
                    && !model.core().is_ordering_cutoff(item.day.clone())
            })
            .take(2)
            .collect();
        assert_eq!(items.len(), 2, "demo menus offer two orderable items");
        for item in &items {
            model.toggle(item);
        }
        model.submit_orders();
        let orders = model
            .state()
            .orders_split
            .expect("orders loaded after submit")
            .upcoming;
        assert_eq!(orders.len(), 2, "the submit placed two orders");
        orders
    }

    /// The soft keyboard follows focus changes and stays put otherwise
    /// (§3.6 typing proof; the iOS side is proven by the XCUITest run).
    /// @req REQ-SCR-06
    #[test]
    fn keyboard_follows_focus_changes() {
        assert_eq!(keyboard_action(false, true), Some(true)); // tap focuses a field
        assert_eq!(keyboard_action(true, false), Some(false)); // focus lost
        assert_eq!(keyboard_action(true, true), None); // unchanged
        assert_eq!(keyboard_action(false, false), None); // unchanged
    }

    /// The compact shell keeps one root, one navigation and one heading
    /// (REQ-UI-03/REQ-UI-04): every destination gets the tab's own heading in
    /// the headline role while a pushed settings screen keeps its own back row
    /// and title instead, the platform bar stays visible on pushes, the active
    /// tab identity never changes, the safe-area top is applied exactly once
    /// outside the column, and the bottom reservation is measured — never the
    /// old blind 100 px, never a second copy of an in-flow bar.
    /// @req REQ-UI-03 @req REQ-UI-04
    #[test]
    fn mobile_shell_preserves_navigation_and_single_root() {
        use crate::components::tab_bar::{reserved_height, BAR_HEIGHT};
        use snackpilot_model::desktop::HostKind;
        use snackpilot_model::navigation::{Screen, Tab};

        // One heading per destination, in the shell's headline role.
        for tab in Tab::ALL {
            assert_eq!(
                shell_heading(tab, false),
                Some(tab.title()),
                "{tab:?} keeps the shell heading"
            );
            assert_eq!(
                shell_heading(tab, true),
                None,
                "a pushed settings screen draws its own back row and title"
            );
        }
        assert_eq!((HEADING_TOP, HEADING_GAP), (16., 16.));

        // The compact plan at the accepted widths: one centred column, the
        // safe top applied once, the bar always visible and the identity kept.
        for (width, content) in [(360., 328.), (390., 358.), (768., 560.), (1280., 560.)] {
            let plan = compact_shell_plan(
                HostKind::Phone,
                width,
                width,
                Tab::Menus,
                Screen::Menus,
                false,
                false,
                59.,
                34.,
                0.,
            );
            assert_eq!(plan.content_width, content, "centred column at {width}");
            assert_eq!(plan.heading, Some("Menüs"));
            assert_eq!(plan.heading_top, 16., "16 below the safe top");
            assert_eq!(plan.safe_top, 59., "the safe top is applied once");
            assert!(plan.bar_visible, "the platform bar stays visible");
            assert_eq!(plan.active, Tab::Menus);
            // The pill is painted over the content, so its height is reserved
            // once, plus the 16 px scroll-end gap.
            assert_eq!(
                plan.reservation,
                reserved_height(false, 34.) + SCROLL_END_GAP
            );
            assert_ne!(plan.reservation, 100., "no blind 100 px padding");
        }

        // A push keeps the navigation topology and the active destination.
        let pushed = compact_shell_plan(
            HostKind::Phone,
            390.,
            390.,
            Tab::Settings,
            Screen::SettingsDetail,
            false,
            false,
            59.,
            34.,
            0.,
        );
        assert_eq!(
            pushed.active,
            Tab::Settings,
            "the active tab identity stays"
        );
        assert!(
            pushed.bar_visible,
            "changing bar visibility on a push is out of scope"
        );
        assert_eq!(pushed.heading, None, "the pushed screen owns its title");
        assert_eq!(pushed.content_width, 358.);

        // A credential form keeps the 440 px cap inside the column at 768/1280.
        for width in [360., 390., 768., 1280.] {
            let form = compact_shell_plan(
                HostKind::Phone,
                width,
                width,
                Tab::Settings,
                Screen::SettingsDetail,
                true,
                false,
                59.,
                34.,
                0.,
            );
            let expected = if width >= 768. { 440. } else { width - 32. };
            assert_eq!(form.content_width, expected, "form cap at {width}");
            assert!(form.content_width <= 440., "never wider than the form cap");
        }

        // Reservation arithmetic: an in-flow bar already owns its height, so it
        // is not counted a second time; a dock adds exactly its measured height.
        assert_eq!(
            obstruction_reservation(true, 34., true, 0.),
            SCROLL_END_GAP,
            "the flat bar is in flow: no duplicate navigation padding"
        );
        assert_eq!(
            obstruction_reservation(false, 34., false, 0.),
            reserved_height(false, 34.) + SCROLL_END_GAP
        );
        assert_eq!(
            obstruction_reservation(false, 34., false, 56.),
            reserved_height(false, 34.) + 56. + SCROLL_END_GAP,
            "the dock adds its measured height"
        );
        assert_eq!(
            obstruction_reservation(true, 34., true, 56.),
            56. + SCROLL_END_GAP,
            "an in-flow bar never contributes its own height twice"
        );
        assert!(
            obstruction_reservation(true, 34., true, 56.) < BAR_HEIGHT + 56.,
            "the in-flow bar's own height is not added again"
        );
    }

    /// The compact column owns one bottom inset (REQ-UI-03/15). The Menüs
    /// screen migrated in task 1.3, so the shell supplies exactly the measured
    /// reservation there (a floating pill plus the present dock), while the
    /// screens still on the legacy inset get only the shortfall topped up. The
    /// real column arithmetic is what `render` uses, and the test fails if the
    /// two insets ever sum.
    /// @req REQ-UI-03 @req REQ-UI-15
    #[test]
    fn compact_shell_does_not_stack_a_screen_bottom_inset() {
        use snackpilot_model::desktop::HostKind;
        use snackpilot_model::navigation::{Screen, Tab};

        for width in [360., 390., 768., 1280.] {
            let plan = compact_shell_plan(
                HostKind::Phone,
                width,
                width,
                Tab::Menus,
                Screen::Menus,
                false,
                false,
                59.,
                34.,
                0.,
            );
            // Menüs owns no inset of its own any more: the measured
            // reservation alone clears the navigation.
            assert!(
                !legacy_screen_owns_bottom_inset(Screen::Menus),
                "the Menüs screen dropped its legacy 16/100 px padding"
            );
            assert_eq!(
                compact_content_bottom(&plan, false, false),
                plan.reservation
            );
            assert_eq!(compact_content_bottom(&plan, true, false), SCROLL_END_GAP);
            // The screens still to migrate own their inset, in both navigation
            // modes; the shell only tops up any shortfall.
            for top in [
                Screen::Orders,
                Screen::Billing,
                Screen::Settings,
                Screen::SettingsDetail,
            ] {
                assert!(legacy_screen_owns_bottom_inset(top), "{top:?}");
                assert_eq!(
                    compact_content_bottom(&plan, false, true),
                    (plan.reservation - LEGACY_SCREEN_BOTTOM_INSET).max(0.)
                );
                assert_eq!(
                    compact_content_bottom(&plan, true, true),
                    (SCROLL_END_GAP - LEGACY_SCREEN_BOTTOM_INSET).max(0.)
                );
            }

            // The effective scroll-end clearance is one copy — the legacy inset
            // topped up to the measured target — never the sum of both.
            let total = LEGACY_SCREEN_BOTTOM_INSET + compact_content_bottom(&plan, false, true);
            assert_eq!(total, plan.reservation.max(LEGACY_SCREEN_BOTTOM_INSET));
            assert!(
                total >= plan.reservation,
                "the last content still clears the measured pill"
            );
            assert_ne!(total, LEGACY_SCREEN_BOTTOM_INSET + plan.reservation);
            let migrated_total = compact_content_bottom(&plan, true, false);
            assert_eq!(migrated_total, SCROLL_END_GAP);
            assert_ne!(migrated_total, SCROLL_END_GAP + plan.reservation);
        }
    }

    /// The cached Menüs refresh keeps its content (REQ-SCR-01): the neutral
    /// `Aktualisiere...` row is the only progress feedback, so the blocking
    /// overlay is not painted over nonempty cached menus; an initial load
    /// without data still shows it.
    /// @req REQ-SCR-01
    #[test]
    fn mobile_cached_refresh_keeps_the_menu_content() {
        let (model, _fake) = claim_demo_model("cached-refresh");
        let full = model.state().menu_snapshot.expect("demo menus");
        assert!(!full.items.is_empty());
        let mut refreshing = full.clone();
        refreshing.refreshing = true;
        refreshing.loading = false;
        assert!(
            cached_menu_refresh(Screen::Menus, Some(&refreshing)),
            "a cached refresh keeps the cards and the day"
        );
        assert!(
            !cached_menu_refresh(Screen::Menus, Some(&full)),
            "a resting screen has no refresh phase"
        );
        let mut empty = refreshing.clone();
        empty.items.clear();
        assert!(
            !cached_menu_refresh(Screen::Menus, Some(&empty)),
            "an initial load without data still shows the blocking overlay"
        );
        assert!(!cached_menu_refresh(Screen::Orders, Some(&refreshing)));
        assert!(!cached_menu_refresh(Screen::Menus, None));
    }

    /// The compact pending dock (REQ-SCR-01/REQ-UI-15): it carries exactly the
    /// model's `fab_label()`, its measured recipe height enters the one
    /// obstruction reservation, it sits 8 px above platform navigation, and the
    /// measured clearance leaves the last card fully above the dock on both
    /// navigation dialects. No current-day count or total is invented.
    /// @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn mobile_pending_dock_wraps_and_reserves_measured_height() {
        let (model, _fake) = claim_demo_model("dock-measure");
        let mut state = model.state();
        let width = 328.;
        let theme =
            crate::theme::AppTheme::build(false, snackpilot_model::AccentColor::Orange, false);

        // No pending changes and no operation: no dock, no reservation.
        assert_eq!(menus_dock_height(&state, Some(&model), width, &theme), 0.);

        // Queue the first orderable dish: the dock is exactly the model label.
        let item = state
            .menu_snapshot
            .as_ref()
            .expect("demo menus")
            .items
            .iter()
            .find(|item| item.available && !item.ordered && !model.is_cutoff(&item.day))
            .cloned()
            .expect("an orderable demo dish");
        model.toggle(&item);
        state = model.state();
        let label = model.fab_label();
        assert_eq!(label, "Bestellen (1)");
        let dock = dock_state(1, state.order_progress.as_deref(), label.clone()).unwrap();
        assert_eq!(dock.action(), Some(label.as_str()));
        let measured = menus_dock_height(&state, Some(&model), width, &theme);
        assert_eq!(measured, dock_height(&dock, width, &theme));
        assert_eq!(measured, 48., "one body line at the action floor");

        // The reservation adds exactly the dock's measured height: floating
        // pill 90 + dock 48 + 16 scroll-end gap, a bar in flow dock + 16.
        let plan = compact_shell_plan(
            HostKind::Phone,
            390.,
            390.,
            Tab::Menus,
            Screen::Menus,
            false,
            false,
            59.,
            34.,
            measured,
        );
        let pill = crate::components::tab_bar::reserved_height(false, 34.);
        assert_eq!(plan.reservation, pill + measured + SCROLL_END_GAP);
        for flat in [false, true] {
            let nav = navigation_top(flat, 34.);
            // The dock's bottom is 8 px above the navigation's top edge.
            assert_eq!(dock_bottom(nav), nav + DOCK_GAP);
            let reserved = obstruction_reservation(flat, 34., flat, measured);
            // Where the scrolled content ends, relative to the window bottom:
            // above an in-flow bar the content box already sits above it.
            let content_end = if flat { nav + reserved } else { reserved };
            let dock_top = dock_bottom(nav) + measured;
            assert_eq!(
                content_end - dock_top,
                SCROLL_END_GAP - DOCK_GAP,
                "the last card ends 8 px above the dock on that dialect"
            );
            assert!(
                content_end > dock_top,
                "the scrolled content never runs under the reserved dock band"
            );
        }

        // During an operation the same band keeps the exact phase and offers no
        // repeat action; a long phase still reserves at least the action floor.
        let progress = dock_state(1, Some("Bestellung wird bestätigt..."), label.clone()).unwrap();
        assert_eq!(progress.text(), "Bestellung wird bestätigt...");
        assert_eq!(progress.action(), None);
        assert!(dock_height(&progress, width, &theme) >= 48.);
    }

    /// One dock activation dispatches the existing all-days submit once
    /// (REQ-UI-08/16): the shell's synchronous gate claims before the model
    /// reports busy, a second activation is inert, and the projection itself
    /// requests nothing — the one submit places exactly one order per pending
    /// dish and no extra fetch happens on render.
    /// @req REQ-UI-08 @req REQ-UI-16
    #[test]
    fn mobile_menu_action_dispatches_once_without_extra_fetch() {
        let (model, fake) = claim_demo_model("dock-once");
        let item = model
            .state()
            .menu_snapshot
            .as_ref()
            .expect("demo menus")
            .items
            .iter()
            .find(|item| item.available && !item.ordered && !model.is_cutoff(&item.day))
            .cloned()
            .expect("an orderable demo dish");
        model.toggle(&item);

        // Building the dock's content and height is presentation only: it
        // issues no platform call and no request.
        let calls_before = fake.calls();
        let state = model.state();
        let dock = dock_state(
            state
                .menu_snapshot
                .as_ref()
                .map(|s| s.pending_orders.len() + s.pending_cancellations.len())
                .unwrap_or(0),
            state.order_progress.as_deref(),
            model.fab_label(),
        )
        .expect("a pending set shows the dock");
        assert_eq!(dock.action(), Some("Bestellen (1)"));
        assert_eq!(
            fake.calls(),
            calls_before,
            "projecting the dock makes no platform call"
        );

        // The one gate that guards the dock action: the first activation
        // claims, a repeated tap while it is claimed is inert, and the claim
        // releases when the dispatch ends.
        let gate = MutationGate::default();
        let first = gate.claim(false, false);
        assert!(first.is_some(), "the first dock activation is accepted");
        assert!(
            gate.claim(false, false).is_none(),
            "a repeated tap cannot submit twice"
        );
        assert!(
            gate.claim(false, true).is_none(),
            "a running op blocks it too"
        );
        drop(first);
        assert!(gate.claim(false, false).is_some(), "the gate is released");

        // One submit places exactly one order; the pending set is consumed and
        // nothing extra is fetched afterwards.
        model.submit_orders();
        let split = model.state().orders_split.expect("orders loaded");
        assert_eq!(split.upcoming.len(), 1, "exactly one order was placed");
        assert!(
            !model.has_pending_changes(),
            "the one dispatch consumed the whole pending set"
        );
        assert!(
            model.state().selected_day.is_some(),
            "the submit kept the incumbent post-submit state"
        );
    }

    /// The workspace header dispatches the active tab's existing refresh and
    /// shows no action on Einstellungen (brief "Workspace header").
    /// @req REQ-UI-07
    #[test]
    fn refresh_intent_follows_the_active_tab() {
        assert_eq!(refresh_intent(Tab::Menus), Some(RefreshIntent::Menus));
        assert_eq!(refresh_intent(Tab::Orders), Some(RefreshIntent::Orders));
        assert_eq!(refresh_intent(Tab::Billing), Some(RefreshIntent::Billing));
        assert_eq!(refresh_intent(Tab::Settings), None);
    }

    /// Off-thread dispatch follows the host, not the rail (task 1.5): a narrow
    /// desktop window projects as `LayoutMode::Phone` and still dispatches its
    /// pointer, shortcut/menu and foreground paths off the render thread,
    /// while a phone or tablet host keeps the synchronous path at every width.
    /// @req REQ-UI-07
    #[test]
    fn blocking_actions_run_off_thread_on_every_desktop_window() {
        use snackpilot_model::desktop::{layout_mode, HostKind, LayoutMode};

        for width in [360., 720.] {
            // Pointer path (submit/confirm/cancel/login), shortcut and menu
            // path (Cmd+R, native Aktualisieren) and the foreground path all
            // dispatch through this one predicate.
            assert!(
                dispatch_off_thread(HostKind::Desktop),
                "desktop dispatches off-thread at {width}"
            );
            let layout = layout_mode(HostKind::Desktop, width);
            assert_eq!(
                layout == LayoutMode::Phone,
                width < 720.,
                "the rail gate would have called {width} synchronous"
            );
        }
        assert!(!dispatch_off_thread(HostKind::Phone));
        assert!(!dispatch_off_thread(HostKind::Tablet));
        assert_eq!(layout_mode(HostKind::Phone, 720.), LayoutMode::Phone);
    }

    /// Selecting a Menüs row previews on every desktop layout and keeps the
    /// incumbent mobile toggle on the phone shell: row activation alone never
    /// changes a pending selection (task 1.3).
    /// @req REQ-SCR-01
    #[test]
    fn menus_rows_preview_on_desktop_and_toggle_on_phone() {
        assert_eq!(menu_row_action(LayoutMode::Phone), MenuRowAction::Toggle);
        for layout in [LayoutMode::DesktopCompact, LayoutMode::DesktopExpanded] {
            assert_eq!(
                menu_row_action(layout),
                MenuRowAction::Preview,
                "{layout:?} must preview instead of toggling"
            );
        }
    }

    /// Every desktop mutation entry point is inert while a submit or another
    /// blocking operation runs, so a duplicate activation cannot dispatch.
    /// @req REQ-UI-12
    #[test]
    fn desktop_mutations_block_while_dispatched() {
        assert!(!desktop_mutation_blocked(false, false));
        assert!(desktop_mutation_blocked(true, false));
        assert!(desktop_mutation_blocked(false, true));
        assert!(desktop_mutation_blocked(true, true));
    }

    /// A Bestellungen row body only previews while the batch check column
    /// toggles that one ID, so previewing a row never approves, cancels or
    /// checks it and a check never changes the preview (task 1.4).
    /// @req REQ-SCR-02
    #[test]
    fn order_row_effect_separates_body_and_checkbox() {
        assert_eq!(order_row_effect(false, false), OrderRowEffect::Preview);
        assert_eq!(order_row_effect(false, true), OrderRowEffect::Preview);
        assert_eq!(order_row_effect(true, false), OrderRowEffect::Preview);
        assert_eq!(order_row_effect(true, true), OrderRowEffect::ToggleCheck);
        assert_eq!(
            desktop_check_effect(false),
            OrderRowEffect::Preview,
            "a check outside the selection mode never toggles"
        );
        assert_eq!(
            desktop_check_effect(true),
            OrderRowEffect::ToggleCheck,
            "the check column toggles exactly while selecting"
        );
    }

    /// The desktop cancel action keeps the brief's precedence (task 1.4):
    /// past hides it, the affected in-flight order replaces its label, the
    /// cutoff disables it with its own notice, and another dispatch keeps it
    /// inert before an eligible order offers the ready action.
    /// @req REQ-SCR-02
    #[test]
    fn order_cancel_state_precedence() {
        assert_eq!(
            order_cancel_state(true, true, false, false),
            OrderCancelState::Hidden,
            "a past order never offers cancel, even while another cancel runs"
        );
        assert_eq!(
            order_cancel_state(false, true, true, true),
            OrderCancelState::InFlight,
            "the affected order shows its in-flight label"
        );
        assert_eq!(
            order_cancel_state(false, false, true, true),
            OrderCancelState::Cutoff,
            "the cutoff notice wins over a blocked workspace"
        );
        assert_eq!(
            order_cancel_state(false, false, false, true),
            OrderCancelState::Busy
        );
        assert_eq!(
            order_cancel_state(false, false, false, false),
            OrderCancelState::Ready
        );
    }

    /// A desktop single cancel claims the shell gate before spawning and
    /// releases it after: a second activation while it runs cannot claim, so
    /// exactly one cancel reaches the core (one `order.cancelled`).
    /// @req REQ-UI-12
    #[test]
    fn desktop_cancel_claims_once_and_dispatches_one_cancel() {
        let (model, platform) = claim_demo_model("cancel-once");
        let item = model
            .state()
            .menu_snapshot
            .expect("demo menus load at login")
            .items
            .into_iter()
            .find(|item| {
                item.available
                    && !item.ordered
                    && !model.core().is_ordering_cutoff(item.day.clone())
            })
            .expect("demo menus have an orderable item past the cutoff");
        model.toggle(&item);
        model.submit_orders();

        let order = model
            .state()
            .orders_split
            .expect("orders loaded after submit")
            .upcoming
            .into_iter()
            .next()
            .expect("the submitted order is upcoming");
        assert!(
            !model.is_cancellation_cutoff(&order),
            "the demo order is still cancellable"
        );

        let gate = MutationGate::default();
        let ticket = gate
            .claim(false, false)
            .expect("the first activation claims the gate");
        assert!(gate.is_claimed(), "the dispatch holds the claim");
        assert!(
            gate.claim(false, false).is_none(),
            "a double activation cannot claim the gate again"
        );

        let worker = Arc::clone(&model);
        let position_id = order.position_id.clone();
        std::thread::spawn(move || {
            let _ticket = ticket;
            desktop_cancel_dispatch(&worker, &position_id);
        })
        .join()
        .unwrap();

        assert!(
            !gate.is_claimed(),
            "the claim is released when the dispatch ends"
        );
        let cancels = platform
            .tracks()
            .into_iter()
            .filter(|(event, _)| event == "order.cancelled")
            .count();
        assert_eq!(cancels, 1, "one dispatched cancel runs the core once");
        assert!(
            model
                .state()
                .orders_split
                .expect("orders still loaded")
                .upcoming
                .iter()
                .all(|o| o.position_id != order.position_id),
            "the cancelled order is gone"
        );
    }

    /// The submit's shell claim is taken synchronously before the spawn and
    /// held until the dispatch ends: a second activation is rejected, and the
    /// one dispatch runs the core submit exactly once (`order.submitted` is
    /// the core's analytics event for one submit pipeline).
    /// @req REQ-UI-12
    #[test]
    fn desktop_submit_claims_once_and_dispatches_one_submit() {
        let (model, platform) = claim_demo_model("submit-once");
        let item = model
            .state()
            .menu_snapshot
            .expect("demo menus load at login")
            .items
            .into_iter()
            .find(|item| {
                item.available
                    && !item.ordered
                    && !model.core().is_ordering_cutoff(item.day.clone())
            })
            .expect("demo menus have an orderable item past the cutoff");
        model.toggle(&item);
        assert!(model.has_pending_changes(), "the submit has work to send");

        let gate = MutationGate::default();
        assert!(
            gate.claim(true, false).is_none(),
            "a running blocking operation rejects a claim"
        );
        let ticket = gate
            .claim(false, false)
            .expect("the first activation claims the gate");
        assert!(gate.is_claimed(), "the dispatch holds the claim");
        assert!(
            gate.claim(false, false).is_none(),
            "a double activation cannot claim the gate again"
        );

        let worker = Arc::clone(&model);
        std::thread::spawn(move || {
            let _ticket = ticket;
            worker.submit_orders();
        })
        .join()
        .unwrap();

        assert!(
            !gate.is_claimed(),
            "the claim is released when the dispatch ends"
        );
        let submits = platform
            .tracks()
            .into_iter()
            .filter(|(event, _)| event == "order.submitted")
            .count();
        assert_eq!(
            submits, 1,
            "one dispatched submit runs the core pipeline once"
        );
    }

    /// The extracted batch dispatch the click handler runs: the shell claims
    /// the gate synchronously, a second activation cannot claim, and the one
    /// dispatch reaches the core as a single batch (`order.cancelled` count=2).
    /// @req REQ-SCR-02 @req REQ-UI-12
    #[test]
    fn desktop_batch_cancel_claims_once_and_dispatches_one_batch() {
        let (model, platform) = claim_demo_model("batch-once");
        let orders = place_two_orders(&model);
        model.begin_order_selection();
        for order in &orders {
            model.toggle_order_selection(&order.position_id);
        }

        let gate = MutationGate::default();
        let ticket = gate
            .claim(false, false)
            .expect("the first activation claims the gate");
        assert!(
            gate.claim(false, false).is_none(),
            "a double activation cannot claim the gate again"
        );

        let worker = Arc::clone(&model);
        std::thread::spawn(move || {
            let _ticket = ticket;
            desktop_cancel_selected_dispatch(&worker);
        })
        .join()
        .unwrap();

        assert!(!gate.is_claimed(), "the claim is released when it ends");
        let batches: Vec<String> = platform
            .tracks()
            .into_iter()
            .filter(|(event, _)| event == "order.cancelled")
            .map(|(_, props)| props.get("count").cloned().unwrap_or_default())
            .collect();
        assert_eq!(
            batches,
            vec!["2".to_string()],
            "one dispatched batch cancels both selected orders in one core call"
        );
        assert!(
            model
                .state()
                .orders_split
                .expect("orders still loaded")
                .upcoming
                .is_empty(),
            "the batch removed both orders"
        );
    }

    /// The confirmation body the desktop banner spawns runs the real model
    /// flow under the same gate, so one activation can reach `confirm_orders`
    /// only once and always leaves the busy flag clear.
    /// @req REQ-SCR-02 @req REQ-UI-12
    #[test]
    fn desktop_confirm_dispatch_claims_once_and_clears_busy() {
        let (model, _platform) = claim_demo_model("confirm-once");
        place_two_orders(&model);
        let busy_log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = Arc::clone(&busy_log);
        model.subscribe(Box::new(move |s: &snackpilot_model::State| {
            log.lock().unwrap().push(s.busy);
        }));

        let gate = MutationGate::default();
        let ticket = gate
            .claim(false, false)
            .expect("the first activation claims the gate");
        assert!(
            gate.claim(false, false).is_none(),
            "a double activation cannot claim the gate again"
        );

        let worker = Arc::clone(&model);
        std::thread::spawn(move || {
            let _ticket = ticket;
            desktop_confirm_dispatch(&worker);
        })
        .join()
        .unwrap();

        assert!(!gate.is_claimed(), "the claim is released when it ends");
        let log = busy_log.lock().unwrap();
        assert!(
            log.contains(&true),
            "the confirm flow ran and published its busy state"
        );
        assert_eq!(
            log.last().copied(),
            Some(false),
            "the confirm flow ended and cleared busy"
        );
    }

    /// The reconciliation basis is read when the dispatch runs (dialog
    /// confirm), not when the dialog opened: a refresh that changed rows in
    /// between is reflected instead of leaving a stale cursor basis.
    /// @req REQ-SCR-02
    #[test]
    fn cancel_basis_is_read_at_dispatch_time() {
        let (model, _platform) = claim_demo_model("cancel-basis");
        let upcoming = place_two_orders(&model);
        let at_open = desktop_cancel_basis(&model);
        assert_eq!(at_open.len(), upcoming.len(), "the upcoming basis");

        // A refresh/switching segment while the dialog is open changes the
        // live rows; the basis read now must not be the one from open time.
        model.set_orders_segment(true);
        let at_confirm = desktop_cancel_basis(&model);
        assert_ne!(
            at_open, at_confirm,
            "the confirm-time basis follows the current segment, not the dialog opening"
        );
    }

    /// Settings containment is model-owned from the desktop 720 breakpoint on:
    /// the shell asks `settings_contained()` instead of deriving a layout rule
    /// of its own, and the one input instance survives the 719→720→719
    /// crossing (REQ-UI-10, REQ-SCR-04).
    /// @req REQ-UI-10 @req REQ-SCR-04
    #[test]
    fn settings_containment_is_model_owned_at_720() {
        use snackpilot_model::desktop::{HostKind, LayoutMode};

        let (model, _platform) = claim_demo_model("containment-projection");
        model.set_host_kind(HostKind::Desktop);
        for (width, layout, contained) in [
            (719., LayoutMode::Phone, false),
            (720., LayoutMode::DesktopCompact, true),
            (768., LayoutMode::DesktopCompact, true),
            (1280., LayoutMode::DesktopExpanded, true),
        ] {
            model.set_content_width(width);
            assert_eq!(model.layout_mode(), layout, "layout at {width}");
            assert_eq!(
                model.settings_contained(),
                contained,
                "containment at {width} is the model's decision"
            );
        }
        model.set_host_kind(HostKind::Phone);
        assert!(
            !model.settings_contained(),
            "a phone host never gets the desktop containment by width alone"
        );

        // Re-entering the same open form keeps its one input instance;
        // crossing the breakpoint does not request new inputs either; a fresh
        // entry after another detail does.
        assert!(!should_create_inputs(
            Detail::LoginKantine,
            Detail::LoginKantine,
            true
        ));
        assert!(should_create_inputs(
            Detail::LoginKantine,
            Detail::LoginAutomaten,
            true
        ));
        assert!(should_create_inputs(
            Detail::LoginKantine,
            Detail::LoginKantine,
            false
        ));
        assert!(!should_create_inputs(
            Detail::Appearance,
            Detail::Appearance,
            false
        ));
    }

    /// The settings detail shown is derived from the model-owned kind, so
    /// the view cannot disagree with the model (REQ-UI-10).
    /// @req REQ-UI-10
    #[test]
    fn settings_detail_round_trips_through_the_model() {
        for detail in [
            Detail::LoginKantine,
            Detail::LoginAutomaten,
            Detail::Appearance,
            Detail::Notifications,
        ] {
            assert_eq!(
                detail_from_model(model_settings_detail(detail)),
                detail,
                "{detail:?} must round-trip through the model identity"
            );
        }
        assert_eq!(detail_from_model(None), Detail::Settings);
        assert_eq!(
            detail_from_model(Some(SettingsDetail::Advanced)),
            Detail::Settings
        );
    }

    /// The containment crossing 719 → 720 → 719 never re-enters the form:
    /// `should_create_inputs` stays false, so the one input instance keeps its
    /// unsaved text, and the model keeps the same settings detail (and with it
    /// the service binding) at every width (REQ-UI-10, REQ-SCR-04).
    /// @req REQ-UI-10 @req REQ-SCR-04
    #[test]
    fn containment_keeps_one_input_and_model_detail_across_719_720_719() {
        use snackpilot_model::desktop::HostKind;

        let (model, _platform) = claim_demo_model("containment-resize");
        model.set_host_kind(HostKind::Desktop);
        // Entry happens at 719: the narrow, pushed form, one input pair.
        model.set_content_width(719.);
        let mut detail = Detail::Settings;
        let mut inputs: Option<u32> = None;
        let mut creations = 0;

        let entered = Detail::LoginKantine;
        if should_create_inputs(detail, entered, inputs.is_some()) {
            creations += 1;
            inputs = Some(creations);
        }
        detail = entered;
        model.open_settings_detail(SettingsDetail::Profile);

        // Both crossings re-render the same model detail: content width is
        // presentation, so nothing may re-enter (or re-prefill) the form.
        for width in [720., 719.] {
            model.set_content_width(width);
            let requested = detail_from_model(model.state().settings_detail);
            assert!(
                !should_create_inputs(detail, requested, inputs.is_some()),
                "the crossing at {width} must reuse the one input instance"
            );
            detail = requested;
            assert_eq!(
                model.state().settings_detail,
                Some(SettingsDetail::Profile),
                "the model keeps one settings detail at {width}"
            );
        }

        assert_eq!(creations, 1, "one entry creates one input pair");
        assert_eq!(inputs, Some(1), "the same instance is still rendered");
        assert_eq!(
            model_settings_detail(detail),
            Some(SettingsDetail::Profile),
            "the same service binding (Kantine) is still open"
        );
        let (username, _password) = model.saved_credentials(detail == Detail::LoginAutomaten);
        assert_eq!(
            username.as_deref(),
            Some("demo"),
            "the form still binds to the Gourmet slot, not Ventopay"
        );
    }

    /// A dialog whose confirmed flow deleted the origin's record does not put
    /// focus back on that dead handle: the wide workspace restores the nearest
    /// surviving row and only falls back to the screen heading when no row
    /// survives, while the narrow shell keeps the incumbent restore
    /// (REQ-UI-14).
    /// @req REQ-UI-14
    #[test]
    fn deleted_dialog_origin_restores_the_nearest_surviving_row() {
        assert_eq!(
            focus_restore(true, true, true),
            FocusRestore::Origin,
            "the origin still exists: exact restore"
        );
        assert_eq!(
            focus_restore(true, false, true),
            FocusRestore::Nearest,
            "a deleted origin hands focus to its nearest surviving row"
        );
        assert_eq!(
            focus_restore(true, false, false),
            FocusRestore::Heading,
            "the heading is the last resort when no row survives"
        );
        assert_eq!(
            focus_restore(false, false, true),
            FocusRestore::Origin,
            "a narrow host keeps the incumbent restore"
        );

        // The real flow: the previewed order is cancelled, the model clears
        // its preview (the row unmounts) and reconciles the cursor to the
        // surviving sibling. The gate is the render set the list retained, not
        // the bare cursor: the sibling is still shown, so it can take focus.
        let (model, _platform) = claim_demo_model("dialog-deleted-origin");
        let orders = place_two_orders(&model);
        let previewed = orders[0].position_id.clone();
        let sibling = orders[1].position_id.clone();
        model.open_preview(ListKind::Orders, &previewed);
        assert_eq!(
            model.preview(ListKind::Orders).as_deref(),
            Some(previewed.as_str())
        );

        desktop_cancel_dispatch(&model, &previewed);
        let survives = model.preview(ListKind::Orders).as_deref() == Some(previewed.as_str());
        assert!(!survives, "the deleted record's preview is gone");
        assert_eq!(
            model.cursor(ListKind::Orders).as_deref(),
            Some(sibling.as_str()),
            "the neighbour is the nearest surviving row"
        );
        let rendered = upcoming_rows(&model);
        assert_eq!(rendered, vec![sibling.clone()], "the sibling still renders");
        assert_eq!(
            nearest_restore_cursor(Some(sibling.as_str()), &rendered),
            Some(sibling.as_str()),
            "a row the render shows may take the restore"
        );
        assert_eq!(
            focus_restore(
                true,
                survives,
                nearest_restore_cursor(Some(sibling.as_str()), &rendered).is_some()
            ),
            FocusRestore::Nearest
        );

        // A stale cursor alone is not proof: after a segment/filter change the
        // cursor can name an id this render no longer shows, the list has
        // pruned its handle, and the restore must fall back to the heading
        // rather than focus a dead row.
        let stale = past_rows(&model);
        assert!(
            stale.is_empty(),
            "a segment change renders a different row set"
        );
        assert_eq!(nearest_restore_cursor(Some(sibling.as_str()), &stale), None);
        assert_eq!(
            focus_restore(
                true,
                false,
                nearest_restore_cursor(Some(sibling.as_str()), &stale).is_some()
            ),
            FocusRestore::Heading
        );

        // Deleting the last surviving row leaves nothing to restore. A single
        // cancel is raised from the previewed order's detail, so the row is
        // previewed first: the model reconciles a cleared preview, and with no
        // surviving row the fallback cursor is empty.
        model.open_preview(ListKind::Orders, &sibling);
        assert_eq!(
            model.preview(ListKind::Orders).as_deref(),
            Some(sibling.as_str()),
            "the row to delete is previewed, as the detail's cancel button is"
        );
        desktop_cancel_dispatch(&model, &sibling);
        assert_eq!(model.preview(ListKind::Orders), None);
        assert_eq!(model.cursor(ListKind::Orders), None);
        let rendered = upcoming_rows(&model);
        assert_eq!(
            nearest_restore_cursor(model.cursor(ListKind::Orders).as_deref(), &rendered),
            None
        );
        assert_eq!(
            focus_restore(
                true,
                false,
                nearest_restore_cursor(model.cursor(ListKind::Orders).as_deref(), &rendered)
                    .is_some()
            ),
            FocusRestore::Heading
        );
    }

    /// The order identities the upcoming segment renders, i.e. the row set a
    /// list render retains.
    fn upcoming_rows(model: &AppModel) -> Vec<String> {
        model
            .state()
            .orders_split
            .as_ref()
            .map(|split| {
                split
                    .upcoming
                    .iter()
                    .map(|order| order.position_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The order identities the past segment renders.
    fn past_rows(model: &AppModel) -> Vec<String> {
        model
            .state()
            .orders_split
            .as_ref()
            .map(|split| {
                split
                    .past
                    .iter()
                    .map(|order| order.position_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The synthetic evidence entry is absent from a release build, so its
/// fixture coverage is debug-only.
#[cfg(all(test, debug_assertions))]
mod evidence_tests {
    use snackpilot_model::{AppModel, FakePlatform};
    use std::sync::Arc;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-app-evidence-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The isolated fixture entry renders every required synthetic state, and
    /// a normal store ignores it. The wrapper host records zero normal-slot
    /// accesses, notifications, permissions or analytics for the whole run.
    /// @req REQ-UI-16
    #[test]
    fn evidence_fixture_states_apply_only_to_an_isolated_store() {
        // A normal launch: the entry is inert, so fixture state cannot leak
        // into production rendering or storage.
        let normal_host = Arc::new(FakePlatform::new());
        let normal = AppModel::new(temp_dir("normal"), normal_host.clone()).unwrap();
        normal.apply_evidence_scenario("pending");
        assert!(
            !normal.has_pending_changes(),
            "a normal store ignores the evidence fixture"
        );
        assert!(
            normal.state().menu_snapshot.is_none(),
            "a normal store has no synthetic snapshot"
        );

        // An isolated run: the same entry fills the selected state.
        let host = Arc::new(FakePlatform::new());
        let model = AppModel::new(
            temp_dir("isolated"),
            AppModel::evidence_platform(host.clone()),
        )
        .unwrap();
        model.set_evidence_mode(true);
        model.bootstrap();

        model.apply_evidence_scenario("populated");
        assert!(
            model
                .state()
                .menu_snapshot
                .is_some_and(|snapshot| !snapshot.items.is_empty()),
            "populated shows the offline demo menu"
        );

        model.apply_evidence_scenario("empty");
        assert!(
            model
                .state()
                .menu_snapshot
                .is_some_and(|snapshot| snapshot.items.is_empty()),
            "empty has no menu rows"
        );
        assert!(
            model
                .state()
                .orders_split
                .is_some_and(|split| split.upcoming.is_empty() && split.past.is_empty()),
            "empty has no order rows"
        );

        model.apply_evidence_scenario("loading");
        assert!(model.state().menu_snapshot.is_none(), "loading has no rows");
        assert!(model.state().busy, "loading shows the initial spinner");

        model.apply_evidence_scenario("refresh");
        model.apply_evidence_scenario("populated");
        model.apply_evidence_scenario("refresh");
        assert!(
            model
                .state()
                .menu_snapshot
                .is_some_and(|snapshot| snapshot.refreshing && !snapshot.items.is_empty()),
            "refresh keeps the rows and adds the neutral refresh state"
        );

        model.apply_evidence_scenario("error");
        assert!(
            model.state().error_text.is_some(),
            "error shows the inline model message"
        );

        model.apply_evidence_scenario("pending");
        assert!(
            model.has_pending_changes(),
            "pending marks the selected day's menu"
        );

        model.apply_evidence_scenario("progress");
        assert_eq!(
            model.state().order_progress.as_deref(),
            Some("Bestellung wird bestätigt..."),
            "progress reuses the existing exact phase label"
        );

        model.apply_evidence_scenario("permission-denied");
        assert_eq!(
            model
                .state()
                .location_alert
                .as_ref()
                .map(|alert| alert.title.as_str()),
            Some("Berechtigung fehlt"),
            "the synthetic permission result is the existing alert"
        );
        model.apply_evidence_scenario("permission-granted");
        assert!(
            model.state().location_alert.is_none(),
            "granted permission shows no alert"
        );

        // The whole fixture run stays inside the wrapper: no normal slot, no
        // notification, no permission request, no geofence, no analytics.
        assert!(
            host.calls().is_empty(),
            "evidence fixtures must not call the host platform: {:?}",
            host.calls()
        );
        assert!(
            host.tracks().is_empty(),
            "evidence fixtures must not send analytics: {:?}",
            host.tracks()
        );
        for slot in [
            "gourmet_username",
            "gourmet_password",
            "ventopay_username",
            "ventopay_password",
        ] {
            assert_eq!(host.credential(slot), None, "normal slot {slot} untouched");
        }
    }
}
