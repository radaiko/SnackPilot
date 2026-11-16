//! Horizontal swipe between menu days — docs/requirements/04-ui-ux.md §3.1.
//!
//! The content area below the day navigator follows the drag: vertical movement
//! stays a scroll, horizontal movement over 10 px claims the gesture. At the
//! first/last date the translation is damped to 30 % (rubber band). Releasing
//! past 50 px switches to the previous/next day; the shell owns the transient
//! drag state.
//!
//! The drag is read from scroll deltas, never from mouse events: both device
//! hosts deliver a finger drag as `ScrollWheel` + `MouseMove` and defer
//! `MouseDown` until the finger lifts (so a drag near a row cannot activate
//! it). Reading `MouseDown` as the gesture start was what kept the swipe from
//! ever firing on a phone — there was no press to start it from.
//!
//! Task 1.3 adds the approved reduced-motion presentation only: the day swap
//! happens without the drag travelling, while the claim/switch thresholds and
//! the gesture events stay exactly as they are.

use std::rc::Rc;

use gpui::*;

/// Gesture claim threshold and switch threshold in logical px (§3.1).
pub const CLAIM_PX: f32 = 10.;
pub const SWITCH_PX: f32 = 50.;
/// How far the content may travel: a long drag stops pushing it further, so it
/// can never be dragged off the surface.
pub const TRAVEL_MAX: f32 = 240.;
/// Rubber-band factor at the first/last date.
pub const EDGE_DAMPING: f32 = 0.3;

pub type ScrollHandler = Rc<dyn Fn(&ScrollWheelEvent, &mut Window, &mut App)>;
pub type UpHandler = Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>;

/// Callbacks the shell wires to its own transient drag state.
pub struct SwipeHandlers {
    pub scrolled: ScrollHandler,
    pub up: UpHandler,
}

/// One drag's travel in logical px: the host reports a finger drag as scroll
/// deltas, so this is the gesture's only input on device.
pub fn drag_delta(event: &ScrollWheelEvent) -> (f32, f32) {
    let delta = event.delta.pixel_delta(px(16.));
    (f32::from(delta.x), f32::from(delta.y))
}

/// Whether one pointer movement claims the day gesture: horizontal movement
/// dominates and passes the 10 px claim threshold (§3.1, unchanged).
pub fn claims_horizontal(dx: f32, dy: f32) -> bool {
    dx.abs() > CLAIM_PX && dx.abs() > dy.abs()
}

/// Whether one scroll delta belongs to the day gesture. It has to travel
/// further across than down, so the list's own vertical scrolling never moves
/// the day — a run of slightly slanted vertical deltas used to accumulate into
/// a day switch on release.
pub fn is_swipe_delta(dx: f32, dy: f32) -> bool {
    dx != 0. && dx.abs() > dy.abs()
}

/// One more delta of travel, capped at [`TRAVEL_MAX`].
pub fn add_travel(total: f32, dx: f32) -> f32 {
    (total + dx).clamp(-TRAVEL_MAX, TRAVEL_MAX)
}

/// The damped horizontal offset painted while the gesture runs. Reduced motion
/// removes the travel — the day still changes on release, but the content never
/// slides. Edge damping (30 %) is unchanged whenever travel is shown.
pub fn swipe_offset(dx: f32, can_back: bool, can_forward: bool, reduce_motion: bool) -> f32 {
    if reduce_motion {
        return 0.;
    }
    if (dx > 0. && !can_back) || (dx < 0. && !can_forward) {
        dx * EDGE_DAMPING
    } else {
        dx
    }
}

/// The day a released gesture switches to: `-1` back, `1` forward, `None` for
/// a short or edge-blocked drag. The 50 px switch threshold is unchanged.
pub fn swipe_day(dx: f32, can_back: bool, can_forward: bool) -> Option<i32> {
    if dx > SWITCH_PX && can_back {
        Some(-1)
    } else if dx < -SWITCH_PX && can_forward {
        Some(1)
    } else {
        None
    }
}

/// Wrap `content` in the swipeable area. The drag is reported to the shell,
/// which positions the day carousel itself, so the content arrives already
/// placed and this only owns the gesture and the clip.
pub fn swipe_days(id: &'static str, content: AnyElement, handlers: SwipeHandlers) -> AnyElement {
    let (scrolled, up) = (handlers.scrolled, handlers.up);
    div()
        .id(id)
        .size_full()
        .overflow_hidden()
        .on_scroll_wheel(move |e, w, cx| scrolled(e, w, cx))
        .on_mouse_up(MouseButton::Left, move |e, w, cx| up(e, w, cx))
        .child(div().size_full().child(content))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        add_travel, claims_horizontal, is_swipe_delta, swipe_day, swipe_offset, CLAIM_PX,
        EDGE_DAMPING, SWITCH_PX, TRAVEL_MAX,
    };

    /// A thumb drag arrives as a run of scroll deltas, not as one movement:
    /// the gesture is claimed once their sum passes the threshold, and the
    /// same sum decides the day.
    /// @req REQ-SCR-01
    #[test]
    fn a_run_of_drag_deltas_accumulates_into_a_day_switch() {
        let travel = |deltas: &[(f32, f32)]| {
            deltas
                .iter()
                .filter(|(dx, dy)| is_swipe_delta(*dx, *dy))
                .fold(0., |total, (dx, _)| add_travel(total, *dx))
        };

        let dx = travel(&[(6., 1.), (9., 1.), (14., 2.), (20., 3.), (9., 1.)]);
        assert!(claims_horizontal(dx, 0.), "59 px across claims the gesture");
        assert_eq!(swipe_day(dx, true, true), Some(-1), "right is the day back");

        let dx = travel(&[(-18., 2.), (-30., 3.), (-16., 2.)]);
        assert_eq!(swipe_day(dx, true, true), Some(1), "left is the day on");

        // A slanted vertical scroll: every delta travels further down than
        // across, so the day never sees it. This used to add up to a switch on
        // release whenever the thumb drifted sideways on the way down.
        let dx = travel(&[(5., 30.), (4., 28.), (6., 22.)]);
        assert_eq!(dx, 0.);
        assert_eq!(swipe_day(dx, true, true), None);

        let dx = travel(&[(-40., 0.), (-40., 0.)]);
        assert_eq!(
            swipe_day(dx, true, false),
            None,
            "the last day cannot go further"
        );
    }

    /// A drag stops moving the content at [`TRAVEL_MAX`], so a long swipe can
    /// never drag the list off its surface.
    /// @req REQ-SCR-01
    #[test]
    fn travel_is_capped() {
        let mut total = 0.;
        for _ in 0..20 {
            total = add_travel(total, 40.);
        }
        assert_eq!(total, TRAVEL_MAX);
        assert_eq!(add_travel(-1000., -1.), -TRAVEL_MAX);
    }

    /// The day gesture keeps its pinned thresholds (REQ-SCR-01) and the
    /// approved reduced-motion presentation changes nothing about them: the
    /// claim/switch decisions stay, only the drag travel goes.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_swipe_thresholds_and_reduced_motion() {
        assert_eq!((CLAIM_PX, SWITCH_PX, EDGE_DAMPING), (10., 50., 0.3));

        // Claim: horizontal dominantly past 10 px, never a vertical scroll.
        assert!(
            !claims_horizontal(10., 0.),
            "the claim threshold is exclusive"
        );
        assert!(claims_horizontal(11., 0.));
        assert!(
            !claims_horizontal(-11., 20.),
            "a vertical scroll stays a scroll"
        );

        // Switch at exactly 50 px is not yet a switch; 51 is.
        assert_eq!(swipe_day(50., true, true), None);
        assert_eq!(swipe_day(51., true, true), Some(-1));
        assert_eq!(swipe_day(-51., true, true), Some(1));
        assert_eq!(
            swipe_day(51., false, true),
            None,
            "no back at the first day"
        );
        assert_eq!(
            swipe_day(-51., true, false),
            None,
            "no forward at the last day"
        );

        // Edge damping 30 % while travelling; no travel under reduced motion.
        assert_eq!(swipe_offset(100., false, true, false), 100. * 0.3);
        assert_eq!(swipe_offset(-100., true, false, false), -100. * 0.3);
        assert_eq!(swipe_offset(100., true, true, false), 100.);
        for dx in [100., -100., 11.] {
            assert_eq!(swipe_offset(dx, false, false, true), 0.);
        }
        // Reduced motion never changes the day decision the release makes.
        assert_eq!(swipe_day(-51., true, true), Some(1));
    }
}
