//! Segment tabs — docs/requirements/04-ui-ux.md §3.2.
//!
//! Two equal-width tabs with counts, 14 px / 600 labels, 3 px bottom border in
//! `colors.primary` for the active one. iOS uses glassSurface with a
//! glassHighlight hairline; Android/desktop use surface with a 1 px border.

use std::rc::Rc;

use gpui::*;

use crate::theme::AppTheme;

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
