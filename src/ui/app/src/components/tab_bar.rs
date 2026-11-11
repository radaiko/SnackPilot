//! Tab bar — docs/requirements/04-ui-ux.md §2.
//!
//! iOS (§2.1): floating glass pill 248×56 r28, icons only (26 px), 5×5 dot
//! under the active icon, wrapper bottom = max(safeBottom, 8).
//! Android/desktop (§2.2): full-width bottom bar, `surface` background,
//! 1 px top `border`, elevation 8, icon + 12 px/600 label.

use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::navigation::Tab;

use crate::theme::AppTheme;

/// Tab-selection closure shared by the bar and its items.
pub type TabSelect = Rc<dyn Fn(Tab, &mut Window, &mut App)>;

/// Height of the iOS floating pill (§2.1).
pub const IOS_PILL_HEIGHT: f32 = 56.;
/// Height of the Android/desktop bottom bar (§2.2).
pub const BAR_HEIGHT: f32 = 64.;
/// Every desktop tab target meets the 44 px floor (brief `desktop.targetMin`);
/// the phone metrics above stay as they are.
pub const DESKTOP_TARGET: f32 = snackpilot_model::desktop::TARGET_MIN;

/// Approximation of Android `elevation: N` (04 §7) as a black drop shadow,
/// scaled like Material's key light (same shape as `theme::elevation_shadow`).
pub(crate) fn elevation_shadow(e: f32) -> BoxShadow {
    BoxShadow {
        color: hsla(0., 0., 0., 0.06 + e * 0.02),
        offset: point(px(0.), px(e * 0.75)),
        blur_radius: px(e * 1.5),
        spread_radius: px(0.),
        inset: false,
    }
}

/// Height the shell must keep free at the bottom for the tab bar, given the
/// bottom safe-area inset (§2.1 pill bottom + height; §2.2 bar height).
pub fn reserved_height(flat: bool, safe_bottom: f32) -> f32 {
    if flat {
        BAR_HEIGHT
    } else {
        safe_bottom.max(8.) + IOS_PILL_HEIGHT
    }
}

/// iOS §2.1 hit slop around each 44×44 tab button: the clickable div is
/// 44+2×8 px, pulled back with negative margins so layout (and the visual
/// icon area) stays 44×44.
const HIT_SLOP: f32 = 8.;

/// Tab icon. §2.1 names a SwiftUI outline+filled pair per tab
/// (`restaurant-outline`/`restaurant`); the vendored Lucide set (gpui-kit
/// `assets`, 1 830 icons) is outline-only — no filled variants for these
/// four — so the active state is conveyed by colour (primary vs tertiary)
/// plus the 5×5 dot, matching §2.2's Android dialect. Documented limitation
/// (see 4.2 evidence note).
pub(crate) fn tab_icon(tab: Tab) -> IconName {
    match tab {
        Tab::Menus => IconName::Utensils,
        Tab::Orders => IconName::Receipt,
        Tab::Billing => IconName::Wallet,
        Tab::Settings => IconName::Settings,
    }
}

/// One tab button: 44×44 icon area; active tab gets the dot (iOS) or the
/// primary-coloured bold label (Android).
fn tab_button(
    tab: Tab,
    active: bool,
    flat: bool,
    theme: &AppTheme,
    on_select: TabSelect,
) -> AnyElement {
    let color = if active {
        theme.primary
    } else {
        theme.text_tertiary
    };

    let icon = div()
        .relative()
        .w(px(44.))
        .h(px(44.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(tab_icon(tab))
                .with_size(px(if flat { 24. } else { 26. }))
                .text_color(color)
                .into_any_element(),
        )
        .when(active && !flat, |d| {
            d.child(
                div()
                    .absolute()
                    .bottom(px(4.))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(div().w(px(5.)).h(px(5.)).rounded(px(2.5)).bg(theme.primary)),
            )
        });

    let button = div()
        .id(SharedString::from(format!("tab-{tab:?}")))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .cursor_pointer()
        // Desktop pointer focus hooks. `flat` covers the desktop host; a
        // pointer hover never occurs on the touch-only phone hosts, and no
        // resting dimension or colour changes.
        .when(flat, |d| {
            d.hover(|s| s.bg(theme.surface_variant))
                .focus(crate::theme::desktop_focus(theme.desktop.focus))
        })
        .on_click(move |_, window, cx| on_select(tab, window, cx))
        .child(icon);

    if flat {
        button
            .flex_1()
            .py(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if active {
                        theme.primary
                    } else {
                        theme.text_tertiary
                    })
                    .child(tab.title()),
            )
            .into_any_element()
    } else {
        // §2.1: 8 px hit slop — the div is 60×60 with -8 margins on every
        // side, so it still occupies exactly 44×44 in the pill's layout.
        button
            .w(px(44. + 2. * HIT_SLOP))
            .h(px(44. + 2. * HIT_SLOP))
            .m(px(-HIT_SLOP))
            .into_any_element()
    }
}

/// The four-tab bar. `on_select` receives the tapped tab; the shell routes it
/// through `Navigation::select_tab` (tap on the active tab is a no-op).
pub fn tab_bar(
    theme: &AppTheme,
    active: Tab,
    safe_bottom: f32,
    on_select: TabSelect,
) -> AnyElement {
    let buttons = Tab::ALL
        .into_iter()
        .map(|tab| tab_button(tab, tab == active, theme.flat, theme, on_select.clone()))
        .collect::<Vec<_>>();

    if theme.flat {
        div()
            .w_full()
            .h(px(BAR_HEIGHT))
            .flex()
            .items_stretch()
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .shadow(vec![elevation_shadow(8.)])
            .children(buttons)
            .into_any_element()
    } else {
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(safe_bottom.max(8.)))
            .flex()
            .justify_center()
            .child(
                div()
                    .w(px(248.))
                    .h(px(IOS_PILL_HEIGHT))
                    .flex()
                    .items_center()
                    .justify_between()
                    // 4×44 buttons + 3×20 gaps = 236, pill 248 → 6 px padding.
                    .px(px(6.))
                    .rounded(px(28.))
                    .bg(theme.glass_surface)
                    // §2.1: 0.5 hairline. Top/left would be glassHighlight,
                    // bottom/right glassShadowEdge; gpui has no per-side
                    // border colour, so one uniform glassShadowEdge hairline.
                    .border(px(0.5))
                    .border_color(theme.glass_shadow_edge)
                    // §2.1: shadow #000, offset (0,8), opacity 0.15, radius 32.
                    .shadow(vec![BoxShadow {
                        color: hsla(0., 0., 0., 0.15),
                        offset: point(px(0.), px(8.)),
                        blur_radius: px(32.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .children(buttons),
            )
            .into_any_element()
    }
}
