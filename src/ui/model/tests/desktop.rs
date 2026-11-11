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
    action_fill, action_hover_fill, action_pressed_fill, boundary_ink, command_allowed,
    contrast_ratio, control_fill, control_ink, danger_fill, desktop_geometry, focus_ink,
    layout_mode, mix_toward, nearest_surviving, on_fill, panel_fill, parse_color, rail_fill,
    readable_ink, text_ink, ControlState, DesktopCommand, EscapeAction, HostKind, KeyScope,
    LayoutMode, ACTION_HOVER_MIX, ACTION_PRESSED_MIX, CONTENT_MAX, CONTROL_MIN_CONTRAST,
    DIVIDER_WIDTH, FOCUS_GAP, FOCUS_WIDTH, MASTER_COMPACT, OUTER_COMPACT, OUTER_EXPANDED,
    PHONE_CONTENT_MAX, RAIL_WIDTH_COMPACT, RAIL_WIDTH_EXPANDED, TARGET_MIN, TEXT_MIN_CONTRAST,
};
use snackpilot_model::navigation::{ListKind, Screen, SettingsDetail, Tab};
use snackpilot_model::tokens::{ACCENTS, BACKGROUND, ERROR, SURFACE, TEXT_PRIMARY, TEXT_TERTIARY};
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

/// Geometry is derived, so every assertion compares within a hundredth of a
/// logical pixel; the brief's numbers are whole pixels.
fn assert_px(got: f32, want: f32, what: &str) {
    assert!(
        (got - want).abs() < 0.01,
        "{what}: got {got}, expected {want}"
    );
}

/// The shell may only ever lay these boxes out edge to edge: the rail, the two
/// region gutters and the workspace tile the content width, and the two panes
/// plus the divider tile the workspace (brief "Responsive geometry").
fn assert_tiles(
    rail: f32,
    gutter: f32,
    workspace: f32,
    master: f32,
    divider: f32,
    detail: f32,
    content: f32,
) {
    assert_px(
        rail + gutter * 2.0 + workspace,
        content,
        "rail + gutters + workspace",
    );
    assert_px(
        master + divider + detail,
        workspace,
        "master + divider + detail",
    );
}

#[test]
/// @req REQ-UI-07
fn desktop_geometry_at_boundaries() {
    // 719: still the phone shell — no rail, no split, incumbent 480 cap.
    let g = desktop_geometry(HostKind::Desktop, 719.0);
    assert_px(g.rail_width, 0., "719 rail");
    assert_px(g.outer_gutter, 0., "719 gutter");
    assert_px(g.divider_width, 0., "719 divider");
    assert_px(g.workspace_width, PHONE_CONTENT_MAX, "719 workspace");
    assert_px(
        g.workspace_width,
        480.,
        "719 workspace equals the phone cap",
    );
    assert_px(g.master_width, 480., "719 master");
    assert_px(g.detail_width, 0., "719 detail");
    assert_px(
        g.master_width + g.divider_width + g.detail_width,
        g.workspace_width,
        "719 panes tile the workspace",
    );

    // Host-gated: a phone or tablet never gets the rail dialect from width.
    assert_px(
        desktop_geometry(HostKind::Phone, 1280.0).rail_width,
        0.,
        "phone rail",
    );
    assert_px(
        desktop_geometry(HostKind::Tablet, 1280.0).rail_width,
        0.,
        "tablet rail",
    );
    assert_px(
        desktop_geometry(HostKind::Phone, 1280.0).workspace_width,
        480.,
        "phone workspace",
    );

    // 720: 176 rail, 16 gutters, 512 workspace = 248 + 1 + 263.
    let g = desktop_geometry(HostKind::Desktop, 720.0);
    assert_px(g.rail_width, RAIL_WIDTH_COMPACT, "720 rail");
    assert_px(g.rail_width, 176., "720 rail is the compact constant");
    assert_px(g.outer_gutter, OUTER_COMPACT, "720 gutter");
    assert_px(g.outer_gutter, 16., "720 gutter is the compact constant");
    assert_px(g.workspace_width, 512., "720 workspace");
    assert_px(g.master_width, MASTER_COMPACT, "720 master");
    assert_px(g.master_width, 248., "720 master is the compact constant");
    assert_px(g.divider_width, DIVIDER_WIDTH, "720 divider");
    assert_px(g.divider_width, 1., "720 divider is one pixel");
    assert_px(g.detail_width, 263., "720 detail");
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        720.0,
    );

    // 768: the evidence checkpoint, not a breakpoint — master stays 248.
    let g = desktop_geometry(HostKind::Desktop, 768.0);
    assert_px(g.rail_width, 176., "768 rail");
    assert_px(g.outer_gutter, 16., "768 gutter");
    assert_px(g.workspace_width, 560., "768 workspace");
    assert_px(g.master_width, 248., "768 master");
    assert_px(g.detail_width, 311., "768 detail");
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        768.0,
    );

    // 1023: the last compact width; detail takes the whole remainder.
    let g = desktop_geometry(HostKind::Desktop, 1023.0);
    assert_px(g.rail_width, 176., "1023 rail");
    assert_px(g.outer_gutter, 16., "1023 gutter");
    assert_px(g.master_width, 248., "1023 master");
    assert_px(g.workspace_width, 1023. - 176. - 32., "1023 workspace");
    assert_px(
        g.detail_width,
        1023. - 176. - 32. - 248. - 1.,
        "1023 detail",
    );
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        1023.0,
    );

    // 1024: expanded — 192 rail, 24 gutters, 56/44 of the interior width.
    let g = desktop_geometry(HostKind::Desktop, 1024.0);
    assert_px(g.rail_width, RAIL_WIDTH_EXPANDED, "1024 rail");
    assert_px(g.rail_width, 192., "1024 rail is the expanded constant");
    assert_px(g.outer_gutter, OUTER_EXPANDED, "1024 gutter");
    assert_px(g.outer_gutter, 24., "1024 gutter is the expanded minimum");
    assert_px(g.workspace_width, 784., "1024 workspace");
    assert_px(g.master_width, (784. - 1.) * 0.56, "1024 master");
    assert_px(g.detail_width, (784. - 1.) * 0.44, "1024 detail");
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        1024.0,
    );

    // 1280: the workspace hits its 960 cap, so the gutters grow to 64 each.
    let g = desktop_geometry(HostKind::Desktop, 1280.0);
    assert_px(g.rail_width, 192., "1280 rail");
    assert_px(g.workspace_width, CONTENT_MAX, "1280 workspace");
    assert_px(g.workspace_width, 960., "1280 workspace is the content cap");
    assert_px(g.outer_gutter, 64., "1280 gutters");
    assert_px(g.master_width, (960. - 1.) * 0.56, "1280 master");
    assert_px(g.detail_width, (960. - 1.) * 0.44, "1280 detail");
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        1280.0,
    );

    // Past the cap the gutters absorb the extra width; the workspace holds.
    let g = desktop_geometry(HostKind::Desktop, 1600.0);
    assert_px(g.workspace_width, 960., "1600 workspace stays capped");
    assert_px(g.outer_gutter, (1600. - 192. - 960.) / 2., "1600 gutters");
    assert_tiles(
        g.rail_width,
        g.outer_gutter,
        g.workspace_width,
        g.master_width,
        g.divider_width,
        g.detail_width,
        1600.0,
    );

    // Every desktop target is at least 44 px, whatever the width.
    assert_px(TARGET_MIN, 44., "desktop target floor");

    // The shell reads the same numbers through the model, on every resize.
    let (model, _fake) = demo_model("geometry");
    model.set_host_kind(HostKind::Desktop);
    model.set_content_width(720.0);
    assert_px(
        model.desktop_geometry().detail_width,
        263.,
        "model 720 detail",
    );
    model.set_content_width(1280.0);
    assert_px(
        model.desktop_geometry().outer_gutter,
        64.,
        "model 1280 gutter",
    );
    model.set_host_kind(HostKind::Phone);
    assert_px(model.desktop_geometry().rail_width, 0., "model phone rail");
}

#[test]
/// @req REQ-UI-13
fn desktop_contrast_all_accents() {
    // The two inks a filled action may resolve to are the existing constants,
    // not new hues: SURFACE.light (white) and BACKGROUND.dark (black).
    let white = parse_color(SURFACE.light);
    let black = parse_color(BACKGROUND.dark);

    for accent in ACCENTS.iter() {
        for dark in [false, true] {
            let colors = if dark { &accent.dark } else { &accent.light };
            let ink = text_ink(dark);
            let panel = panel_fill(dark);
            let rail = rail_fill(dark);

            let rest = action_fill(colors, dark);
            let hover = action_hover_fill(rest, ink);
            let pressed = action_pressed_fill(rest, ink);
            let danger = danger_fill(dark);

            // Hover and press are the brief's 8 % / 14 % linear-sRGB mixes of
            // the actual fill with text_primary, and really differ.
            assert_eq!(
                hover,
                mix_toward(rest, ink, ACTION_HOVER_MIX),
                "{} hover mix",
                accent.id
            );
            assert_eq!(
                pressed,
                mix_toward(rest, ink, ACTION_PRESSED_MIX),
                "{} pressed mix",
                accent.id
            );
            assert_ne!(hover, rest, "{} hover must differ", accent.id);
            assert_ne!(
                pressed, hover,
                "{} pressed must differ from hover",
                accent.id
            );

            for (state, fill) in [
                ("action rest", rest),
                ("action hover", hover),
                ("action pressed", pressed),
                ("danger rest", danger),
                ("danger hover", action_hover_fill(danger, ink)),
                ("danger pressed", action_pressed_fill(danger, ink)),
            ] {
                // Text on the actual fill: the higher-contrast of the two
                // constants, chosen independently per fill, at 4.5:1 or better.
                let on = on_fill(fill);
                let ratio = contrast_ratio(on, fill);
                assert!(
                    ratio >= TEXT_MIN_CONTRAST,
                    "{} {state}: ink {on:?} only reaches {ratio:.2}:1",
                    accent.id
                );
                let preferred = if contrast_ratio(white, fill) >= contrast_ratio(black, fill) {
                    white
                } else {
                    black
                };
                assert_eq!(
                    on, preferred,
                    "{} {state}: ink is not the higher-contrast constant",
                    accent.id
                );

                // The fill is a control boundary: 3:1 against both containers.
                let on_panel = contrast_ratio(fill, panel);
                let on_rail = contrast_ratio(fill, rail);
                assert!(
                    on_panel >= CONTROL_MIN_CONTRAST && on_rail >= CONTROL_MIN_CONTRAST,
                    "{} {state}: fill {fill:?} reaches only {on_panel:.2}:1 / {on_rail:.2}:1",
                    accent.id
                );
            }

            // Accent as text (active rail row, selected desktop chip) is
            // resolved against the fill it really sits on: the accent stays
            // only where it clears 4.5:1, otherwise `text_primary` takes over.
            let accent_ink = parse_color(colors.primary);
            for (name, fill) in [
                ("panel", panel),
                ("rail", rail),
                ("selection", control_fill(ControlState::Selected, dark)),
            ] {
                let accent_text = readable_ink(fill, accent_ink);
                let ratio = contrast_ratio(accent_text, fill);
                assert!(
                    ratio >= TEXT_MIN_CONTRAST,
                    "{}: accent text on {name} is {ratio:.2}:1",
                    accent.id
                );
                let want = if contrast_ratio(accent_ink, fill) >= TEXT_MIN_CONTRAST {
                    accent_ink
                } else {
                    ink
                };
                assert_eq!(
                    accent_text, want,
                    "{}: accent text on {name} resolves against that fill",
                    accent.id
                );
            }

            // The selected desktop chip paints the accent on the selection fill
            // (not the panel), so that pair resolves there and must clear 4.5:1.
            let selected = control_fill(ControlState::Selected, dark);
            let chip_text = readable_ink(selected, accent_ink);
            let chip_ratio = contrast_ratio(chip_text, selected);
            assert!(
                chip_ratio >= TEXT_MIN_CONTRAST,
                "{}: selected chip text is {chip_ratio:.2}:1",
                accent.id
            );
            // Every desktop accent *edge* sits on that same selection fill, so
            // all three share one resolution at the 3:1 control floor: the
            // selected card's 1 px border (card.rs), the selected chip's 1 px
            // border (chip.rs) and the record table's 3 px selected-row rule
            // (record_table.rs) all paint `theme.desktop.accent_boundary`.
            // Light golden's accent misses 3:1 here, so a raw `theme.primary`
            // would be a real contrast defect, not a cosmetic one.
            let edge_ink = boundary_ink(selected, accent_ink);
            let edge = contrast_ratio(edge_ink, selected);
            assert!(
                edge >= CONTROL_MIN_CONTRAST,
                "{}: selected card/chip/table edge on its fill is {edge:.2}:1",
                accent.id
            );
            assert_eq!(
                edge_ink,
                if contrast_ratio(accent_ink, selected) >= CONTROL_MIN_CONTRAST {
                    accent_ink
                } else {
                    ink
                },
                "{}: selected card/chip/table edge resolves against its fill",
                accent.id
            );

            // Focus ring and body text against the containers they sit on.
            let focus = focus_ink(dark);
            let want_focus = if dark {
                TEXT_PRIMARY.dark
            } else {
                TEXT_PRIMARY.light
            };
            assert_eq!(focus, parse_color(want_focus), "focus ink is text_primary");
            for (name, container) in [("panel", panel), ("rail", rail)] {
                let ring = contrast_ratio(focus, container);
                assert!(
                    ring >= CONTROL_MIN_CONTRAST,
                    "{}: focus ring on {name} is {ring:.2}:1",
                    accent.id
                );
                let body = contrast_ratio(ink, container);
                assert!(
                    body >= TEXT_MIN_CONTRAST,
                    "{}: body ink on {name} is {body:.2}:1",
                    accent.id
                );
            }
        }
    }

    // readable_ink keeps the preferred ink only when it actually clears 4.5:1.
    use snackpilot_model::tokens::{TEXT_SECONDARY, TEXT_TERTIARY};
    let panel = panel_fill(false);
    let ink = text_ink(false);
    let secondary = parse_color(TEXT_SECONDARY.light);
    assert!(
        contrast_ratio(secondary, panel) >= TEXT_MIN_CONTRAST,
        "the pinned secondary ink is meant to stay readable"
    );
    assert_eq!(
        readable_ink(panel, secondary),
        secondary,
        "a readable preferred ink is kept"
    );
    let tertiary = parse_color(TEXT_TERTIARY.light);
    assert!(
        contrast_ratio(tertiary, panel) < TEXT_MIN_CONTRAST,
        "this test needs an ink that really fails 4.5:1"
    );
    assert_eq!(
        readable_ink(panel, tertiary),
        ink,
        "an unreadable preferred ink falls back to text_primary"
    );

    // The danger fill is the incumbent error token, not a new hue.
    assert_eq!(danger_fill(false), parse_color(ERROR.light));
    assert_eq!(danger_fill(true), parse_color(ERROR.dark));
}

/// A desktop component's accent edge is a control boundary on the selection
/// fill, so it paints `theme.desktop.accent_boundary` (`boundary_ink`), never
/// the raw `theme.primary`: light golden's accent misses 3:1 there. This is a
/// source guard because the app crate cannot be compiled on this host; it
/// reads the three component files as text and fails on a raw accent edge.
/// @req REQ-UI-13
#[test]
fn desktop_accent_edges_use_the_resolved_boundary_ink() {
    // Whitespace runs collapsed, so an edge written across lines still reads
    // as one expression.
    let squeeze = |src: &str| src.split_whitespace().collect::<Vec<_>>().join(" ");
    let selection = control_fill(ControlState::Selected, false);

    for (name, src) in [
        ("card.rs", include_str!("../../app/src/components/card.rs")),
        ("chip.rs", include_str!("../../app/src/components/chip.rs")),
        (
            "record_table.rs",
            include_str!("../../app/src/components/record_table.rs"),
        ),
    ] {
        let src = squeeze(src);
        assert!(
            src.contains("theme.desktop.accent_boundary"),
            "{name}: the accent edge must resolve through accent_boundary"
        );
        for raw in [
            ".border_color(if selected { theme.primary }",
            ".border_color(if state == ControlState::Selected { theme.primary }",
            ".bg(if selected { theme.primary }",
        ] {
            assert!(
                !src.contains(raw),
                "{name}: `{raw}` paints the raw accent; use theme.desktop.accent_boundary"
            );
        }
    }

    // The resolution is not a no-op for the accent that misses the floor.
    let golden = ACCENTS
        .iter()
        .find(|a| a.id == "golden")
        .expect("the golden accent exists");
    let golden_accent = parse_color(golden.light.primary);
    assert!(
        contrast_ratio(golden_accent, selection) < CONTROL_MIN_CONTRAST,
        "this guard needs an accent that really fails 3:1 on the selection fill"
    );
    assert_ne!(
        boundary_ink(selection, golden_accent),
        golden_accent,
        "light golden's accent cannot carry a selected edge; it falls back to ink"
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

/// The host wiring the desktop shell performs at startup and on resize: the
/// model starts as a phone, and set_host_kind/set_content_width drive the
/// exact geometry tiles the shell paints (rail 0 on phone, full shell on a
/// desktop-wide window, phone-like content below the 720 breakpoint).
/// @req REQ-UI-07
#[test]
fn host_layout_wiring_drives_geometry() {
    let (model, _fake) = demo_model("host-layout");
    let phone = model.desktop_geometry();
    assert_eq!(
        model.state().host_kind,
        HostKind::Phone,
        "model default host is phone"
    );
    assert_eq!(phone.rail_width, 0.0, "phone shows no rail");

    model.set_host_kind(HostKind::Desktop);
    model.set_content_width(1024.0);
    let wide = model.desktop_geometry();
    assert_eq!(wide.rail_width, RAIL_WIDTH_EXPANDED, "wide desktop rail");
    assert_eq!(wide.outer_gutter, OUTER_EXPANDED, "wide desktop gutter");
    assert_eq!(
        wide.workspace_width, 784.0,
        "1024 workspace is rail-and-gutter remainder"
    );
    assert_eq!(wide.divider_width, DIVIDER_WIDTH, "wide desktop divider");
    assert!(
        wide.detail_width > 0.0,
        "wide desktop shows the detail pane"
    );

    model.set_content_width(1280.0);
    let capped = model.desktop_geometry();
    assert_eq!(
        capped.workspace_width, CONTENT_MAX,
        "wide window hits the content cap"
    );
    assert_eq!(
        capped.outer_gutter, 64.0,
        "past the cap the gutters absorb the rest"
    );

    model.set_content_width(768.0);
    let mid = model.desktop_geometry();
    assert_eq!(mid.rail_width, RAIL_WIDTH_COMPACT, "compact desktop rail");
    assert_eq!(
        mid.master_width, MASTER_COMPACT,
        "compact desktop master is pinned"
    );

    model.set_content_width(600.0);
    let narrow = model.desktop_geometry();
    assert_eq!(narrow.rail_width, 0.0, "desktop below 720 is phone-like");
    assert_eq!(
        narrow.workspace_width, PHONE_CONTENT_MAX,
        "narrow desktop uses the phone column"
    );

    model.set_host_kind(HostKind::Phone);
    let phone = model.desktop_geometry();
    assert_eq!(phone.rail_width, 0.0, "back to phone: no rail");
    assert_eq!(
        phone.workspace_width, PHONE_CONTENT_MAX,
        "phone column stays capped"
    );
}

/// The shell's push/pop affordance goes through the model's navigation, so
/// one shared Root has no second authoritative stack (REQ-UI-04).
/// @req REQ-UI-04
#[test]
fn shell_push_and_pop_use_model_navigation() {
    let (model, _fake) = demo_model("push-pop");
    model.select_tab(Tab::Settings);
    assert_eq!(model.navigation().top(), Screen::Settings);
    model.push_screen(Screen::SettingsDetail);
    assert_eq!(model.navigation().top(), Screen::SettingsDetail);
    assert_eq!(model.navigation().depth(), 2);
    model.pop_screen();
    assert_eq!(model.navigation().top(), Screen::Settings);
    assert_eq!(model.navigation().depth(), 1);
    model.pop_screen();
    assert_eq!(model.navigation().depth(), 1, "the root screen never pops");
}

/// The orders segmented control is model state, not view state: the shell
/// renders `state.orders_past` and writes through set_orders_segment, so the
/// segment survives any view rebuild.
/// @req REQ-UI-04
#[test]
fn orders_segment_is_model_state() {
    let (model, _fake) = demo_model("orders-segment");
    assert!(
        !model.state().orders_past,
        "orders opens on the upcoming segment"
    );
    model.set_orders_segment(true);
    assert!(model.state().orders_past, "past segment is model state");
    model.set_orders_segment(false);
    assert!(!model.state().orders_past, "back to upcoming");
}

/// The shell dispatches desktop operations on the background executor, so the
/// future it spawns must be `Send`; the `Arc<AppModel>` it captures therefore
/// requires `AppModel: Send + Sync`. This is a compile-time guard, not a
/// runtime check (task 1.2, "move desktop operation dispatch off the render
/// thread").
/// @req REQ-UI-07
#[test]
fn app_model_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<snackpilot_model::AppModel>();
}

/// Shared compositions paint the brief's pointer/focus states from the pinned
/// tokens, so the desktop extension adds no colour literal and mobile keeps
/// its own defaults. Rest/hover/selected are the incumbent surface fills;
/// focus keeps the rest fill because the reserved ring carries it; disabled
/// keeps the fill and changes to the tertiary ink. The 44 px target and the
/// reserved 2 + 2 px ring are the brief's numbers.
/// @req REQ-UI-13
#[test]
fn desktop_control_states_resolve_from_tokens() {
    assert_eq!(TARGET_MIN, 44.0, "target floor");
    assert_eq!(FOCUS_WIDTH, 2.0, "focus ring width");
    assert_eq!(FOCUS_GAP, 2.0, "focus ring gap");

    for dark in [false, true] {
        let surface = parse_color(if dark { SURFACE.dark } else { SURFACE.light });
        let variant = parse_color(if dark {
            snackpilot_model::tokens::SURFACE_VARIANT.dark
        } else {
            snackpilot_model::tokens::SURFACE_VARIANT.light
        });
        let selection = parse_color(if dark {
            snackpilot_model::tokens::PRIMARY_SURFACE.dark
        } else {
            snackpilot_model::tokens::PRIMARY_SURFACE.light
        });
        let tertiary = parse_color(if dark {
            TEXT_TERTIARY.dark
        } else {
            TEXT_TERTIARY.light
        });

        assert_eq!(
            control_fill(ControlState::Rest, dark),
            surface,
            "rest is the surface fill"
        );
        assert_eq!(
            control_fill(ControlState::Hover, dark),
            variant,
            "hover is surface_variant"
        );
        assert_eq!(
            control_fill(ControlState::Selected, dark),
            selection,
            "selected is primary_surface"
        );
        assert_eq!(
            control_fill(ControlState::Focus, dark),
            surface,
            "focus keeps the rest fill; the ring carries focus"
        );
        assert_eq!(
            control_fill(ControlState::Disabled, dark),
            surface,
            "disabled keeps the rest fill"
        );

        assert_eq!(
            control_ink(ControlState::Rest, dark),
            text_ink(dark),
            "rest ink is text_primary"
        );
        assert_eq!(
            control_ink(ControlState::Selected, dark),
            text_ink(dark),
            "selected ink is text_primary"
        );
        assert_eq!(
            control_ink(ControlState::Disabled, dark),
            tertiary,
            "disabled ink is text_tertiary"
        );

        // Focus is visible, distinct from rest and from selection, on both the
        // panel and the rail at the 3:1 control floor.
        assert!(ControlState::Focus.ring_visible());
        assert!(!ControlState::Selected.ring_visible());
        for (name, bg) in [("panel", panel_fill(dark)), ("rail", rail_fill(dark))] {
            let ratio = contrast_ratio(focus_ink(dark), bg);
            assert!(
                ratio >= CONTROL_MIN_CONTRAST,
                "{name} focus ring only reaches {ratio:.2}:1"
            );
        }
    }
}
