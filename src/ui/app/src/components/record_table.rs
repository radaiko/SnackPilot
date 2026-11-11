//! Desktop record table (desktop-ui-redesign brief "Record table"): the
//! read-only list rows of the workspace master pane. Header 36 px on
//! `background`, rows on `surface` at 56 px minimum with a 1 px bottom
//! border, no zebra and no shadows; hover `surface_variant`; the selected
//! row takes a `primary_surface` tint and a 3 px inset accent rule in the
//! resolved `accent_boundary` ink (3:1 on that tint, like the card edge and
//! chip border) that never changes the row width (the rule is always there,
//! transparent when unselected). Cells are read-only: no pointer cursor, and
//! the row is the single interaction target.
//! @req REQ-UI-07 @req REQ-UI-12

use gpui::*;

use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// Header row height (brief: 36 px).
const HEADER_HEIGHT: f32 = 36.;
/// Row minimum height: the 44 px target floor rounded up to the brief's
/// 56 px list row.
const ROW_HEIGHT: f32 = 56.;
/// Horizontal cell padding (brief: 12 px).
const CELL_PAD: f32 = 12.;
/// Gap between cells (brief: 12 px).
const CELL_GAP: f32 = 12.;
/// Selected-row inset accent rule (brief: 3 px).
const SELECT_RULE: f32 = 3.;
/// Batch-check hit column (brief `desktop.batchCheckColumn`: 44 px).
pub const CHECK_COLUMN: f32 = 44.;
/// Drawn batch checkbox (brief "Multi-selection": 20×20).
const CHECK_BOX: f32 = 20.;

/// The column header row: 36 px, `background` fill, 12 px/600 labels.
pub fn record_header(columns: Vec<&'static str>, theme: &AppTheme) -> AnyElement {
    div()
        .h(px(HEADER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        .px(px(CELL_PAD))
        .gap(px(CELL_GAP))
        .bg(theme.background)
        .children(columns.into_iter().map(|label| {
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .whitespace_nowrap()
                .text_ellipsis()
                .child(label)
        }))
        .into_any_element()
}

/// One record row. `id` is the row's stable identity (the record key, never
/// the index); `selected` paints the tint and the inset accent rule;
/// `on_click` is the row's only interaction (cells stay read-only).
pub fn record_row(
    id: impl Into<ElementId>,
    cells: Vec<AnyElement>,
    selected: bool,
    on_click: Option<ClickHandler>,
    theme: &AppTheme,
) -> AnyElement {
    match on_click {
        Some(on_click) => record_row_body(id, cells, selected, theme)
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx)),
        None => record_row_body(id, cells, selected, theme),
    }
    .into_any_element()
}

/// Whether Space on a focused row toggles its batch checkbox: only while the
/// selection mode is active, so Space in plain preview mode does nothing
/// (brief "Multi-selection").
pub fn row_space_toggles(key: &str, selection_mode: bool) -> bool {
    selection_mode && key == "space"
}

/// Focusable variant of [`record_row`]: the same row chrome plus a tab stop,
/// the desktop focus ring and, in selection mode, Space-to-toggle. `on_space`
/// is passed only while batch selection runs.
pub fn record_row_focusable(
    id: impl Into<ElementId>,
    cells: Vec<AnyElement>,
    selected: bool,
    on_click: Option<ClickHandler>,
    selection_mode: bool,
    on_space: Option<ClickHandler>,
    theme: &AppTheme,
) -> AnyElement {
    let mut row = match on_click {
        Some(on_click) => record_row_body(id, cells, selected, theme)
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx)),
        None => record_row_body(id, cells, selected, theme),
    }
    .tab_index(0)
    .focus(theme::desktop_focus(theme.desktop.focus));
    if let Some(on_space) = on_space {
        row = row.on_key_down(move |event: &KeyDownEvent, window, cx| {
            if row_space_toggles(event.keystroke.key.as_str(), selection_mode) {
                on_space(window, cx);
                // The row consumed its one Space activation: no ancestor may
                // toggle the same batch ID a second time.
                cx.stop_propagation();
            }
        });
    }
    row.into_any_element()
}

/// The shared row chrome behind both row entry points.
fn record_row_body(
    id: impl Into<ElementId>,
    cells: Vec<AnyElement>,
    selected: bool,
    theme: &AppTheme,
) -> Stateful<Div> {
    // The accent rule is an always-present 3 px child so selecting a row
    // never changes its width; unselected rows keep it transparent. gpui
    // has one border colour per element, so the rule cannot be a border.
    div()
        .id(id)
        .min_h(px(ROW_HEIGHT))
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
        .child(
            div()
                .w(px(SELECT_RULE))
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
                .min_h(px(ROW_HEIGHT))
                .flex()
                .items_center()
                .px(px(CELL_PAD))
                .gap(px(CELL_GAP))
                .children(cells),
        )
}

/// A read-only record cell: text at the body size, no cursor, truncates.
pub fn record_cell(text: impl Into<SharedString>, theme: &AppTheme) -> AnyElement {
    div()
        .text_size(px(14.))
        .line_height(px(20.))
        .text_color(theme.text_primary)
        .text_ellipsis()
        .whitespace_nowrap()
        .child(text.into())
        .into_any_element()
}

/// Width-aware header row: same 36 px chrome as `record_header`, but the
/// cells come in pre-sized by the column helpers below.
pub fn record_header_cells(cells: Vec<AnyElement>, theme: &AppTheme) -> AnyElement {
    div()
        .h(px(HEADER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        // The inset accent rule of a body row is always present, so the
        // header keeps the same leading offset to stay column-aligned.
        .pl(px(CELL_PAD + SELECT_RULE))
        .pr(px(CELL_PAD))
        .gap(px(CELL_GAP))
        .bg(theme.background)
        .children(cells)
        .into_any_element()
}

fn record_header_label(text: impl Into<SharedString>, theme: &AppTheme) -> AnyElement {
    div()
        .text_size(px(12.))
        .line_height(px(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_secondary)
        .whitespace_nowrap()
        .text_ellipsis()
        .child(text.into())
        .into_any_element()
}

/// A header cell at a fixed column width.
pub fn record_header_fixed(
    text: impl Into<SharedString>,
    width: f32,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .child(record_header_label(text, theme))
        .into_any_element()
}

/// A header cell that fills the remaining table width.
pub fn record_header_flex(text: impl Into<SharedString>, theme: &AppTheme) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .child(record_header_label(text, theme))
        .into_any_element()
}

/// A fixed-width header cell whose label is end-aligned (amount columns).
pub fn record_header_right(
    text: impl Into<SharedString>,
    width: f32,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .text_right()
        .child(record_header_label(text, theme))
        .into_any_element()
}

/// A body cell at a fixed column width.
pub fn record_cell_fixed(
    text: impl Into<SharedString>,
    width: f32,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .child(record_cell(text, theme))
        .into_any_element()
}

/// A body cell that fills the remaining table width.
pub fn record_cell_flex(text: impl Into<SharedString>, theme: &AppTheme) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .child(record_cell(text, theme))
        .into_any_element()
}

/// A fixed-width, end-aligned, tabular body cell (amount columns).
pub fn record_cell_right(
    text: impl Into<SharedString>,
    width: f32,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .text_right()
        .font_features(FontFeatures(std::sync::Arc::new(vec![(
            "tnum".to_string(),
            1,
        )])))
        .child(record_cell(text, theme))
        .into_any_element()
}

/// An empty header cell occupying the batch-check column, so the header and
/// the body rows stay column-aligned while selection mode is active.
pub fn record_header_blank(width: f32) -> AnyElement {
    div().w(px(width)).flex_shrink_0().into_any_element()
}

/// The 44 px batch-check hit column of a row in selection mode: a 20×20 box
/// that toggles exactly this record's ID. The click stops propagating, so
/// checking a row never also previews it (brief "Multi-selection").
pub fn record_check_cell(
    id: impl Into<ElementId>,
    checked: bool,
    on_toggle: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let action = theme.desktop.action;
    div()
        .id(id)
        .w(px(CHECK_COLUMN))
        .flex_shrink_0()
        .self_stretch()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface_variant))
        .on_click(move |_, window, cx| {
            on_toggle(window, cx);
            cx.stop_propagation();
        })
        .child(
            div()
                .w(px(CHECK_BOX))
                .h(px(CHECK_BOX))
                .rounded(px(4.))
                .border_1()
                .border_color(if checked { theme.primary } else { theme.border })
                .bg(if checked { action.fill } else { theme.surface })
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .text_color(action.ink)
                .child(if checked { "✓" } else { "" }),
        )
        .into_any_element()
}

/// A table date: de-AT `dd.MM.yyyy` from an epoch, empty when unformattable.
pub fn table_date(epoch_ms: i64) -> String {
    let key = snackpilot_model::day_key_from_epoch_ms(epoch_ms);
    let mut parts = key.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day)) => format!("{day}.{month}.{year}"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{row_space_toggles, table_date};

    // 2026-11-10 12:00 Europe/Vienna.
    /// @req REQ-SCR-03
    #[test]
    fn table_date_formats_de_at_day() {
        assert_eq!(table_date(1_794_308_400_000), "10.11.2026");
    }

    /// @req REQ-SCR-02
    #[test]
    fn row_space_toggles_only_in_selection_mode() {
        assert!(row_space_toggles("space", true));
        assert!(!row_space_toggles("space", false));
        assert!(!row_space_toggles("enter", true));
    }
}
