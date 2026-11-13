//! Tab bar: 56 px high, hairline top, icon + label. The active tab is the
//! accent icon with a semibold label.

use std::rc::Rc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::navigation::Tab;
use snackpilot_model::tokens;

use crate::theme::AppTheme;

/// Tab-selection closure shared by the bar and its items.
pub type TabSelect = Rc<dyn Fn(Tab, &mut Window, &mut App)>;

/// Height of the bar itself; the safe-area strip sits below it.
pub const BAR_HEIGHT: f32 = 56.;

/// Height the shell keeps free at the bottom, including the safe-area strip.
pub fn reserved_height(safe_bottom: f32) -> f32 {
    BAR_HEIGHT + safe_bottom
}

pub(crate) fn tab_icon(tab: Tab) -> IconName {
    match tab {
        Tab::Menus => IconName::Utensils,
        Tab::Orders => IconName::Receipt,
        Tab::Billing => IconName::Wallet,
        Tab::Settings => IconName::Settings,
    }
}

fn tab_button(tab: Tab, active: bool, theme: &AppTheme, on_select: TabSelect) -> AnyElement {
    let color = if active {
        theme.accent
    } else {
        theme.text_secondary
    };
    div()
        .id(SharedString::from(format!("tab-{tab:?}")))
        .flex_1()
        .h(px(BAR_HEIGHT))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(tokens::SPACE_1))
        .cursor_pointer()
        .hover(|s| s.bg(theme.surface_alt))
        .focus(crate::theme::desktop_focus(theme.accent))
        .on_click(move |_, window, cx| on_select(tab, window, cx))
        .child(
            Icon::new(tab_icon(tab))
                .with_size(px(tokens::ICON))
                .text_color(color),
        )
        .child(
            div()
                .text_size(px(tokens::META.size))
                .line_height(px(tokens::META.line_height))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(color)
                .child(tab.title()),
        )
        .into_any_element()
}

/// The four-tab bar.
pub fn tab_bar(
    theme: &AppTheme,
    active: Tab,
    _safe_bottom: f32,
    on_select: TabSelect,
) -> AnyElement {
    div()
        .w_full()
        .h(px(BAR_HEIGHT))
        .flex()
        .items_stretch()
        .bg(theme.surface)
        .border_t_1()
        .border_color(theme.border)
        .children(
            Tab::ALL
                .into_iter()
                .map(|tab| tab_button(tab, tab == active, theme, on_select.clone())),
        )
        .pb(px(0.))
        .into_any_element()
}
