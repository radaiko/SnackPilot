//! Loading overlay — docs/requirements/04-ui-ux.md §4.2.
//!
//! Fills the shell absolutely (painted last by the shell) and centres a spinner card:
//! radius 24, padding 28, `bannerSurface` style, elevation 6, large spinner
//! in `colors.primary`. Android uses an `overlay`-tinted layer; iOS would use
//! `AdaptiveBlurView`, which gpui has no primitive for, so the same tinted
//! layer stands in (same approximation as the §4.3 banners in task 4.1).

use gpui::*;

use crate::components::spinner::spinner;
use crate::theme::{self, AppTheme};

/// The §4.2 full-screen loading layer. Rendered by the shell while
/// `AppModel` state reports an in-flight action.
pub fn loading_overlay(theme: &AppTheme) -> AnyElement {
    let card = theme.banner_surface();
    div()
        .id("loading-overlay")
        .occlude()
        .absolute()
        .inset_0()
        .bg(theme.overlay)
        .flex()
        .items_center()
        .justify_center()
        .child(
            theme::apply(
                div().flex().items_center().justify_center().p(px(28.)),
                &card,
            )
            .rounded(px(24.))
            .shadow(vec![
                // §4.2: elevation 6 on the spinner card.
                crate::components::tab_bar::elevation_shadow(6.),
            ])
            .child(spinner(theme.primary, 36.)),
        )
        .into_any_element()
}
