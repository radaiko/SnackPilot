//! Buttons (04 §4): primary, secondary, danger and the FAB, wrapped around
//! `gpui_kit::component::Button` with the §7 recipes' colours and radii.
//! The iOS soft/accent shadow is carried by the kit's custom variant
//! (`shadow(true)`); Android stays flat (v1 `useFlatStyle`).

use gpui::{hsla, px};
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{div, App, ClickEvent, Div, ElementId, SharedString, Window};
use snackpilot_model::desktop::ControlState;

use crate::theme::{self, AppTheme};

/// Desktop action target floor (brief: 44 px), shared with the model's
/// `TARGET_MIN` so the geometry and the control agree.
const DESKTOP_TARGET: f32 = snackpilot_model::desktop::TARGET_MIN;

/// The kit's `Custom` variant fades its normal background to 20 % alpha
/// (`bg_color` mixes with transparent), but §7 wants the opaque token fill.
/// Laying the button over a div with the recipe's own background restores the
/// exact colour: 20 % of `c` over `c` is `c`. The same shell carries the
/// recipe's border and shadow (04 §7), which the kit variant does not paint.
fn shell(button: Button, r: &theme::Recipe) -> Div {
    theme::apply(div(), r).child(button)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

/// One §4 button: German `label` in Inter, colours and radius from the §7
/// recipe for `kind`.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let r = match kind {
        ButtonKind::Primary => theme.button_primary(),
        ButtonKind::Secondary => theme.button_secondary(),
        ButtonKind::Danger => theme.button_danger(),
    };
    let fg = match kind {
        ButtonKind::Primary | ButtonKind::Danger => theme.on_primary,
        ButtonKind::Secondary => theme.text_primary,
    };
    let hover = theme::darken(r.bg, 0.08);
    shell(
        Button::new(id).label(label).rounded(r.radius).custom(
            ButtonCustomVariant::new(cx)
                .color(r.bg)
                .foreground(fg)
                .hover(hover)
                .active(theme::darken(hover, 0.06))
                .shadow(r.shadow.is_some()),
        ),
        &r,
    )
}

/// Same as [`button`] but tappable — used by the gallery page switch.
pub fn button_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let r = match kind {
        ButtonKind::Primary => theme.button_primary(),
        ButtonKind::Secondary => theme.button_secondary(),
        ButtonKind::Danger => theme.button_danger(),
    };
    let fg = match kind {
        ButtonKind::Primary | ButtonKind::Danger => theme.on_primary,
        ButtonKind::Secondary => theme.text_primary,
    };
    let hover = theme::darken(r.bg, 0.08);
    shell(
        Button::new(id)
            .label(label)
            .rounded(r.radius)
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(r.bg)
                    .foreground(fg)
                    .hover(hover)
                    .active(theme::darken(hover, 0.06))
                    .shadow(r.shadow.is_some()),
            )
            .on_click(on_click),
        &r,
    )
}

/// Floating action button (04 §7 `fab`): round, accent-filled, German or
/// icon `label`.
pub fn fab(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let r = theme.fab();
    shell(
        Button::new(id).label(label).rounded(r.radius).custom(
            ButtonCustomVariant::new(cx)
                .color(r.bg)
                .foreground(theme.on_primary)
                .hover(theme::darken(r.bg, 0.08))
                .active(theme::darken(r.bg, 0.14))
                .shadow(r.shadow.is_some()),
        ),
        &r,
    )
}

/// Circular icon button (04 §7 `circleButton`), e.g. the back button.
pub fn circle_button(
    id: impl Into<ElementId>,
    icon: gpui_kit::component::Icon,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let r = theme.circle_button();
    shell(
        Button::new(id).icon(icon).rounded(r.radius).custom(
            ButtonCustomVariant::new(cx)
                .color(r.bg)
                .foreground(theme.text_primary)
                .hover(theme::darken(r.bg, 0.05))
                .active(theme::darken(r.bg, 0.1))
                .shadow(r.shadow.is_some()),
        ),
        &r,
    )
}

/// Desktop action button (desktop-ui-redesign brief "Action states"): filled
/// primary/danger using the derived rest/hover (8 %)/pressed (14 %) fills and
/// the readable ink, or a transparent secondary with a 1 px border and
/// `surface_variant` hover. Min height 44 px, radius 8, 16 px side padding,
/// no shadow — shadows are the mobile dialect here.
/// @req REQ-UI-07 @req REQ-UI-13
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

/// Same action with the pointer/focus state the desktop host or gallery
/// forces: disabled swaps only the ink (`text_tertiary`), focus paints the
/// reserved ring, and rest/hover/selected keep the recipe fill — hover and
/// press still come from the kit's pointer states.
/// @req REQ-UI-07 @req REQ-UI-13
pub fn desktop_action_state(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    state: ControlState,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let clear = hsla(0., 0., 0., 0.);
    let (action, fill) = match kind {
        ButtonKind::Primary => (theme.desktop.action, Some(theme.desktop.action.fill)),
        ButtonKind::Danger => (theme.desktop.danger, Some(theme.desktop.danger.fill)),
        ButtonKind::Secondary => (
            theme::DesktopAction {
                fill: clear,
                hover: theme.surface_variant,
                pressed: theme::darken(theme.surface_variant, 0.06),
                ink: theme.text_primary,
            },
            None,
        ),
    };
    // Same trick as `shell`: the kit's Custom variant paints 20 % of the fill
    // over the parent, so the parent carries the opaque recipe fill.
    let frame = div().flex().min_h(px(DESKTOP_TARGET)).rounded(px(8.));
    let frame = match fill {
        Some(fill) => frame.bg(fill),
        None => frame.border(px(1.)).border_color(theme.border),
    };
    let ink = if state == ControlState::Disabled {
        theme.control_ink(state)
    } else {
        action.ink
    };
    let button = frame.child(
        Button::new(id)
            .label(label)
            .rounded(px(8.))
            .min_h(px(DESKTOP_TARGET))
            .px(px(16.))
            .text_size(px(14.))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(action.fill)
                    .foreground(ink)
                    .hover(action.hover)
                    .active(action.pressed)
                    .shadow(false),
            )
            .on_click(on_click),
    );
    theme::focus_frame(button, state, 8., theme)
}
