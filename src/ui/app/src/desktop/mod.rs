//! Desktop host module (desktop-ui-redesign tasks 1.5/1.6).
//!
//! One place for the desktop-only host concerns: the standard decorated
//! window (default 1180×780, minimum 360×480, clamped to the work area, title
//! `SnackPilot`), the shared keyboard command map, the native macOS menu bar
//! and the Dock-reopen rule that keeps one model and one main window.
//!
//! Everything that can be pure (command mapping, window clamping, About and
//! privacy copy) is a plain function with unit tests here; the GPUI adapter is
//! [`DesktopFrame`] and [`mount`].
//! @req REQ-UI-07 @req REQ-UI-11 @req REQ-UI-14

pub mod macos;
pub mod metadata;
pub mod storage;

use std::sync::Mutex;

use gpui::*;
use snackpilot_model::desktop::DesktopCommand;
use snackpilot_model::AppModel;

use crate::app::AppShell;
use gpui_kit::component::Root;

/// Window title (brief "Native macOS host": the titlebar is simply
/// `SnackPilot`, never a duplicate custom toolbar).
pub const WINDOW_TITLE: &str = "SnackPilot";
/// Default content size at first launch.
pub const WINDOW_DEFAULT_WIDTH: f32 = 1180.;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 780.;
/// Minimum content size: 360 wide, 480 high.
pub const WINDOW_MIN_WIDTH: f32 = 360.;
pub const WINDOW_MIN_HEIGHT: f32 = 480.;

/// Which native modifier carries the app shortcuts (`Cmd` on macOS, `Ctrl`
/// on Windows; Linux builds are compile-only and use Ctrl).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformModifier {
    Command,
    Control,
}

impl PlatformModifier {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            PlatformModifier::Command
        } else {
            PlatformModifier::Control
        }
    }

    /// Keymap prefix for [`gpui::KeyBinding`].
    fn prefix(self) -> &'static str {
        match self {
            PlatformModifier::Command => "cmd",
            PlatformModifier::Control => "ctrl",
        }
    }
}

/// Map one keystroke to a desktop command (REQ-UI-11).
///
/// Only the platform modifier is accepted; the other platform's modifier,
/// Alt/Option, Shift and auto-repeat all reject the chord. Repeat rejection is
/// deliberately applied to navigation too: a held Cmd+R must never queue a
/// second refresh, and the mutation guards above it stay the only dispatch
/// path.
pub fn shortcut_command(
    platform: PlatformModifier,
    key: &str,
    cmd: bool,
    ctrl: bool,
    alt: bool,
    shift: bool,
    is_held: bool,
) -> Option<DesktopCommand> {
    if is_held || alt || shift {
        return None;
    }
    let primary = match platform {
        PlatformModifier::Command => cmd && !ctrl,
        PlatformModifier::Control => ctrl && !cmd,
    };
    if !primary {
        return None;
    }
    match key.to_ascii_lowercase().as_str() {
        "1" => Some(DesktopCommand::OpenMenus),
        "2" => Some(DesktopCommand::OpenOrders),
        "3" => Some(DesktopCommand::OpenBilling),
        "4" | "," => Some(DesktopCommand::OpenSettings),
        "r" => Some(DesktopCommand::Refresh),
        _ => None,
    }
}

/// The app key bindings for the current desktop platform. Escape goes to the
/// AppShell's Back action; arrows are never bound globally, so text editing and
/// read-only receipt scrolling keep their native meaning.
///
/// Only macOS registers the six Bearbeiten key equivalents, because only macOS
/// has that native menu bar: the menu draws a key equivalent from the binding
/// of the item's own action, so without these the native Edit items would show
/// names and OS selectors but no ⌘ glyphs. The items keep their native
/// selectors, and an unhandled binding propagates to the focused element, whose
/// own text editing bindings run — so no editing chord is intercepted.
/// Windows has no global menu, so it registers neither the Edit equivalents nor
/// the Cmd-only window chords, and text editing keeps every key it owns.
pub fn key_bindings(platform: PlatformModifier) -> Vec<KeyBinding> {
    let prefix = platform.prefix();
    let chord = |key: &str| format!("{prefix}-{key}");
    let mut bindings = vec![
        KeyBinding::new(&chord("1"), macos::MenuOpenMenus, None),
        KeyBinding::new(&chord("2"), macos::MenuOpenOrders, None),
        KeyBinding::new(&chord("3"), macos::MenuOpenBilling, None),
        KeyBinding::new(&chord(","), macos::MenuOpenSettings, None),
        KeyBinding::new(&chord("4"), macos::MenuOpenSettings, None),
        KeyBinding::new(&chord("r"), macos::MenuRefresh, None),
        KeyBinding::new("escape", macos::MenuBack, None),
    ];
    if platform == PlatformModifier::Command {
        bindings.extend([
            KeyBinding::new("cmd-z", macos::MenuUndo, None),
            KeyBinding::new("cmd-shift-z", macos::MenuRedo, None),
            KeyBinding::new("cmd-x", macos::MenuCut, None),
            KeyBinding::new("cmd-c", macos::MenuCopy, None),
            KeyBinding::new("cmd-v", macos::MenuPaste, None),
            KeyBinding::new("cmd-a", macos::MenuSelectAll, None),
            KeyBinding::new("cmd-w", macos::MenuCloseWindow, None),
            KeyBinding::new("cmd-m", macos::MenuMinimize, None),
            KeyBinding::new("cmd-h", macos::MenuHide, None),
            KeyBinding::new("alt-cmd-h", macos::MenuHideOthers, None),
            KeyBinding::new("cmd-q", macos::MenuQuit, None),
        ]);
    }
    bindings
}

/// Clamp the desired window into the display's visible work area, keeping the
/// minimum content size. Returns `(x, y, width, height)` in logical px.
pub fn clamp_window(desired: (f32, f32), work: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let (work_x, work_y, work_w, work_h) = work;
    let width = desired.0.min(work_w).max(WINDOW_MIN_WIDTH.min(work_w));
    let height = desired.1.min(work_h).max(WINDOW_MIN_HEIGHT.min(work_h));
    let x = work_x + ((work_w - width) / 2.0).max(0.0);
    let y = work_y + ((work_h - height) / 2.0).max(0.0);
    (x, y, width, height)
}

/// The standard decorated desktop window: title `SnackPilot`, resizable, no
/// transparent titlebar, 1180×780 clamped to the visible work area and a
/// 360×480 minimum (REQ-UI-07).
pub fn window_options(cx: &App) -> WindowOptions {
    let desired = (WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT);
    let (x, y, width, height) = match cx.primary_display() {
        Some(display) => {
            let bounds = display.visible_bounds();
            clamp_window(
                desired,
                (
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ),
            )
        }
        None => clamp_window(desired, (0., 0., desired.0, desired.1)),
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(x), px(y)),
            size(px(width), px(height)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some(WINDOW_TITLE.into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        is_resizable: true,
        window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
        ..Default::default()
    }
}

pub use metadata::{product_build, product_version};

/// The themed About body lines (brief §9), with real version/build metadata
/// and the retained GourmetClient credit.
pub fn about_message(version: &str, build: &str) -> String {
    format!(
        "SnackPilot\nVersion {version} ({build})\n\nBestellungen und Abrechnung für die Betriebskantine.\n\nBasierend auf GourmetClient von patrickl92.\ngithub.com/patrickl92/GourmetClient"
    )
}

/// The human-approved Aptabase correction (`docs/requirements/07-release.md`
/// is authoritative), shared by the mobile Datenschutz alert and the desktop
/// summary.
pub const PRIVACY_PARAGRAPH: &str = "Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über eine selbst gehostete Aptabase-Instanz — ohne Nutzer- oder Geräte-IDs und ohne appübergreifendes Tracking. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.";

/// The Datenschutz summary: the approved Aptabase paragraph plus the truthful
/// desktop-only sentence, which differs only in the platform name.
pub fn privacy_message() -> String {
    let desktop = if cfg!(target_os = "windows") {
        "Unter Windows werden derzeit keine Nutzungsstatistiken gesendet."
    } else {
        "Auf dem Mac werden derzeit keine Nutzungsstatistiken gesendet."
    };
    format!("{PRIVACY_PARAGRAPH}\n\n{desktop}")
}

/// Install the host pieces that need no window: key bindings, the native menu
/// handlers and the Dock reopen rule. The menu bar itself is synced from the
/// shell on every state change.
pub fn install(cx: &mut App) {
    cx.bind_keys(key_bindings(PlatformModifier::current()));
    macos::install_handlers(cx);
}

/// Register the Dock-reopen rule on the application builder. One model, one
/// main window: activation after a close reopens the window instead of
/// constructing a second session (REQ-UI-07). The desktop binary applies this
/// before `run`; without it macOS still starts with the same installed model.
pub fn application_hook(app: gpui::Application) -> gpui::Application {
    app.on_reopen(|cx| open_main_window(cx));
    app
}

/// Rebuild the native menu bar only when its checked/enabled state moved.
pub fn sync_menus(cx: &mut App, model: Option<&AppModel>) {
    static SIGNATURE: Mutex<Option<String>> = Mutex::new(None);
    let has_window = !cx.windows().is_empty();
    let signature = macos::signature(model, has_window);
    {
        let mut current = SIGNATURE.lock().expect("menu signature lock");
        if current.as_deref() == Some(signature.as_str()) {
            return;
        }
        *current = Some(signature);
    }
    let menus = macos::menus(model, has_window);
    #[cfg(target_os = "macos")]
    cx.set_menus(menus);
    #[cfg(not(target_os = "macos"))]
    let _ = (cx, menus);
}

/// Report host focus facts to the model without waking its poll loop when
/// nothing changed.
pub fn report_scope(model: Option<&AppModel>, modal: bool, text_focused: bool) {
    if let Some(model) = model {
        if model.state().modal_open != modal {
            model.set_modal_open(modal);
        }
        if model.state().text_input_focused != text_focused {
            model.set_text_input_focused(text_focused);
        }
    }
}

/// Desktop frame: reports the live content width and host kind to the model,
/// then lays the shell out at [`crate::desktop_content_width`].
pub struct DesktopFrame {
    pub root: Entity<AppShell>,
}

impl Render for DesktopFrame {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        let content_width = crate::desktop_content_width(width);
        let model = self.root.read(cx).model.clone();
        if let Some(model) = model {
            if model.state().host_kind != snackpilot_model::desktop::HostKind::Desktop {
                model.set_host_kind(snackpilot_model::desktop::HostKind::Desktop);
            }
            if model.state().content_width != content_width {
                model.set_content_width(content_width);
            }
        }
        div()
            .size_full()
            .flex()
            .justify_center()
            .child(div().h_full().w(px(content_width)).child(self.root.clone()))
    }
}

/// Mount the desktop shell: shared init, host installation and one main
/// window. A gallery launch never reaches this path (debug only).
pub fn mount(cx: &mut App) {
    crate::init_kit(cx);
    install(cx);
    open_main_window(cx);
}

/// Open the single main window when none is visible. Dock reopen calls this
/// with the process-wide model already installed, so the session and its
/// navigation survive closing the window (REQ-UI-07).
pub fn open_main_window(cx: &mut App) {
    if !cx.windows().is_empty() {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
        }
        return;
    }
    let options = window_options(cx);
    cx.open_window(options, |window, cx| {
        let content = cx.new(|cx| AppShell::new(crate::shell_model(), cx));
        let frame = cx.new(|_cx| DesktopFrame { root: content });
        cx.new(|cx| Root::new(frame, window, cx))
    })
    .expect("open desktop window");
    cx.activate(true);
}

#[cfg(test)]
mod tests {
    use super::{
        about_message, clamp_window, key_bindings, privacy_message, product_build, product_version,
        shortcut_command, PlatformModifier, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
    };
    use snackpilot_model::desktop::DesktopCommand;

    /// Cmd+1–4/comma/R map to the documented commands on macOS, the Ctrl
    /// equivalents on Windows, and every other modifier/repeat is rejected
    /// (REQ-UI-11).
    /// @req REQ-UI-11
    #[test]
    fn command_mapping_is_platform_exact() {
        let mac = PlatformModifier::Command;
        assert_eq!(
            shortcut_command(mac, "1", true, false, false, false, false),
            Some(DesktopCommand::OpenMenus)
        );
        assert_eq!(
            shortcut_command(mac, "2", true, false, false, false, false),
            Some(DesktopCommand::OpenOrders)
        );
        assert_eq!(
            shortcut_command(mac, "3", true, false, false, false, false),
            Some(DesktopCommand::OpenBilling)
        );
        assert_eq!(
            shortcut_command(mac, "4", true, false, false, false, false),
            Some(DesktopCommand::OpenSettings)
        );
        assert_eq!(
            shortcut_command(mac, ",", true, false, false, false, false),
            Some(DesktopCommand::OpenSettings)
        );
        assert_eq!(
            shortcut_command(mac, "r", true, false, false, false, false),
            Some(DesktopCommand::Refresh)
        );
        assert_eq!(
            shortcut_command(mac, "r", true, false, false, false, true),
            None,
            "key repeat must never dispatch a second refresh"
        );
        assert_eq!(
            shortcut_command(mac, "r", true, true, false, false, false),
            None,
            "the other platform's modifier must not match"
        );
        assert_eq!(
            shortcut_command(mac, "1", true, false, true, false, false),
            None
        );
        assert_eq!(
            shortcut_command(mac, "1", false, false, false, false, false),
            None
        );

        let windows = PlatformModifier::Control;
        assert_eq!(
            shortcut_command(windows, "2", false, true, false, false, false),
            Some(DesktopCommand::OpenOrders)
        );
        assert_eq!(
            shortcut_command(windows, "r", false, true, false, false, false),
            Some(DesktopCommand::Refresh)
        );
        assert_eq!(
            shortcut_command(windows, "r", true, false, false, false, false),
            None,
            "macOS Cmd must not be the Windows binding"
        );

        // The bindings exist as parseable chords for the exact shortcuts and
        // never bind arrows, so text editing and read-only scrolling keep
        // their keys. `unparse` gives the chord names; `Display` would render
        // macOS glyphs (⌘1) and cannot be compared to `cmd-1`.
        let keys = |platform| -> Vec<String> {
            key_bindings(platform)
                .iter()
                .flat_map(|binding| binding.keystrokes().iter().map(|stroke| stroke.unparse()))
                .collect()
        };
        let mac_keys = keys(mac);
        for expected in [
            "cmd-1", "cmd-2", "cmd-3", "cmd-4", "cmd-,", "cmd-r", "escape",
        ] {
            assert!(
                mac_keys.iter().any(|key| key == expected),
                "missing binding {expected}"
            );
        }
        let windows_keys = keys(windows);
        for expected in [
            "ctrl-1", "ctrl-2", "ctrl-3", "ctrl-4", "ctrl-,", "ctrl-r", "escape",
        ] {
            assert!(
                windows_keys.iter().any(|key| key == expected),
                "missing binding {expected}"
            );
        }
        // Only macOS has the native Bearbeiten menu, so only macOS registers
        // its key equivalents; Windows keeps every editing chord for the
        // focused text field and has no Cmd-only window chords at all.
        assert!(
            mac_keys.iter().all(|key| key.starts_with("cmd-")
                || key == "escape"
                || key.starts_with("alt-cmd-")),
            "macOS registers the app chords and the Bearbeiten equivalents"
        );
        for edit in [
            "ctrl-z",
            "ctrl-shift-z",
            "ctrl-x",
            "ctrl-c",
            "ctrl-v",
            "ctrl-a",
        ] {
            assert!(
                !windows_keys.iter().any(|key| key == edit),
                "{edit} must stay with text editing on a host without a menu"
            );
        }
        for keys in [&mac_keys, &windows_keys] {
            assert!(
                keys.iter().all(|key| !key.contains("up"))
                    && keys.iter().all(|key| !key.contains("down")),
                "arrow keys must stay with the focused group"
            );
        }
    }

    /// First launch is 1180×780 clamped to the visible work area; the minimum
    /// content size survives a smaller screen, and the window stays centred.
    /// @req REQ-UI-07
    #[test]
    fn window_bounds_clamp_to_the_work_area() {
        let (x, y, width, height) = clamp_window((1180., 780.), (0., 0., 1440., 900.));
        assert_eq!((width, height), (1180., 780.));
        assert_eq!((x, y), (130., 60.), "centred inside the work area");

        let (_, _, width, height) = clamp_window((1180., 780.), (0., 25., 1000., 700.));
        assert_eq!((width, height), (1000., 700.));
        assert!(width >= WINDOW_MIN_WIDTH && height >= WINDOW_MIN_HEIGHT);

        let (_, _, width, height) = clamp_window((1180., 780.), (0., 0., 320., 400.));
        assert_eq!(
            (width, height),
            (320., 400.),
            "an OS constraint below the minimum still fits the visible area"
        );
    }

    /// About uses real metadata (never the app crate's 0.0.0) and keeps the
    /// GourmetClient credit; the desktop privacy paragraph is truthful and
    /// leaves the incumbent paragraph intact.
    /// @req REQ-UI-14
    #[test]
    fn about_and_privacy_copy_are_truthful() {
        let version = product_version();
        let build = product_build();
        assert_ne!(version, "0.0.0");
        assert!(!version.is_empty());
        assert_ne!(build, "0");
        assert_eq!(
            super::metadata::yaml_value(
                "a: 1\nMARKETING_VERSION: \"2.1.0\"\n",
                "MARKETING_VERSION"
            )
            .as_deref(),
            Some("2.1.0")
        );
        assert_eq!(
            super::metadata::yaml_value("CURRENT_PROJECT_VERSION: 35", "CURRENT_PROJECT_VERSION")
                .as_deref(),
            Some("35")
        );
        let about = about_message("2.1.0", "35");
        assert!(about.starts_with("SnackPilot\nVersion 2.1.0 (35)"));
        assert!(about.contains("github.com/patrickl92/GourmetClient"));
        assert!(about.contains("Basierend auf GourmetClient von patrickl92."));
        let privacy = privacy_message();
        assert!(privacy.contains("selbst gehostete Aptabase-Instanz"));
        assert!(
            !privacy.contains("TelemetryDeck"),
            "07-release.md is authoritative: the historical provider is gone"
        );
        assert!(privacy.contains("ohne Nutzer- oder Geräte-IDs"));
        assert!(
            privacy.contains("Auf dem Mac werden derzeit keine Nutzungsstatistiken gesendet.")
                || privacy
                    .contains("Unter Windows werden derzeit keine Nutzungsstatistiken gesendet.")
        );
    }
}
