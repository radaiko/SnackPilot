//! Text fields: 48 px high, 10 px radius, label above (drawn by the caller),
//! surface fill, 1 px strong border and a 2 px accent focus ring at 2 px.

use gpui::{hsla, AnyElement, Focusable, Hsla};
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

/// The bare 48 px row: fill, 1 px border, and an optional control inside the
/// trailing edge (the password reveal), 8 px from the text.
fn field_row(
    state: &Entity<InputState>,
    fill: Hsla,
    border: Hsla,
    trailing: Option<AnyElement>,
) -> Div {
    div()
        .w_full()
        .min_h(px(tokens::ACTION_HEIGHT))
        .flex()
        .items_center()
        .gap(px(tokens::SPACE_2))
        .px(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill)
        .border(px(1.))
        .border_color(border)
        .child(
            Input::new(state)
                .bg(fill)
                .rounded(px(tokens::RADIUS_CONTROL))
                .bordered(false),
        )
        .when_some(trailing, |row, trailing| row.child(trailing))
}

/// The desktop field: the same 48 px input, with the state's fill and border.
pub fn desktop_field_with_state(
    state: &Entity<InputState>,
    control: ControlState,
    theme: &AppTheme,
) -> Div {
    desktop_field_slot(state, control, theme, None, false)
}

/// The desktop field with a trailing control and an error border.
pub fn desktop_field_slot(
    state: &Entity<InputState>,
    control: ControlState,
    theme: &AppTheme,
    trailing: Option<AnyElement>,
    invalid: bool,
) -> Div {
    let fill = theme.control_fill(control);
    let border = if invalid {
        theme.error
    } else {
        theme.control_border(control)
    };
    theme::focus_frame(
        field_row(state, fill, border, trailing),
        control,
        tokens::RADIUS_CONTROL,
        theme,
    )
}

/// Whether native copy/cut is available for a text field.
pub fn clipboard_copyable(masked: bool, has_selection: bool) -> bool {
    has_selection && !masked
}

/// The 48 px field with a 2 px accent focus ring offset by 2 px.
pub fn field_with_state(state: &Entity<InputState>, theme: &AppTheme, cx: &App) -> Div {
    field_slot(state, theme, cx, None, false)
}

/// The mobile field with a trailing control and an error border.
pub fn field_slot(
    state: &Entity<InputState>,
    theme: &AppTheme,
    cx: &App,
    trailing: Option<AnyElement>,
    invalid: bool,
) -> Div {
    let focus = state.read(cx).focus_handle(cx);
    let border = if invalid {
        theme.error
    } else {
        theme.text_tertiary
    };
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
        .child(field_row(state, theme.surface, border, trailing))
}
