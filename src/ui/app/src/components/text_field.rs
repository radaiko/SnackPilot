//! Text field (04 §4), wrapping `gpui_kit::component::input::Input` with the
//! §7 `inputField` recipe (surface fill, borderInput, radius 10).

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{div, App, AppContext, Div, Entity, SharedString, Window};

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

/// Style an existing `InputState` with the §7 `inputField` recipe.
pub fn field_with_state(state: &Entity<InputState>, theme: &AppTheme) -> Div {
    let r = theme.input_field();
    // The kit's Input paints its own theme background and a 1 px
    // `theme.input` border (a bright ring in dark mode), so it is switched
    // off; the §7 fill, radius and border all come from the wrapper.
    theme::apply(div().w_full(), &r)
        .child(Input::new(state).bg(r.bg).rounded(r.radius).bordered(false))
}
