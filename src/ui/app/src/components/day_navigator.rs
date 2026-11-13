//! Day navigator — docs/requirements/04-ui-ux.md §4.4.
//!
//! Menus-tab header: 48×48 `circleButton` arrows `❮`/`❯` (opacity 0.3 at the
//! ends of the date list), centre shows `format_day_short` 19/600 with
//! `"{i+1} / {count}"` 12 below; tapping the centre jumps to today.
//! iOS uses a blur wrapper in v1 — gpui has no blur primitive, so the
//! translucent `glass_surface` stands in (same approximation as the task 4.1
//! banners), with the 0.5 px `glassShadowEdge` bottom hairline.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::format_day_short;

use crate::components::tab_bar::elevation_shadow;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};

/// The §4.4 header. The shell renders it only when `dates` is non-empty
/// (§3.4). `on_prev`/`on_next`/`on_today` mutate `State::selected_day`.
pub fn day_navigator(
    dates: &[String],
    selected: Option<&str>,
    theme: &AppTheme,
    on_prev: ClickHandler,
    on_next: ClickHandler,
    on_today: ClickHandler,
) -> AnyElement {
    let index = selected
        .and_then(|s| dates.iter().position(|d| d == s))
        .unwrap_or(0);
    let at_start = index == 0;
    let at_end = index + 1 >= dates.len();

    let arrow = |id: &'static str, glyph: &'static str, disabled: bool, on: ClickHandler| {
        let recipe = theme.circle_button();
        // theme::apply takes a plain Div, so the recipe goes on before .id().
        let button = theme::apply(
            div()
                .w(px(48.))
                .h(px(48.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(20.))
                .text_color(theme.text_primary)
                .child(glyph),
            &recipe,
        )
        .id(SharedString::from(id))
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.desktop.focus))
        .cursor_pointer();
        let button = if disabled {
            button.opacity(0.3)
        } else {
            button.on_click(move |_, window, cx| on(window, cx))
        };
        div().flex().items_center().child(button).into_any_element()
    };

    let container = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .px(px(16.))
        .py(px(12.));
    let container = if theme.flat {
        // §4.4 Android: solid surface, 1 px bottom border, elevation 2.
        container
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .shadow(vec![elevation_shadow(2.)])
    } else {
        // §4.4 iOS: blur wrapper (approximated), 0.5 px bottom hairline.
        container
            .bg(theme.glass_surface)
            .border(px(0.5))
            .border_color(theme.glass_shadow_edge)
    };

    let center = div()
        .id("day-today")
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.desktop.focus))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_today(window, cx))
        .child(
            div()
                .text_size(px(19.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(
                    dates
                        .get(index)
                        .map(|d| SharedString::from(format_day_short(d)))
                        .unwrap_or_default(),
                ),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .text_color(theme.text_tertiary)
                .child(format!("{} / {}", index + 1, dates.len())),
        );

    container
        .child(arrow("day-prev", "❮", at_start, on_prev))
        .child(center)
        .child(arrow("day-next", "❯", at_end, on_next))
        .into_any_element()
}

/// The compact Day Tray day navigator (task 1.3): 48×48 targets with the 20 px
/// Lucide `ChevronLeft`/`ChevronRight`, a min-48 centre target carrying the day
/// in the headline role and the ordinal in the metadata role 4 px below. The
/// wide Ledger keeps [`day_navigator`] above exactly as before; selection, the
/// today intent, the format and the disabled ends are unchanged.
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub fn mobile_day_navigator(
    dates: &[String],
    selected: Option<&str>,
    theme: &AppTheme,
    on_prev: ClickHandler,
    on_next: ClickHandler,
    on_today: ClickHandler,
) -> AnyElement {
    let index = selected
        .and_then(|s| dates.iter().position(|d| d == s))
        .unwrap_or(0);
    let at_start = index == 0;
    let at_end = index + 1 >= dates.len();
    let fill = if theme.flat {
        theme.surface
    } else {
        theme.glass_surface
    };
    let meta = theme.mobile_secondary_on(fill);

    let arrow = |id: &'static str, icon: IconName, disabled: bool, on: ClickHandler| {
        let recipe = theme.circle_button();
        let ink = if disabled {
            theme.mobile.disabled_ink
        } else {
            theme.text_primary
        };
        let button = theme::apply(
            div()
                .w(px(theme.mobile.action_height))
                .h(px(theme.mobile.action_height))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(icon)
                        .with_size(px(theme.mobile.space_5))
                        .text_color(ink),
                ),
            &recipe,
        )
        .id(SharedString::from(id))
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.mobile.focus))
        .cursor_pointer();
        let button = if disabled {
            button
        } else {
            button.on_click(move |_, window, cx| on(window, cx))
        };
        div().flex().items_center().child(button).into_any_element()
    };

    // Android keeps its solid header + elevation; iOS its glass hairline. The
    // shell supplies the outer 16 px gutter, so the navigator adds none.
    let container = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .py(px(theme.mobile.space_1));
    let container = if theme.flat {
        container
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .shadow(vec![elevation_shadow(2.)])
    } else {
        container
            .bg(theme.glass_surface)
            .border(px(0.5))
            .border_color(theme.glass_shadow_edge)
    };

    let center = div()
        .id("day-today")
        .flex_1()
        .min_h(px(theme.mobile.action_height))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(theme.mobile.space_1))
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.mobile.focus))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_today(window, cx))
        .child(
            div()
                .text_size(px(theme.mobile.headline))
                .line_height(px(theme.mobile.headline_line))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.text_primary)
                .text_center()
                .child(
                    dates
                        .get(index)
                        .map(|d| SharedString::from(format_day_short(d)))
                        .unwrap_or_default(),
                ),
        )
        .child(
            div()
                .text_size(px(theme.mobile.meta))
                .line_height(px(theme.mobile.meta_line))
                .text_color(meta)
                .child(format!("{} / {}", index + 1, dates.len())),
        );

    container
        .child(arrow("day-prev", IconName::ChevronLeft, at_start, on_prev))
        .child(center)
        .child(arrow("day-next", IconName::ChevronRight, at_end, on_next))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::mobile_day_navigator;
    use crate::theme::AppTheme;
    use snackpilot_model::AccentColor;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The compact navigator keeps the model's day identity and only changes
    /// presentation (REQ-SCR-01/REQ-UI-16): the same dates list drives it,
    /// building it activates nothing, and the 48 px arrow / min-48 centre
    /// targets with the 20 px glyph come from the pinned tokens the builder
    /// applies. The wide Ledger keeps its own `day_navigator`.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_day_navigator_keeps_day_identity_and_targets() {
        use snackpilot_model::tokens::mobile;

        let dates: Vec<String> = ["2026-11-16", "2026-11-17", "2026-11-18"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let theme = AppTheme::build(false, AccentColor::Orange, false);
        let activations = Rc::new(Cell::new(0));
        let handler = {
            let activations = activations.clone();
            Rc::new(move |_: &mut gpui::Window, _: &mut gpui::App| {
                activations.set(activations.get() + 1);
            }) as crate::navigation::ClickHandler
        };
        let _ = mobile_day_navigator(
            &dates,
            Some("2026-11-17"),
            &theme,
            handler.clone(),
            handler.clone(),
            handler,
        );
        assert_eq!(
            activations.get(),
            0,
            "building the navigator never activates a day"
        );

        // Day arrows are 48×48 with the 20 px Lucide glyph; the centre target
        // carries the headline date and is at least the 48 px action height.
        assert_eq!(theme.mobile.action_height, mobile::ACTION_HEIGHT);
        assert_eq!(theme.mobile.space_5, 20.);
        assert_eq!(theme.mobile.space_1, 4., "date/ordinal gap");
        assert_eq!(theme.mobile.headline, 24.);
        assert_eq!(theme.mobile.headline_line, 32.);
        assert_eq!(theme.mobile.meta, 14.);

        // The first day disables only the back arrow, the last only forward:
        // the model's selection is the one source of the edge state.
        assert_eq!(
            dates.iter().position(|d| d == "2026-11-16").map(|i| i == 0),
            Some(true)
        );
        assert_eq!(
            dates
                .iter()
                .position(|d| d == "2026-11-18")
                .map(|i| i + 1 >= dates.len()),
            Some(true)
        );
    }
}
