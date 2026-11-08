//! Day navigator — docs/requirements/04-ui-ux.md §4.4.
//!
//! Menus-tab header: 48×48 `circleButton` arrows `❮`/`❯` (opacity 0.3 at the
//! ends of the date list), centre shows `format_day_short` 19/600 with
//! `"{i+1} / {count}"` 12 below; tapping the centre jumps to today.
//! iOS uses a blur wrapper in v1 — gpui has no blur primitive, so the
//! translucent `glass_surface` stands in (same approximation as the task 4.1
//! banners), with the 0.5 px `glassShadowEdge` bottom hairline.

use gpui::*;
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
