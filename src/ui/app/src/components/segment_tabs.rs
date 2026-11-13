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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Upcoming,
    Past,
}

/// Callback for a segment tap.
pub type SegmentHandler = Rc<dyn Fn(Segment, &mut Window, &mut App)>;

/// The compact segment rule's 3 px stroke, reserved in both states so the
/// labels never shift when the selection moves (design "Bestellungen").
const SELECTION_RULE: f32 = 3.;

/// The compact segment target height: the 48 px action floor.
pub fn mobile_segment_height(theme: &AppTheme) -> f32 {
    theme.mobile.action_height
}

/// The selected rule: its reserved 3 px thickness in both states, and the
/// accent ink only on the selected segment (transparent otherwise). Painting
/// the same thickness in both states is what keeps the geometry stable.
pub fn mobile_segment_rule(selected: bool, theme: &AppTheme) -> (f32, Hsla) {
    let ink = if selected {
        theme.primary
    } else {
        hsla(0., 0., 0., 0.)
    };
    (SELECTION_RULE, ink)
}

/// Render the two compact segments with their counts and a select callback
/// (task 1.4): two equal min-48 targets, body Semibold labels whose counts wrap
/// instead of truncating, the 3 px selected rule reserved in both states, and
/// the readable secondary projection for the inactive one.
/// @req REQ-SCR-02 @req REQ-UI-15 @req REQ-UI-16
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
    let fill = if theme.flat {
        theme.surface
    } else {
        theme.glass_surface
    };
    let inactive_ink = theme.mobile_secondary_on(fill);

    let tab = |segment: Segment, label: String| {
        let selected = segment == active;
        let action = on_select.clone();
        let (rule, rule_ink) = mobile_segment_rule(selected, theme);
        div()
            .id(SharedString::from(match segment {
                Segment::Upcoming => "orders-tab-upcoming",
                Segment::Past => "orders-tab-past",
            }))
            .flex_1()
            .min_h(px(mobile_segment_height(theme)))
            .flex()
            .items_center()
            .justify_center()
            .px(px(theme.mobile.space_2))
            .py(px(theme.mobile.space_2))
            .cursor_pointer()
            .border_b(px(rule))
            .border_color(rule_ink)
            .on_click(move |_, window, cx| action(segment, window, cx))
            .child(
                div()
                    .min_w(px(0.))
                    .text_size(px(theme.mobile.body))
                    .line_height(px(theme.mobile.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_center()
                    .text_color(if selected {
                        theme.text_primary
                    } else {
                        inactive_ink
                    })
                    .child(label),
            )
    };

    div()
        .flex()
        .w_full()
        .bg(fill)
        .border_b_1()
        .border_color(border)
        .child(tab(Segment::Upcoming, format!("Kommende ({upcoming})")))
        .child(tab(Segment::Past, format!("Vergangene ({past})")))
}

/// Left/Right on the focused segment group: move to the neighbouring segment
/// and select it immediately, never wrap, and do nothing while focus is
/// elsewhere (the `Auswählen` button keeps its own arrows).
pub fn segment_arrow(key: &str, active: Segment, focused: bool) -> Option<Segment> {
    if !focused {
        return None;
    }
    match (key, active) {
        ("right", Segment::Upcoming) => Some(Segment::Past),
        ("left", Segment::Past) => Some(Segment::Upcoming),
        _ => None,
    }
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
    group_focus: Option<&FocusHandle>,
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
    // The two segments are one option group with one tab stop: Left/Right move
    // to the neighbouring option and select it immediately, without wrapping
    // (brief keyboard map). Only the group's own focus acts, so a focused
    // `Auswählen` button keeps its arrow keys.
    if let Some(group_focus) = group_focus {
        let select = on_select.clone();
        row = row
            .tab_index(0)
            .track_focus(group_focus)
            .focus(theme::desktop_focus(theme.desktop.focus))
            .on_key_down({
                let focus = group_focus.clone();
                move |event: &KeyDownEvent, window, cx| {
                    let Some(next) = segment_arrow(
                        event.keystroke.key.as_str(),
                        active,
                        focus.is_focused(window),
                    ) else {
                        return;
                    };
                    select(next, window, cx);
                    cx.stop_propagation();
                }
            });
    }
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

#[cfg(test)]
mod tests {
    use super::{mobile_segment_height, mobile_segment_rule, segment_arrow, Segment};
    use crate::theme::AppTheme;
    use snackpilot_model::AccentColor;

    /// The compact segments keep one geometry in both states (REQ-SCR-02/16):
    /// the 3 px rule is reserved whether or not a segment is selected, so
    /// tapping never shifts the labels, both targets are at least the 48 px
    /// action floor, and only the selected one paints the accent rule.
    /// @req REQ-SCR-02 @req REQ-UI-16
    #[test]
    fn mobile_segments_reserve_the_rule_and_equal_targets() {
        for dark in [false, true] {
            for flat in [false, true] {
                for accent in [AccentColor::Orange, AccentColor::Golden] {
                    let theme = AppTheme::build(dark, accent, flat);
                    let (thickness, active) = mobile_segment_rule(true, &theme);
                    let (inactive_thickness, inactive) = mobile_segment_rule(false, &theme);
                    assert_eq!(
                        thickness, inactive_thickness,
                        "the rule is reserved in both states, so selecting never shifts"
                    );
                    assert!(thickness > 0.);
                    assert_ne!(active, inactive, "only the selected rule is painted");
                    assert!(mobile_segment_height(&theme) >= 48.);
                }
            }
        }
    }

    /// The focused segment group moves one segment on Left/Right without
    /// wrapping, and a group without focus (e.g. the `Auswählen` button) leaves
    /// the arrow keys alone (brief keyboard map).
    /// @req REQ-UI-11
    #[test]
    fn segment_arrows_move_one_step_without_wrapping() {
        assert_eq!(
            segment_arrow("right", Segment::Upcoming, true),
            Some(Segment::Past)
        );
        assert_eq!(
            segment_arrow("left", Segment::Past, true),
            Some(Segment::Upcoming)
        );
        assert_eq!(
            segment_arrow("left", Segment::Upcoming, true),
            None,
            "no wrap at the first segment"
        );
        assert_eq!(
            segment_arrow("right", Segment::Past, true),
            None,
            "no wrap at the last segment"
        );
        for key in ["up", "down", "home", "end", "enter", "space"] {
            assert_eq!(segment_arrow(key, Segment::Upcoming, true), None, "{key}");
        }
        assert_eq!(
            segment_arrow("right", Segment::Upcoming, false),
            None,
            "focus elsewhere keeps its arrow keys"
        );
    }
}
