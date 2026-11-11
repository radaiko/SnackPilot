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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

/// One row's roving focus inside a wide record list: the programmatic handle
/// (rows are not tab stops) plus the scroll anchor that reveals the row when
/// the keyboard cursor lands on it.
#[derive(Clone)]
pub struct RowFocus {
    pub handle: FocusHandle,
    pub anchor: ScrollAnchor,
}

/// Where Enter on a focused wide record row hands focus (REQ-UI-11): the
/// detail's first enabled control when the previewed record offers one, and
/// the detail's read-only scroll region when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailEntry {
    Control,
    Region,
}

pub fn detail_entry(has_enabled_action: bool) -> DetailEntry {
    if has_enabled_action {
        DetailEntry::Control
    } else {
        DetailEntry::Region
    }
}

/// Roving focus owner for one wide record list (REQ-UI-11): the list container
/// is the single tab stop, every row is focused programmatically by its stable
/// identity, and Up/Down/Home/End move that cursor without wrapping. Handles
/// are kept per identity across repaints, so focus survives a data change.
#[derive(Clone)]
pub struct RecordListFocus {
    container: FocusHandle,
    detail: FocusHandle,
    scroll: ScrollHandle,
    rows: Rc<RefCell<HashMap<String, RowFocus>>>,
}

impl RecordListFocus {
    /// `detail` is the pane's read-only focus target: the region Enter lands
    /// on when the previewed record has no enabled action.
    pub fn new(container: FocusHandle, detail: FocusHandle) -> Self {
        Self {
            container,
            detail,
            scroll: ScrollHandle::new(),
            rows: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// The list's single tab stop.
    pub fn container(&self) -> &FocusHandle {
        &self.container
    }

    /// The detail pane's read-only scroll region.
    pub fn detail(&self) -> &FocusHandle {
        &self.detail
    }

    /// Enter leaves the list for the detail (REQ-UI-11): the pane's next tab
    /// stop is its first control when the preview offers an enabled action,
    /// otherwise the read-only region takes the focus itself so the user can
    /// scroll the details.
    pub fn enter(&self, target: DetailEntry, window: &mut Window, cx: &mut App) {
        match target {
            DetailEntry::Control => window.focus_next(cx),
            DetailEntry::Region => window.focus(&self.detail, cx),
        }
    }

    /// The master pane's scroll handle, so a headed row can be revealed.
    pub fn scroll(&self) -> ScrollHandle {
        self.scroll.clone()
    }

    /// The handle and anchor of one row, created once per identity and reused
    /// across repaints so focus never lands on a fresh element.
    pub fn row<T>(&self, id: &str, cx: &mut Context<T>) -> RowFocus {
        if let Some(row) = self.rows.borrow().get(id) {
            return row.clone();
        }
        let row = RowFocus {
            handle: cx.focus_handle(),
            anchor: ScrollAnchor::for_handle(self.scroll.clone()),
        };
        self.rows.borrow_mut().insert(id.to_string(), row.clone());
        row
    }

    /// Whether this list renders a row for `id` right now.
    pub fn has_row(&self, id: &str) -> bool {
        self.rows.borrow().contains_key(id)
    }

    /// The identities this list renders right now, i.e. the handle set the
    /// last [`Self::retain`] left behind.
    pub fn rendered_ids(&self) -> Vec<String> {
        self.rows.borrow().keys().cloned().collect()
    }

    /// Keep only the handles of the rows this render shows. A refresh, filter
    /// or segment change re-renders a different set, so an identity that
    /// unmounted must not stay focusable or count as a restore target
    /// (REQ-UI-11/14).
    pub fn retain(&self, ids: &[String]) {
        retain_rendered(&mut self.rows.borrow_mut(), ids);
    }

    /// The identity whose row currently owns focus, if any.
    pub fn focused(&self, window: &Window) -> Option<String> {
        self.rows
            .borrow()
            .iter()
            .find_map(|(id, row)| row.handle.is_focused(window).then(|| id.clone()))
    }

    /// Focus one row by identity and reveal it. False when that identity has
    /// no row in this list (a deleted record, or another tab's list).
    pub fn focus(&self, id: &str, window: &mut Window, cx: &mut App) -> bool {
        let row = self.rows.borrow().get(id).cloned();
        let Some(row) = row else {
            return false;
        };
        window.focus(&row.handle, cx);
        row.anchor.scroll_to(window, cx);
        true
    }

    /// Move the roving cursor over the visible row order `ids` for `key` and
    /// return the identity that now owns focus.
    pub fn step(
        &self,
        ids: &[String],
        key: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let current = self
            .focused(window)
            .and_then(|id| ids.iter().position(|visible| *visible == id));
        let next = roving_index(key, current, ids.len())?;
        let id = ids.get(next)?.clone();
        self.focus(&id, window, cx).then_some(id)
    }
}

/// Drop the handles of every identity the render no longer shows: the
/// surviving set is exactly what [`RecordListFocus::retain`] was given.
fn retain_rendered<V>(rows: &mut HashMap<String, V>, ids: &[String]) {
    rows.retain(|id, _| ids.iter().any(|visible| visible == id));
}

/// Roving movement for a focused wide record list (brief keyboard map):
/// Down/Up move one record, Home/End jump to the ends, nothing wraps, and any
/// other key leaves the list to the focused element. `current` is the focused
/// row, or `None` while the list container still owns focus; from no cursor
/// Down selects the first record and Up the last.
pub fn roving_index(key: &str, current: Option<usize>, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    match (key, current) {
        ("down", Some(index)) => (index + 1 < count).then_some(index + 1),
        ("down", None) => Some(0),
        ("up", Some(index)) => index.checked_sub(1),
        ("up", None) => Some(count - 1),
        ("home", _) => Some(0),
        ("end", _) => Some(count - 1),
        _ => None,
    }
}

/// Focusable variant of [`record_row`]: the same row chrome, the desktop focus
/// ring and, in selection mode, Space-to-toggle. `on_space` is passed only
/// while batch selection runs. Inside a [`RecordListFocus`] list the caller
/// passes that row's `focus` (`Some`): the row is then focused
/// programmatically by the list's roving cursor and is NOT a tab stop, so the
/// whole list stays one stop. Without a list (gallery, a standalone row) the
/// row keeps its own tab stop.
#[allow(clippy::too_many_arguments)]
pub fn record_row_focusable(
    id: impl Into<ElementId>,
    cells: Vec<AnyElement>,
    selected: bool,
    on_click: Option<ClickHandler>,
    selection_mode: bool,
    on_space: Option<ClickHandler>,
    focus: Option<&RowFocus>,
    theme: &AppTheme,
) -> AnyElement {
    let mut row = match on_click {
        Some(on_click) => record_row_body(id, cells, selected, theme)
            .cursor_pointer()
            .on_click(move |_, window, cx| on_click(window, cx)),
        None => record_row_body(id, cells, selected, theme),
    }
    .focus(theme::desktop_focus(theme.desktop.focus));
    row = match focus {
        Some(focus) => row
            .track_focus(&focus.handle)
            .anchor_scroll(Some(focus.anchor.clone())),
        None => row.tab_index(0),
    };
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
    use super::{
        detail_entry, retain_rendered, roving_index, row_space_toggles, table_date, DetailEntry,
    };
    use std::collections::HashMap;

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

    /// Up/Down move one record, Home/End jump to the ends, no wrap, and other
    /// keys leave the focused element alone (brief keyboard map).
    /// @req REQ-UI-11
    #[test]
    fn roving_cursor_moves_without_wrapping() {
        assert_eq!(roving_index("down", None, 3), Some(0), "no cursor: first");
        assert_eq!(roving_index("up", None, 3), Some(2), "no cursor: last");
        assert_eq!(roving_index("down", Some(1), 3), Some(2));
        assert_eq!(roving_index("down", Some(2), 3), None, "no wrap at the end");
        assert_eq!(roving_index("up", Some(1), 3), Some(0));
        assert_eq!(roving_index("up", Some(0), 3), None, "no wrap at the start");
        assert_eq!(roving_index("home", Some(2), 3), Some(0));
        assert_eq!(roving_index("end", Some(0), 3), Some(2));
        for key in ["left", "right", "enter", "space", "a", "escape"] {
            assert_eq!(roving_index(key, Some(1), 3), None, "{key} is not ours");
        }
        assert_eq!(
            roving_index("down", None, 0),
            None,
            "an empty list has none"
        );
    }

    /// A row that a later render no longer shows loses its handle: after a
    /// refresh, filter or segment change the dialog restore can no longer
    /// focus a dead row (REQ-UI-11).
    /// @req REQ-UI-11
    #[test]
    fn stale_row_handle_is_dropped_when_the_render_set_changes() {
        let mut rows: HashMap<String, ()> = ["a", "b", "c"]
            .into_iter()
            .map(|id| (id.to_string(), ()))
            .collect();
        retain_rendered(&mut rows, &["a".to_string(), "c".to_string()]);
        assert!(
            !rows.contains_key("b"),
            "an unmounted row must not stay focusable"
        );
        assert!(rows.contains_key("a") && rows.contains_key("c"));

        retain_rendered(&mut rows, &[]);
        assert!(rows.is_empty(), "an empty render keeps no handle");
    }

    /// Enter leaves a focused wide list for the detail: the first enabled
    /// control when the previewed record offers one, otherwise the read-only
    /// scroll region (brief keyboard map).
    /// @req REQ-UI-11
    #[test]
    fn enter_handed_to_detail_control_or_region() {
        assert_eq!(detail_entry(true), DetailEntry::Control);
        assert_eq!(detail_entry(false), DetailEntry::Region);
    }
}
