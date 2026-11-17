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
/// One pass of the bar, entry to exit.
const SWEEP: Duration = Duration::from_millis(1400);
/// Share of the track the sweeping segment occupies.
const SEGMENT: f32 = 0.35;
/// Share of the pass the segment spends fading in and out.
const FADE: f32 = 0.12;

/// Where the segment is at `t` of one pass. It starts and ends fully outside
/// the track, so the loop point is off screen: the sweep never cuts back to the
/// left edge, which is what the jump at the end of each pass looked like.
fn segment_left(t: f32) -> f32 {
    -SEGMENT + t * (1. + SEGMENT)
}

/// The segment's ink at `t`: it fades in as it arrives and out as it leaves,
/// so neither edge is a hard start.
fn edge_fade(t: f32) -> f32 {
    (t / FADE).min((1. - t) / FADE).clamp(0., 1.)
}

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
                    el.left(relative(segment_left(t))).opacity(edge_fade(t))
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{edge_fade, segment_left, SEGMENT};

    /// One pass is continuous: the segment arrives from outside the track and
    /// leaves outside it, so repeating the pass never shows a jump.
    #[test]
    fn the_sweep_has_no_jump_at_the_loop_point() {
        assert_eq!(segment_left(0.), -SEGMENT, "starts off the left edge");
        assert_eq!(segment_left(1.), 1., "ends past the right edge");
        // The same pixels are on screen at both ends of the pass: none.
        assert_eq!(segment_left(0.) + SEGMENT, 0.);
        assert!(segment_left(1.) >= 1.);

        // Monotone, so it only ever travels one way.
        let mut previous = segment_left(0.);
        for step in 1..=20 {
            let left = segment_left(step as f32 / 20.);
            assert!(left > previous);
            previous = left;
        }

        assert_eq!(edge_fade(0.), 0., "invisible as it enters");
        assert_eq!(edge_fade(0.5), 1.);
        assert_eq!(edge_fade(1.), 0., "invisible as it leaves");
        assert!(edge_fade(0.06) > 0. && edge_fade(0.06) < 1.);
    }
}
