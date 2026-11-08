//! Switch (04 §4), wrapped around `gpui_kit::component::Switch` with the
//! accent fill from the resolved theme.

use gpui_kit::component::switch::Switch;
use gpui_kit::component::text::Text;
use gpui_kit::{App, ElementId, Window};

use crate::theme::AppTheme;

/// A switch with a German `label`; the track uses the accent colour when on.
pub fn switch(
    id: impl Into<ElementId>,
    label: impl Into<Text>,
    checked: bool,
    theme: &AppTheme,
    on_change: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> Switch {
    Switch::new(id)
        .checked(checked)
        .label(label.into())
        .color(theme.primary)
        .on_change(on_change)
}
