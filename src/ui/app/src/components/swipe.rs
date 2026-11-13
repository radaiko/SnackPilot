//! Horizontal swipe between menu days — docs/requirements/04-ui-ux.md §3.1.
//!
//! The content area below the day navigator follows the pointer: vertical
//! movement stays a scroll, horizontal movement over 10 px claims the gesture.
//! At the first/last date the translation is damped to 30 % (rubber band).
//! Releasing past 50 px switches to the previous/next day; the shell owns the
//! transient drag state.
//!
//! Task 1.3 adds the approved reduced-motion presentation only: the day swap
//! happens without the drag travelling, while the claim/switch thresholds and
//! the gesture events stay exactly as they are.

use std::rc::Rc;

use gpui::*;

/// Gesture claim threshold and switch threshold in logical px (§3.1).
pub const CLAIM_PX: f32 = 10.;
pub const SWITCH_PX: f32 = 50.;
/// Rubber-band factor at the first/last date.
pub const EDGE_DAMPING: f32 = 0.3;

pub type DownHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;
pub type MoveHandler = Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>;
pub type UpHandler = Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>;

/// Callbacks the shell wires to its own transient drag state.
pub struct SwipeHandlers {
    pub down: DownHandler,
    pub moved: MoveHandler,
    pub up: UpHandler,
}

/// Whether one pointer movement claims the day gesture: horizontal movement
/// dominates and passes the 10 px claim threshold (§3.1, unchanged).
pub fn claims_horizontal(dx: f32, dy: f32) -> bool {
    dx.abs() > CLAIM_PX && dx.abs() > dy.abs()
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

/// Wrap `content` in the swipeable area, translating it by the shell's current
/// (damped) offset while a horizontal gesture is in flight.
pub fn swipe_days(
    id: &'static str,
    content: AnyElement,
    offset: f32,
    handlers: SwipeHandlers,
) -> AnyElement {
    let (down, moved, up) = (handlers.down, handlers.moved, handlers.up);
    div()
        .id(id)
        .size_full()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, move |e, w, cx| down(e, w, cx))
        .on_mouse_move(move |e, w, cx| moved(e, w, cx))
        .on_mouse_up(MouseButton::Left, move |e, w, cx| up(e, w, cx))
        .child(div().size_full().ml(px(offset)).child(content))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{claims_horizontal, swipe_day, swipe_offset, CLAIM_PX, EDGE_DAMPING, SWITCH_PX};

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
