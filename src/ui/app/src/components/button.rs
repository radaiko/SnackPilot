//! Buttons: primary filled accent, secondary outlined, quiet text, and the
//! destructive variant for cancel actions. One look on every platform.

use gpui::{px, Hsla};
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::prelude::Styled;
use gpui_kit::prelude::*;
use gpui_kit::{div, App, ClickEvent, Div, ElementId, SharedString, Window};
use snackpilot_model::desktop::ControlState;
use snackpilot_model::tokens;

use crate::theme::AppTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
    Quiet,
}

/// Every action is 48 px high with the 10 px control radius.
pub fn mobile_action_metrics(_kind: ButtonKind) -> (f32, f32) {
    (tokens::ACTION_HEIGHT, tokens::RADIUS_CONTROL)
}

/// Whether a control meets the 44×44 two-axis target floor.
pub fn meets_target(width: f32, height: f32) -> bool {
    width >= tokens::TARGET && height >= tokens::TARGET
}

fn colours(kind: ButtonKind, enabled: bool, theme: &AppTheme) -> (Hsla, Hsla, Option<Hsla>) {
    if !enabled {
        return (theme.surface_alt, theme.text_tertiary, None);
    }
    match kind {
        ButtonKind::Primary => (theme.accent, theme.on_accent, None),
        ButtonKind::Secondary => (theme.surface, theme.text_primary, Some(theme.text_tertiary)),
        ButtonKind::Danger => (theme.error, theme.on_error, None),
        ButtonKind::Quiet => (theme.surface, theme.accent, None),
    }
}

fn action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    enabled: bool,
    theme: &AppTheme,
    cx: &App,
) -> Button {
    let (fill, ink, border) = colours(kind, enabled, theme);
    let pressed = if enabled { theme.pressed(fill) } else { fill };
    let mut button = Button::new(id)
        .label(label)
        .min_w(px(tokens::TARGET))
        .min_h(px(tokens::ACTION_HEIGHT))
        .px(px(tokens::SPACE_4))
        .rounded(px(tokens::RADIUS_CONTROL))
        .text_size(px(tokens::BODY.size))
        .custom(
            ButtonCustomVariant::new(cx)
                .color(fill)
                .foreground(ink)
                .hover(pressed)
                .active(pressed)
                .shadow(false),
        );
    if let Some(border) = border {
        button = button.border_1().border_color(border);
    }
    button
}

/// One action, not tappable (used where the caller wraps it).
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let (fill, _ink, border) = colours(kind, true, theme);
    let mut frame = div()
        .flex()
        .min_h(px(tokens::ACTION_HEIGHT))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill);
    if let Some(border) = border {
        frame = frame.border(px(1.)).border_color(border);
    }
    frame.child(action(id, label, kind, true, theme, cx))
}

/// One action with an enabled state; a disabled one installs no callback.
pub fn mobile_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    enabled: bool,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let (fill, _ink, border) = colours(kind, enabled, theme);
    let mut frame = div()
        .flex()
        .min_w(px(tokens::TARGET))
        .min_h(px(tokens::ACTION_HEIGHT))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill);
    if let Some(border) = border {
        frame = frame.border(px(1.)).border_color(border);
    }
    frame.child(action(id, label, kind, enabled, theme, cx))
}

/// One tappable action.
pub fn button_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    mobile_action_action(id, label, kind, theme, cx, on_click)
}

/// One tappable action in the shared component.
pub fn mobile_action_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let (fill, _ink, border) = colours(kind, true, theme);
    let mut frame = div()
        .flex()
        .min_w(px(tokens::TARGET))
        .min_h(px(tokens::ACTION_HEIGHT))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill);
    if let Some(border) = border {
        frame = frame.border(px(1.)).border_color(border);
    }
    frame.child(action(id, label, kind, true, theme, cx).on_click(on_click))
}

/// Floating action button: accent-filled, 10 px radius, 48 px high.
pub fn fab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    div()
        .flex()
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(theme.accent)
        .child(
            Button::new(id)
                .label(label)
                .min_h(px(tokens::ACTION_HEIGHT))
                .rounded(px(tokens::RADIUS_CONTROL))
                .px(px(tokens::SPACE_4))
                .text_size(px(tokens::BODY.size))
                .custom(
                    ButtonCustomVariant::new(cx)
                        .color(theme.accent)
                        .foreground(theme.on_accent)
                        .hover(theme.accent_hover)
                        .active(theme.accent_pressed)
                        .shadow(false),
                ),
        )
}

/// Circular icon button, e.g. the back affordance.
pub fn circle_button(
    id: impl Into<ElementId>,
    icon: gpui_kit::component::Icon,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    div()
        .flex()
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(theme.surface_alt)
        .child(
            Button::new(id)
                .icon(icon)
                .min_w(px(tokens::TARGET))
                .min_h(px(tokens::TARGET))
                .rounded(px(tokens::RADIUS_CHIP))
                .custom(
                    ButtonCustomVariant::new(cx)
                        .color(theme.surface_alt)
                        .foreground(theme.text_primary)
                        .hover(theme.pressed(theme.surface_alt))
                        .active(theme.pressed(theme.surface_alt))
                        .shadow(false),
                ),
        )
}

/// Desktop action: the same action at the 44 px target with forced state.
pub fn desktop_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    desktop_action_state(id, label, kind, ControlState::Rest, theme, cx, on_click)
}

/// Desktop action with the pointer/focus state the host or gallery forces.
pub fn desktop_action_state(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    state: ControlState,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let enabled = state != ControlState::Disabled;
    let (fill, _ink, border) = colours(kind, enabled, theme);
    let mut frame = div()
        .flex()
        .min_h(px(tokens::ACTION_HEIGHT))
        .rounded(px(tokens::RADIUS_CONTROL))
        .bg(fill);
    if let Some(border) = border {
        frame = frame.border(px(1.)).border_color(border);
    }
    let button = frame.child(
        action(id, label, kind, enabled, theme, cx)
            .min_h(px(tokens::ACTION_HEIGHT))
            .rounded(px(tokens::RADIUS_CONTROL))
            .on_click(on_click),
    );
    crate::theme::focus_frame(button, state, tokens::RADIUS_CONTROL, theme)
}

#[cfg(test)]
mod tests {
    use super::{meets_target, mobile_action_metrics, ButtonKind};
    use crate::theme::AppTheme;
    use snackpilot_model::tokens;

    #[test]
    fn targets_and_radii_follow_the_scale() {
        assert!(meets_target(44., 44.));
        assert!(!meets_target(43.9, 44.));
        for kind in [
            ButtonKind::Primary,
            ButtonKind::Secondary,
            ButtonKind::Danger,
            ButtonKind::Quiet,
        ] {
            let (height, radius) = mobile_action_metrics(kind);
            assert_eq!(height, tokens::ACTION_HEIGHT);
            assert_eq!(radius, tokens::RADIUS_CONTROL);
        }
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.target, tokens::TARGET);
        assert_eq!(
            theme.scale.focus_width + theme.scale.focus_gap,
            4.,
            "the reserved focus ring room"
        );
    }
}
