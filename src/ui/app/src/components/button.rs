//! Buttons (04 §4): primary, secondary, danger and the FAB, wrapped around
//! `gpui_kit::component::Button` with the §7 recipes' colours and radii.
//! The iOS soft/accent shadow is carried by the kit's custom variant
//! (`shadow(true)`); Android stays flat (v1 `useFlatStyle`).

use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, Div, ElementId, SharedString};

use crate::theme::{self, AppTheme};

/// The kit's `Custom` variant fades its normal background to 20 % alpha
/// (`bg_color` mixes with transparent), but §7 wants the opaque token fill.
/// Laying the button over a div with the recipe's own background restores the
/// exact colour: 20 % of `c` over `c` is `c`.
fn shell(button: Button, r: &theme::Recipe) -> Div {
    div().bg(r.bg).rounded(r.radius).child(button)
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

/// Small helper so this module does not import `px` at call sites.
#[allow(dead_code)]
fn _px(v: f32) -> gpui_kit::Pixels {
    px(v)
}
