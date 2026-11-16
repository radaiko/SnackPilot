//! The refresh progress bar.
//!
//! A refresh has a beginning and an end, so it reads better as a line than as
//! a spinner: during a pull the bar fills with the drag (the user can see how
//! much further the threshold is), and while the request runs it sweeps.

use std::time::Duration;

use gpui::*;

use crate::theme::AppTheme;

/// Bar thickness.
pub const BAR_HEIGHT: f32 = 2.;
/// One sweep of the indeterminate bar.
const SWEEP: Duration = Duration::from_millis(1100);
/// Share of the track the sweeping segment occupies.
const SEGMENT: f32 = 0.35;

/// A 2 px refresh bar on a hairline track. `progress` fills it (a pull in
/// flight); `None` sweeps it (a request in flight).
pub fn refresh_bar(theme: &AppTheme, progress: Option<f32>, id: &'static str) -> AnyElement {
    let track = div()
        .w_full()
        .h(px(BAR_HEIGHT))
        .relative()
        .rounded(px(BAR_HEIGHT / 2.))
        .bg(theme.border)
        .overflow_hidden();
    match progress {
        Some(progress) => track
            .child(
                div()
                    .h_full()
                    .w(relative(progress.clamp(0., 1.)))
                    .rounded(px(BAR_HEIGHT / 2.))
                    .bg(theme.accent),
            )
            .into_any_element(),
        None => track
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
            .into_any_element(),
    }
}
