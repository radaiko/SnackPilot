//! Chips (04 §4): filter chips and status tags, styled by the §7
//! selected/unselected chip and badge recipes.

use gpui_kit::component::{tag::Tag, Icon};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};

use crate::theme::{self, AppTheme};

/// A filter chip: German `label` in Inter; `selected` picks the §7
/// `selectedChip` fill (accent tint), otherwise `unselectedChip`.
pub fn chip(label: impl Into<SharedString>, selected: bool, theme: &AppTheme) -> Div {
    chip_inner(None, label, selected, theme)
}

/// A chip with a leading Lucide icon (04 §2 icon mapping).
pub fn icon_chip(
    icon: Icon,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &AppTheme,
) -> Div {
    chip_inner(Some(icon), label, selected, theme)
}

fn chip_inner(
    icon: Option<Icon>,
    label: impl Into<SharedString>,
    selected: bool,
    theme: &AppTheme,
) -> Div {
    let r = if selected {
        theme.selected_chip()
    } else {
        theme.unselected_chip()
    };
    let fg = if selected {
        theme.primary
    } else {
        theme.text_secondary
    };
    let mut chip = theme::apply(
        div().flex().items_center().gap(px(6.)).px_3().py(px(6.)),
        &r,
    );
    if let Some(icon) = icon {
        chip = chip.child(icon.text_color(fg));
    }
    chip.child(div().text_size(px(13.)).text_color(fg).child(label.into()))
}

/// Row of chips (the filter strip).
pub fn chip_row(chips: impl IntoIterator<Item = Div>) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(8.))
        .children(chips)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Success,
    Warning,
    Error,
}

/// Small status tag (order/billing states): the §7 badge colours —
/// glassSuccess/Warning/Error on iOS, the matching surface tokens on Android
/// (already folded into `theme.glass_*`).
pub fn status_tag(label: impl Into<SharedString>, tone: StatusTone, theme: &AppTheme) -> Tag {
    let (bg, fg, edge) = match tone {
        StatusTone::Success => (theme.glass_success, theme.success_text, theme.success),
        StatusTone::Warning => (theme.glass_warning, theme.warning_text, theme.warning),
        StatusTone::Error => (theme.glass_error, theme.error_text, theme.error),
    };
    // Android: `{success,warning,error}Surface` + 1 px border in the tone
    // colour (04 §7); iOS badges are plain glass fills, so the border
    // (always 1 px on the kit Tag) matches the background there.
    let border = if theme.flat { edge } else { bg };
    Tag::custom(bg, fg, border).child(div().text_size(px(12.)).text_color(fg).child(label.into()))
}
