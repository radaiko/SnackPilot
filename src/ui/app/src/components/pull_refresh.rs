//! Pull-to-refresh — docs/requirements/04-ui-ux.md §3.3 / §4.7.
//!
//! A drag of 60+ px that starts while the list is at its top forces a fetch.
//! The gesture is only a trigger; the spinner comes from the model's loading
//! state, so no extra view state is kept beyond the live pull distance the
//! shell holds for the indicator.

use std::rc::Rc;

use gpui::*;

/// Downward drag distance that triggers a refresh.
pub const PULL_PX: f32 = 60.;

/// How far the drag may reveal before it stops following the finger.
pub const PULL_MAX: f32 = 90.;

/// One pull's gesture callbacks. The shell builds them because it owns the
/// drag state (start position, pull distance, whether the list was at its top),
/// and the list it measures is the one the screen renders.
pub struct PullGesture {
    pub down: Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>,
    pub moved: Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>,
    pub up: Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>,
}

/// Wrap `content` so pulling from the top of its list reveals `pull` px with a
/// spinner and calls the refresh on release.
pub fn pull_refresh(
    id: &'static str,
    content: AnyElement,
    pull: f32,
    gesture: PullGesture,
    theme: &crate::theme::AppTheme,
) -> AnyElement {
    let PullGesture { down, moved, up } = gesture;
    let mut root = div()
        .id(id)
        .flex_1()
        .min_h(px(0.))
        .relative()
        .flex()
        .flex_col()
        .on_mouse_down(MouseButton::Left, move |e, w, cx| down(e, w, cx))
        .on_mouse_move(move |e, w, cx| moved(e, w, cx))
        .on_mouse_up(MouseButton::Left, move |e, w, cx| up(e, w, cx));
    if pull > 0. {
        root = root.child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .right(px(0.))
                .h(px(pull))
                .flex()
                .items_center()
                .justify_center()
                .opacity((pull / PULL_PX).min(1.))
                .child(crate::components::spinner::spinner(
                    theme.text_secondary,
                    16.,
                )),
        );
    }
    root.child(
        div()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .pt(px(pull))
            .child(content),
    )
    .into_any_element()
}
