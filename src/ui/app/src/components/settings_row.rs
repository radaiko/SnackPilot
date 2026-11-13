//! Settings row — docs/requirements/04-ui-ux.md §3.4.
//!
//! One grouped surface row: 56 px high, leading Lucide icon, body title, meta
//! hint and a trailing chevron; no dividers, no shadow.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::navigation::ClickHandler;
use crate::theme;
use crate::theme::AppTheme;

/// The compact settings row's one target height.
pub const MOBILE_SETTINGS_ROW_HEIGHT: f32 = 56.;

/// Desktop settings row: a 40 px flat row, `BODY` title, `META` hint and a
/// trailing chevron; the selected row is `SURFACE_ALT` with a 3 px accent
/// rule and accent ink. Mobile keeps [`settings_row`] unchanged.
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
    let income = if selected {
        theme.accent
    } else {
        theme.text_primary
    };
    let mut row = div()
        .id(SharedString::from(id))
        .w_full()
        .h(px(snackpilot_model::desktop::ROW_HEIGHT))
        .flex()
        .items_center()
        .gap(px(theme.scale.space_2))
        .pr(px(theme.scale.space_3))
        .bg(if selected {
            theme.surface_alt
        } else {
            theme.surface
        })
        .hover(move |s| {
            s.bg(if selected {
                theme.surface_alt
            } else {
                theme.hover(theme.surface)
            })
        })
        .track_focus(focus)
        .focus(theme::desktop_focus(theme.accent))
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
                .w(px(3.))
                .self_stretch()
                .flex_shrink_0()
                .bg(if selected {
                    theme.accent
                } else {
                    hsla(0., 0., 0., 0.)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(income)
                .text_ellipsis()
                .whitespace_nowrap()
                .child(title),
        );
    if let Some(hint) = hint {
        row = row.child(
            div()
                .flex_shrink_0()
                .max_w(relative(0.55))
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .text_ellipsis()
                .whitespace_nowrap()
                .child(hint),
        );
    }
    row.child(
        Icon::new(IconName::ChevronRight)
            .with_size(px(theme.scale.icon_meta))
            .text_color(theme.text_secondary),
    )
}

/// One compact grouped-list row: surface, hairline border, 12 radius, 16
/// padding, a leading Lucide icon, body title, meta hint and a trailing
/// chevron. The whole 56 px row is the target and the pressed fill is the 8 %
/// text mix.
/// @req REQ-SCR-04 @req REQ-UI-15
pub fn settings_row(
    id: &'static str,
    icon: IconName,
    title: &'static str,
    hint: Option<String>,
    on_tap: ClickHandler,
    theme: &AppTheme,
) -> Stateful<Div> {
    let mut text = div().flex_1().min_w(px(0.)).flex().flex_col().child(
        div()
            .text_size(px(theme.scale.body))
            .line_height(px(theme.scale.body_line))
            .text_color(theme.text_primary)
            .child(title),
    );
    if let Some(hint) = hint {
        text = text.child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(hint),
        );
    }
    div()
        .id(SharedString::from(id))
        .w_full()
        .min_h(px(MOBILE_SETTINGS_ROW_HEIGHT))
        .flex()
        .items_center()
        .gap(px(theme.scale.space_3))
        .p(px(theme.scale.space_4))
        .rounded(px(theme.scale.radius_card))
        .bg(theme.surface)
        .border(px(1.))
        .border_color(theme.border)
        .tab_index(0)
        .cursor_pointer()
        .hover(|s| s.bg(theme.pressed(theme.surface)))
        .focus(theme::desktop_focus(theme.accent))
        .on_click(move |_, window, cx| on_tap(window, cx))
        .child(
            Icon::new(icon)
                .with_size(px(theme.scale.icon))
                .text_color(theme.text_secondary),
        )
        .child(text)
        .child(
            Icon::new(IconName::ChevronRight)
                .with_size(px(theme.scale.icon))
                .text_color(theme.text_tertiary),
        )
}

#[cfg(test)]
mod tests {
    use super::MOBILE_SETTINGS_ROW_HEIGHT;
    use crate::theme::AppTheme;

    /// @req REQ-UI-12
    #[test]
    fn desktop_settings_row_is_a_40px_flat_row() {
        assert_eq!(snackpilot_model::desktop::ROW_HEIGHT, 40.);
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.body, 17.);
        assert_eq!(theme.scale.meta, 13.);
    }

    /// The compact grouped row (REQ-SCR-04/REQ-UI-15): 56 px high, 16 px
    /// padding and the 12 px card radius, with the 20 px icon and chevron.
    /// @req REQ-SCR-04 @req REQ-UI-15
    #[test]
    fn mobile_settings_row_uses_the_grouped_contract() {
        let theme = AppTheme::build(false);
        assert_eq!(MOBILE_SETTINGS_ROW_HEIGHT, 56.);
        assert_eq!(theme.scale.space_4, 16.);
        assert_eq!(theme.scale.radius_card, 12.);
        assert_eq!(theme.scale.icon, 20.);
        assert_eq!(theme.scale.meta, 13.);
        assert_eq!(theme.scale.body, 17.);
    }
}
