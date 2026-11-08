//! Banners (04 §4): refresh/order-progress/error/confirm banners, built from
//! the §7 `bannerSurface` and `tintedBanner` recipes.

use gpui_kit::component::Icon;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};

use crate::theme::{self, AppTheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerTone {
    /// Neutral `bannerSurface` (refresh hint, tab bar).
    Neutral,
    /// `tintedBanner` with `glassError` (error banners).
    Error,
    /// `tintedBanner` with `glassWarning` (confirm/attention banners).
    Warning,
    /// `tintedBanner` with `glassPrimary` (Android hint banner).
    Info,
}

/// A banner row: optional leading Lucide `icon`, German `text`, styled by the
/// §7 surface or tinted recipe for `tone`.
pub fn banner(
    text: impl Into<SharedString>,
    tone: BannerTone,
    icon: Option<Icon>,
    theme: &AppTheme,
) -> Div {
    let (r, fg) = match tone {
        BannerTone::Neutral => (theme.banner_surface(), theme.text_secondary),
        BannerTone::Error => (theme.tinted_banner(theme.glass_error), theme.error_text),
        BannerTone::Warning => (theme.tinted_banner(theme.glass_warning), theme.warning_text),
        BannerTone::Info => (theme.tinted_banner(theme.glass_primary), theme.primary),
    };
    let mut row = theme::apply(div().flex().items_center().gap(px(8.)).p_3(), &r);
    if let Some(icon) = icon {
        row = row.child(Icon::new(icon).text_color(fg));
    }
    row.child(div().text_size(px(13.)).text_color(fg).child(text.into()))
}

/// Refresh banner: spinner + "Aktualisiere..." (04 §4 Menus).
pub fn refresh_banner(theme: &AppTheme) -> Div {
    let r = theme.tinted_banner(theme.glass_primary);
    theme::apply(div().flex().items_center().gap(px(8.)).p_3(), &r)
        .child(crate::components::spinner::spinner(theme.primary, 14.))
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.primary)
                .child("Aktualisiere..."),
        )
}

/// Error banner with the store message (04 §4, all tabs).
pub fn error_banner(message: impl Into<SharedString>, theme: &AppTheme) -> Div {
    banner(
        message,
        BannerTone::Error,
        Some(Icon::new(gpui_kit::assets::IconName::CircleAlert)),
        theme,
    )
}

/// A muted centred banner used as a container for empty states and the
/// billing spinner card (04 §4 `bannerSurface`).
pub fn surface_card(
    children: impl IntoIterator<Item = gpui_kit::AnyElement>,
    theme: &AppTheme,
) -> Div {
    let r = theme.banner_surface();
    theme::apply(div().flex().flex_col().items_center().gap(px(8.)).p_4(), &r).children(children)
}
