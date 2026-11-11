//! Time chips — docs/requirements/04-ui-ux.md §3.8.
//!
//! Twelve 15-minute options from 11:00 to 13:45, horizontal and scrollbar-free;
//! the selected chip takes the primary surface with a primary bottom border.

use std::rc::Rc;

use gpui::*;

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

/// The chip row; `selected` is the persisted time (if any).
pub fn time_chips(
    selected: Option<(u8, u8)>,
    on_pick: Rc<dyn Fn(u8, u8, &mut Window, &mut App)>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let mut row = div()
        .id("reminder-times")
        .flex()
        .gap(px(8.))
        .overflow_x_scroll();
    for (hour, minute) in times() {
        let is_selected = selected == Some((hour, minute));
        let handler = on_pick.clone();
        let fill = if is_selected {
            if theme.flat {
                theme.primary_surface
            } else {
                theme.glass_primary
            }
        } else {
            theme.banner_surface().bg
        };
        row = row.child(
            div()
                .id(SharedString::from(format!(
                    "reminder-{}",
                    label(hour, minute)
                )))
                .px(px(14.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(fill)
                .border_b_3()
                .border_color(if is_selected {
                    theme.primary
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .tab_index(0)
                .focus(crate::theme::desktop_focus(theme.desktop.focus))
                .cursor_pointer()
                .on_click(move |_, window, cx| handler(hour, minute, window, cx))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if is_selected {
                            theme.primary
                        } else {
                            theme.text_secondary
                        })
                        .child(label(hour, minute)),
                ),
        );
    }
    row
}
