//! Floating action button — docs/requirements/04-ui-ux.md §3.1 / §4.
//!
//! Absolute bottom-right, right 24, bottom 80 on iOS (clears the floating
//! pill) and 24 on the flat bar; padding 24×14, §7 `fab` recipe, white 16/700.

use gpui::*;

use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// Render the FAB with a model-supplied label (`Bestellen (1)`,
/// `Stornieren (2)`, `Änderungen bestätigen (3)`).
pub fn fab(label: String, on_click: ClickHandler, theme: &AppTheme) -> AnyElement {
    let bottom = if theme.flat { 24. } else { 80. };
    let r = theme.fab();
    theme::apply(
        div()
            .absolute()
            .right(px(24.))
            .bottom(px(bottom))
            .flex()
            .items_center()
            .justify_center()
            .px(px(24.))
            .py(px(14.)),
        &r,
    )
    .id("menus-fab")
    .tab_index(0)
    .focus(crate::theme::desktop_focus(theme.desktop.focus))
    .cursor_pointer()
    .on_click(move |_, window, cx| on_click(window, cx))
    .child(
        div()
            .text_size(px(16.))
            .font_weight(FontWeight::BOLD)
            .text_color(theme.on_primary)
            .child(label),
    )
    .into_any_element()
}
