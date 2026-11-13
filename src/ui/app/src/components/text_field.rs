//! Text fields: 48 px high, 10 px radius, label above (drawn by the caller),
//! surface fill, 1 px strong border and a 2 px accent focus ring at 2 px.

use gpui::{hsla, Focusable};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, AppContext, Div, Entity, SharedString, Window};
use snackpilot_model::desktop::ControlState;
use snackpilot_model::tokens;

use crate::theme::{self, AppTheme};

/// A text field; the caller keeps the returned `Entity<InputState>`.
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
    (state.clone(), field_with_state(&state, theme, cx))
}

/// The desktop field: the same 48 px input, with the state's fill and border.
pub fn desktop_field_with_state(
    state: &Entity<InputState>,
    control: ControlState,
    theme: &AppTheme,
) -> Div {
    let fill = theme.control_fill(control);
    let field = div()
        .w_full()
        .min_h(px(tokens::ACTION_HEIGHT))
        .flex()
        .items_center()
        .px(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill)
        .border(px(1.))
        .border_color(theme.control_border(control))
        .child(
            Input::new(state)
                .bg(fill)
                .rounded(px(tokens::RADIUS_CONTROL))
                .bordered(false),
        );
    theme::focus_frame(field, control, tokens::RADIUS_CONTROL, theme)
}

/// Whether native copy/cut is available for a text field.
pub fn clipboard_copyable(masked: bool, has_selection: bool) -> bool {
    has_selection && !masked
}

/// The 48 px field with a 2 px accent focus ring offset by 2 px.
pub fn field_with_state(state: &Entity<InputState>, theme: &AppTheme, cx: &App) -> Div {
    let focus = state.read(cx).focus_handle(cx);
    let field = div()
        .w_full()
        .min_h(px(tokens::ACTION_HEIGHT))
        .flex()
        .items_center()
        .px(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(theme.surface)
        .border(px(1.))
        .border_color(theme.text_tertiary)
        .child(
            Input::new(state)
                .bg(theme.surface)
                .rounded(px(tokens::RADIUS_CONTROL))
                .bordered(false),
        );
    div()
        .w_full()
        .p(px(tokens::FOCUS_GAP))
        .rounded(px(tokens::RADIUS_CONTROL
            + tokens::FOCUS_GAP
            + tokens::FOCUS_WIDTH))
        .border(px(tokens::FOCUS_WIDTH))
        .border_color(hsla(0., 0., 0., 0.))
        .track_focus(&focus)
        .focus(theme::desktop_focus(theme.accent))
        .child(field)
}
