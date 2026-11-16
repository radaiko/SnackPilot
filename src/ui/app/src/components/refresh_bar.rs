//! The refresh progress bar.
//!
//! A refresh has a beginning and an end, so it reads better as a line than as
//! a spinner: one accent segment sweeps across a hairline track for as long as
//! the request runs.

use std::time::Duration;

use gpui::*;

use crate::theme::AppTheme;

/// Bar thickness.
pub const BAR_HEIGHT: f32 = 2.;
/// One sweep of the bar.
const SWEEP: Duration = Duration::from_millis(1100);
/// Share of the track the sweeping segment occupies.
const SEGMENT: f32 = 0.35;

/// A 2 px accent segment sweeping a hairline track, for the length of a
/// refresh. `id` keys the animation, so two bars on one screen stay separate.
pub fn refresh_bar(theme: &AppTheme, id: &'static str) -> AnyElement {
    div()
        .w_full()
        .h(px(BAR_HEIGHT))
        .relative()
        .rounded(px(BAR_HEIGHT / 2.))
        .bg(theme.border)
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .h_full()
                .w(relative(SEGMENT))
                .rounded(px(BAR_HEIGHT / 2.))
                .bg(theme.accent)
                .with_animation(id, Animation::new(SWEEP).repeat(), |el, t| {
                    el.left(relative(t * (1. - SEGMENT)))
                }),
        )
        .into_any_element()
}
