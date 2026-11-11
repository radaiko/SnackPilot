//! Text field (04 §4), wrapping `gpui_kit::component::input::Input` with the
//! §7 `inputField` recipe (surface fill, borderInput, radius 10).

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, AppContext, Div, Entity, SharedString, Window};
use snackpilot_model::desktop::{ControlState, TARGET_MIN};

use crate::theme::{self, AppTheme};

/// A labelled text field. The caller keeps the returned `Entity<InputState>`
/// to read the value back.
pub fn text_field(
    placeholder: impl Into<SharedString>,
    value: Option<&str>,
    theme: &AppTheme,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<InputState>, Div) {
    let state = cx.new(|cx| {
        let state = InputState::new(window, cx).placeholder(placeholder.into());
        match value {
            Some(value) => state.default_value(value),
            None => state,
        }
    });
    (state.clone(), field_with_state(&state, theme))
}

/// Desktop text field (brief `desktop.inputField`): the same `InputState`,
/// but a 44 px minimum target, 8 px radius, 12 px inner padding, `surface`
/// fill and the input boundary from the pointer/focus state. The reserved
/// 4 px ring room keeps focus from shifting or clipping. The mobile field
/// keeps its incumbent recipe.
/// @req REQ-UI-07 @req REQ-UI-13
pub fn desktop_field_with_state(
    state: &Entity<InputState>,
    control: ControlState,
    theme: &AppTheme,
) -> Div {
    let fill = theme.control_fill(control);
    let field = div()
        .w_full()
        .min_h(px(TARGET_MIN))
        .flex()
        .items_center()
        .px(px(12.))
        .rounded(px(8.))
        .bg(fill)
        .border(px(1.))
        .border_color(theme.control_border(control))
        .hover(|s| s.bg(theme.control_fill(ControlState::Hover)))
        .child(Input::new(state).bg(fill).rounded(px(8.)).bordered(false));
    theme::focus_frame(field, control, 8., theme)
}

/// Style an existing `InputState` with the §7 `inputField` recipe.
pub fn field_with_state(state: &Entity<InputState>, theme: &AppTheme) -> Div {
    let r = theme.input_field();
    // The kit's Input paints its own theme background and a 1 px
    // `theme.input` border (a bright ring in dark mode), so it is switched
    // off; the §7 fill, radius and border all come from the wrapper.
    theme::apply(div().w_full(), &r)
        .child(Input::new(state).bg(r.bg).rounded(r.radius).bordered(false))
}
