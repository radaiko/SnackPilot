//! Chips (04 §4): filter chips and status tags, styled by the §7
//! selected/unselected chip and badge recipes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{tag::Tag, Icon};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};
use snackpilot_model::desktop::{ControlState, TARGET_MIN};

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

/// Desktop filter option (brief `desktop.filterOption`): a 44 px target, 12 px
/// side padding, radius 8. The `state` hook paints rest/hover/focus/disabled
/// and the selected tint with a check mark; the selected border is a control
/// boundary on that tint, so it paints the resolved `accent_boundary` (3:1 on
/// the selection fill for every accent). The reserved 4 px ring room keeps
/// focus from clipping. The mobile chip above keeps its metrics.
/// @req REQ-UI-07 @req REQ-UI-13
pub fn desktop_chip(
    icon: Option<Icon>,
    label: impl Into<SharedString>,
    state: ControlState,
    theme: &AppTheme,
) -> Div {
    let selected = state == ControlState::Selected;
    let fg = if selected {
        theme.desktop.accent_on_selection
    } else {
        theme.control_ink(state)
    };
    let hover_fill = if selected {
        theme.control_fill(ControlState::Selected)
    } else {
        theme.control_fill(ControlState::Hover)
    };
    let chip = div()
        .flex()
        .items_center()
        .min_h(px(TARGET_MIN))
        .gap(px(6.))
        .px(px(12.))
        .rounded(px(8.))
        .bg(theme.control_fill(state))
        .border(px(1.))
        .border_color(if selected {
            theme.desktop.accent_boundary
        } else {
            theme.control_border(state)
        })
        .hover(move |s| s.bg(hover_fill));
    let chip = if selected {
        chip.child(Icon::new(IconName::Check).text_color(fg))
    } else {
        chip
    };
    let chip = match icon {
        Some(icon) => chip.child(icon.text_color(fg)),
        None => chip,
    };
    theme::focus_frame(
        chip.child(
            div()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(fg)
                .child(label.into()),
        ),
        state,
        8.,
        theme,
    )
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
