//! gpui-kit views, theme, assets and entry points (openspec: gpui-kit-ui-rewrite).
//!
//! Task 2.1 spike: one `gpui_kit::component::Root` (REQ-UI-03) showing the four
//! tab labels and a counter button, mounted through `gpui_kit::init(cx)` — the
//! mobile hosts never call `gpui_kit::application()`.

pub mod app;
pub mod assets;
pub mod components;
pub mod desktop;
pub mod gallery;
pub mod navigation;
pub mod platform;
pub mod screens;
pub mod theme;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use gpui_kit::component::button::Button;
use gpui_kit::component::{Root, Theme, ThemeMode};
use gpui_kit::prelude::*;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use gpui_kit::{div, Context, Entity, Window};
use gpui_kit::{App, WindowOptions};
#[cfg(not(any(target_os = "ios", target_os = "android")))]

/// The spike root view: tab labels plus a tappable counter.
pub struct SpikeRoot {
    count: usize,
}

/// Mirror of the counter for the iOS host's accessibility label (task 2.2),
/// which the XCUITest asserts after a coordinate tap.
pub static SPIKE_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// C ABI read of the counter, polled by `Host/App.swift` (Embedding.h).
#[no_mangle]
pub extern "C" fn spike_count() -> usize {
    SPIKE_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl SpikeRoot {
    fn counter_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity: Entity<Self> = cx.entity().clone();
        Button::new("counter")
            .label(format!("Erhöhen — Zähler: {}", self.count))
            .on_click(move |_event, _window, cx| {
                log::info!("spike: counter button clicked");
                entity.update(cx, |this, cx| {
                    this.count += 1;
                    SPIKE_COUNT.store(this.count, std::sync::atomic::Ordering::Relaxed);
                    cx.notify();
                });
            })
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl Render for SpikeRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_6()
            .child(
                div()
                    .text_lg()
                    .child("Menüs / Bestellungen / Abrechnung / Einstellungen"),
            )
            .child(div().text_2xl().child(format!("{}", self.count)))
            .child(self.counter_button(cx))
    }
}

/// Read a debug system property (Android only; `debug.*` props are
/// shell-settable without root, and env vars cannot reach an app process).
#[cfg(all(target_os = "android", debug_assertions))]
fn debug_prop(name: &str) -> Option<String> {
    extern "C" {
        fn __system_property_get(
            name: *const std::ffi::c_char,
            value: *mut std::ffi::c_char,
        ) -> i32;
    }
    let name = std::ffi::CString::new(name).ok()?;
    let mut buf = [0; 32];
    if unsafe { __system_property_get(name.as_ptr(), buf.as_mut_ptr()) } <= 0 {
        return None;
    }
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| *b as u8)
        .collect();
    Some(String::from_utf8_lossy(&bytes).trim().to_string())
}

// Unused in Android release builds, where the only callers are stubbed out.
#[cfg(not(all(target_os = "android", debug_assertions)))]
#[allow(dead_code)]
fn debug_prop(_name: &str) -> Option<String> {
    None
}

/// Debug entry (task 4.1): `SNACKPILOT_GALLERY=1` (desktop, iOS simulator) or
/// `adb shell setprop debug.snackpilot.gallery 1` (Android) opens the component
/// gallery instead of the spike content. The section knob picks one half for a
/// screenshot — `SNACKPILOT_GALLERY_SECTION=light|dark`, or
/// `adb shell setprop debug.snackpilot.gallery.section light` — and defaults to
/// both stacked. On the iOS simulator the same two knobs are also accepted as
/// launch arguments (`--gallery`, `--gallery-section=light`), because
/// `simctl launch` env propagation is unreliable. `None` means the gallery was
/// not requested. Compiled out of release builds entirely: the gallery is a
/// debug entry, never reachable in a release binary.
#[cfg(debug_assertions)]
fn gallery_requested() -> Option<gallery::Section> {
    let arg = |name: &str| std::env::args().any(|a| a == name);
    let arg_value =
        |prefix: &str| std::env::args().find_map(|a| a.strip_prefix(prefix).map(str::to_owned));
    let on = std::env::var("SNACKPILOT_GALLERY").is_ok()
        || arg("--gallery")
        || debug_prop("debug.snackpilot.gallery").as_deref() == Some("1");
    if !on {
        return None;
    }
    let section = std::env::var("SNACKPILOT_GALLERY_SECTION")
        .ok()
        .or_else(|| debug_prop("debug.snackpilot.gallery.section"))
        .or_else(|| arg_value("--gallery-section="))
        .unwrap_or_default();
    Some(match section.as_str() {
        "light" => gallery::Section::Light,
        "dark" => gallery::Section::Dark,
        _ => gallery::Section::Both,
    })
}

/// Release builds never open the gallery (task 4.1 debug entry).
#[cfg(not(debug_assertions))]
fn gallery_requested() -> Option<gallery::Section> {
    None
}

/// Which board to show: `SNACKPILOT_GALLERY_PAGE=desktop|mobile|menus|orders|
/// billing` (or `--gallery-page=…`, or `debug.snackpilot.gallery.page`)
/// narrows the debug gallery to one board.
#[cfg(debug_assertions)]
fn gallery_page() -> Option<String> {
    let from = |s: String| {
        let s = s.to_lowercase();
        (!s.is_empty()).then_some(s)
    };
    std::env::var("SNACKPILOT_GALLERY_PAGE")
        .ok()
        .and_then(from)
        .or_else(|| {
            std::env::args()
                .find(|a| a.starts_with("--gallery-page="))
                .and_then(|a| from(a["--gallery-page=".len()..].to_string()))
        })
        .or_else(|| debug_prop("debug.snackpilot.gallery.page").and_then(from))
}

#[cfg(not(debug_assertions))]
fn gallery_page() -> Option<String> {
    None
}

/// The `AppModel` the shell renders from. Installed by the hosts before the
/// window opens (task 4.2): desktop in `bin/desktop.rs`, Android in
/// `mobile.rs` (`android_main`) and iOS in `platform/ios.rs`
/// (`snackpilot_ios_register_shims`), which runs the iOS launch order there
/// before the GPUI window exists.
static SHELL_MODEL: std::sync::OnceLock<std::sync::Arc<snackpilot_model::AppModel>> =
    std::sync::OnceLock::new();

/// Install the shared model before the window opens (first call wins).
pub fn install_shell_model(model: std::sync::Arc<snackpilot_model::AppModel>) {
    let _ = SHELL_MODEL.set(model);
}

/// The installed model, if any.
pub fn shell_model() -> Option<std::sync::Arc<snackpilot_model::AppModel>> {
    SHELL_MODEL.get().cloned()
}

/// Debug-only isolated offline evidence harness (task 1.5).
///
/// A capture run must show known fixture data without a real host, a real
/// credential or a live broadcast. The three host entry points build their
/// model through [`evidence::model`], which wraps the host platform in an
/// in-memory credential store: the normal OS/keychain slots are never read or
/// written, and the seeded dummy pair is the core's offline demo credentials.
/// The model is switched into evidence mode *before* `bootstrap`, so the
/// launch order already skips the live broadcast, the permission prompt and
/// the geofence restore. A release build can never reach any of this.
pub mod evidence {
    use std::path::PathBuf;
    use std::sync::Arc;

    use snackpilot_model::{AppModel, Platform, PlatformError};

    /// Whether this process asked for the harness (`--evidence`, or
    /// `SNACKPILOT_EVIDENCE=1` on a simulator/emulator; debug builds only).
    pub fn requested() -> bool {
        AppModel::evidence_requested()
    }

    /// The isolated evidence data directory: a subdirectory, so a capture run
    /// never mixes its cache with the normal one.
    pub fn storage_dir(base: PathBuf) -> PathBuf {
        if requested() {
            base.join("evidence")
        } else {
            base
        }
    }

    /// Build the shell model for one host: the harness credential store in an
    /// evidence run, the host platform otherwise. The model is switched into
    /// evidence mode before the caller runs `bootstrap`.
    pub fn model(
        storage_dir: PathBuf,
        platform: Arc<dyn Platform>,
    ) -> Result<Arc<AppModel>, PlatformError> {
        let platform = if requested() {
            AppModel::evidence_platform(platform)
        } else {
            platform
        };
        let model = AppModel::new(storage_dir, platform)?;
        if requested() {
            model.set_evidence_mode(true);
        }
        Ok(Arc::new(model))
    }

    /// A logical capture viewport in points, selected before bootstrap by
    /// `--viewport=390x844`, `SNACKPILOT_VIEWPORT` or the Android debug
    /// property `debug.snackpilot.viewport`. Never a production layout
    /// override: only an isolated evidence run consumes it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Viewport {
        pub width: u32,
        pub height: u32,
    }

    impl Viewport {
        /// Parses `WIDTHxHEIGHT` in logical points; zero-sized and malformed
        /// values select nothing.
        pub fn parse(value: &str) -> Option<Self> {
            let value = value.trim();
            let (width, height) = value.split_once('x').or_else(|| value.split_once('X'))?;
            let width = width.trim().parse::<u32>().ok()?;
            let height = height.trim().parse::<u32>().ok()?;
            (width > 0 && height > 0).then_some(Self { width, height })
        }

        /// The logical content width the evidence model reports.
        pub fn logical_width(self) -> f32 {
            self.width as f32
        }
    }

    /// The selected capture viewport, or `None`. Debug builds only: a release
    /// binary has no viewport knob at all.
    #[cfg(debug_assertions)]
    pub fn viewport() -> Option<Viewport> {
        std::env::var("SNACKPILOT_VIEWPORT")
            .ok()
            .or_else(|| {
                std::env::args().find_map(|a| a.strip_prefix("--viewport=").map(str::to_owned))
            })
            .or_else(|| crate::debug_prop("debug.snackpilot.viewport"))
            .and_then(|value| Viewport::parse(&value))
    }

    #[cfg(not(debug_assertions))]
    pub fn viewport() -> Option<Viewport> {
        None
    }

    /// The selected synthetic scenario, normalized to the model's names.
    /// Debug builds only; a release binary has no scenario knob.
    #[cfg(debug_assertions)]
    pub fn scenario() -> Option<String> {
        std::env::var("SNACKPILOT_SCENARIO")
            .ok()
            .or_else(|| {
                std::env::args().find_map(|a| a.strip_prefix("--scenario=").map(str::to_owned))
            })
            .or_else(|| crate::debug_prop("debug.snackpilot.scenario"))
            .map(|value| value.trim().to_ascii_lowercase().replace('_', "-"))
            .filter(|value| !value.is_empty())
    }

    #[cfg(not(debug_assertions))]
    pub fn scenario() -> Option<String> {
        None
    }

    /// Reports the selected evidence viewport to the model before `bootstrap`:
    /// an isolated run always stays a phone (single-column) surface, whatever
    /// pixel width the simulator reports. A viewport knob is never a
    /// production layout override, so a normal credential-backed store is left
    /// exactly as its platform reported it. Inert in a release build.
    pub fn install_viewport(model: &AppModel, viewport: Viewport) {
        #[cfg(debug_assertions)]
        {
            if !model.evidence_mode() {
                return;
            }
            model.set_host_kind(snackpilot_model::desktop::HostKind::Phone);
            model.set_content_width(viewport.logical_width());
        }
        #[cfg(not(debug_assertions))]
        let _ = (model, viewport);
    }

    /// Applies the selected synthetic scenario after `bootstrap`. The model
    /// ignores it unless the store is already isolated, and a release build
    /// has no scenario to apply.
    pub fn apply_scenario(model: &AppModel, scenario: &str) {
        #[cfg(debug_assertions)]
        model.apply_evidence_scenario(scenario);
        #[cfg(not(debug_assertions))]
        let _ = (model, scenario);
    }
}

/// Mount the single spike Root: bundled Inter fonts, kit init, one window.
///
/// Called exactly once per app start by each platform host (REQ-UI-03);
/// the log line below is the evidence checked in `adb logcat`.
pub fn mount(cx: &mut App) {
    init_kit(cx);
    if let Some(section) = gallery_requested() {
        gallery::mount_gallery(cx, section);
        return;
    }
    cx.open_window(
        WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |window, cx| {
            let content = cx.new(|cx| app::AppShell::new(shell_model(), cx));
            cx.new(|cx| Root::new(content, window, cx))
        },
    )
    .expect("open spike window");
    cx.activate(true);
}

/// Shared init for every host: bundled Inter fonts, kit init, light theme.
pub(crate) fn init_kit(cx: &mut App) {
    log::info!("spike: mounting single gpui-kit Root");
    cx.text_system()
        .add_fonts(vec![
            std::borrow::Cow::Borrowed(assets::INTER_REGULAR),
            std::borrow::Cow::Borrowed(assets::INTER_SEMIBOLD),
            std::borrow::Cow::Borrowed(assets::INTER_BOLD),
        ])
        .expect("register bundled Inter fonts");
    gpui_kit::init(cx);
    Theme::change(ThemeMode::Light, None, cx);
    Theme::global_mut(cx).font_family = "Inter".into();
}

/// Desktop content width rule (desktop-ui-redesign task 1.2, REQ-UI-07;
/// mobile-ui-rework task 1.2). The frame reports the window's own content
/// width at every width, because the Day Tray column (`min(viewport − 32,
/// 560)`) is now centred by the shell through the model's geometry — the old
/// 480 px cap below the 720 px breakpoint is gone, and the breakpoint itself
/// stays the model's (`layout_mode`), never a second literal here.
pub(crate) fn desktop_content_width(window_width: f32) -> f32 {
    window_width
}

/// The width a compact surface is laid out at (REQ-UI-07): the host-reported
/// content width when there is one (a desktop frame or an evidence viewport),
/// otherwise the live window viewport — a phone host reports no width, so it
/// must never fall back to zero.
pub(crate) fn compact_width(reported: f32, viewport: f32) -> f32 {
    if reported > 0. {
        reported
    } else {
        viewport
    }
}

/// The compact single-column projection for this host and reported content
/// width (REQ-UI-07): 16 px gutters under the 560 px cap, 104 at 768, and
/// `None` for the wide Desktop Ledger, which this presentation never restyles.
pub(crate) fn compact_column(
    host: snackpilot_model::desktop::HostKind,
    width: f32,
) -> Option<snackpilot_model::tokens::Geometry> {
    snackpilot_model::tokens::geometry(host, width)
}

#[cfg(all(test, not(any(target_os = "ios", target_os = "android"))))]
mod tests {
    use super::{compact_column, compact_width, desktop_content_width};

    /// The desktop frame reports the live window width at every width, so the
    /// shell can centre the Day Tray column itself; the old 480 px cap below
    /// the 720 px breakpoint is gone (REQ-UI-07).
    /// @req REQ-UI-07
    #[test]
    fn desktop_frame_width_follows_the_breakpoint() {
        for width in [360., 390., 719., 720., 768., 1280.] {
            assert_eq!(
                desktop_content_width(width),
                width,
                "the frame reports the live content width at {width}"
            );
        }
    }

    /// The compact shell projection is host-safe (REQ-UI-07): the Day Tray
    /// column is `min(viewport − 32, 560)` with 16/16/104 gutters and a 440 px
    /// form cap for a phone/tablet host at any width and for a narrow desktop,
    /// while the wide Desktop Ledger (desktop at 720 and above) is never given
    /// this presentation. The accepted widths and caps are exactly the shared
    /// token table, so no screen derives a second column.
    /// @req REQ-UI-07
    #[test]
    fn compact_projection_is_host_safe() {
        use snackpilot_model::desktop::HostKind;

        // (viewport, gutter, content, form)
        let cases = [
            (360., 16., 328., 328.),
            (390., 16., 358., 358.),
            (768., 104., 560., 440.),
            (1280., 360., 560., 440.),
        ];
        for (width, gutter, content, form) in cases {
            for host in [HostKind::Phone, HostKind::Tablet] {
                let geometry = compact_column(host, width)
                    .unwrap_or_else(|| panic!("{host:?} gets the Day Tray column at {width}"));
                assert_eq!(geometry.gutter, gutter, "{host:?} gutter at {width}");
                assert_eq!(geometry.content, content, "{host:?} content at {width}");
                assert_eq!(geometry.form, form, "{host:?} form cap at {width}");
                // The column always leaves its 16 px outer inset, except where
                // the 560 cap produces the larger centred gutter.
                assert!(geometry.gutter >= 16., "outer inset at {width}");
            }
        }
        // Narrow desktop is the same flat Day Tray column as a phone.
        for (width, content) in [(360., 328.), (390., 358.), (719., 560.)] {
            let desktop = compact_column(HostKind::Desktop, width).expect("narrow desktop");
            let phone = compact_column(HostKind::Phone, width).expect("phone");
            assert_eq!(desktop, phone, "same column at {width}");
            assert_eq!(
                desktop.content, content,
                "narrow desktop content at {width}"
            );
        }

        // The compact form cap never exceeds the content column.
        for width in [360., 390., 768., 1280.] {
            let geometry = compact_column(HostKind::Phone, width).expect("phone column");
            assert!(geometry.form <= geometry.content);
            assert_eq!(geometry.form, geometry.content.min(440.));
        }

        // A phone host reports no content width, so the live viewport is the
        // layout width; a reported width (desktop frame, evidence viewport)
        // wins over the window size.
        assert_eq!(compact_width(0., 390.), 390., "no report uses the viewport");
        assert_eq!(compact_width(390., 402.), 390., "the report wins");
        assert_eq!(
            compact_column(HostKind::Phone, compact_width(0., 390.))
                .expect("phone column")
                .content,
            358.,
            "a phone shell never collapses to a zero-width column"
        );

        // The wide Ledger is not this presentation: on a desktop host the
        // projection exists only below the model's 720 px breakpoint.
        assert!(compact_column(HostKind::Desktop, 719.).is_some());
        for width in [720., 768., 1280.] {
            assert!(
                compact_column(HostKind::Desktop, width).is_none(),
                "desktop at {width} keeps the Desktop Ledger"
            );
        }
        // A phone host never leaves the Day Tray, whatever width it reports.
        for width in [360., 720., 1280.] {
            assert!(
                compact_column(HostKind::Phone, width).is_some(),
                "a phone host keeps the Day Tray at {width}"
            );
        }
    }

    /// Evidence viewport/scenario selection is explicit and debug-only: it
    /// parses only a well-formed `WxH`, and nothing is selected unless the
    /// capture knob really asks for it.
    /// @req REQ-UI-16
    #[test]
    fn evidence_viewport_and_scenario_selection_is_debug_only() {
        use crate::evidence;

        assert_eq!(
            evidence::Viewport::parse("390x844"),
            Some(evidence::Viewport {
                width: 390,
                height: 844
            })
        );
        assert_eq!(
            evidence::Viewport::parse(" 768x1024 ").map(|v| v.logical_width()),
            Some(768.),
            "surrounding whitespace is tolerated"
        );
        assert_eq!(evidence::Viewport::parse("390"), None, "height required");
        assert_eq!(evidence::Viewport::parse("0x844"), None, "zero width");
        assert_eq!(evidence::Viewport::parse("390x0"), None, "zero height");
        assert_eq!(evidence::Viewport::parse("abcx844"), None, "numeric only");

        // The test runner passes no capture knob: an ordinary run renders the
        // normal shell, and a release build always returns `None` here.
        assert_eq!(
            evidence::viewport(),
            None,
            "no capture viewport unless a knob selects one"
        );
        assert_eq!(
            evidence::scenario(),
            None,
            "no synthetic scenario unless a knob selects one"
        );
    }

    /// A viewport knob is never a production layout override: the same call on
    /// a normal credential-backed store leaves the host and content width the
    /// platform reported untouched.
    /// @req REQ-UI-16
    #[test]
    fn evidence_viewport_never_overrides_a_normal_store() {
        use crate::evidence;
        use snackpilot_model::desktop::HostKind;
        use snackpilot_model::{AppModel, FakePlatform};
        use std::sync::Arc;

        let dir = std::env::temp_dir().join(format!(
            "snackpilot-app-viewport-normal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let model = AppModel::new(dir, Arc::new(FakePlatform::new())).unwrap();
        assert!(!model.evidence_mode(), "a normal launch is not isolated");
        model.set_host_kind(HostKind::Desktop);
        model.set_content_width(1280.);

        evidence::install_viewport(&model, evidence::Viewport::parse("390x844").unwrap());

        assert_eq!(
            model.state().host_kind,
            HostKind::Desktop,
            "the reported host stays as it was"
        );
        assert_eq!(
            model.state().content_width,
            1280.,
            "no fixed capture width on a normal store"
        );
    }
}

/// Desktop entry (task 2.3): same Root as mobile, in a resizable window with
/// minimum width 360 px, opened through `gpui_kit::application()` by
/// `src/bin/desktop.rs` (REQ-UI-01, REQ-UI-07).
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn mount_desktop(cx: &mut App) {
    if let Some(section) = gallery_requested() {
        init_kit(cx);
        gallery::mount_gallery(cx, section);
        return;
    }
    desktop::mount(cx);
}
