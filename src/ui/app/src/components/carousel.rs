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

use crate::components::swipe::{add_travel, claims_horizontal, is_swipe_delta, AXIS_LOCK_PX};

/// One surface's drag state. The shell owns one of these per carousel, so a
/// phone with both surfaces never shares a gesture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Carousel {
    /// Live travel of the gesture under the finger.
    pub dx: f32,
    /// Travel left after a release, eased to zero by the page finish.
    pub settle: f32,
    /// Set once the gesture has clearly gone down instead of across: from
    /// there on it is a scroll, and no later delta can turn it into a page
    /// swipe.
    pub blocked: bool,
    pub live: bool,
    pub claimed: bool,
}

impl Carousel {
    /// What the row is painted at: the finger, plus whatever page finish is
    /// still running.
    pub fn offset(&self) -> f32 {
        self.dx + self.settle
    }

    /// A new touch: it takes the page over. Whatever is still on screen is
    /// kept — a finish that was running keeps easing from the pixel it had —
    /// so grabbing the row mid-flight never moves it.
    pub fn begin(&mut self) {
        self.live = true;
        self.claimed = false;
        self.blocked = false;
        self.settle += self.dx;
        self.dx = 0.;
    }

    /// One delta of the live gesture. Returns whether it moved anything.
    ///
    /// A gesture that travels clearly down locks itself to the list: one
    /// vertical scroll must never slide the page sideways as well, which is
    /// what made the billing list feel like it moved in three dimensions.
    pub fn travel(&mut self, dx: f32, dy: f32) -> bool {
        if !self.live || self.blocked {
            return false;
        }
        if !is_swipe_delta(dx, dy) {
            if !self.claimed && dy.abs() >= AXIS_LOCK_PX {
                // A scroll: the page sits still, including the pixels an
                // earlier, shorter delta had already pulled it by.
                self.blocked = true;
                self.dx = 0.;
            }
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

/// The row's painted travel: the live drag, damped once where there is no
/// neighbour, plus the page finish that is still easing. The finish is a
/// painted pixel already, so it is never damped a second time — that is what
/// keeps a release, and a grab during a release, from jumping.
pub fn painted_travel(
    page: &Carousel,
    has_previous: bool,
    has_next: bool,
    reduce_motion: bool,
    width: f32,
) -> f32 {
    page_travel(page.dx, has_previous, has_next, reduce_motion, width) + page.settle
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
    use super::{neighbours, page_fade, page_travel, painted_travel, settle_after, Carousel};

    /// The page finish keeps the arriving page on the same pixel, and the fade
    /// across the switch never steps.
    /// @req REQ-SCR-01
    #[test]
    fn the_page_finish_and_the_fade_are_continuous() {
        let width = 360.;
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
        assert_eq!(page_travel(100., false, true, false, 390.), 100. * 0.3);
        assert_eq!(page_travel(-100., true, false, false, 390.), -100. * 0.3);
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
        assert!(carousel.travel(-40., 2.));
        assert_eq!(carousel.offset(), -44.);

        assert_eq!(carousel.release(), Some(-44.), "the travel is handed over");
        assert_eq!(carousel.release(), None, "one release per touch");
        assert_eq!(carousel.offset(), 0., "the gesture state is gone");
    }

    /// An edge is damped once: the gesture and the finish that follows it are
    /// the same damped pixel, so the release does not slide.
    /// @req REQ-SCR-01
    #[test]
    fn an_edge_is_damped_once() {
        let width = 360.;
        let mut carousel = Carousel::default();
        carousel.begin();
        carousel.travel(100., 0.);
        let painted = painted_travel(&carousel, false, true, false, width);
        assert_eq!(painted, 100. * 0.3, "30 % at a dead end");

        assert_eq!(carousel.release(), Some(100.), "the drag itself");
        carousel.finish(painted, false);
        assert_eq!(
            painted_travel(&carousel, false, true, false, width),
            painted,
            "the finish keeps exactly the pixel that was on screen, undamped"
        );
    }

    /// A grab while the page finish is still easing keeps the row on the pixel
    /// it is painted at, and the new gesture's travel is measured from there —
    /// so interrupting a finish can neither jump nor switch a page by itself.
    /// @req REQ-SCR-01
    #[test]
    fn a_grab_during_the_finish_never_jumps() {
        let mut carousel = Carousel::default();
        carousel.begin();
        carousel.travel(-60., 2.);
        carousel.finish(settle_after(-60., 1, 360.), false);
        let painted = carousel.offset();
        assert_eq!(painted, 240.);

        carousel.begin();
        assert_eq!(carousel.offset(), painted, "the row did not move");

        assert!(carousel.travel(-20., 1.));
        assert_eq!(carousel.offset(), painted - 20.);

        let painted = carousel.offset();
        assert_eq!(carousel.release(), Some(-20.), "this finger's travel only");
        carousel.finish(painted, false);
        assert_eq!(carousel.offset(), painted, "and the finish keeps its pixel");
    }

    /// The axis lock: a scroll that goes down can never slide the page, and it
    /// stays a scroll for the rest of that touch however the thumb wanders.
    /// @req REQ-SCR-01
    #[test]
    fn a_vertical_gesture_never_becomes_a_page_swipe() {
        let mut carousel = Carousel::default();
        carousel.begin();

        // A thumb scroll's first few pixels: down, with the sideways drift
        // every thumb has. Too short to decide anything, and nothing moves.
        assert!(!carousel.travel(2., 4.));
        assert!(!carousel.travel(5., 7.));
        assert!(!carousel.blocked, "nothing has been decided yet");
        assert_eq!(carousel.offset(), 0.);

        // Clearly down now: the gesture is a scroll for good.
        assert!(!carousel.travel(9., 22.));
        assert!(carousel.blocked);
        assert!(
            !carousel.travel(-60., 2.),
            "a sideways jerk does not revive it"
        );
        assert_eq!(carousel.offset(), 0., "the page never moved");
        assert_eq!(carousel.release(), None, "and nothing is handed over");

        // The next touch starts clean.
        carousel.begin();
        assert!(!carousel.blocked);
        assert!(carousel.travel(-60., 2.));
        assert_eq!(carousel.release(), Some(-60.));
    }

    /// Small noisy deltas at the start of a real swipe do not lock it: only
    /// travel that is clearly vertical does.
    /// @req REQ-SCR-01
    #[test]
    fn a_noisy_start_still_swipes() {
        let mut carousel = Carousel::default();
        carousel.begin();
        assert!(!carousel.travel(2., 4.), "under the lock distance");
        assert!(!carousel.blocked);
        assert!(carousel.travel(-30., 4.));
        assert!(
            carousel.travel(-40., 6.),
            "one diagonal step of a real swipe"
        );
        assert_eq!(carousel.offset(), -70.);
        assert!(carousel.release().is_some(), "still one page gesture");
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
