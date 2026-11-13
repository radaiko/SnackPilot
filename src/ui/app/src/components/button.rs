//! Buttons (04 §4): primary, secondary, danger and the FAB, wrapped around
//! `gpui_kit::component::Button` with the §7 recipes' colours and radii.
//! The iOS soft/accent shadow is carried by the kit's custom variant
//! (`shadow(true)`); Android stays flat (v1 `useFlatStyle`).

use gpui::{hsla, px};
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_kit::prelude::Styled;
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

/// Compact action metrics (REQ-UI-16): every action is at least 48 px high,
/// the option radius is the pinned 8 px, and the primary action keeps its
/// 14/12 surface radius. The hit box is never smaller than `mobile.target`
/// on either axis.
pub fn mobile_action_metrics(kind: ButtonKind, flat: bool) -> (f32, f32) {
    let radius = match kind {
        ButtonKind::Primary | ButtonKind::Danger => {
            if flat {
                snackpilot_model::tokens::mobile::SPACE_3
            } else {
                14.
            }
        }
        ButtonKind::Secondary => 8.,
    };
    (snackpilot_model::tokens::mobile::ACTION_HEIGHT, radius)
}

/// Whether a compact control really meets the two-axis 44×44 target floor.
/// Production `mobile_action` applies the same floor as `min_w`/`min_h`, so the
/// helper is what the control paints, not a detached calculator.
pub fn meets_target(width: f32, height: f32) -> bool {
    width >= snackpilot_model::tokens::mobile::TARGET
        && height >= snackpilot_model::tokens::mobile::TARGET
}

/// What one press releases into (REQ-UI-16) is the platform's click rule: gpui
/// fires a click only when the up lands on the same hovered hitbox as the down,
/// so an outside release or a content move between them (a scroll claim) cancels
/// the activation, and a disabled control is built without any callback at all.
/// The compact builder therefore installs no press state machine of its own —
/// styling is fill-only and never the duplicate-submit protection.
/// One §4 button: German `label` in Inter, colours and radius from the §7
/// recipe for `kind`.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    mobile_action(id, label, kind, true, theme, cx)
}

/// One compact action with the Day Tray recipe (REQ-UI-15/16): 48 px high, the
/// primary fill from `mobile.actionFill` with its readable ink and 14 % pressed
/// fill, or a neutral outlined secondary. A disabled action paints the pinned
/// disabled recipe and installs no callback at all.
pub fn mobile_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    enabled: bool,
    theme: &AppTheme,
    cx: &App,
) -> Div {
    let (height, radius) = mobile_action_metrics(kind, theme.flat);
    debug_assert!(meets_target(theme.mobile.target, height));
    let (fill, ink) = if !enabled {
        (theme.mobile.disabled_fill, theme.mobile.disabled_ink)
    } else {
        match kind {
            ButtonKind::Primary => (theme.mobile.action_fill, theme.mobile.action_ink),
            ButtonKind::Danger => (theme.mobile.danger_fill, theme.mobile.danger_action_ink),
            ButtonKind::Secondary => (theme.mobile.secondary_fill, theme.mobile.secondary),
        }
    };
    let pressed = theme.mobile_pressed(fill);
    let frame = div()
        .flex()
        .min_w(px(theme.mobile.target))
        .min_h(px(height))
        .rounded(px(radius))
        .bg(fill)
        .border(px(1.))
        .border_color(if kind == ButtonKind::Secondary {
            theme.mobile.secondary_boundary
        } else {
            fill
        });
    frame.child(
        Button::new(id)
            .label(label)
            .min_w(px(theme.mobile.target))
            .min_h(px(height))
            .px(px(theme.mobile.space_4))
            .rounded(px(radius))
            .text_size(px(theme.mobile.body))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(fill)
                    .foreground(ink)
                    .hover(if enabled { pressed } else { fill })
                    .active(if enabled { pressed } else { fill })
                    .shadow(false),
            ),
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
    mobile_action_action(id, label, kind, theme, cx, on_click)
}

/// Tappable compact action: the same recipe as [`mobile_action`] with the one
/// click handler.
pub fn mobile_action_action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    theme: &AppTheme,
    cx: &App,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let (height, radius) = mobile_action_metrics(kind, theme.flat);
    debug_assert!(meets_target(theme.mobile.target, height));
    let (fill, ink) = match kind {
        ButtonKind::Primary => (theme.mobile.action_fill, theme.mobile.action_ink),
        ButtonKind::Danger => (theme.mobile.danger_fill, theme.mobile.danger_action_ink),
        ButtonKind::Secondary => (theme.mobile.secondary_fill, theme.mobile.secondary),
    };
    let pressed = theme.mobile_pressed(fill);
    let frame = div()
        .flex()
        .min_w(px(theme.mobile.target))
        .min_h(px(height))
        .rounded(px(radius))
        .bg(fill)
        .border(px(1.))
        .border_color(if kind == ButtonKind::Secondary {
            theme.mobile.secondary_boundary
        } else {
            fill
        });
    frame.child(
        Button::new(id)
            .label(label)
            .min_w(px(theme.mobile.target))
            .min_h(px(height))
            .px(px(theme.mobile.space_4))
            .rounded(px(radius))
            .text_size(px(theme.mobile.body))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(fill)
                    .foreground(ink)
                    .hover(pressed)
                    .active(pressed)
                    .shadow(false),
            )
            .on_click(on_click),
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
    let fill = theme.mobile_card_fill();
    shell(
        Button::new(id)
            .icon(icon)
            .rounded(r.radius)
            .min_h(px(theme.mobile.action_height))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(fill)
                    .foreground(theme.text_primary)
                    .hover(theme.mobile_pressed(fill))
                    .active(theme.mobile_pressed(fill))
                    .shadow(false),
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

#[cfg(test)]
mod tests {
    use super::{meets_target, mobile_action_metrics, ButtonKind};
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_model::desktop::{contrast_ratio, HostKind};
    use snackpilot_model::tokens::mobile;
    use snackpilot_model::AccentColor;

    /// The shared compact controls keep the interaction contract (REQ-UI-16):
    /// 44×44 two-axis targets, primary actions 48 high, the disabled recipe
    /// distinct from the actionable one (an inert `mobile_action` is built with
    /// no callback), and an independent 2+2 focus ring. Dismissal on an outside
    /// release or scroll claim is the platform's click rule: gpui only fires a
    /// click when the up lands on the same hovered hitbox as the down, so the
    /// control needs no second state machine. The recipe fields asserted here
    /// are the same fields `mobile_action` paints.
    /// @req REQ-UI-16
    #[test]
    fn mobile_action_recipes_targets_and_disabled_context() {
        // Hit bounds: 44×44 on both axes, never smaller. The builder applies
        // the same floor as min_w/min_h on the control it returns.
        assert!(meets_target(44., 44.));
        assert!(!meets_target(43.9, 44.), "width below the floor");
        assert!(!meets_target(44., 43.9), "height below the floor");
        let theme = AppTheme::build(false, AccentColor::Orange, true);
        assert!(meets_target(
            theme.mobile.target,
            theme.mobile.action_height
        ));

        // Action metrics: 48 high, option radius 8, primary 14 glass/12 flat.
        for flat in [false, true] {
            for kind in [
                ButtonKind::Primary,
                ButtonKind::Secondary,
                ButtonKind::Danger,
            ] {
                let (height, radius) = mobile_action_metrics(kind, flat);
                assert_eq!(height, mobile::ACTION_HEIGHT, "primary actions are 48 high");
                assert!(height >= mobile::TARGET);
                match kind {
                    ButtonKind::Primary | ButtonKind::Danger => {
                        assert_eq!(radius, if flat { 12. } else { 14. });
                    }
                    ButtonKind::Secondary => assert_eq!(radius, mobile::RADIUS_OPTION),
                }
            }
        }

        // The focus ring is independent of selection: a 2 px stroke in the
        // resolved focus ink with a 2 px gap, so the reserved room is 4 px and
        // the rest dimensions never move.
        assert_eq!(
            (theme.mobile.focus_width, theme.mobile.focus_gap),
            (mobile::FOCUS_WIDTH, mobile::FOCUS_GAP)
        );
        assert_eq!(theme.mobile.focus_width + theme.mobile.focus_gap, 4.);
        assert_eq!(
            theme.mobile.focus,
            crate::theme::from_rgb(mobile::focus_ink(false)),
            "the ring is the pinned focus ink, independent of the accent"
        );
        let wide = AppTheme::build(false, AccentColor::Orange, false);
        assert_eq!(theme.mobile.focus, wide.mobile.focus);
        for dark in [false, true] {
            let built = AppTheme::build(dark, AccentColor::Berry, true);
            assert_eq!(
                built.mobile.focus,
                crate::theme::from_rgb(mobile::focus_ink(dark))
            );
        }

        // Danger and the outlined secondary paint their projected fill/ink and
        // boundary pairs — not the unpainted `error`/`on_primary` or
        // `border`/`text_primary` pair — and each pair keeps its floor on the
        // fill the control really covers.
        for dark in [false, true] {
            for flat in [false, true] {
                let theme = AppTheme::build(dark, AccentColor::Golden, flat);
                let danger_fill = to_rgb(theme.mobile.danger_fill, theme.mobile.danger_fill);
                assert!(
                    contrast_ratio(
                        to_rgb(theme.mobile.danger_action_ink, theme.mobile.danger_fill),
                        danger_fill
                    ) >= mobile::TEXT_MIN_CONTRAST,
                    "danger ink reads on the projected error fill"
                );
                let secondary_fill =
                    to_rgb(theme.mobile.secondary_fill, theme.mobile.secondary_fill);
                assert!(
                    contrast_ratio(
                        to_rgb(theme.mobile.secondary, theme.mobile.secondary_fill),
                        secondary_fill
                    ) >= mobile::TEXT_MIN_CONTRAST
                );
                assert!(
                    contrast_ratio(
                        to_rgb(theme.mobile.secondary_boundary, theme.mobile.secondary_fill),
                        secondary_fill
                    ) >= mobile::CONTROL_MIN_CONTRAST
                );
            }
        }

        // Disabled keeps readable context and never dims the whole content:
        // it swaps to the pinned disabled fill + readable ink only, so an
        // inert control cannot be mistaken for an actionable primary.
        assert_eq!(
            theme.mobile.disabled_fill,
            crate::theme::from_rgb(mobile::disabled_fill(false))
        );
        assert_eq!(
            theme.mobile.disabled_ink,
            crate::theme::from_rgb(mobile::disabled_ink(false))
        );
        assert_ne!(theme.mobile.disabled_fill, theme.mobile.action_fill);
        assert_ne!(theme.mobile.disabled_fill, theme.mobile.danger_fill);

        // The compact column is a host projection, never a desktop restyle.
        assert!(theme.mobile_geometry(HostKind::Desktop, 360.).is_some());
        assert!(theme.mobile_geometry(HostKind::Desktop, 720.).is_none());
    }
}
