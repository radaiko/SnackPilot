//! Pull-to-refresh — docs/requirements/04-ui-ux.md §3.3 / §4.7.
//!
//! A vertical drag of 60+ px over the list forces a fetch (`reloadBilling`).
//! The gesture is only a trigger; the spinner comes from the model's loading
//! state, so no extra view state is kept.

use std::rc::Rc;

use gpui::*;

use crate::navigation::ClickHandler;

/// Downward drag distance that triggers a refresh.
pub const PULL_PX: f32 = 60.;

/// Wrap `content` so that a downward drag past [`PULL_PX`] calls `on_refresh`.
pub fn pull_refresh(id: &'static str, content: AnyElement, on_refresh: ClickHandler) -> AnyElement {
    let start = Rc::new(std::cell::Cell::new(None::<f32>));
    let down_start = Rc::clone(&start);
    let up_start = Rc::clone(&start);
    div()
        .id(id)
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        .on_mouse_down(MouseButton::Left, move |e, _w, _cx| {
            down_start.set(Some(f32::from(e.position.y)));
        })
        .on_mouse_up(MouseButton::Left, move |e, window, cx| {
            if let Some(start) = up_start.take() {
                if f32::from(e.position.y) - start >= PULL_PX {
                    on_refresh(window, cx);
                }
            }
        })
        .child(content)
        .into_any_element()
}
