//! Desktop workspace chrome: the screen header, the master/detail split with
//! its 1 px divider, the action strip and the shared empty/loading/section
//! helpers every screen composes. All widths come from
//! `snackpilot_model::desktop::DesktopGeometry`; this module never derives
//! geometry itself, so the geometry tests in the model cover the numbers.
//! @req REQ-UI-07 @req REQ-UI-12

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{tooltip::Tooltip, Icon, Sizable};
use snackpilot_model::desktop::DesktopGeometry;
use snackpilot_model::tokens;

use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// Header minimum height.
const HEADER_HEIGHT: f32 = 56.;
/// Action strip minimum height.
const STRIP_HEIGHT: f32 = 56.;

/// The screen header: title left, trailing actions right, 56 px min height.
/// `on_refresh` is the trailing 44×44 icon button (`Aktualisieren`); the
/// Einstellungen tab passes `None` and gets no button.
pub fn workspace_header(
    title: &'static str,
    on_refresh: Option<ClickHandler>,
    theme: &AppTheme,
) -> AnyElement {
    let mut header = div()
        .min_h(px(HEADER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px(px(theme.scale.space_4))
        .bg(theme.surface)
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .whitespace_nowrap()
                .text_ellipsis()
                .child(title),
        );
    header = match on_refresh {
        Some(on_click) => header.child(header_action(on_click, theme)),
        None => header.child(div().w(px(0.))),
    };
    header.into_any_element()
}

/// The trailing 44×44 header action button (icon-only, tooltip
/// `Aktualisieren`). The shell decides per tab whether it exists.
pub fn header_action(on_click: ClickHandler, theme: &AppTheme) -> AnyElement {
    div()
        .id("workspace-refresh")
        .tab_index(0)
        .w(px(tokens::TARGET))
        .h(px(tokens::TARGET))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .rounded(px(tokens::RADIUS_CHIP))
        .text_color(theme.text_primary)
        .hover(move |s| s.bg(theme.hover(theme.surface)))
        .focus(theme::desktop_focus(theme.accent))
        .on_click(move |_, window, cx| on_click(window, cx))
        .tooltip(|window, cx| Tooltip::new("Aktualisieren").build(window, cx))
        .child(
            Icon::new(IconName::RefreshCw)
                .with_size(px(theme.scale.icon))
                .text_color(theme.text_primary),
        )
        .into_any_element()
}

/// The master/detail split: two independently scrolling panes separated by
/// the 1 px divider from the geometry. Widths are the model's; the panes
/// never size themselves. `master`/`detail` are the pane contents.
pub fn split_workspace(
    master: AnyElement,
    detail: AnyElement,
    geometry: &DesktopGeometry,
    theme: &AppTheme,
) -> AnyElement {
    split_panes(master, detail, geometry, theme, None, None)
}

/// The same split with a tracked master scroll region, so a keyboard cursor
/// can reveal the row it focused, and a focusable detail region that Enter
/// hands focus to when the preview offers no action.
pub fn split_workspace_scrolled(
    master: AnyElement,
    detail: AnyElement,
    geometry: &DesktopGeometry,
    theme: &AppTheme,
    master_scroll: &ScrollHandle,
    detail_focus: &FocusHandle,
) -> AnyElement {
    split_panes(
        master,
        detail,
        geometry,
        theme,
        Some(master_scroll),
        Some(detail_focus),
    )
}

fn split_panes(
    master: AnyElement,
    detail: AnyElement,
    geometry: &DesktopGeometry,
    theme: &AppTheme,
    master_scroll: Option<&ScrollHandle>,
    detail_focus: Option<&FocusHandle>,
) -> AnyElement {
    let mut master_pane = div()
        .id("workspace-master")
        .w(px(geometry.master_width))
        .flex_shrink_0()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    if let Some(scroll) = master_scroll {
        master_pane = master_pane.track_scroll(scroll);
    }
    // The read-only detail region: programmatically focusable (Enter from the
    // list) but not a tab stop, so Tab keeps moving through the controls.
    let mut detail_pane = div()
        .id("workspace-detail")
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_y_scroll();
    if let Some(focus) = detail_focus {
        detail_pane = detail_pane
            .track_focus(focus)
            .focus(theme::desktop_focus(theme.accent));
    }
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .child(master_pane.child(master))
        .child(
            div()
                .w(px(geometry.divider_width))
                .flex_shrink_0()
                .bg(theme.border),
        )
        .child(detail_pane.child(detail))
        .into_any_element()
}

/// The action strip under a workspace list: min 56 px, 8 px gap, wraps to a
/// second row when the pane is narrow, 1 px top border.
pub fn action_strip(items: Vec<AnyElement>, theme: &AppTheme) -> AnyElement {
    div()
        .min_h(px(STRIP_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.scale.space_2))
        .px(px(theme.scale.space_4))
        .border_t_1()
        .border_color(theme.border)
        .children(items)
        .into_any_element()
}

/// The empty-detail prompt: centred text at a 16 px inset.
pub fn detail_prompt(text: &'static str, theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .p(px(theme.scale.space_4))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(theme.text_secondary)
                .text_center()
                .child(text),
        )
        .into_any_element()
}

/// A section label above a group: `META` semibold `TEXT_SECONDARY`.
pub fn section_label(text: impl Into<SharedString>, theme: &AppTheme) -> Div {
    div()
        .flex_shrink_0()
        .px(px(theme.scale.space_4))
        .pt(px(theme.scale.space_3))
        .pb(px(theme.scale.space_2))
        .text_size(px(theme.scale.meta))
        .line_height(px(theme.scale.meta_line))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text_secondary)
        .child(text.into())
}

/// A centred two-line empty state that fills its pane: the fact in
/// `BODY_STRONG` and the next step in `META`, 8 apart inside the gutter.
pub fn state_message(
    fact: impl Into<SharedString>,
    hint: impl Into<SharedString>,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .size_full()
        .p(px(theme.scale.space_4))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .text_center()
                .child(fact.into()),
        )
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .text_center()
                .child(hint.into()),
        )
        .into_any_element()
}

/// A centred initial-load spinner.
pub fn loading_state(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(spinner(theme.accent, 36.))
        .into_any_element()
}
