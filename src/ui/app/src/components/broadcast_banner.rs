//! Operator broadcast banner (task 5.7, REQ-SCR-09).
//!
//! Dismissible banner above the tab content, returning on the next foreground.

use std::rc::Rc;

use gpui::*;

use crate::theme::{self, AppTheme};

/// Render the broadcast message with a close button.
pub fn broadcast_banner(
    message: &str,
    on_dismiss: Rc<dyn Fn(&mut Window, &mut App)>,
    theme: &AppTheme,
) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_warning);
    theme::apply(
        div()
            .mx(px(16.))
            .mt(px(8.))
            .p(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .text_size(px(14.))
                    .text_color(theme.warning_text)
                    .child(message.to_string()),
            )
            .child(
                div()
                    .id("broadcast-dismiss")
                    .px(px(8.))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_dismiss(window, cx))
                    .child(
                        div()
                            .text_size(px(18.))
                            .text_color(theme.warning_text)
                            .child("✕"),
                    ),
            ),
        &r,
    )
    .into_any_element()
}
