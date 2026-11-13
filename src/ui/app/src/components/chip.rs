//! Chips (04 §4): filter chips and status tags, styled by the §7
//! selected/unselected chip and badge recipes.

use gpui_kit::assets::IconName;
use gpui_kit::component::{tag::Tag, Icon, Sizable, StyledExt};
use gpui_kit::prelude::*;
use gpui_kit::{div, px, Div, SharedString};
use snackpilot_model::desktop::ControlState;
use snackpilot_model::tokens;

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
    // Compact option recipe (REQ-UI-15): `mobile.option` — 44 px two-axis
    // target, 12 px side padding, r8, and the accent tint plus an independent
    // 8 px check slot when selected. The colour cue is never the only one.
    let radius = theme.scale.radius_chip;
    let fill = if selected {
        theme.accent_surface
    } else {
        theme.surface
    };
    let fg = if selected {
        theme.text_primary
    } else {
        theme.text_secondary
    };
    let mut chip = div()
        .flex()
        .items_center()
        .gap(px(theme.scale.space_2))
        .min_h(px(theme.scale.target))
        .px(px(theme.scale.space_3))
        .py(px(theme.scale.space_1))
        .rounded(px(radius))
        .bg(fill)
        .border(px(1.))
        .border_color(if selected { theme.accent } else { theme.border });
    if selected {
        chip = chip.child(
            Icon::new(IconName::Check)
                .with_size(px(theme.scale.space_5))
                .text_color(fg),
        );
    }
    if let Some(icon) = icon {
        chip = chip.child(icon.text_color(fg));
    }
    chip.child(
        div()
            .text_size(px(theme.scale.body))
            .line_height(px(theme.scale.body_line))
            .font_semibold()
            .text_color(fg)
            .child(label.into()),
    )
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
        theme.accent
    } else {
        theme.control_ink(state)
    };
    let hover_fill = if selected {
        theme.control_fill(ControlState::Selected)
    } else {
        theme.hover(theme.surface)
    };
    let chip = div()
        .flex()
        .items_center()
        .min_h(px(tokens::TARGET))
        .gap(px(theme.scale.space_2))
        .px(px(theme.scale.space_3))
        .rounded(px(theme.scale.radius_chip))
        .bg(theme.control_fill(state))
        .border(px(1.))
        .border_color(if selected {
            theme.accent
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
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(fg)
                .child(label.into()),
        ),
        state,
        theme.scale.radius_chip,
        theme,
    )
}

/// Row of chips (the filter strip).
pub fn chip_row(chips: impl IntoIterator<Item = Div>) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap(px(snackpilot_model::tokens::SPACE_2))
        .children(chips)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Success,
    Warning,
    Error,
}

/// Small status tag (order/billing states) on its toned semantic surface.
pub fn status_tag(label: impl Into<SharedString>, tone: StatusTone, theme: &AppTheme) -> Tag {
    let (bg, fg, edge) = match tone {
        StatusTone::Success => (theme.success_surface, theme.success, theme.success),
        StatusTone::Warning => (theme.warning_surface, theme.warning, theme.warning),
        StatusTone::Error => (theme.error_surface, theme.error, theme.error),
    };
    // Every platform paints the toned surface with a 1 px tone border.
    let border = edge;
    Tag::custom(bg, fg, border).child(div().text_size(px(12.)).text_color(fg).child(label.into()))
}
