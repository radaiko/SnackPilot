//! Menu card — docs/requirements/04-ui-ux.md §4.5.
//!
//! One card per menu item: state-driven surface, at most one badge, dish text,
//! allergens and price, plus the selected checkmark disc. Tappability is
//! decided by the caller from the model (`ordered || (available && !cutoff)`).

use gpui::*;

use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MenuItem;

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
    let mut bottom = div()
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
    if selected {
        // §4.5: 24×24 white disc, ✓ in colors.primary, top-right 8.
        bottom = bottom.child(
            div()
                .absolute()
                .top(px(-8.))
                .right(px(-8.))
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
    card.relative().child(bottom)
}
