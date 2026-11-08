//! Bill card — docs/requirements/04-ui-ux.md §4.7.
//!
//! Header with date/time and the source badge plus amount; Gourmet cards list
//! their item lines, Ventopay cards show the optional restaurant line only.

use gpui::*;

use crate::theme::{self, AppTheme};
use snackpilot_model::format_time;
use snackpilot_model::{format_date_long, format_eur, BillingEntry, BillingLine, EntrySource};

fn source_badge(source: EntrySource, theme: &AppTheme) -> Div {
    let (label, bg) = match source {
        EntrySource::Gourmet => (
            "KANTINE",
            if theme.flat {
                theme.primary_surface
            } else {
                theme.glass_primary
            },
        ),
        EntrySource::Ventopay => (
            "AUTOMATEN",
            if theme.flat {
                theme.success_surface
            } else {
                theme.glass_success
            },
        ),
    };
    div()
        .px(px(8.))
        .py(px(2.))
        .rounded(px(8.))
        .bg(bg)
        .text_size(px(10.))
        .font_weight(FontWeight::BOLD)
        .text_color(theme.text_secondary)
        .child(label)
}

fn item_row(line: &BillingLine, theme: &AppTheme) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(
            div()
                .w(px(28.))
                .text_size(px(13.))
                .text_color(theme.text_tertiary)
                .child(format!("{}x", line.count)),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(14.))
                .text_color(theme.text_secondary)
                .child(line.description.clone()),
        )
        .child(
            div()
                .ml(px(8.))
                .text_size(px(13.))
                .text_color(theme.text_tertiary)
                .child(format_eur(line.total)),
        )
}

/// One §4.7 bill card.
pub fn bill_card(entry: &BillingEntry, theme: &AppTheme) -> Div {
    let header = div()
        .flex()
        .items_start()
        .justify_between()
        .mb(px(12.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(format_date_long(entry.epoch_ms)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.text_tertiary)
                        .child(format_time(entry.epoch_ms)),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(4.))
                .child(source_badge(entry.source, theme))
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.primary)
                        .child(format_eur(entry.amount)),
                ),
        );

    let mut card = theme::apply(
        div().flex().flex_col().p(px(16.)).mb(px(8.)),
        &theme.card_surface(),
    )
    .child(header);

    match entry.source {
        EntrySource::Gourmet => {
            let mut rows = div().flex().flex_col().gap(px(6.));
            for line in &entry.items {
                rows = rows.child(item_row(line, theme));
            }
            card = card.child(rows);
        }
        EntrySource::Ventopay => {
            if !entry.description.is_empty() {
                card = card.child(
                    div()
                        .text_size(px(13.))
                        .text_color(theme.text_secondary)
                        .child(entry.description.clone()),
                );
            }
        }
    }
    card
}
