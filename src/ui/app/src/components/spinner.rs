//! Loading spinner (04 §4), wrapped around `gpui_kit::component::Spinner`
//! with the accent/muted colour from the resolved theme.

use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::Sizable;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, Hsla, SharedString};

use crate::theme::AppTheme;

/// A cycling spinner in `color` at `size` px.
pub fn spinner(color: Hsla, size: f32) -> Spinner {
    Spinner::new().color(color).with_size(px(size))
}

/// A spinner sized and coloured for a primary button's busy state.
pub fn button_spinner(theme: &AppTheme) -> Spinner {
    Spinner::new().color(theme.on_accent).with_size(px(16.))
}

/// Spinner plus German loading label, e.g. "Wird in den Warenkorb gelegt...",
/// "Aktualisiere..." (04 §4).
pub fn loading_row(text: impl Into<SharedString>, theme: &AppTheme) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(spinner(theme.text_secondary, 16.))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(text.into()),
        )
}
