//! Bill card — docs/requirements/04-ui-ux.md §4.7.
//!
//! The compact list is one shared card: each two-line row paints the
//! description with the trailing amount, then one metadata line of date, time
//! and source, separated by 1 px hairlines. Desktop keeps its own table row
//! and detail below, so the wide Ledger is unchanged.

use gpui::*;

use crate::components::record_table::{
    record_cell, record_cell_right, record_cell_right_body, record_header_cells,
    record_header_fixed, record_header_flex, record_header_right, record_row_focusable, table_date,
    RowFocus,
};
use crate::navigation::ClickHandler;
use crate::theme::AppTheme;
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

/// The compact receipt row's height: two lines and 12 px vertical padding.
pub const MOBILE_RECEIPT_ROW_HEIGHT: f32 = 72.;
/// The shared receipt card's inner padding.
pub const MOBILE_RECEIPT_CARD_PADDING: f32 = 16.;
/// The 1 px hairline between receipt rows.
pub const MOBILE_RECEIPT_HAIRLINE: f32 = 1.;
/// The row's vertical padding.
pub const MOBILE_RECEIPT_ROW_PAD: f32 = 12.;
/// The gap between the description line and the metadata line.
pub const MOBILE_RECEIPT_LINE_GAP: f32 = 2.;

/// Whether a compact receipt row is an activation target. It never is: a
/// receipt is read-only content (design A12), so the row paints no pointer
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

/// A receipt row's one-line description: the supplied supplier text, otherwise
/// the Kantine bill's own supplied items, otherwise the service name. Nothing
/// is invented and no supplied item is silently dropped.
/// @req REQ-SCR-03
pub fn mobile_receipt_title(entry: &BillingEntry) -> String {
    if !entry.description.is_empty() {
        return entry.description.clone();
    }
    if entry.items.is_empty() || entry.source != EntrySource::Gourmet {
        return bill_source_label(entry.source).to_string();
    }
    entry
        .items
        .iter()
        .map(|line| {
            if line.count > 1 {
                format!("{}x {}", line.count, line.description)
            } else {
                line.description.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The receipt row's second line: the existing de-AT date and time and the
/// sentence-case service name, joined with `·`.
/// @req REQ-SCR-03
pub fn mobile_receipt_meta(entry: &BillingEntry) -> String {
    format!(
        "{} · {} · {}",
        format_date_long(entry.epoch_ms),
        format_time(entry.epoch_ms),
        bill_source_label(entry.source)
    )
}

/// One compact receipt row: the description strong on the left and the amount
/// strong, nowrap and trailing on the first line, then one metadata line with
/// the date, time and source. Read-only, never a button.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub fn bill_row(entry: &BillingEntry, theme: &AppTheme) -> Div {
    let m = &theme.scale;
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(MOBILE_RECEIPT_LINE_GAP))
        .py(px(MOBILE_RECEIPT_ROW_PAD))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(m.space_2))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_ellipsis()
                        .text_size(px(m.body))
                        .line_height(px(m.body_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(mobile_receipt_title(entry)),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .text_size(px(m.body))
                        .line_height(px(m.body_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(format_eur(entry.amount)),
                ),
        )
        .child(
            div()
                .whitespace_nowrap()
                .text_size(px(m.meta))
                .line_height(px(m.meta_line))
                .text_color(theme.text_secondary)
                .child(mobile_receipt_meta(entry)),
        )
        .cursor_default()
}

/// The one shared receipt surface: 12 radius, 1 px hairline and 16 px inner
/// padding around all rows, which are separated by 1 px hairlines instead of
/// one card each.
/// @req REQ-SCR-03 @req REQ-UI-15
pub fn bill_list(entries: &[BillingEntry], theme: &AppTheme) -> AnyElement {
    let mut card = div()
        .w_full()
        .flex()
        .flex_col()
        .p(px(MOBILE_RECEIPT_CARD_PADDING))
        .rounded(px(theme.scale.radius_card))
        .bg(theme.surface)
        .border(px(1.))
        .border_color(theme.border);
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            card = card.child(div().h(px(MOBILE_RECEIPT_HAIRLINE)).bg(theme.border));
        }
        card = card.child(bill_row(entry, theme));
    }
    card.into_any_element()
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
/// passive, trailing, tabular value. Dates are de-AT `dd.MM.yyyy`; the row is
/// one `BODY` date with `META` source beside it, so it stays 40 px high.
/// @req REQ-SCR-03 @req REQ-UI-12
pub fn desktop_bill_row(
    entry: &BillingEntry,
    selected: bool,
    compact: bool,
    on_preview: ClickHandler,
    focus: Option<&RowFocus>,
    theme: &AppTheme,
) -> AnyElement {
    let income = if selected {
        theme.accent
    } else {
        theme.text_primary
    };
    let date = |width: Option<f32>| {
        let cell = div()
            .text_size(px(theme.scale.body))
            .line_height(px(theme.scale.body_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(income)
            .whitespace_nowrap()
            .child(table_date(entry.epoch_ms));
        match width {
            Some(width) => cell.w(px(width)).flex_shrink_0(),
            None => cell,
        }
    };
    let source = record_cell(bill_source_label(entry.source), theme);
    let cells = if compact {
        vec![
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(theme.scale.space_2))
                .child(date(None))
                .child(source)
                .into_any_element(),
            record_cell_right_body(format_eur(entry.amount), bill_amount_width(compact), theme),
        ]
    } else {
        vec![
            date(Some(BILL_DATE_EXPANDED)).into_any_element(),
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .child(source)
                .into_any_element(),
            record_cell_right_body(format_eur(entry.amount), bill_amount_width(compact), theme),
        ]
    };
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
        EntrySource::Gourmet => theme.accent_surface,
        EntrySource::Ventopay => theme.success_surface,
    };
    div()
        .flex()
        .child(
            div()
                .px(px(theme.scale.space_2))
                .py(px(theme.scale.space_1))
                .rounded(px(theme.scale.radius_chip))
                .bg(fill)
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .w(px(BILL_LINE_QTY))
                .flex_shrink_0()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(format!("{}x", line.count)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
        .p(px(if compact {
            theme.scale.space_4
        } else {
            theme.scale.space_5
        }))
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child("Beleg"),
        )
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(format_eur(entry.amount)),
        );

    match entry.source {
        EntrySource::Gourmet => {
            let mut rows = div()
                .flex()
                .flex_col()
                .gap(px(theme.scale.space_2))
                .mt(px(theme.scale.space_2));
            for line in &entry.items {
                rows = rows.child(desktop_line_row(line, theme));
            }
            detail = detail.child(rows);
        }
        EntrySource::Ventopay => {
            if !entry.description.is_empty() {
                detail = detail.child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
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
        bill_amount_width, bill_source_label, mobile_receipt_interaction, mobile_receipt_meta,
        mobile_receipt_title, ReceiptInteraction, BILL_AMOUNT_COMPACT, BILL_AMOUNT_EXPANDED,
        BILL_DATE_EXPANDED, BILL_LINE_AMOUNT, BILL_LINE_QTY, MOBILE_RECEIPT_CARD_PADDING,
        MOBILE_RECEIPT_HAIRLINE, MOBILE_RECEIPT_LINE_GAP, MOBILE_RECEIPT_ROW_HEIGHT,
        MOBILE_RECEIPT_ROW_PAD,
    };
    use crate::theme::AppTheme;
    use snackpilot_model::{format_eur, BillingEntry, BillingLine, EntrySource};

    /// The compact receipt row keeps every supplied Gourmet item in its title,
    /// never invents a Ventopay item, keeps a supplied restaurant and falls
    /// back to the service name instead of an empty row. Amounts use the
    /// incumbent `format_eur`, including negative credits and large values.
    /// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
    #[test]
    fn mobile_receipt_rows_keep_supplied_content_and_remain_read_only() {
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
        let title = mobile_receipt_title(&gourmet);
        assert!(title.contains(LONG), "every supplied Kantine item is kept");
        assert!(
            title.contains("2x "),
            "a multi-count item keeps its supplied quantity"
        );
        assert!(title.contains("Kürbiskernöl"));
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
            format_eur(gourmet.items[0].total),
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
        // becomes the title.
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
        assert_eq!(mobile_receipt_title(&ventopay), "Kaffeeautomat Foyer");
        let without = BillingEntry {
            description: String::new(),
            ..ventopay.clone()
        };
        assert_eq!(
            mobile_receipt_title(&without),
            "Automaten",
            "an absent restaurant falls back to the service name, never a fabricated item"
        );

        // Read-only receipts: neither source is an activation target.
        for source in [EntrySource::Gourmet, EntrySource::Ventopay] {
            assert_eq!(
                mobile_receipt_interaction(source),
                ReceiptInteraction::ReadOnly,
                "a receipt is never a button"
            );
        }
        assert_eq!(
            bill_source_label(EntrySource::Gourmet),
            "Kantine",
            "the receipt meta names the service in sentence case"
        );
    }

    /// One receipt row is two lines: the description strong on the left with
    /// the amount strong and nowrap on the right, then one metadata line that
    /// joins the existing date, time and source strings with `·`. The row and
    /// its shared card stay at or under the density budget.
    /// @req REQ-SCR-03 @req REQ-UI-15
    #[test]
    fn mobile_receipt_rows_join_meta_and_fit_the_density_budget() {
        let entry = BillingEntry {
            id: "v-81".into(),
            epoch_ms: 1_795_596_240_000,
            source: EntrySource::Ventopay,
            description: "Kaffeeautomat".into(),
            amount: -0.5,
            items: Vec::new(),
        };
        assert_eq!(
            mobile_receipt_meta(&entry),
            "Mi., 25. Nov. 2026 · 09:44 · Automaten",
            "the meta line joins date, time and source with the middle dot"
        );
        assert_eq!(mobile_receipt_meta(&entry).matches('·').count(), 2);
        assert_eq!(mobile_receipt_title(&entry), "Kaffeeautomat");
        assert_eq!(
            mobile_receipt_title(&BillingEntry {
                description: String::new(),
                ..entry.clone()
            }),
            "Automaten",
            "an empty supplier description falls back to the service name, never an empty row"
        );

        assert_eq!(MOBILE_RECEIPT_ROW_HEIGHT, 72.);
        assert_eq!(MOBILE_RECEIPT_CARD_PADDING, 16.);
        assert_eq!(MOBILE_RECEIPT_HAIRLINE, 1.);
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.radius_card, 12.);
        assert_eq!(
            MOBILE_RECEIPT_ROW_HEIGHT,
            theme.scale.body_line
                + MOBILE_RECEIPT_LINE_GAP
                + theme.scale.meta_line
                + MOBILE_RECEIPT_ROW_PAD * 2.
        );
    }

    /// A Kantine receipt has no supplier description; its own supplied item
    /// lines become the one-line title instead of disappearing.
    /// @req REQ-SCR-03
    #[test]
    fn mobile_receipt_title_keeps_kantine_items() {
        let entry = BillingEntry {
            id: "g-9".into(),
            epoch_ms: 1_795_596_240_000,
            source: EntrySource::Gourmet,
            description: String::new(),
            amount: -6.4,
            items: vec![
                BillingLine {
                    count: 2,
                    description: "Rindsgulasch".into(),
                    total: 6.4,
                },
                BillingLine {
                    count: 1,
                    description: "Kürbiskernöl".into(),
                    total: 0.5,
                },
            ],
        };
        assert_eq!(
            mobile_receipt_title(&entry),
            "2x Rindsgulasch, Kürbiskernöl",
            "the supplied items stay visible in the row title"
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
