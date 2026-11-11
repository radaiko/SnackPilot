//! New-menu toast — docs/requirements/04-ui-ux.md §4.8.
//!
//! Slides down 100 px over 300 ms, holds 4 s, slides back out over 300 ms
//! and disappears; it is never manually dismissible. The shell owns the
//! phase machine (In → Hold → Out → gone) and drives it with timers; this
//! module only renders the current phase.
//!
//! v1 used `Animated.View` translateY; gpui-pre has no `translate`, so the
//! slide is expressed as an animated `top` offset plus opacity.

use std::time::Duration;

use gpui::*;

use crate::theme::{self, AppTheme};

/// Slide distance in px (§4.8 slide-down).
const SLIDE: f32 = 100.;
/// One-way slide duration (§4.8).
pub const SLIDE_DURATION: Duration = Duration::from_millis(300);
/// Time the toast stays fully visible (§4.8).
pub const HOLD_DURATION: Duration = Duration::from_millis(4000);
/// Desktop toast timings (brief `desktop.toastDurationIn/Hold/Out`): the
/// incumbent three values are preserved, not retuned by the desktop shell.
pub const DESKTOP_SLIDE_DURATION: Duration = SLIDE_DURATION;
pub const DESKTOP_HOLD_DURATION: Duration = HOLD_DURATION;

/// Phase of the shell-owned toast state machine.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastPhase {
    /// Sliding in from the top.
    In,
    /// Fully visible.
    Hold,
    /// Sliding back out.
    Out,
}

/// A visible §4.8 toast.
pub struct Toast {
    /// Toast text, e.g. "Neue Menüs verfügbar!".
    pub message: SharedString,
    /// Current animation phase.
    pub phase: ToastPhase,
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        let x = -2.0 * t + 2.0;
        1.0 - x * x / 2.0
    }
}

/// The toast box: `left/right 16`, `padding 12`, tinted banner in
/// `glassPrimary`, text 14/600 in `primary`, top edge at `safeTop + 8`.
pub fn toast_overlay(toast: &Toast, safe_top: f32, theme: &AppTheme) -> AnyElement {
    let base_top = safe_top + 8.;
    let recipe = theme.tinted_banner(theme.glass_primary);
    let card = theme::apply(
        div()
            .absolute()
            .left(px(16.))
            .right(px(16.))
            .rounded(px(16.))
            .p(px(12.))
            .flex()
            .justify_center()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.primary)
                    .child(toast.message.clone()),
            ),
        &recipe,
    );

    match toast.phase {
        ToastPhase::In => card
            .with_animation(
                "toast-in",
                Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| el.top(px(base_top - SLIDE * (1.0 - t))).opacity(t),
            )
            .into_any_element(),
        ToastPhase::Hold => card.top(px(base_top)).opacity(1.).into_any_element(),
        ToastPhase::Out => card
            .with_animation(
                "toast-out",
                Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| el.top(px(base_top - SLIDE * t)).opacity(1.0 - t),
            )
            .into_any_element(),
    }
}

/// Desktop toast (brief "Overlays"): the same 300/4000/300 ms phases, 16 px
/// inset and 8 px below the workspace top the shell passes, on the opaque
/// panel recipe (radius 12, 1 px border, no shadow) with `text_primary` ink.
/// The shell paints it over the workspace only, so it never covers the rail
/// and its arrival stays the one authored motion moment.
/// @req REQ-UI-07 @req REQ-UI-13
pub fn desktop_toast(toast: &Toast, top: f32, theme: &AppTheme) -> AnyElement {
    let recipe = theme.workspace_panel();
    let card = theme::apply(
        div()
            .absolute()
            .left(px(16.))
            .right(px(16.))
            .p(px(12.))
            .flex()
            .justify_center()
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.desktop.ink)
                    .child(toast.message.clone()),
            ),
        &recipe,
    );

    match toast.phase {
        ToastPhase::In => card
            .with_animation(
                "desktop-toast-in",
                Animation::new(DESKTOP_SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| el.top(px(top - SLIDE * (1.0 - t))).opacity(t),
            )
            .into_any_element(),
        ToastPhase::Hold => card.top(px(top)).opacity(1.).into_any_element(),
        ToastPhase::Out => card
            .with_animation(
                "desktop-toast-out",
                Animation::new(DESKTOP_SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| el.top(px(top - SLIDE * t)).opacity(1.0 - t),
            )
            .into_any_element(),
    }
}
