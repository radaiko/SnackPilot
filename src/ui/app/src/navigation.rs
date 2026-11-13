//! Push/pop stack content — docs/requirements/04-ui-ux.md §1, back affordance
//! §3.5–3.8.
//!
//! Stack screens declare `headerShown: false` (§1); each draws its own back
//! affordance: `chevron-back` (24, `colors.primary`) + the destination title
//! (17, `colors.primary`). Only `SettingsDetail` is pushable in v2; the four
//! real sub-screens arrive with tasks 5.x and reuse `back_affordance`.

use std::rc::Rc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::theme::AppTheme;

/// Shared click-closure type for the shell components.
pub type ClickHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// §3.5 back affordance row; `label` is the screen it returns to
/// (`Einstellungen` for every pushed screen, §3.5–3.8).
pub fn back_affordance(label: &'static str, theme: &AppTheme, on_back: ClickHandler) -> AnyElement {
    // Compact back row (REQ-UI-15): a 48 px minimum row with the incumbent
    // `ChevronLeft` at 24 px, on the readable primary ink, wrapping rather than
    // ellipsizing. The 48 px row already clears the 44 px touch floor.
    div()
        .id("back-affordance")
        .flex()
        .items_center()
        .gap(px(theme.mobile.space_1))
        .min_h(px(theme.mobile.action_height))
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.mobile.focus))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_back(window, cx))
        .child(
            Icon::new(IconName::ChevronLeft)
                .with_size(px(24.))
                .text_color(theme.text_primary),
        )
        .child(
            div()
                .text_size(px(theme.mobile.body))
                .line_height(px(theme.mobile.body_line))
                .text_color(theme.text_primary)
                .child(label),
        )
        .into_any_element()
}

/// Placeholder body for the pushed `SettingsDetail` screen: back affordance
/// plus the page-title type of §3.5 (28/700). Tasks 5.x replace the body with
/// the four real sub-screens.
pub fn settings_detail_screen(
    title: &'static str,
    theme: &AppTheme,
    on_back: ClickHandler,
) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(theme.mobile.space_4))
        .gap(px(theme.mobile.space_6))
        .child(back_affordance("Einstellungen", theme, on_back))
        .child(
            div()
                .text_size(px(theme.mobile.headline))
                .line_height(px(theme.mobile.headline_line))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .into_any_element()
}
