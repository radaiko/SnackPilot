//! Desktop workspace chrome (desktop-ui-redesign brief "Workspace"):
//! the screen header, the master/detail split with its 1 px divider, the
//! action strip and the empty-detail prompt. All widths come from
//! `snackpilot_model::desktop::DesktopGeometry`; this module never derives
//! geometry itself, so the geometry tests in the model cover the numbers.
//! @req REQ-UI-07

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{tooltip::Tooltip, Icon, Sizable};
use snackpilot_model::desktop::DesktopGeometry;

use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// Header minimum height (brief: 56 px).
const HEADER_HEIGHT: f32 = 56.;
/// Action strip minimum height (brief: 56 px).
const STRIP_HEIGHT: f32 = 56.;
/// Gap between header/strip items (brief: 8 px).
const ITEM_GAP: f32 = 8.;
/// Touch target floor for the header actions (brief: 44 px).
const ACTION_TARGET: f32 = 44.;

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
        .px(px(16.))
        .bg(theme.desktop.panel)
        .child(
            div()
                .text_size(px(18.))
                .line_height(px(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.desktop.ink)
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
        .w(px(ACTION_TARGET))
        .h(px(ACTION_TARGET))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .rounded(px(8.))
        .text_color(theme.desktop.ink)
        .hover(|s| s.bg(theme.surface_variant))
        .focus(theme::desktop_focus(theme.desktop.focus))
        .on_click(move |_, window, cx| on_click(window, cx))
        .tooltip(|window, cx| Tooltip::new("Aktualisieren").build(window, cx))
        .child(
            Icon::new(IconName::RefreshCw)
                .with_size(px(20.))
                .text_color(theme.desktop.ink),
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
/// can reveal the row it focused (brief "reveal it"), and a focusable detail
/// region that Enter hands focus to when the preview offers no action. Only
/// the wide record lists need it; every other screen keeps the plain split.
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
            .focus(theme::desktop_focus(theme.desktop.focus));
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
/// second row when the pane is narrow, 1 px top border (brief "Toolbar /
/// action strip").
pub fn action_strip(items: Vec<AnyElement>, theme: &AppTheme) -> AnyElement {
    div()
        .min_h(px(STRIP_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(ITEM_GAP))
        .px(px(16.))
        .border_t_1()
        .border_color(theme.border)
        .children(items)
        .into_any_element()
}

/// The empty-detail prompt: centred text at a 16 px inset (brief: nothing is
/// selected, so the detail pane invites a selection).
pub fn detail_prompt(text: &'static str, theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .p(px(16.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .text_center()
                .child(text),
        )
        .into_any_element()
}
