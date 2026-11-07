//! gpui-kit views, theme, assets and entry points (openspec: gpui-kit-ui-rewrite).
//!
//! Task 2.1 spike: one `gpui_kit::component::Root` (REQ-UI-03) showing the four
//! tab labels and a counter button, mounted through `gpui_kit::init(cx)` — the
//! mobile hosts never call `gpui_kit::application()`.

pub mod assets;

#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile;

use gpui_kit::component::button::Button;
use gpui_kit::component::{Root, Theme, ThemeMode};
use gpui_kit::prelude::*;
use gpui_kit::{div, App, Context, Entity, Window, WindowOptions};
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use gpui_kit::{px, size, Bounds, WindowBounds};

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

/// Mount the single spike Root: bundled Inter fonts, kit init, one window.
///
/// Called exactly once per app start by each platform host (REQ-UI-03);
/// the log line below is the evidence checked in `adb logcat`.
pub fn mount(cx: &mut App) {
    init_kit(cx);
    cx.open_window(
        WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |window, cx| {
            let content = cx.new(|_cx| SpikeRoot { count: 0 });
            cx.new(|cx| Root::new(content, window, cx))
        },
    )
    .expect("open spike window");
    cx.activate(true);
}

/// Shared init for every host: bundled Inter fonts, kit init, light theme.
fn init_kit(cx: &mut App) {
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

/// Desktop frame (task 2.3, REQ-UI-07): the same `SpikeRoot` centred in a
/// column of at most 480 logical px; no sidebar (human-approved decision).
#[cfg(not(any(target_os = "ios", target_os = "android")))]
struct DesktopFrame {
    root: Entity<SpikeRoot>,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl Render for DesktopFrame {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().justify_center().child(
            div()
                .h_full()
                .w_full()
                .max_w(px(480.))
                .child(self.root.clone()),
        )
    }
}

/// Desktop entry (task 2.3): same Root as mobile, in a resizable window with
/// minimum width 360 px, opened through `gpui_kit::application()` by
/// `src/bin/desktop.rs` (REQ-UI-01, REQ-UI-07).
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub fn mount_desktop(cx: &mut App) {
    init_kit(cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(420.), px(760.)),
                cx,
            ))),
            is_resizable: true,
            window_min_size: Some(size(px(360.), px(0.))),
            ..Default::default()
        },
        |window, cx| {
            let content = cx.new(|_cx| SpikeRoot { count: 0 });
            let frame = cx.new(|_cx| DesktopFrame { root: content });
            cx.new(|cx| Root::new(frame, window, cx))
        },
    )
    .expect("open desktop window");
    cx.activate(true);
}
