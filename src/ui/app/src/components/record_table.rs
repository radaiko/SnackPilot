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
use crate::theme::AppTheme;

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
    // The accent rule is an always-present 3 px child so selecting a row
    // never changes its width; unselected rows keep it transparent. gpui
    // has one border colour per element, so the rule cannot be a border.
    let row = div()
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
        );
    let row = match on_click {
        Some(on_click) => row
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx)),
        None => row,
    };
    row.into_any_element()
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
