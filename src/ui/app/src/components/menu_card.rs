//! Menu card — docs/requirements/04-ui-ux.md §4.5.
//!
//! One card per menu item: state-driven surface, at most one badge, dish text,
//! allergens and price, plus the selected checkmark disc. Tappability is
//! decided by the caller from the model (`ordered || (available && !cutoff)`).

use gpui::*;

use crate::components::button::ButtonKind;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MenuItem;
use snackpilot_model::menu_key;

/// §4.5 state overlays, later wins.
pub struct MenuCardState {
    pub selected: bool,
    pub pending_cancel: bool,
    pub ordered: bool,
    pub available: bool,
    pub cutoff: bool,
}

fn badge(state: &MenuCardState, theme: &AppTheme) -> Option<(&'static str, Hsla, Hsla, Hsla)> {
    // (label, background, border, text) in the §4.5 order.
    if state.pending_cancel {
        let bg = if theme.flat {
            theme.warning_surface
        } else {
            theme.glass_warning
        };
        return Some(("Wird storniert", bg, theme.warning, theme.warning_text));
    }
    if state.ordered {
        let bg = if theme.flat {
            theme.success_surface
        } else {
            theme.glass_success
        };
        return Some(("Bestellt", bg, theme.success, theme.success_text));
    }
    if !state.available {
        return Some((
            "Ausverkauft",
            theme.error_surface,
            theme.error,
            theme.error_text,
        ));
    }
    if state.cutoff {
        let bg = if theme.flat {
            theme.warning_surface
        } else {
            theme.glass_warning
        };
        return Some(("Geschlossen", bg, theme.warning, theme.warning_text));
    }
    None
}

/// One §4.5 card.
pub fn menu_card(
    item: &MenuItem,
    state: MenuCardState,
    on_tap: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let selected = state.selected;
    let dim =
        (state.pending_cancel || (!state.available && !state.ordered) || state.cutoff) && !selected;

    let text = if selected {
        theme.on_primary
    } else if state.pending_cancel {
        theme.text_tertiary
    } else {
        theme.text_primary
    };
    let meta = if selected {
        theme.on_primary
    } else {
        theme.text_secondary
    };

    let mut card = theme::apply(
        div().flex().flex_col().p(px(16.)).mb(px(10.)),
        &theme.card_surface(),
    )
    .id(SharedString::from(format!("menu-{}|{}", item.id, item.day)));

    if selected {
        card = card
            .bg(theme.primary)
            .border_1()
            .border_color(theme.primary_dark);
        if !theme.flat {
            // iOS: white highlight on top/left, primaryDark below (§4.5).
            card = card
                .border_t_2()
                .border_l_2()
                .border_b_2()
                .border_color(theme.primary_dark);
        }
    } else if state.pending_cancel {
        card = card
            .border_dashed()
            .border_1()
            .border_color(theme.warning)
            .opacity(0.55);
    } else if state.ordered {
        let bg = if theme.flat {
            theme.success_surface
        } else {
            theme.glass_success
        };
        card = card.bg(bg).border_1().border_color(theme.success_border);
    } else if dim {
        card = card.opacity(0.5);
    }

    if let Some(on_tap) = on_tap {
        card = card
            .cursor_pointer()
            .on_click(move |_, window, cx| on_tap(window, cx));
    }

    if let Some((label, bg, border, fg)) = badge(&state, theme) {
        card = card.child(
            div().mb(px(2.)).flex().child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(border)
                    .bg(bg)
                    .text_size(px(10.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(fg)
                    .child(label),
            ),
        );
    }

    let mut title = div()
        .text_size(px(16.))
        .line_height(px(21.))
        .max_h(px(84.))
        .overflow_hidden()
        .text_color(text)
        .child(item.title.clone());
    if state.pending_cancel {
        title = title.line_through();
    }

    card = card.child(title);

    let allergens = if item.allergens.is_empty() {
        String::new()
    } else {
        format!("Allergene: {}", item.allergens.join(", "))
    };
    let bottom = div()
        .mt(px(6.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .flex_1()
                .text_size(px(11.))
                .text_color(meta)
                .child(allergens),
        )
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(meta)
                .child(item.price.clone()),
        );
    let mut card = card.relative().child(bottom);
    if selected {
        // §4.5: 24×24 white disc, ✓ in colors.primary, top-right 8.
        card = card.child(
            div()
                .absolute()
                .top(px(8.))
                .right(px(8.))
                .w(px(24.))
                .h(px(24.))
                .rounded(px(12.))
                .bg(hsla(0., 0., 1., 1.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.primary)
                .child("✓"),
        );
    }
    card
}

/// Desktop row state: the same domain facts as [`MenuCardState`] plus the
/// wide-desktop preview selection. Kept apart from the mobile card so that
/// recipe (and its opaque selected fill) stays exactly as before.
#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopMenuRowState {
    pub selected: bool,
    pub pending_order: bool,
    pub pending_cancel: bool,
    pub ordered: bool,
    pub available: bool,
    pub cutoff: bool,
}

/// The brief's one desktop row/detail status: `(label, ink, fill)` in
/// precedence order. A pending order is its check plus `Vorgemerkt`, never
/// the row cursor.
pub fn desktop_menu_status(
    state: &DesktopMenuRowState,
    theme: &AppTheme,
) -> Option<(&'static str, Hsla, Hsla)> {
    if state.pending_cancel {
        return Some(("Wird storniert", theme.warning_text, theme.warning_surface));
    }
    if state.pending_order {
        return Some((
            "✓ Vorgemerkt",
            theme.desktop.accent_on_selection,
            theme.primary_surface,
        ));
    }
    if state.ordered {
        return Some(("Bestellt", theme.success_text, theme.success_surface));
    }
    if !state.available {
        return Some(("Ausverkauft", theme.error_text, theme.error_surface));
    }
    if state.cutoff {
        return Some(("Geschlossen", theme.warning_text, theme.warning_surface));
    }
    None
}

/// The desktop detail's explicit action `(label, kind)`, in the brief's
/// precedence. `None` when the incumbent model allows no toggle at all
/// (`ordered || (available && !cutoff)` is the mobile gate).
pub fn desktop_menu_action(state: &DesktopMenuRowState) -> Option<(&'static str, ButtonKind)> {
    if state.pending_cancel {
        Some(("Stornierung aufheben", ButtonKind::Secondary))
    } else if state.pending_order {
        Some(("Auswahl aufheben", ButtonKind::Secondary))
    } else if state.ordered {
        Some(("Stornierung vormerken", ButtonKind::Danger))
    } else if state.available && !state.cutoff {
        Some(("Zur Bestellung hinzufügen", ButtonKind::Primary))
    } else {
        None
    }
}

/// One wide-desktop grouped menu row: min 88 px, 12/16 padding, category/title
/// 14/20 Semibold, a two-line 14/20 excerpt and price plus one status 12/16.
/// The 3 px accent rule is always present so selecting never shifts the row;
/// picking it previews only, which the caller wires.
/// @req REQ-SCR-01 @req REQ-UI-12
pub fn desktop_menu_row(
    item: &MenuItem,
    state: DesktopMenuRowState,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let selected = state.selected;
    // Never fabricate a dish: an empty subtitle falls back to the title.
    let excerpt = if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    };
    let excerpt = div()
        .max_h(px(40.))
        .overflow_hidden()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(if state.pending_cancel {
            theme.text_tertiary
        } else {
            theme.text_secondary
        })
        .child(excerpt);
    let excerpt = if state.pending_cancel {
        excerpt.line_through()
    } else {
        excerpt
    };

    let mut bottom = div()
        .mt(px(4.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(item.price.clone()),
        );
    if let Some((label, ink, fill)) = desktop_menu_status(&state, theme) {
        bottom = bottom.child(
            div()
                .px(px(8.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(fill)
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .whitespace_nowrap()
                .child(label),
        );
    }

    div()
        .id(SharedString::from(format!(
            "desktop-menu-{}",
            menu_key(item)
        )))
        .min_h(px(88.))
        .flex_shrink_0()
        .flex()
        .border_b_1()
        .border_color(theme.border)
        .bg(if selected {
            theme.primary_surface
        } else {
            theme.surface
        })
        .hover(|s| s.bg(theme.surface_variant))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(
            div()
                .w(px(3.))
                .self_stretch()
                .flex_shrink_0()
                .bg(if selected {
                    theme.desktop.accent_boundary
                } else {
                    hsla(0., 0., 0., 0.)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.))
                .px(px(16.))
                .py(px(12.))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(item.title.clone()),
                )
                .child(excerpt)
                .child(bottom),
        )
        .into_any_element()
}
