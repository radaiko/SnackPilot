//! Horizontal swipe between menu days — docs/requirements/04-ui-ux.md §3.1.
//!
//! The content area below the DayNavigator follows the pointer: vertical
//! movement stays a scroll, horizontal movement over 10 px claims the gesture.
//! At the first/last date the translation is damped to 30 % (rubber band).
//! Releasing past 50 px switches to the previous/next day; the shell owns the
//! transient drag state.

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
