//! Switch (04 §4), wrapped around `gpui_kit::component::Switch` with the
//! accent fill from the resolved theme.

use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::*;
use gpui_kit::{div, px, App, Div, ElementId, SharedString, Window};

use crate::theme::AppTheme;

/// A switch with a German `label`; the track uses the accent colour when on.
/// The label is drawn by us rather than the kit's `label`, which follows the
/// kit's global theme and would stay dark-on-dark in a dark section.
pub fn switch(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    checked: bool,
    theme: &AppTheme,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(theme.mobile.space_2))
        .min_h(px(theme.mobile.target))
        .child(
            Switch::new(id)
                .checked(checked)
                .color(theme.primary)
                .on_change(on_change),
        )
        .child(
            div()
                .text_size(px(theme.mobile.body))
                .line_height(px(theme.mobile.body_line))
                .text_color(theme.text_primary)
                .child(label.into()),
        )
}
