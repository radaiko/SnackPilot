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

/// §3.5 back affordance row; `label` is the screen it returns to
/// (`Einstellungen` for every pushed screen, §3.5–3.8).
pub fn back_affordance(
    label: &'static str,
    theme: &AppTheme,
    on_back: Rc<dyn Fn(&mut Window, &mut App)>,
) -> AnyElement {
    div()
        .id("back-affordance")
        .flex()
        .items_center()
        .gap(px(2.))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_back(window, cx))
        .child(
            Icon::new(IconName::ChevronLeft)
                .with_size(px(24.))
                .text_color(theme.primary),
        )
        .child(
            div()
                .text_size(px(17.))
                .text_color(theme.primary)
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
    on_back: Rc<dyn Fn(&mut Window, &mut App)>,
) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .p(px(20.))
        .pb(px(100.))
        .gap(px(24.))
        .child(back_affordance("Einstellungen", theme, on_back))
        .child(
            div()
                .text_size(px(28.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .child(title),
        )
        .into_any_element()
}
