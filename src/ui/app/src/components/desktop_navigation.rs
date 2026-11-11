//! Desktop navigation rail (desktop-ui-redesign brief §"Navigation rail").
//!
//! Fixed-width column at the model's `DesktopGeometry::rail_width`, filled
//! with `desktop.rail`; brand row 56 px, four 44 px rows (icon 20 + label
//! 14/20 Semibold, 4 px gap, 12 px inset), version footer that is not a
//! control. The rail scrolls when its content does not fit. Icons are the
//! same Lucide glyphs as the mobile tab bar (`tab_bar::tab_icon`).
//! @req REQ-UI-07

use gpui::*;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::navigation::Tab;

use crate::components::tab_bar::{tab_icon, TabSelect};
use crate::theme::{self, AppTheme};

/// Brand row height (brief: 56 px).
const BRAND_HEIGHT: f32 = 56.;
/// Rail row minimum height: the 44 px target floor (brief, REQ-UI-07).
const ROW_HEIGHT: f32 = 44.;
/// Gap between rail rows (brief: 4 px).
const ROW_GAP: f32 = 4.;
/// Horizontal inset of brand, rows and footer (brief: 12 px).
const ROW_INSET: f32 = 12.;
/// Rail icon size (brief: 20 px).
const ICON_SIZE: f32 = 20.;

/// The left navigation rail: brand, the four tabs, version footer.
/// `width` comes from `AppModel::desktop_geometry()` — the shell never
/// derives geometry itself.
pub fn desktop_rail(active: Tab, width: f32, theme: &AppTheme, on_select: TabSelect) -> AnyElement {
    let mut column = div()
        .id("desktop-rail")
        .w(px(width))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme.desktop.rail)
        .overflow_y_scroll()
        .child(
            div()
                .h(px(BRAND_HEIGHT))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(px(ROW_INSET))
                .child(
                    div()
                        .text_size(px(16.))
                        .line_height(px(24.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.desktop.ink)
                        .whitespace_nowrap()
                        .child("SnackPilot"),
                ),
        );

    column = column
        .gap(px(ROW_GAP))
        .children(Tab::ALL.iter().enumerate().map(|(index, tab)| {
            let tab = *tab;
            let is_active = tab == active;
            let on_select = on_select.clone();
            div()
                .id(SharedString::from(format!("rail-{tab:?}")))
                .tab_index(index as isize + 1)
                .min_h(px(ROW_HEIGHT))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(ROW_INSET))
                .cursor_pointer()
                .text_color(if is_active {
                    theme.desktop.accent_on_rail
                } else {
                    theme.text_secondary
                })
                .hover(|s| s.bg(theme.surface_variant))
                .focus(theme::desktop_focus(theme.desktop.focus))
                .on_click(move |_, window, cx| on_select(tab, window, cx))
                .child(
                    Icon::new(tab_icon(tab))
                        .with_size(px(ICON_SIZE))
                        .text_color(if is_active {
                            theme.desktop.accent_on_rail
                        } else {
                            theme.text_secondary
                        }),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
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
            .py(px(12.))
            .px(px(ROW_INSET))
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(theme.text_secondary)
                    .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
            ),
    );

    column.into_any_element()
}
