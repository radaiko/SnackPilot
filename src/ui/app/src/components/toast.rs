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
/// The approved in-app toast copy (C1, human 2026-11-13): the incumbent German
/// string without the exclamation mark. OS notification copy is unchanged
/// (REQ-UI-06).
pub const NEW_MENU_MESSAGE: &str = "Neue Menüs verfügbar";
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
    /// Toast text; the in-app new-menu message is [`NEW_MENU_MESSAGE`].
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

/// Slide distance for one presentation (REQ-UI-16): reduced motion removes the
/// travel only. The 300/4000/300 ms lifetime, the event policy and the copy stay
/// exactly as they are, and no new timing constant is introduced.
pub fn slide_distance(reduce_motion: bool) -> f32 {
    if reduce_motion {
        0.
    } else {
        SLIDE
    }
}

/// The toast's top offset for one animation sample. `toast_overlay_for` feeds
/// this with `slide_distance(reduce_motion)`, so the test below checks the
/// painted offsets: reduced motion starts and ends at the resting top instead of
/// travelling 100 px, while every phase keeps its duration.
pub fn toast_top(base_top: f32, slide: f32, phase: ToastPhase, t: f32) -> f32 {
    match phase {
        ToastPhase::In => base_top - slide * (1.0 - t),
        ToastPhase::Hold => base_top,
        ToastPhase::Out => base_top - slide * t,
    }
}

/// The toast box: `left/right 16`, `padding 12`, tinted banner in
/// the accent surface, semibold text, top edge at `safeTop + 8`.
pub fn toast_overlay(toast: &Toast, safe_top: f32, theme: &AppTheme) -> AnyElement {
    toast_overlay_motion(toast, safe_top, SLIDE, theme)
}

/// The shell's compact toast, honouring the host's reduced-motion setting: the
/// same box and the same phases, without the travel.
/// @req REQ-UI-16
pub fn toast_overlay_for(
    toast: &Toast,
    safe_top: f32,
    reduce_motion: bool,
    theme: &AppTheme,
) -> AnyElement {
    toast_overlay_motion(toast, safe_top, slide_distance(reduce_motion), theme)
}

fn toast_overlay_motion(toast: &Toast, safe_top: f32, slide: f32, theme: &AppTheme) -> AnyElement {
    let base_top = safe_top + 8.;
    let recipe = theme.tinted_banner(theme.accent_surface);
    let card = theme::apply(
        div()
            .absolute()
            .left(px(16.))
            .right(px(16.))
            .rounded(px(theme.scale.radius_card))
            .p(px(12.))
            .flex()
            .justify_center()
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
                    .child(toast.message.clone()),
            ),
        &recipe,
    );

    match toast.phase {
        ToastPhase::In => card
            .with_animation(
                "toast-in",
                Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| {
                    el.top(px(toast_top(base_top, slide, ToastPhase::In, t)))
                        .opacity(t)
                },
            )
            .into_any_element(),
        ToastPhase::Hold => card.top(px(base_top)).opacity(1.).into_any_element(),
        ToastPhase::Out => card
            .with_animation(
                "toast-out",
                Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
                move |el, t| {
                    el.top(px(toast_top(base_top, slide, ToastPhase::Out, t)))
                        .opacity(1.0 - t)
                },
            )
            .into_any_element(),
    }
}

/// Desktop toast: the same 300/4000/300 ms phases, 16 px inset and 8 px
/// below the workspace top the shell passes, on a surface panel (radius 12,
/// 1 px border, no shadow). The shell paints it over the workspace only, so
/// it never covers the rail.
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
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_primary)
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

#[cfg(test)]
mod tests {
    use super::{
        slide_distance, toast_top, ToastPhase, DESKTOP_HOLD_DURATION, DESKTOP_SLIDE_DURATION,
        HOLD_DURATION, NEW_MENU_MESSAGE, SLIDE_DURATION,
    };
    use std::time::Duration;

    /// The approved in-app toast carries the exact C1 copy — no exclamation
    /// mark, nothing else changed — keeps the incumbent 300/4000/300 ms
    /// lifetime, and reduced motion removes the travel. The assertion is on the
    /// offsets `toast_overlay_for` paints through `toast_top`, so a reduced
    /// presentation starts and ends at the resting top instead of merely
    /// returning a zero constant.
    /// @req REQ-UI-06 @req REQ-UI-16
    #[test]
    fn mobile_toast_copy_timing_and_reduced_motion() {
        assert_eq!(NEW_MENU_MESSAGE, "Neue Menüs verfügbar");
        assert!(
            !NEW_MENU_MESSAGE.contains('!'),
            "the C1 approval removed the exclamation mark"
        );
        // The three incumbent phases are unchanged constants.
        assert_eq!(SLIDE_DURATION, Duration::from_millis(300));
        assert_eq!(HOLD_DURATION, Duration::from_millis(4000));
        assert_eq!(SLIDE_DURATION, Duration::from_millis(300));
        assert_eq!(DESKTOP_SLIDE_DURATION, SLIDE_DURATION);
        assert_eq!(DESKTOP_HOLD_DURATION, HOLD_DURATION);

        // Exactly the offsets the overlay paints: full travel when motion is
        // allowed, no travel at all under reduced motion, same resting top.
        let base = 59. + 8.;
        for (t, expected) in [(0., base - 100.), (0.5, base - 50.), (1., base)] {
            assert_eq!(
                toast_top(base, slide_distance(false), ToastPhase::In, t),
                expected
            );
            assert_eq!(
                toast_top(base, slide_distance(true), ToastPhase::In, t),
                base,
                "reduced motion never travels"
            );
        }
        for t in [0., 0.5, 1.] {
            assert_eq!(
                toast_top(base, slide_distance(false), ToastPhase::Out, t),
                base - 100. * t
            );
            assert_eq!(
                toast_top(base, slide_distance(true), ToastPhase::Out, t),
                base,
                "the out phase does not move either"
            );
            assert_eq!(
                toast_top(base, slide_distance(true), ToastPhase::Hold, t),
                base
            );
        }
    }
}
