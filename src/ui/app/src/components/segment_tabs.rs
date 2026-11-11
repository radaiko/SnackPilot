//! Segment tabs — docs/requirements/04-ui-ux.md §3.2.
//!
//! Two equal-width tabs with counts, 14 px / 600 labels, 3 px bottom border in
//! `colors.primary` for the active one. iOS uses glassSurface with a
//! glassHighlight hairline; Android/desktop use surface with a 1 px border.

use std::rc::Rc;

use gpui::*;

use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// Which segment is shown.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Upcoming,
    Past,
}

/// Callback for a segment tap.
pub type SegmentHandler = Rc<dyn Fn(Segment, &mut Window, &mut App)>;

/// Render the two segments with their counts and a select callback.
pub fn segment_tabs(
    active: Segment,
    upcoming: usize,
    past: usize,
    on_select: SegmentHandler,
    theme: &AppTheme,
) -> Div {
    let border = if theme.flat {
        theme.border
    } else {
        theme.glass_highlight
    };
    let bg = if theme.flat {
        theme.surface
    } else {
        theme.glass_surface
    };

    let tab = |segment: Segment, label: String| {
        let selected = segment == active;
        let action = on_select.clone();
        div()
            .id(SharedString::from(match segment {
                Segment::Upcoming => "orders-tab-upcoming",
                Segment::Past => "orders-tab-past",
            }))
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .py(px(14.))
            .cursor_pointer()
            .border_b_3()
            .border_color(if selected {
                theme.primary
            } else {
                hsla(0., 0., 0., 0.)
            })
            .on_click(move |_, window, cx| action(segment, window, cx))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if selected {
                        theme.primary
                    } else {
                        theme.text_tertiary
                    })
                    .child(label),
            )
    };

    div()
        .flex()
        .w_full()
        .bg(bg)
        .border_b_1()
        .border_color(border)
        .child(tab(Segment::Upcoming, format!("Kommende ({upcoming})")))
        .child(tab(Segment::Past, format!("Vergangene ({past})")))
}

/// The wide Bestellungen toolbar (task 1.4): the same two segment options and
/// counts plus, for upcoming only, the `Auswählen` batch-mode entry. Targets
/// stay at 44 px and the row wraps instead of overlapping.
/// @req REQ-SCR-02 @req REQ-UI-12
pub fn desktop_segment_toolbar(
    active: Segment,
    upcoming: usize,
    past: usize,
    on_select: SegmentHandler,
    on_select_mode: Option<ClickHandler>,
    theme: &AppTheme,
) -> AnyElement {
    let tab = |segment: Segment, label: String| {
        let selected = segment == active;
        let action = on_select.clone();
        div()
            .id(SharedString::from(match segment {
                Segment::Upcoming => "desktop-orders-tab-upcoming",
                Segment::Past => "desktop-orders-tab-past",
            }))
            .tab_index(0)
            .min_h(px(44.))
            .px(px(12.))
            .py(px(8.))
            .flex()
            .items_center()
            .cursor_pointer()
            .border_b_3()
            .border_color(if selected {
                theme.primary
            } else {
                hsla(0., 0., 0., 0.)
            })
            .focus(theme::desktop_focus(theme.desktop.focus))
            .on_click(move |_, window, cx| action(segment, window, cx))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if selected {
                        theme.primary
                    } else {
                        theme.text_secondary
                    })
                    .child(label),
            )
    };

    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(8.))
        .px(px(16.))
        .py(px(8.))
        .child(tab(Segment::Upcoming, format!("Kommende ({upcoming})")))
        .child(tab(Segment::Past, format!("Vergangene ({past})")));
    if let Some(on_select_mode) = on_select_mode {
        row = row.child(
            div()
                .id("desktop-orders-select")
                .ml_auto()
                .tab_index(0)
                .min_h(px(44.))
                .px(px(12.))
                .flex()
                .items_center()
                .cursor_pointer()
                .rounded(px(8.))
                .hover(|s| s.bg(theme.surface_variant))
                .focus(theme::desktop_focus(theme.desktop.focus))
                .on_click(move |_, window, cx| on_select_mode(window, cx))
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.desktop.accent_on_panel)
                        .child("Auswählen"),
                ),
        );
    }
    row.into_any_element()
}
