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
        // Readable ink on the fill the banner really paints (REQ-UI-15): a
        // semantic hue only while it clears 4.5:1, otherwise primary text.
        BannerTone::Neutral => {
            let r = theme.banner_surface();
            let fill = theme.mobile_composited(r.bg);
            (r.clone(), theme.mobile_secondary_on(fill))
        }
        BannerTone::Error => {
            let r = theme.tinted_banner(theme.glass_error);
            let fill = theme.mobile_composited(r.bg);
            (r.clone(), theme.mobile_status_on(fill, "error"))
        }
        BannerTone::Warning => {
            let r = theme.tinted_banner(theme.glass_warning);
            let fill = theme.mobile_composited(r.bg);
            (r.clone(), theme.mobile_status_on(fill, "warning"))
        }
        BannerTone::Info => {
            let r = theme.tinted_banner(theme.glass_primary);
            let fill = theme.mobile_composited(r.bg);
            (r.clone(), theme.mobile_ink_on(fill, theme.primary))
        }
    };
    let mut row = theme::apply(
        div()
            .flex()
            .items_center()
            .gap(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_3)),
        &r,
    );
    if let Some(icon) = icon {
        row = row.child(Icon::new(icon).text_color(fg));
    }
    row.child(
        div()
            .text_size(px(theme.mobile.meta))
            .line_height(px(theme.mobile.meta_line))
            .text_color(fg)
            .child(text.into()),
    )
}

/// Cached-refresh row: a neutral 16 px spinner + the exact `Aktualisiere...`
/// text (REQ-UI-15). It never replaces the cached content, so it stays a small
/// neutral row instead of a blocking overlay.
pub fn refresh_banner(theme: &AppTheme) -> Div {
    let r = theme.banner_surface();
    let ink = theme.mobile_secondary_on(theme.mobile_composited(r.bg));
    theme::apply(
        div()
            .flex()
            .items_center()
            .gap(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_3)),
        &r,
    )
    .child(crate::components::spinner::spinner(ink, 16.))
    .child(
        div()
            .text_size(px(theme.mobile.meta))
            .line_height(px(theme.mobile.meta_line))
            .text_color(ink)
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
    theme::apply(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_4)),
        &r,
    )
    .children(children)
}
