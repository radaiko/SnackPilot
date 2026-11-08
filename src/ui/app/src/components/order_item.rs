//! Order item card — docs/requirements/04-ui-ux.md §4.6.
//!
//! Row card: date / category / dish on the left, approval badge and the cancel
//! affordance on the right. The caller passes `can_cancel`, whether this order
//! is currently being cancelled, and whether the cutoff has passed.

use gpui::*;

use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::OrderedMenu;
use snackpilot_model::format_date_long;

/// One §4.6 card.
pub fn order_item(
    order: &OrderedMenu,
    dish: Option<String>,
    can_cancel: bool,
    cancelling: bool,
    cutoff: bool,
    select_mode: bool,
    selected: bool,
    on_cancel: ClickHandler,
    on_select: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let description = dish
        .filter(|d| !d.is_empty())
        .or_else(|| (!order.subtitle.is_empty()).then(|| order.subtitle.clone()))
        .unwrap_or_else(|| order.title.clone());

    let mut card = theme::apply(
        div()
            .flex()
            .items_center()
            .justify_between()
            .p(px(16.))
            .mb(px(8.)),
        &theme.card_surface(),
    )
    .id(SharedString::from(format!("order-{}", order.position_id)));
    if cancelling {
        card = card.opacity(0.6);
    }
    if let (true, Some(on_select)) = (select_mode, on_select) {
        card = card
            .cursor_pointer()
            .on_click(move |_, window, cx| on_select(window, cx));
    }

    let left = div()
        .flex_1()
        .mr(px(12.))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_tertiary)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.primary)
                .child(order.title.clone()),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme.text_primary)
                .child(description),
        );

    let (badge_bg, badge_border, badge_fg, badge_label) = if order.approved {
        (
            if theme.flat {
                theme.success_surface
            } else {
                theme.glass_success
            },
            theme.success,
            theme.success_text,
            "Bestätigt",
        )
    } else {
        (
            if theme.flat {
                theme.warning_surface
            } else {
                theme.glass_warning
            },
            theme.warning,
            theme.warning_text,
            "Ausstehend",
        )
    };
    let mut right = div().flex().flex_col().items_end().gap(px(8.)).child(
        div()
            .px(px(10.))
            .py(px(4.))
            .rounded(px(12.))
            .border_1()
            .border_color(badge_border)
            .bg(badge_bg)
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(badge_fg)
            .child(badge_label),
    );

    if select_mode {
        right = right.child(
            div()
                .w(px(24.))
                .h(px(24.))
                .rounded(px(12.))
                .border_1()
                .border_color(if selected {
                    theme.primary
                } else {
                    theme.border
                })
                .bg(if selected {
                    theme.primary
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.on_primary)
                .child(if selected { "✓" } else { "" }),
        );
    } else if cancelling {
        right = right.child(
            div()
                .w(px(32.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.error, 16.)),
        );
    } else if can_cancel {
        let bg = if theme.flat {
            theme.error_surface
        } else {
            theme.glass_error
        };
        let (fill, border, glyph, button) = if cutoff {
            (
                if theme.flat {
                    theme.surface_variant
                } else {
                    theme.glass_surface_variant
                },
                theme.text_tertiary,
                theme.text_tertiary,
                div().opacity(0.4),
            )
        } else {
            (bg, theme.error, theme.error, div())
        };
        right = right.child(
            button
                .id("order-cancel")
                .w(px(32.))
                .h(px(32.))
                .rounded(px(16.))
                .bg(fill)
                .border_1()
                .border_color(border)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_click(move |_, window, cx| on_cancel(window, cx))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(glyph)
                        .child("✕"),
                ),
        );
    }

    card.child(left).child(right)
}
