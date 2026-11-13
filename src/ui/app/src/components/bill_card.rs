//! Bill card — docs/requirements/04-ui-ux.md §4.7.
//!
//! The compact card paints a date/time header with the neutral source metadata
//! and the trailing amount, then the Gourmet lines or the optional Ventopay
//! restaurant line. Desktop keeps its own table row and detail below, so the
//! wide Ledger is unchanged.

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

/// The compact receipt's source words: the incumbent uppercase badge copy,
/// now painted as neutral metadata instead of a tinted service badge.
/// @req REQ-SCR-03 @req REQ-UI-15
pub fn mobile_source_label(source: EntrySource) -> &'static str {
    match source {
        EntrySource::Gourmet => "KANTINE",
        EntrySource::Ventopay => "AUTOMATEN",
    }
}

/// Whether a compact receipt card is an activation target. It never is: a
/// receipt is read-only content (design A12), so the card paints no pointer
/// affordance, no tab stop and no handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptInteraction {
    ReadOnly,
}

/// @req REQ-SCR-03 @req REQ-UI-16
pub fn mobile_receipt_interaction(source: EntrySource) -> ReceiptInteraction {
    match source {
        EntrySource::Gourmet | EntrySource::Ventopay => ReceiptInteraction::ReadOnly,
    }
}

/// The lines a compact receipt paints: the Kantine bill's own items, and none
/// for Automaten (a transaction carries no invented item list).
/// @req REQ-SCR-03
pub fn mobile_receipt_lines(entry: &BillingEntry) -> &[BillingLine] {
    match entry.source {
        EntrySource::Gourmet => &entry.items,
        EntrySource::Ventopay => &[],
    }
}

/// The optional Automaten restaurant line: only the supplied, non-empty text.
/// @req REQ-SCR-03
pub fn mobile_restaurant_line(entry: &BillingEntry) -> Option<&str> {
    match entry.source {
        EntrySource::Gourmet => None,
        EntrySource::Ventopay => {
            (!entry.description.is_empty()).then_some(entry.description.as_str())
        }
    }
}

/// One compact Gourmet receipt line: the `{count}x` quantity in its 28 px
/// metadata column, the full wrapping description in the body role and the
/// trailing formatted line amount in metadata.
fn mobile_line_row(line: &BillingLine, ink: Hsla, meta: Hsla) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .child(
            div()
                .w(px(BILL_LINE_QTY))
                .flex_shrink_0()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(meta)
                .child(format!("{}x", line.count)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(16.))
                .line_height(px(24.))
                .text_color(ink)
                .child(line.description.clone()),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(meta)
                .child(format_eur(line.total)),
        )
}

/// One compact §4.7 receipt card (mobile task 1.5): date in the body role,
/// time/source in readable metadata, the amount trailing in the body role,
/// then — after a neutral hairline — either the Kantine lines or the supplied
/// Automaten restaurant. Padding16, gap8, `card_surface`; heights follow
/// content and the card is never a button.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub fn bill_card(entry: &BillingEntry, theme: &AppTheme) -> Div {
    let m = &theme.mobile;
    let card_fill = theme.mobile_card_fill();
    let meta = theme.mobile_secondary_on(card_fill);
    let amount = format_eur(entry.amount);

    let header = div()
        .flex()
        .items_start()
        .justify_between()
        .gap(px(m.space_2))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(m.space_1))
                .child(
                    div()
                        .text_size(px(m.body))
                        .line_height(px(m.body_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(format_date_long(entry.epoch_ms)),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(m.space_2))
                        .child(
                            div()
                                .text_size(px(m.meta))
                                .line_height(px(m.meta_line))
                                .text_color(meta)
                                .child(format_time(entry.epoch_ms)),
                        )
                        .child(
                            div()
                                .text_size(px(m.meta))
                                .line_height(px(m.meta_line))
                                .text_color(meta)
                                .child(mobile_source_label(entry.source)),
                        ),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(m.body))
                .line_height(px(m.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .whitespace_nowrap()
                .child(amount),
        );

    let mut card = theme::apply(
        div()
            .flex()
            .flex_col()
            .gap(px(m.space_2))
            .p(px(m.space_4))
            .mb(px(m.space_2)),
        &theme.card_surface(),
    )
    .child(header);

    let lines = mobile_receipt_lines(entry);
    if !lines.is_empty() {
        card = card
            .child(div().h(px(1.)).w_full().bg(theme.divider()))
            .child(
                div().flex().flex_col().gap(px(m.space_2)).children(
                    lines
                        .iter()
                        .map(|line| mobile_line_row(line, theme.text_primary, meta)),
                ),
            );
    }
    if let Some(restaurant) = mobile_restaurant_line(entry) {
        card = card
            .child(div().h(px(1.)).w_full().bg(theme.divider()))
            .child(
                div()
                    .text_size(px(m.body))
                    .line_height(px(m.body_line))
                    .text_color(theme.text_primary)
                    .child(restaurant.to_string()),
            );
    }

    // Read-only content: no pointer affordance on either source.
    match mobile_receipt_interaction(entry.source) {
        ReceiptInteraction::ReadOnly => card.cursor_default(),
    }
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
        bill_amount_width, bill_source_label, mobile_receipt_interaction, mobile_receipt_lines,
        mobile_restaurant_line, mobile_source_label, ReceiptInteraction, BILL_AMOUNT_COMPACT,
        BILL_AMOUNT_EXPANDED, BILL_DATE_EXPANDED, BILL_LINE_AMOUNT, BILL_LINE_QTY,
    };
    use snackpilot_model::{format_eur, BillingEntry, BillingLine, EntrySource};

    /// The compact receipt (task 1.5) renders every supplied Gourmet line in
    /// full, never invents a Ventopay item, shows the existing optional
    /// restaurant only when it is non-empty, and is read-only for both
    /// sources with the neutral uppercase source words. Amounts use the
    /// incumbent `format_eur`, including negative credits and large values.
    /// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
    #[test]
    fn mobile_receipt_lines_wrap_and_remain_read_only() {
        const LONG: &str =
            "Rindsgulasch mit Serviettenknödel, Rahmsauce und Blattsalat mit Kürbiskernöl";
        let gourmet = BillingEntry {
            id: "g-1001".into(),
            epoch_ms: 1_794_308_400_000,
            source: EntrySource::Gourmet,
            description: String::new(),
            amount: -6.4,
            items: vec![
                BillingLine {
                    count: 2,
                    description: LONG.into(),
                    total: 6.4,
                },
                BillingLine {
                    count: 1,
                    description: "Kürbiskernöl".into(),
                    total: 0.5,
                },
            ],
        };
        let lines = mobile_receipt_lines(&gourmet);
        assert_eq!(lines.len(), 2, "every supplied Kantine line is shown");
        assert_eq!(
            lines[0].description, LONG,
            "the description is rendered in full, never truncated or replaced"
        );
        assert_eq!(
            BILL_LINE_QTY, 28.,
            "the quantity stays in its 28 px metadata column"
        );
        assert_eq!(
            format_eur(gourmet.amount),
            "-€ 6,40",
            "credit keeps its sign"
        );
        assert_eq!(
            format_eur(lines[0].total),
            "€ 6,40",
            "a line amount uses the incumbent formatter"
        );
        assert_eq!(
            format_eur(1_234_567.89),
            "€ 1.234.567,89",
            "a large amount keeps the pinned thousands grouping"
        );

        // Ventopay carries transactions, not items: nothing is invented even
        // if a line were present, and only a supplied non-empty restaurant
        // becomes a line.
        let ventopay = BillingEntry {
            id: "v-77".into(),
            epoch_ms: 1_794_222_000_000,
            source: EntrySource::Ventopay,
            description: "Kaffeeautomat Foyer".into(),
            amount: -1.2,
            items: vec![BillingLine {
                count: 1,
                description: "invented".into(),
                total: 1.2,
            }],
        };
        assert!(
            mobile_receipt_lines(&ventopay).is_empty(),
            "no receipt item is fabricated for Automaten"
        );
        assert_eq!(
            mobile_restaurant_line(&ventopay),
            Some("Kaffeeautomat Foyer"),
            "the supplied restaurant line is kept"
        );
        let without = BillingEntry {
            description: String::new(),
            ..ventopay.clone()
        };
        assert_eq!(
            mobile_restaurant_line(&without),
            None,
            "an absent restaurant paints no empty line"
        );

        // Read-only receipts: neither source is an activation target, and the
        // source is neutral uppercase metadata, not a success-tinted badge.
        for source in [EntrySource::Gourmet, EntrySource::Ventopay] {
            assert_eq!(
                mobile_receipt_interaction(source),
                ReceiptInteraction::ReadOnly,
                "a receipt is never a button"
            );
        }
        assert_eq!(mobile_source_label(EntrySource::Gourmet), "KANTINE");
        assert_eq!(mobile_source_label(EntrySource::Ventopay), "AUTOMATEN");
        assert_eq!(
            bill_source_label(EntrySource::Gourmet),
            "Kantine",
            "the desktop detail keeps its sentence-case service name"
        );
    }

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
