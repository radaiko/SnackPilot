//! Desktop navigation rail on the one flat system.
//!
//! Fixed-width column at the model's `DesktopGeometry::rail_width`, filled
//! with `BG` and closed by a hairline `BORDER`. The brand row is 56 px, the
//! section label is `META` `TEXT_SECONDARY`, every tab is a 40 px row
//! (`BODY`) with a reserved 3 px accent bar, and the active row is
//! `SURFACE_ALT` with accent ink. Icons are the same Lucide glyphs as the
//! mobile tab bar (`tab_bar::tab_icon`).
//! @req REQ-UI-07

use gpui::*;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::desktop::ROW_HEIGHT;
use snackpilot_model::navigation::Tab;

use crate::components::tab_bar::{tab_icon, TabSelect};
use crate::components::workspace::section_label;
use crate::theme::{self, AppTheme};

/// Brand row height.
const BRAND_HEIGHT: f32 = 56.;
/// Gap between rail rows.
const ROW_GAP: f32 = 4.;
/// Active-row accent rule.
const ACTIVE_RULE: f32 = 3.;

/// The left navigation rail: brand, a section label, the four tabs and the
/// version footer. `width` comes from `AppModel::desktop_geometry()` — the
/// shell never derives geometry itself.
pub fn desktop_rail(
    active: Tab,
    width: f32,
    row_focus: Option<&[FocusHandle; 4]>,
    theme: &AppTheme,
    on_select: TabSelect,
) -> AnyElement {
    let m = &theme.scale;
    let mut column = div()
        .id("desktop-rail")
        .w(px(width))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme.background)
        .border_r_1()
        .border_color(theme.border)
        .overflow_y_scroll()
        .on_key_down({
            let row_focus = row_focus.cloned();
            move |event: &KeyDownEvent, window, cx| {
                let Some(row_focus) = &row_focus else {
                    return;
                };
                let key = event.keystroke.key.as_str();
                let current = row_focus
                    .iter()
                    .position(|handle| handle.is_focused(window));
                let next = match (key, current) {
                    ("down", Some(index)) => (index + 1 < Tab::ALL.len()).then_some(index + 1),
                    ("down", None) => Some(0),
                    ("up", Some(index)) => index.checked_sub(1),
                    ("up", None) => Some(Tab::ALL.len() - 1),
                    ("home", _) => Some(0),
                    ("end", _) => Some(Tab::ALL.len() - 1),
                    _ => None,
                };
                if let Some(index) = next {
                    window.focus(&row_focus[index], cx);
                    cx.stop_propagation();
                }
            }
        })
        .child(
            div()
                .h(px(BRAND_HEIGHT))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(px(m.space_4))
                .child(
                    div()
                        .text_size(px(m.headline))
                        .line_height(px(m.headline_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .whitespace_nowrap()
                        .child("SnackPilot"),
                ),
        )
        .child(section_label("Navigation", theme));

    column = column
        .gap(px(ROW_GAP))
        .children(Tab::ALL.iter().enumerate().map(|(index, tab)| {
            let tab = *tab;
            let is_active = tab == active;
            let on_select = on_select.clone();
            let activate = on_select.clone();
            let handle = row_focus.map(|handles| handles[index].clone());
            let mut row = div()
                .id(SharedString::from(format!("rail-{tab:?}")))
                .tab_index(index as isize + 1);
            if let Some(handle) = &handle {
                row = row.track_focus(handle);
            }
            row.on_key_down(move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    activate(tab, window, cx);
                    cx.stop_propagation();
                }
            })
            .h(px(ROW_HEIGHT))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(m.space_2))
            .pr(px(m.space_4))
            .cursor_pointer()
            .text_color(if is_active {
                theme.accent
            } else {
                theme.text_secondary
            })
            .bg(if is_active {
                theme.surface_alt
            } else {
                theme.background
            })
            .hover(move |s| {
                s.bg(if is_active {
                    theme.surface_alt
                } else {
                    theme.hover(theme.background)
                })
            })
            .focus(theme::desktop_focus(theme.accent))
            .on_click(move |_, window, cx| on_select(tab, window, cx))
            .child(
                div()
                    .w(px(ACTIVE_RULE))
                    .self_stretch()
                    .flex_shrink_0()
                    .bg(if is_active {
                        theme.accent
                    } else {
                        hsla(0., 0., 0., 0.)
                    }),
            )
            .child(
                Icon::new(tab_icon(tab))
                    .with_size(px(m.icon))
                    .text_color(if is_active {
                        theme.accent
                    } else {
                        theme.text_secondary
                    }),
            )
            .child(
                div()
                    .text_size(px(m.body))
                    .line_height(px(m.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .whitespace_nowrap()
                    .child(tab.title()),
            )
        }));

    // Footer: version only, never a control (no cursor, no click handler).
    column = column.child(
        div()
            .mt_auto()
            .flex_shrink_0()
            .py(px(m.space_3))
            .px(px(m.space_4))
            .child(
                div()
                    .text_size(px(m.meta))
                    .line_height(px(m.meta_line))
                    .text_color(theme.text_secondary)
                    .child(format!("Version {}", crate::desktop::product_version())),
            ),
    );

    column.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{ACTIVE_RULE, BRAND_HEIGHT, ROW_GAP};
    use snackpilot_model::desktop::ROW_HEIGHT;

    /// The rail keeps one 40 px flat row beside the hairline and the 3 px
    /// active rule; no compact/expanded row or rail duplication remains.
    /// @req REQ-UI-07
    #[test]
    fn rail_rows_use_the_flat_scale() {
        assert_eq!(ROW_HEIGHT, 40.);
        assert_eq!(ROW_GAP, 4.);
        assert_eq!(BRAND_HEIGHT, 56.);
        assert_eq!(ACTIVE_RULE, 3.);
    }
}
