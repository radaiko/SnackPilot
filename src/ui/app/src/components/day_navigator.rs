//! Day navigator — docs/requirements/04-ui-ux.md §4.4.
//!
//! Menus-tab header: 48×48 `circleButton` arrows `❮`/`❯` (opacity 0.3 at the
//! ends of the date list), centre shows `format_day_short` 19/600 with
//! `"{i+1} / {count}"` 12 below; tapping the centre jumps to today.
//! One contained surface with a hairline boundary on every platform.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use snackpilot_model::format_day_short;

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
        .focus(crate::theme::desktop_focus(theme.accent))
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
    let container = container
        .bg(theme.surface)
        .border_b_1()
        .border_color(theme.border);

    let center = div()
        .id("day-today")
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .tab_index(0)
        .focus(crate::theme::desktop_focus(theme.accent))
        .cursor_pointer()
        .on_click(move |_, window, cx| on_today(window, cx))
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
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

/// The compact day row's one geometry, as data the builder paints: a 44 px
/// chevron target, the 20 px Lucide glyph, an 8 px gap and a min-44 centre
/// target holding the date in the headline role above the ordinal in the
/// metadata role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobileDayNavigator {
    /// Bare arrow target, both axes (44 px touch floor).
    pub arrow: f32,
    /// Arrow glyph (Lucide 20).
    pub glyph: f32,
    /// Date-to-ordinal and row gap.
    pub gap: f32,
    /// The centre target's minimum height.
    pub centre_min: f32,
}

pub fn mobile_day_navigator_recipe(theme: &AppTheme) -> MobileDayNavigator {
    MobileDayNavigator {
        arrow: theme.scale.target,
        glyph: theme.scale.icon,
        gap: theme.scale.space_2,
        centre_min: theme.scale.target,
    }
}

/// Which ends of the day list are closed for the selected index: the model's
/// list is the one source, the first day closes back, the last closes forward,
/// and a selection that is not in the list falls back to the first day.
pub fn day_edge(dates_len: usize, index: usize) -> (bool, bool) {
    (index == 0, index + 1 >= dates_len)
}

/// The compact Day Tray day row: surface, 1 px hairline, 12 radius, 16
/// padding, bare 44 px arrows carrying the 20 px Lucide `ChevronLeft`/
/// `ChevronRight`, and a min-44 centre target holding the day in the headline
/// role with the ordinal in the metadata role 8 px below. Selection, the today
/// intent, the formatter and the disabled ends are unchanged; the wide Ledger
/// keeps [`day_navigator`] above exactly as before.
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
    let (at_start, at_end) = day_edge(dates.len(), index);
    let recipe = mobile_day_navigator_recipe(theme);
    let meta = theme.text_secondary;

    let arrow = |id: &'static str, icon: IconName, disabled: bool, on: ClickHandler| {
        let ink = if disabled {
            theme.text_tertiary
        } else {
            theme.text_primary
        };
        let button = div()
            .w(px(recipe.arrow))
            .h(px(recipe.arrow))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(theme.scale.radius_chip))
            .child(Icon::new(icon).with_size(px(recipe.glyph)).text_color(ink))
            .id(SharedString::from(id))
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.accent));
        let button = if disabled {
            button
        } else {
            button
                .cursor_pointer()
                .hover(move |s| s.bg(theme.surface_alt))
                .on_click(move |_, window, cx| on(window, cx))
        };
        button.into_any_element()
    };

    // The row is one surface: the shell supplies the outer 16 px gutter.
    let container = theme::apply(
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(recipe.gap))
            .p(px(theme.scale.space_4)),
        &theme.card_surface(),
    );

    let center = div()
        .id("day-today")
        .flex_1()
        .min_w(px(0.))
        .min_h(px(recipe.centre_min))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(recipe.gap))
        .rounded(px(theme.scale.radius_chip))
        .tab_index(0)
        .cursor_pointer()
        .hover(move |s| s.bg(theme.surface_alt))
        .focus(crate::theme::desktop_focus(theme.accent))
        .on_click(move |_, window, cx| on_today(window, cx))
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
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
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
    use super::{day_edge, mobile_day_navigator, mobile_day_navigator_recipe};
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_model::desktop::contrast_ratio;
    use snackpilot_model::tokens;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The compact day row (REQ-SCR-01/REQ-UI-16): one surface row with the
    /// 1 px hairline, 12 radius and 16 padding, bare 44 px chevron targets
    /// carrying the 20 px Lucide glyph, and a min-44 centre target with an
    /// 8 px date/ordinal gap. The edge state stays the model's one list, and
    /// building the navigator activates nothing.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_day_navigator_is_one_row_with_44_targets() {
        for dark in [false, true] {
            let theme = AppTheme::build(dark);
            let recipe = mobile_day_navigator_recipe(&theme);

            assert_eq!(recipe.arrow, tokens::TARGET);
            assert_eq!(recipe.arrow, 44.);
            assert_eq!(recipe.centre_min, 44.);
            assert_eq!(recipe.glyph, 20.);
            assert_eq!(recipe.glyph, tokens::ICON);
            assert_eq!(recipe.gap, 8.);
            assert_eq!(recipe.gap, tokens::SPACE_2);

            // The row is the shared flat surface, not a full-bleed band.
            let row = theme.card_surface();
            assert_eq!(row.radius, gpui_kit::px(tokens::RADIUS_CARD));
            assert_eq!(row.border.map(|(w, _)| w), Some(gpui_kit::px(1.)));
            assert_eq!(theme.scale.space_4, 16.);
            // The hairline stays quieter than the text it separates.
            let fill = to_rgb(row.bg, row.bg);
            assert!(
                contrast_ratio(to_rgb(theme.border, row.bg), fill) < 3.,
                "the hairline is a quiet border, not a control boundary"
            );
        }

        assert_eq!(day_edge(1, 0), (true, true));
        assert_eq!(day_edge(3, 0), (true, false));
        assert_eq!(day_edge(3, 1), (false, false));
        assert_eq!(day_edge(3, 2), (false, true));

        let dates: Vec<String> = ["2026-11-16", "2026-11-17", "2026-11-18"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let theme = AppTheme::build(false);
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
    }
}
