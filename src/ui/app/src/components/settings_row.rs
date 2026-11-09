//! Settings row — docs/requirements/04-ui-ux.md §3.4.
//!
//! Title 18/600, optional hint 13/tertiary, trailing chevron 20; rows are
//! separated by hairline dividers in the caller.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::navigation::ClickHandler;
use crate::theme::AppTheme;

/// One tappable settings row.
pub fn settings_row(
    id: &'static str,
    title: &'static str,
    hint: Option<String>,
    on_tap: ClickHandler,
    theme: &AppTheme,
) -> Stateful<Div> {
    let mut text = div().flex().flex_col().child(
        div()
            .text_size(px(18.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.text_primary)
            .child(title),
    );
    if let Some(hint) = hint {
        text = text.child(
            div()
                .mt(px(2.))
                .text_size(px(13.))
                .text_color(theme.text_tertiary)
                .child(hint),
        );
    }
    div()
        .id(SharedString::from(id))
        .flex()
        .items_center()
        .justify_between()
        .py(px(8.))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_tap(window, cx))
        .child(text)
        .child(
            Icon::new(IconName::ChevronRight)
                .with_size(px(20.))
                .text_color(theme.text_tertiary),
        )
}
