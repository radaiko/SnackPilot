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

/// Whether native copy/cut is available for a text field. The pinned gpui-kit
/// input implements exactly this rule (`is_copyable = selection && !masked`),
/// so a masked credential field can never place its text on the clipboard
/// (brief "masked password copy/cut is disabled", REQ-UI-11).
pub fn clipboard_copyable(masked: bool, has_selection: bool) -> bool {
    has_selection && !masked
}

/// Style an existing `InputState` with the compact input recipe (REQ-UI-15):
/// 48 px minimum height, 12 px radius, 16 px horizontal / 12 px vertical inner
/// padding, `input_field` fill and `borderInput` boundary. The kit's Input
/// paints its own theme background and a 1 px `theme.input` border (a bright
/// ring in dark mode), so it is switched off and the wrapper carries the recipe.
pub fn field_with_state(state: &Entity<InputState>, theme: &AppTheme) -> Div {
    let r = theme.input_field();
    theme::apply(
        div()
            .w_full()
            .min_h(px(theme.mobile.action_height))
            .flex()
            .items_center()
            .px(px(theme.mobile.space_4))
            .py(px(theme.mobile.space_2)),
        &r,
    )
    .child(Input::new(state).bg(r.bg).rounded(r.radius).bordered(false))
}
