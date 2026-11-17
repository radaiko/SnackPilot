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

use crate::components::swipe::SwipeHandlers;
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

/// How wide one day chip is drawn: five and a half of them fill a phone's
/// content width, so the strip shows that there is more and is swiped
/// sideways rather than wrapped.
pub const DAY_CHIP_W: f32 = 56.;

/// How far a finger must travel before the strip treats the gesture as a
/// swipe. A sloppy tap moves two or three pixels; below this the row stays
/// put and the chip under the finger still gets its tap.
pub const STRIP_SLOP: f32 = 20.;

/// One chip plus the gap after it: the step a settled strip lands on.
pub fn strip_step(theme: &AppTheme) -> f32 {
    DAY_CHIP_W + theme.scale.space_1
}

/// The strip's whole content width at `days` chips, its own side padding
/// included.
pub fn strip_content_width(days: usize, theme: &AppTheme) -> f32 {
    if days == 0 {
        return 0.;
    }
    let m = &theme.scale;
    m.space_4 * 2. + days as f32 * DAY_CHIP_W + (days - 1) as f32 * m.space_1
}

/// How far the strip may travel: the part of the row that does not fit.
pub fn strip_max_offset(days: usize, viewport: f32, theme: &AppTheme) -> f32 {
    (strip_content_width(days, theme) - viewport).max(0.)
}

/// What a live gesture moves the row by: nothing until the finger passes the
/// slop, then the travel beyond it, so the row starts from where it stood
/// instead of jumping by the slop.
pub fn strip_travel_offset(raw: f32) -> f32 {
    if raw.abs() <= STRIP_SLOP {
        0.
    } else {
        raw - STRIP_SLOP * raw.signum()
    }
}

/// What the strip is painted at: the settled offset, the gesture's travel
/// (past the slop) and any running settle, clamped to the row's own ends.
pub fn strip_painted(
    base: f32,
    raw: f32,
    settle: f32,
    days: usize,
    viewport: f32,
    theme: &AppTheme,
) -> f32 {
    (base + strip_travel_offset(raw) + settle).clamp(0., strip_max_offset(days, viewport, theme))
}

/// A settled strip sits on a chip boundary, never between two, and never past
/// either end.
pub fn strip_snap(offset: f32, days: usize, viewport: f32, theme: &AppTheme) -> f32 {
    let step = strip_step(theme);
    ((offset / step).round() * step).clamp(0., strip_max_offset(days, viewport, theme))
}

/// What a release hands the painter: the chip the row settles on, and the
/// settle offset to start from. Painted as `base + settle`, the row sits on the
/// pixel it was released at and only then eases to the chip — the release frame
/// never moves.
pub fn strip_release(painted: f32, days: usize, viewport: f32, theme: &AppTheme) -> (f32, f32) {
    let target = strip_snap(painted, days, viewport, theme);
    (target, painted - target)
}

/// The settled offset that puts chip `index` at the left edge of the visible
/// part.
pub fn strip_offset_for_index(index: usize, days: usize, viewport: f32, theme: &AppTheme) -> f32 {
    strip_snap(index as f32 * strip_step(theme), days, viewport, theme)
}

/// The compact day strip: every available day at once, weekday over date, the
/// selected one on an accent tint. A dot says what the day carries — filled
/// accent for a placed order, hollow accent for marked dishes, a quiet grey
/// dot for a day that was never opened. Today's date wears the accent. Tapping
/// a chip selects that day; the strip itself is swiped sideways.
///
/// The strip owns its travel instead of handing it to the platform scroller:
/// a slow release used to keep the row creeping for a second in one- and
/// two-pixel steps, and it could come to rest between two chips. The gesture
/// reports to the shell, which paints `offset` and settles it onto a chip
/// boundary, so a release always comes to rest on a day.
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub fn mobile_day_strip(
    dates: &[String],
    facts: &[DayFacts],
    selected: Option<&str>,
    today: &str,
    offset: f32,
    hover: bool,
    theme: &AppTheme,
    on_select: DaySelectHandler,
    handlers: SwipeHandlers,
) -> AnyElement {
    let m = &theme.scale;
    let mut strip = div()
        .id("day-strip")
        .flex()
        .gap(px(m.space_1))
        .px(px(m.space_4))
        .py(px(m.space_2));

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
        // Hover is a pointer cue. A phone paints it where the finger landed,
        // which read as the wrong chip being highlighted during a drag.
        if hover {
            chip = chip.hover(move |s| s.bg(theme.pressed(theme.background)));
        }
        if is_selected {
            chip = chip.bg(theme.accent_surface);
        }
        strip = strip.child(chip);
    }
    // The end padding has to be content, or the last chip scrolled into view
    // sits flush against the screen edge.
    strip = strip.child(div().w(px(m.space_4)).flex_shrink_0());

    let (scrolled, up) = (handlers.scrolled, handlers.up);
    div()
        .id("day-strip-swipe")
        .w_full()
        .overflow_hidden()
        .border_b_1()
        .border_color(theme.border)
        .on_scroll_wheel(move |e, w, cx| scrolled(e, w, cx))
        .on_mouse_up(MouseButton::Left, move |e, w, cx| up(e, w, cx))
        .child(div().w_full().ml(px(-offset)).child(strip))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        day_dot, mobile_day_strip, strip_max_offset, strip_offset_for_index, strip_painted,
        strip_release, strip_snap, strip_travel_offset, DayDot, DayFacts, DAY_CHIP_W, STRIP_SLOP,
    };
    use crate::components::swipe::SwipeHandlers;
    use crate::theme::AppTheme;
    use gpui::{App, MouseUpEvent, ScrollWheelEvent, Window};
    use snackpilot_model::tokens;
    use std::cell::Cell;
    use std::rc::Rc;

    fn dates() -> Vec<String> {
        ["2026-11-16", "2026-11-17", "2026-11-18", "2026-11-19"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    /// A touch that moves a few pixels is still a touch: the row does not
    /// begin to travel until the finger has passed 20 px, and then starts from
    /// where it stood, so nothing jumps by the slop.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn a_sloppy_touch_does_not_drag_the_strip() {
        let theme = AppTheme::build(false);
        let (days, viewport) = (6, 358.);
        assert_eq!(STRIP_SLOP, 20.);
        assert_eq!(strip_travel_offset(0.), 0.);
        assert_eq!(strip_travel_offset(12.), 0.);
        assert_eq!(strip_travel_offset(-19.), 0.);
        assert_eq!(strip_travel_offset(20.), 0.);
        assert_eq!(strip_travel_offset(25.), 5.);
        assert_eq!(strip_travel_offset(-25.), -5.);

        // Painted: base + travel past the slop + settle, clamped to the row.
        // Positive travel scrolls the row forward (towards later days).
        assert_eq!(strip_painted(0., 12., 0., days, viewport, &theme), 0.);
        assert_eq!(strip_painted(0., 25., 0., days, viewport, &theme), 5.);
        assert_eq!(
            strip_painted(0., 200., 0., days, viewport, &theme),
            30.,
            "a long drag stops at the row's end"
        );
        assert_eq!(strip_painted(30., 25., 0., days, viewport, &theme), 30.);
        assert_eq!(
            strip_painted(30., -25., 0., days, viewport, &theme),
            25.,
            "dragging back uncovers the row from its end"
        );
        assert_eq!(strip_painted(5., -25., 0., days, viewport, &theme), 0.);
    }

    /// The release frame does not move the row: base + settle is exactly the
    /// pixel it was released at, and the settle eases to zero so the strip
    /// lands on the chip instead of jumping there first.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn a_release_continues_from_the_pixel_it_was_let_go_at() {
        let theme = AppTheme::build(false);
        let (days, viewport) = (9, 268.);
        for painted in [0., 7., 40., 80., 119., 288., 300.] {
            let (base, settle) = strip_release(painted, days, viewport, &theme);
            assert_eq!(
                base + settle,
                painted,
                "{painted} px must not move on the release frame"
            );
            // Easing the settle to zero leaves the row on a chip boundary.
            assert_eq!(base, strip_snap(painted, days, viewport, &theme));
            assert_eq!(base % super::strip_step(&theme), 0.);
        }
        assert_eq!(strip_release(80., days, viewport, &theme), (60., 20.));
        assert_eq!(strip_release(100., days, viewport, &theme), (120., -20.));
    }

    /// The strip settles onto whole chips, never between two, and never past
    /// either end. A slow release used to leave the platform scroller creeping
    /// in one- and two-pixel steps; the app owns the travel now, and a release
    /// always lands on a day.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn the_strip_settles_on_whole_chips() {
        let theme = AppTheme::build(false);
        let viewport = 358.;
        let days = 6;
        // 16 + 6·56 + 5·4 + 16 = 388 content, so 30 px of travel at most.
        assert_eq!(super::strip_content_width(days, &theme), 388.);
        assert_eq!(strip_max_offset(days, viewport, &theme), 30.);
        assert_eq!(strip_max_offset(3, viewport, &theme), 0.);

        // The step is one chip plus its gap, and a released offset snaps to it.
        assert_eq!(super::strip_step(&theme), 60.);
        assert_eq!(strip_snap(0., days, viewport, &theme), 0.);
        assert_eq!(strip_snap(8., days, viewport, &theme), 0.);
        assert_eq!(strip_snap(40., days, viewport, &theme), 30.);
        assert_eq!(strip_snap(-30., days, viewport, &theme), 0.);

        // A wider strip: chips at 60 px steps, clamped at the end.
        let long = 9;
        let viewport = 268.;
        let max = strip_max_offset(long, viewport, &theme);
        assert_eq!(max, 300.);
        assert_eq!(strip_snap(118., long, viewport, &theme), 120.);
        assert_eq!(strip_snap(500., long, viewport, &theme), max);
        assert_eq!(strip_offset_for_index(2, long, viewport, &theme), 120.);
        assert_eq!(
            strip_offset_for_index(8, long, viewport, &theme),
            max,
            "the last chip never scrolls past the end"
        );
    }

    /// The compact day strip (REQ-SCR-01/REQ-UI-16): every available day at
    /// once on one row, so a week never wraps into a second block; five chips
    /// and their gaps fill most of the phone's content width, which is what
    /// tells a person the strip can be swiped sideways. Building the strip
    /// activates nothing.
    /// @req REQ-SCR-01 @req REQ-UI-16
    #[test]
    fn mobile_day_strip_shows_every_day_and_activates_nothing() {
        assert_eq!(DAY_CHIP_W, 56.);
        assert_eq!(tokens::SPACE_1, 4.);
        let content = 358. - 32.;
        assert!(
            DAY_CHIP_W * 6. + tokens::SPACE_1 * 5. > content,
            "six chips overflow the row: the strip scrolls instead of wrapping"
        );
        assert!(
            DAY_CHIP_W * 5. + tokens::SPACE_1 * 4. <= content,
            "five chips and their gaps fit, with the sixth cut off"
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
            0.,
            false,
            &theme,
            handler,
            SwipeHandlers {
                scrolled: Rc::new(|_: &gpui::ScrollWheelEvent, _: &mut Window, _: &mut App| {}),
                up: Rc::new(|_: &gpui::MouseUpEvent, _: &mut Window, _: &mut App| {}),
            },
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
