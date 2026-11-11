//! Settings row — docs/requirements/04-ui-ux.md §3.4.
//!
//! Title 18/600, optional hint 13/tertiary, trailing chevron 20; rows are
//! separated by hairline dividers in the caller.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::navigation::ClickHandler;
use crate::theme;
use crate::theme::AppTheme;

/// Desktop settings row (brief `desktop.settingsRow`): 72 px minimum, 12 px
/// padding, whole row a target, title 14/20 Semibold and hint 12/16; the
/// selected row takes the selection tint and the inset accent rule, and focus
/// is the independent ring. Mobile keeps [`settings_row`] unchanged.
/// @req REQ-SCR-04 @req REQ-UI-12
pub fn settings_row_desktop(
    id: &'static str,
    title: &'static str,
    hint: Option<String>,
    selected: bool,
    focus: &FocusHandle,
    on_tap: ClickHandler,
    theme: &AppTheme,
) -> Stateful<Div> {
    let mut text = div().flex().flex_col().min_w_0().child(
        div()
            .text_size(px(14.))
            .line_height(px(20.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.desktop.ink)
            .child(title),
    );
    if let Some(hint) = hint {
        text = text.child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(hint),
        );
    }
    let row = div()
        .id(SharedString::from(id))
        .w_full()
        .min_h(px(72.))
        .flex()
        .items_center()
        .justify_between()
        .px(px(12.))
        .py(px(12.))
        .rounded(px(8.))
        .bg(if selected {
            theme.primary_surface
        } else {
            theme.surface
        })
        .hover(|s| s.bg(theme.surface_variant))
        .track_focus(focus)
        .focus(theme::desktop_focus(theme.desktop.focus))
        .cursor_pointer()
        .on_click({
            let on_tap = on_tap.clone();
            move |_, window, cx| on_tap(window, cx)
        })
        .on_key_down({
            let on_tap = on_tap.clone();
            move |event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    on_tap(window, cx);
                    cx.stop_propagation();
                }
            }
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .w(px(3.))
                        .h(px(32.))
                        .flex_shrink_0()
                        .rounded(px(2.))
                        .bg(if selected {
                            theme.desktop.accent_boundary
                        } else {
                            hsla(0., 0., 0., 0.)
                        }),
                )
                .child(text),
        )
        .child(
            Icon::new(IconName::ChevronRight)
                .with_size(px(20.))
                .text_color(theme.text_tertiary),
        );
    row
}

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
