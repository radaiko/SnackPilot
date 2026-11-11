//! Bill card — docs/requirements/04-ui-ux.md §4.7.
//!
//! Header with date/time and the source badge plus amount; Gourmet cards list
//! their item lines, Ventopay cards show the optional restaurant line only.

use gpui::*;

use crate::components::record_table::{
    record_cell_right, record_header_cells, record_header_fixed, record_header_flex,
    record_header_right, record_row_focusable, table_date, RowFocus,
};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::format_time;
use snackpilot_model::{format_date_long, format_eur, BillingEntry, BillingLine, EntrySource};

/// Compact amount column (brief `desktop.billAmountWidth`: 80 px).
pub const BILL_AMOUNT_COMPACT: f32 = 80.;
/// Expanded amount column (brief `desktop.billAmountWidth`: 96 px).
pub const BILL_AMOUNT_EXPANDED: f32 = 96.;
/// Expanded date column (brief `desktop.billDateExpanded`: 112 px).
pub const BILL_DATE_EXPANDED: f32 = 112.;
/// Gourmet line quantity column (brief "Receipt detail": 28 px).
pub const BILL_LINE_QTY: f32 = 28.;
/// Gourmet line total column (brief "Receipt detail": 80 px).
pub const BILL_LINE_AMOUNT: f32 = 80.;

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

/// The amount column width of the table density (brief columns).
pub fn bill_amount_width(compact: bool) -> f32 {
    if compact {
        BILL_AMOUNT_COMPACT
    } else {
        BILL_AMOUNT_EXPANDED
    }
}

/// The desktop source words of the brief's receipt detail/table: the service
/// name, not the mobile badge caps.
pub fn bill_source_label(source: EntrySource) -> &'static str {
    match source {
        EntrySource::Gourmet => "Kantine",
        EntrySource::Ventopay => "Automaten",
    }
}

/// The desktop receipt table header. Compact is `Datum / Quelle` (flex) plus
/// the trailing amount; expanded splits date 112, source flex, amount 96.
pub fn desktop_bill_header(compact: bool, theme: &AppTheme) -> AnyElement {
    let mut cells = Vec::new();
    if compact {
        cells.push(record_header_flex("Datum / Quelle", theme));
    } else {
        cells.push(record_header_fixed("Datum", BILL_DATE_EXPANDED, theme));
        cells.push(record_header_flex("Quelle", theme));
    }
    cells.push(record_header_right(
        "Betrag",
        bill_amount_width(compact),
        theme,
    ));
    record_header_cells(cells, theme)
}

/// One receipt table row: the body previews a receipt, the amount stays a
/// passive, trailing, tabular value. Dates are de-AT `dd.MM.yyyy`.
/// @req REQ-SCR-03 @req REQ-UI-12
pub fn desktop_bill_row(
    entry: &BillingEntry,
    selected: bool,
    compact: bool,
    on_preview: ClickHandler,
    focus: Option<&RowFocus>,
    theme: &AppTheme,
) -> AnyElement {
    let date = div()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(theme.text_primary)
        .whitespace_nowrap()
        .child(table_date(entry.epoch_ms));
    let time = div()
        .text_size(px(12.))
        .line_height(px(16.))
        .text_color(theme.text_secondary)
        .whitespace_nowrap()
        .child(format_time(entry.epoch_ms));
    let mut cells = Vec::new();
    if compact {
        cells.push(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .justify_center()
                .child(date)
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(theme.text_secondary)
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(bill_source_label(entry.source)),
                )
                .into_any_element(),
        );
    } else {
        cells.push(
            div()
                .w(px(BILL_DATE_EXPANDED))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .justify_center()
                .child(date)
                .child(time)
                .into_any_element(),
        );
        cells.push(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .text_ellipsis()
                .whitespace_nowrap()
                .child(bill_source_label(entry.source))
                .into_any_element(),
        );
    }
    cells.push(record_cell_right(
        format_eur(entry.amount),
        bill_amount_width(compact),
        theme,
    ));
    record_row_focusable(
        SharedString::from(format!("desktop-bill-{}", entry.id)),
        cells,
        selected,
        Some(on_preview),
        false,
        None,
        focus,
        theme,
    )
}

fn desktop_source_badge(source: EntrySource, theme: &AppTheme) -> AnyElement {
    let fill = match source {
        EntrySource::Gourmet => theme.primary_surface,
        EntrySource::Ventopay => theme.success_surface,
    };
    div()
        .flex()
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(8.))
                .bg(fill)
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(bill_source_label(source)),
        )
        .into_any_element()
}

/// One Gourmet receipt line: `{count}x`, the full wrapping description and
/// the trailing formatted total. At the compact detail width the description
/// wraps instead of truncating (brief "Abrechnung").
fn desktop_line_row(line: &BillingLine, theme: &AppTheme) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_start()
        .gap(px(8.))
        .child(
            div()
                .w(px(BILL_LINE_QTY))
                .flex_shrink_0()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .child(format!("{}x", line.count)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .child(line.description.clone()),
        )
        .child(record_cell_right(
            format_eur(line.total),
            BILL_LINE_AMOUNT,
            theme,
        ))
        .into_any_element()
}

/// One receipt detail (brief "Abrechnung"): `Beleg`, long de-AT date and
/// time, source, amount and the source's own content. No receipt number,
/// location, tax or per-receipt subsidy is invented from unavailable fields.
/// @req REQ-SCR-03 @req REQ-UI-08
pub fn desktop_bill_detail(entry: &BillingEntry, compact: bool, theme: &AppTheme) -> AnyElement {
    let mut detail = div()
        .p(px(if compact { 16. } else { 24. }))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.desktop.ink)
                .child("Beleg"),
        )
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format!(
                    "{} · {}",
                    format_date_long(entry.epoch_ms),
                    format_time(entry.epoch_ms)
                )),
        )
        .child(desktop_source_badge(entry.source, theme))
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.desktop.ink)
                .child(format_eur(entry.amount)),
        );

    match entry.source {
        EntrySource::Gourmet => {
            let mut rows = div().flex().flex_col().gap(px(8.)).mt(px(8.));
            for line in &entry.items {
                rows = rows.child(desktop_line_row(line, theme));
            }
            detail = detail.child(rows);
        }
        EntrySource::Ventopay => {
            if !entry.description.is_empty() {
                detail = detail.child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(theme.text_secondary)
                        .child(entry.description.clone()),
                );
            }
        }
    }
    detail.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        bill_amount_width, bill_source_label, BILL_AMOUNT_COMPACT, BILL_AMOUNT_EXPANDED,
        BILL_DATE_EXPANDED, BILL_LINE_AMOUNT, BILL_LINE_QTY,
    };
    use snackpilot_model::EntrySource;

    /// The two table densities use the brief's exact receipt columns.
    /// @req REQ-SCR-03
    #[test]
    fn bill_columns_match_the_brief() {
        assert_eq!(bill_amount_width(true), 80.);
        assert_eq!(bill_amount_width(false), 96.);
        assert_eq!(BILL_AMOUNT_COMPACT, 80.);
        assert_eq!(BILL_AMOUNT_EXPANDED, 96.);
        assert_eq!(BILL_DATE_EXPANDED, 112.);
        assert_eq!(BILL_LINE_QTY, 28.);
        assert_eq!(BILL_LINE_AMOUNT, 80.);
    }

    /// The desktop receipt shows the service names of the brief, never the
    /// mobile badge caps.
    /// @req REQ-SCR-03
    #[test]
    fn bill_source_labels_match_the_brief() {
        assert_eq!(bill_source_label(EntrySource::Gourmet), "Kantine");
        assert_eq!(bill_source_label(EntrySource::Ventopay), "Automaten");
    }
}
