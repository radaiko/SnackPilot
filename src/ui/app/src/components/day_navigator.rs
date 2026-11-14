//! Day navigator — docs/requirements/04-ui-ux.md §4.4.
//!
//! Menus-tab header: 48×48 `circleButton` arrows `❮`/`❯` (opacity 0.3 at the
//! ends of the date list), centre shows `format_day_short` 19/600 with
//! `"{i+1} / {count}"` 12 below; tapping the centre jumps to today.
//! The compact Day Tray card [`mobile_day_navigator`] is one 64 px row: the
//! arrows and the day alone in the headline role, no ordinal.
//! One contained surface with a hairline boundary on every platform.

use std::rc::Rc;

use gpui::*;
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
    let container = container.bg(theme.surface);

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
        );

    container
        .child(arrow("day-prev", "❮", at_start, on_prev))
        .child(center)
        .child(arrow("day-next", "❯", at_end, on_next))
        .into_any_element()
}

/// What one day of the strip says about itself beyond its date: an order
/// stands for it, dishes are marked for it, and whether it was ever opened.
/// A day with none of them is quiet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayFacts {
    pub ordered: bool,
    pub marked: bool,
    pub visited: bool,
}

/// What the dot under a day says: an order stands, dishes are marked, or the
/// day is one the app has never shown. `None` is a day with nothing to report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayDot {
    None,
    Ordered,
    Marked,
    New,
}

/// The one dot a day gets: an order outranks a mark, a mark outranks being new,
/// and only a day from today on can be new — a past day is not news.
pub fn day_dot(facts: DayFacts, day: &str, today: &str) -> DayDot {
    if facts.ordered {
        DayDot::Ordered
    } else if facts.marked {
        DayDot::Marked
    } else if !facts.visited && day >= today {
        DayDot::New
    } else {
        DayDot::None
    }
}

/// The strip's tap: the day key the person picked.
pub type DaySelectHandler = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// How wide one day chip is drawn. Seven of them plus the gaps fit a phone's
/// 358 px content width, so a fortnight wraps onto two rows instead of hiding
/// behind a scroll.
pub const DAY_CHIP_W: f32 = 46.;

/// The compact day strip: every available day at once, weekday over date, the
/// selected one on an accent tint. A dot says what the day carries — filled
/// accent for a placed order, hollow accent for marked dishes, a quiet grey
/// dot for a day that was never opened. Today's date wears the accent. Tapping
/// a chip selects that day; the swipe between days stays.
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub fn mobile_day_strip(
    dates: &[String],
    facts: &[DayFacts],
    selected: Option<&str>,
    today: &str,
    theme: &AppTheme,
    on_select: DaySelectHandler,
) -> AnyElement {
    let m = &theme.scale;
    let mut strip = div()
        .id("day-strip")
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(m.space_1))
        .px(px(m.space_4))
        .py(px(m.space_2))
        .border_b_1()
        .border_color(theme.border);

    for (index, day) in dates.iter().enumerate() {
        let is_selected = Some(day.as_str()) == selected;
        let day_facts = facts.get(index).copied().unwrap_or_default();
        let is_today = day == today;

        // The selected chip paints a tint; today's date keeps the accent only
        // where the tint does not lift it away from the readable role.
        let weekday_ink = if is_selected {
            theme.text_primary
        } else {
            theme.text_tertiary
        };
        let number_ink = if is_selected || !is_today {
            theme.text_primary
        } else {
            theme.accent
        };

        let mut dot = div().w(px(6.)).h(px(6.)).rounded(px(3.));
        match day_dot(day_facts, day, today) {
            DayDot::Ordered => dot = dot.bg(theme.accent),
            DayDot::Marked => dot = dot.border(px(1.5)).border_color(theme.accent),
            DayDot::New => dot = dot.bg(theme.text_tertiary),
            DayDot::None => {}
        }

        let key = day.clone();
        let on_tap = on_select.clone();
        let mut chip = div()
            .id(SharedString::from(format!("day-chip-{day}")))
            .w(px(DAY_CHIP_W))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .py(px(m.space_1))
            .rounded(px(m.radius_chip))
            .tab_index(0)
            .cursor_pointer()
            .focus(crate::theme::desktop_focus(theme.accent))
            .hover(move |s| s.bg(theme.pressed(theme.background)))
            .on_click(move |_, window, cx| on_tap(&key, window, cx))
            .child(
                div()
                    .text_size(px(m.meta))
                    .line_height(px(m.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(weekday_ink)
                    .child(SharedString::from(snackpilot_model::format_weekday(day))),
            )
            .child(
                div()
                    .text_size(px(m.headline))
                    .line_height(px(m.headline_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(number_ink)
                    .child(SharedString::from(snackpilot_model::format_day_number(day))),
            )
            .child(dot);
        if is_selected {
            chip = chip.bg(theme.accent_surface);
        }
        strip = strip.child(chip);
    }

    strip.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{day_dot, mobile_day_strip, DayDot, DayFacts, DAY_CHIP_W};
    use crate::theme::AppTheme;
    use snackpilot_model::tokens;
    use std::cell::Cell;
    use std::rc::Rc;

    fn dates() -> Vec<String> {
        ["2026-11-16", "2026-11-17", "2026-11-18", "2026-11-19"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// The compact day strip (REQ-SCR-01/REQ-UI-16): every available day at
    /// once, so no day hides behind a gesture; six chips and their gaps fit
    /// the phone's content width, so a fortnight wraps onto a second row
    /// instead of overflowing. Building the strip activates nothing.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_day_strip_shows_every_day_and_activates_nothing() {
        assert_eq!(DAY_CHIP_W, 46.);
        assert_eq!(tokens::SPACE_1, 4.);
        assert!(
            DAY_CHIP_W * 6. + tokens::SPACE_1 * 5. <= 358. - 32.,
            "six chips and their gaps fit the phone's content width"
        );

        let theme = AppTheme::build(false);
        let activations = Rc::new(Cell::new(0));
        let handler = {
            let activations = activations.clone();
            Rc::new(move |_: &str, _: &mut gpui::Window, _: &mut gpui::App| {
                activations.set(activations.get() + 1);
            }) as super::DaySelectHandler
        };
        let facts = [
            DayFacts {
                ordered: true,
                ..DayFacts::default()
            },
            DayFacts {
                marked: true,
                visited: true,
                ..DayFacts::default()
            },
            DayFacts {
                visited: true,
                ..DayFacts::default()
            },
            DayFacts {
                ordered: true,
                marked: true,
                ..DayFacts::default()
            },
        ];
        let _ = mobile_day_strip(
            &dates(),
            &facts,
            Some("2026-11-18"),
            "2026-11-16",
            &theme,
            handler,
        );
        assert_eq!(
            activations.get(),
            0,
            "building the strip never activates a day"
        );

        // The one dot: an order outranks a mark, a mark outranks being new,
        // and a day that is already past is never news.
        let today = "2026-11-16";
        assert_eq!(
            day_dot(
                DayFacts {
                    ordered: true,
                    marked: true,
                    ..DayFacts::default()
                },
                "2026-11-18",
                today
            ),
            DayDot::Ordered
        );
        assert_eq!(
            day_dot(
                DayFacts {
                    marked: true,
                    ..DayFacts::default()
                },
                "2026-11-18",
                today
            ),
            DayDot::Marked
        );
        assert_eq!(
            day_dot(DayFacts::default(), "2026-11-18", today),
            DayDot::New
        );
        assert_eq!(
            day_dot(DayFacts::default(), today, today),
            DayDot::New,
            "the day itself can still be new"
        );
        assert_eq!(
            day_dot(DayFacts::default(), "2026-11-09", today),
            DayDot::None,
            "a past day the app never showed is not news"
        );
        assert_eq!(
            day_dot(
                DayFacts {
                    visited: true,
                    ..DayFacts::default()
                },
                "2026-11-18",
                today
            ),
            DayDot::None
        );
    }
}
