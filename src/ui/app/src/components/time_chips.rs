//! Time chips — docs/requirements/04-ui-ux.md §3.8.
//!
//! Twelve 15-minute options from 11:00 to 13:45, horizontal and scrollbar-free;
//! the selected chip takes the primary surface with a primary bottom border.

use std::rc::Rc;

use gpui::*;
use gpui_kit::component::{Icon, Sizable};

use crate::theme::AppTheme;

/// The twelve reminder times (§3.8): hour 11–13 × minute 0/15/30/45.
pub fn times() -> Vec<(u8, u8)> {
    let mut out = Vec::with_capacity(12);
    for hour in 11..=13 {
        for minute in [0, 15, 30, 45] {
            out.push((hour, minute));
        }
    }
    out
}

/// `HH:MM`, zero padded.
pub fn label(hour: u8, minute: u8) -> String {
    format!("{hour:02}:{minute:02}")
}

/// The chip row; `selected` is the persisted time (if any). Each option is a
/// 44 px tall r8 target with 12 px padding and an 8 px gap; the selected one
/// adds the accent tint, a visible boundary and a 20 px Lucide check, so colour
/// is never the only cue (REQ-UI-15). The twelve existing options and the
/// horizontal strip are unchanged.
pub fn time_chips(
    selected: Option<(u8, u8)>,
    on_pick: Rc<dyn Fn(u8, u8, &mut Window, &mut App)>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let card = theme.surface;
    let mut row = div()
        .id("reminder-times")
        .flex()
        .gap(px(theme.scale.space_2))
        .overflow_x_scroll();
    for (hour, minute) in times() {
        let is_selected = selected == Some((hour, minute));
        let handler = on_pick.clone();
        let fill = if is_selected {
            theme.accent_surface
        } else {
            card
        };
        let ink = if is_selected {
            theme.text_primary
        } else {
            theme.text_secondary
        };
        let mut chip = div()
            .id(SharedString::from(format!(
                "reminder-{}",
                label(hour, minute)
            )))
            .flex()
            .items_center()
            .gap(px(theme.scale.space_2))
            .min_h(px(theme.scale.target))
            .px(px(theme.scale.space_3))
            .py(px(theme.scale.space_3))
            .rounded(px(theme.scale.radius_chip))
            .bg(fill)
            .border_b_3()
            .border_color(if is_selected {
                theme.accent
            } else {
                hsla(0., 0., 0., 0.)
            })
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.accent))
            .cursor_pointer()
            .on_click(move |_, window, cx| handler(hour, minute, window, cx));
        if is_selected {
            chip = chip.child(
                Icon::new(gpui_kit::assets::IconName::Check)
                    .with_size(px(theme.scale.space_5))
                    .text_color(ink),
            );
        }
        row = row.child(
            chip.child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(label(hour, minute)),
            ),
        );
    }
    row
}
