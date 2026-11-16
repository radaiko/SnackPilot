//! The page carousel: the drag state and the page arithmetic shared by the day
//! strip (Menüs) and the billing months (Abrechnung).
//!
//! One surface is three pages side by side in a clipped row: the one before,
//! the one on screen and the one after. The drag slides the row, the release
//! moves the selection by one and leaves the row at the travel that keeps the
//! arriving page on the pixel it already had, and that travel eases to zero —
//! so the incoming page comes to rest instead of snapping. Pages cross-fade
//! with their distance from centre, so a transition always shows exactly one
//! page of ink and the switch never changes brightness.

use crate::components::swipe::{add_travel, claims_horizontal, is_swipe_delta};

/// One surface's drag state. The shell owns one of these per carousel, so a
/// phone with both surfaces never shares a gesture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Carousel {
    /// Live travel of the gesture under the finger.
    pub dx: f32,
    /// Travel left after a release, eased to zero by the page finish.
    pub settle: f32,
    pub live: bool,
    pub claimed: bool,
}

impl Carousel {
    /// What the row is painted at: the finger, plus whatever page finish is
    /// still running.
    pub fn offset(&self) -> f32 {
        self.dx + self.settle
    }

    /// A new touch: it takes the page over, cancelling any finish.
    pub fn begin(&mut self) {
        self.live = true;
        self.claimed = false;
        self.dx = 0.;
        self.settle = 0.;
    }

    /// One delta of the live gesture. Returns whether it moved anything.
    pub fn travel(&mut self, dx: f32, dy: f32) -> bool {
        if !self.live || !is_swipe_delta(dx, dy) {
            return false;
        }
        self.dx = add_travel(self.dx, dx);
        let claimed = !self.claimed && claims_horizontal(self.dx, 0.);
        self.claimed |= claimed;
        true
    }

    /// The finger lifted: the claimed travel, if this gesture ever became a
    /// page gesture.
    pub fn release(&mut self) -> Option<f32> {
        if !std::mem::take(&mut self.live) {
            return None;
        }
        let claimed = std::mem::take(&mut self.claimed);
        let dx = std::mem::take(&mut self.dx);
        claimed.then_some(dx)
    }

    /// Take the page finish after a release: the travel that keeps the arriving
    /// page in place. Reduced motion finishes instantly, and so does a travel
    /// too small to be seen.
    pub fn finish(&mut self, settle: f32, reduce_motion: bool) {
        self.settle = if reduce_motion || settle.abs() <= 0.5 {
            0.
        } else {
            settle
        };
    }

    /// The system took the touch: drop the gesture without switching.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The travel that leaves the arriving page exactly where it is on screen when
/// it becomes the middle one, so the page finish starts without a jump.
pub fn settle_after(dx: f32, direction: i32, width: f32) -> f32 {
    let width = width.max(1.);
    match direction {
        -1 => dx - width,
        1 => dx + width,
        _ => dx,
    }
}

/// One page's opacity as the row slides: solid where it belongs, fading exactly
/// as far as it is out of place. The two pages of a transition add up to one
/// page of ink, so nothing pops when the release makes the arriving page the
/// middle one.
pub fn page_fade(travel: f32, slot: usize, width: f32) -> f32 {
    let width = width.max(1.);
    let offset = travel + (slot as f32 - 1.) * width;
    (1. - offset.abs() / width).clamp(0., 1.)
}

/// How far the row may travel: one page either way, damped to 30 % where there
/// is no neighbour, so the edge never shows the page beyond it.
pub fn page_travel(
    offset: f32,
    has_previous: bool,
    has_next: bool,
    reduce_motion: bool,
    width: f32,
) -> f32 {
    if reduce_motion {
        return 0.;
    }
    let width = width.max(1.);
    let damped = if (offset > 0. && !has_previous) || (offset < 0. && !has_next) {
        offset * 0.3
    } else {
        offset
    };
    damped.clamp(-width, width)
}

/// The items either side of `index` in a list, for the three panes.
pub fn neighbours<T: Clone>(list: &[T], index: usize) -> (Option<T>, Option<T>) {
    (
        index.checked_sub(1).and_then(|i| list.get(i)).cloned(),
        list.get(index + 1).cloned(),
    )
}

#[cfg(test)]
mod tests {
    use super::{neighbours, page_fade, page_travel, settle_after, Carousel};

    /// The page finish keeps the arriving page on the same pixel, and the fade
    /// across the switch never steps.
    /// @req REQ-SCR-01
    #[test]
    fn the_page_finish_and_the_fade_are_continuous() {
        let width = 390.;
        assert_eq!(settle_after(-50., 1, width), -50. + width);
        assert_eq!(settle_after(-width, 1, width), 0.);
        assert_eq!(settle_after(50., -1, width), 50. - width);
        assert_eq!(settle_after(width, -1, width), 0.);
        assert_eq!(settle_after(30., 0, width), 30.);

        assert_eq!(page_fade(0., 1, width), 1.);
        assert_eq!(page_fade(0., 0, width), 0.);
        let travel = -90.;
        assert_eq!(page_fade(travel, 1, width), 0.75);
        assert_eq!(page_fade(travel, 2, width), 0.25);
        assert_eq!(
            page_fade(settle_after(travel, 1, width), 1, width),
            page_fade(travel, 2, width),
            "no brightness step at the switch"
        );
    }

    /// Travel stops at one page, and an edge without a neighbour only gives
    /// 30 % so nothing beyond it is ever shown.
    /// @req REQ-SCR-01
    #[test]
    fn travel_is_one_page_and_damped_at_the_edges() {
        assert_eq!(page_travel(40., true, true, false, 390.), 40.);
        assert_eq!(page_travel(-2000., true, true, false, 390.), -390.);
        assert_eq!(page_travel(2000., true, true, false, 390.), 390.);
        assert_eq!(page_travel(100., false, true, false, 390.), 30.);
        assert_eq!(page_travel(-100., true, false, false, 390.), -30.);
        assert_eq!(page_travel(100., true, true, true, 390.), 0.);
    }

    /// One gesture per touch: the states only advance while it is live.
    /// @req REQ-SCR-01
    #[test]
    fn a_gesture_is_one_touch_and_one_release() {
        let mut carousel = Carousel::default();
        assert_eq!(carousel.release(), None, "nothing was touched");

        carousel.begin();
        assert!(carousel.travel(-4., 1.), "small travel is still travel");
        assert!(
            !carousel.travel(-40., 30.),
            "a mostly vertical delta is not"
        );
        assert!(carousel.travel(-40., 2.));
        assert_eq!(carousel.offset(), -44.);

        assert_eq!(carousel.release(), Some(-44.), "the travel is handed over");
        assert_eq!(carousel.release(), None, "one release per touch");
        assert_eq!(carousel.offset(), 0., "the gesture state is gone");
    }

    /// The three panes take the items either side of the one on screen.
    /// @req REQ-SCR-01
    #[test]
    fn neighbours_are_the_items_either_side() {
        let list = ["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(neighbours(&list, 1), (Some("a".into()), Some("c".into())));
        assert_eq!(neighbours(&list, 0), (None, Some("b".into())));
        assert_eq!(neighbours(&list, 2), (Some("b".into()), None));
    }
}
