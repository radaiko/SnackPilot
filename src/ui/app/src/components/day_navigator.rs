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

/// The compact navigator's one control recipe, as data the builder paints: a
/// contained group on the projected mobile fill with the option radius and a
/// resolved boundary (never the full-bleed §4.4 band), each arrow a bare 48 px
/// target carrying the 20 px Lucide glyph instead of the legacy circle disc, and
/// a min-48 centre target with a 4 px gap under the headline date.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobileDayNavigator {
    /// Bare arrow target, both axes (48 px, above the 44 px floor).
    pub arrow: f32,
    /// Arrow glyph (Lucide 20).
    pub glyph: f32,
    /// The centre target's minimum height.
    pub centre_min: f32,
    /// Date-to-ordinal gap.
    pub gap: f32,
    /// The contained group's radius (the projected option radius).
    pub radius: f32,
    /// The group's resting fill (the projected card surface).
    pub fill: Hsla,
    /// The group's neutral boundary (the readable mobile secondary ink, at
    /// least 3:1 on that fill). It is never the accent: the navigator is
    /// chrome, and the accent belongs to the pending dock and real selections.
    pub boundary: Hsla,
}

pub fn mobile_day_navigator_recipe(theme: &AppTheme) -> MobileDayNavigator {
    MobileDayNavigator {
        arrow: theme.mobile.action_height,
        glyph: theme.mobile.space_5,
        centre_min: theme.mobile.action_height,
        gap: theme.mobile.space_1,
        radius: theme.mobile.radius_option,
        fill: theme.mobile_card_fill(),
        boundary: theme.mobile.secondary,
    }
}

/// Which ends of the day list are closed for the selected index: the model's
/// list is the one source, the first day closes back, the last closes forward,
/// and a selection that is not in the list falls back to the first day.
pub fn day_edge(dates_len: usize, index: usize) -> (bool, bool) {
    (index == 0, index + 1 >= dates_len)
}

/// The compact Day Tray day navigator (task 1.3): a contained control group
/// carrying bare 48 px arrows with the 20 px Lucide `ChevronLeft`/
/// `ChevronRight`, and a min-48 centre target holding the day in the headline
/// role with the ordinal in the metadata role 4 px below. The wide Ledger keeps
/// [`day_navigator`] above exactly as before; selection, the today intent, the
/// format and the disabled ends are unchanged.
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
    let meta = theme.mobile_secondary_on(recipe.fill);

    let arrow = |id: &'static str, icon: IconName, disabled: bool, on: ClickHandler| {
        let ink = if disabled {
            theme.mobile.disabled_ink
        } else {
            theme.text_primary
        };
        // A bare target: the hit box is the 48 px arrow itself, with no disc
        // behind it (the incumbent §4.4 `circle_button` stays on the Ledger).
        let button = div()
            .w(px(recipe.arrow))
            .h(px(recipe.arrow))
            .flex()
            .items_center()
            .justify_center()
            .child(Icon::new(icon).with_size(px(recipe.glyph)).text_color(ink))
            .id(SharedString::from(id))
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.mobile.focus))
            .cursor_pointer();
        let button = if disabled {
            button
        } else {
            button.on_click(move |_, window, cx| on(window, cx))
        };
        button.into_any_element()
    };

    // A contained group: the projected card fill, the option radius and the
    // resolved boundary, inset by the 4 px grid rather than bleeding across the
    // viewport. The shell supplies the outer 16 px gutter.
    let container = div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .p(px(recipe.gap))
        .rounded(px(recipe.radius))
        .border_1()
        .border_color(recipe.boundary)
        .bg(recipe.fill);

    let center = div()
        .id("day-today")
        .flex_1()
        .min_h(px(recipe.centre_min))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(recipe.gap))
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
    use super::{day_edge, mobile_day_navigator, mobile_day_navigator_recipe};
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_model::desktop::contrast_ratio;
    use snackpilot_model::tokens::mobile;
    use snackpilot_model::AccentColor;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The compact navigator is a contained control, not the incumbent
    /// full-bleed §4.4 band (REQ-SCR-01/REQ-UI-16): the projected option radius
    /// on the projected card fill with a boundary that reads at 3:1, bare 48 px
    /// arrow targets carrying the 20 px Lucide glyph instead of the legacy grey
    /// circle discs, and a min-48 centre target with a 4 px date/ordinal gap.
    /// The edge state stays the model's one list, and building the navigator
    /// activates nothing. The wide Ledger keeps its own `day_navigator`.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_day_navigator_is_a_contained_control_with_bare_arrow_targets() {
        for (dark, accent, flat) in [
            (false, AccentColor::Orange, false),
            (false, AccentColor::Orange, true),
            (true, AccentColor::Berry, false),
            (true, AccentColor::Golden, true),
        ] {
            let theme = AppTheme::build(dark, accent, flat);
            let recipe = mobile_day_navigator_recipe(&theme);

            // Contained: a rounded control on the card surface, not a square
            // band on the page background, with a boundary that is visible on
            // its own fill (the incumbent `border` does not reach 3:1).
            assert_eq!(recipe.radius, theme.mobile.radius_option);
            assert_eq!(recipe.radius, mobile::RADIUS_OPTION);
            assert_eq!(recipe.fill, theme.mobile_card_fill());
            assert_ne!(recipe.fill, theme.background, "the group is a surface");
            assert_ne!(recipe.boundary, recipe.fill);
            let fill = to_rgb(recipe.fill, recipe.fill);
            assert!(
                contrast_ratio(to_rgb(recipe.boundary, recipe.fill), fill)
                    >= mobile::CONTROL_MIN_CONTRAST,
                "the contained boundary reads on its fill"
            );
            assert_ne!(
                recipe.boundary, theme.primary,
                "the navigator is chrome: its boundary is neutral, not the accent"
            );

            // Bare targets: the 48 px arrow itself is the hit box, the glyph is
            // the 20 px Lucide icon, and no disc is painted behind it.
            assert_eq!(recipe.arrow, theme.mobile.action_height);
            assert!(recipe.arrow >= theme.mobile.target);
            assert_eq!(recipe.glyph, 20.);
            assert!(recipe.glyph < recipe.arrow);
            assert_eq!(recipe.centre_min, theme.mobile.action_height);
            assert_eq!(recipe.gap, mobile::SPACE_1);
            assert_eq!(recipe.gap, 4.);
        }

        // The edge state is the model's list: a single day closes both ends,
        // the first closes back only, the last closes forward only.
        assert_eq!(day_edge(1, 0), (true, true));
        assert_eq!(day_edge(3, 0), (true, false));
        assert_eq!(day_edge(3, 1), (false, false));
        assert_eq!(day_edge(3, 2), (false, true));

        // Building the navigator activates nothing: it only projects the day
        // the model already selected.
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
    }
}
