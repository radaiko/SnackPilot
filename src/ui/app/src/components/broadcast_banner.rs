//! Operator broadcast banner (task 5.7, REQ-SCR-09).
//!
//! Dismissible banner above the tab content, returning on the next foreground.

use std::rc::Rc;

use gpui::*;
use gpui_kit::component::Sizable;

use crate::theme::{self, AppTheme};

/// Render the broadcast message with a close button.
pub fn broadcast_banner(
    message: &str,
    on_dismiss: Rc<dyn Fn(&mut Window, &mut App)>,
    theme: &AppTheme,
) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_warning);
    let ink = theme.mobile_status_on(theme.mobile_composited(r.bg), "warning");
    theme::apply(
        div()
            .mx(px(theme.mobile.space_4))
            .mt(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_3))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(theme.mobile.space_2))
            .child(
                div()
                    .flex_1()
                    .text_size(px(theme.mobile.meta))
                    .line_height(px(theme.mobile.meta_line))
                    .text_color(ink)
                    .child(message.to_string()),
            )
            .child(
                // The dismissal is a full 44×44 target, above the safe area and
                // never smaller than the touch floor (REQ-UI-16).
                div()
                    .id("broadcast-dismiss")
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(theme.mobile.target))
                    .h(px(theme.mobile.target))
                    .tab_index(0)
                    .focus(crate::theme::desktop_focus(theme.mobile.focus))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_dismiss(window, cx))
                    .child(
                        gpui_kit::component::Icon::new(gpui_kit::assets::IconName::X)
                            .with_size(px(theme.mobile.space_5))
                            .text_color(ink),
                    ),
            ),
        &r,
    )
    .into_any_element()
}
