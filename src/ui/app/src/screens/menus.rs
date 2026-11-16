//! Menus tab — docs/requirements/04-ui-ux.md §3.1.
//!
//! Task 1.3 reworks the compact Day Tray screen: the day navigator, the
//! neutral refresh row, the inline error, the dish-first grouped cards and the
//! 50 px day swipe. The pending dock lives in the shell, which reserves its
//! measured height above platform navigation. The wide Desktop Ledger path in
//! this module is unchanged. Every value comes from `AppModel`; the only
//! view-local state is the transient drag offset stored on [`AppShell`].

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;

use crate::app::{desktop_mutation_blocked, run_blocking, AppShell};
use crate::components::button::desktop_action_state;
use crate::components::day_navigator::{mobile_day_strip, DayFacts, DaySelectHandler};
use crate::components::fab::{dock_bottom, dock_height, dock_state, mobile_dock, DockState};
use crate::components::menu_card::{
    desktop_menu_action, desktop_menu_status, menu_card, mobile_menu_tappable, DesktopMenuRowState,
    MenuCardState,
};
use crate::components::record_table::{
    detail_entry, record_cell_right, record_row_focusable, record_status_cell, RecordListFocus,
};
use crate::components::refresh_bar::refresh_bar;
use crate::components::spinner::spinner;
use crate::components::swipe::{
    add_travel, claims_horizontal, is_swipe_delta, swipe_day, swipe_days, swipe_offset,
    SwipeHandlers,
};
use crate::components::workspace::{
    action_strip, detail_prompt, loading_state as desktop_loading_state, section_label,
    split_workspace_scrolled, state_message,
};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::{MenuCategory, MenuItem};
use snackpilot_model::desktop::{desktop_geometry, ControlState, HostKind};
use snackpilot_model::navigation::ListKind;
use snackpilot_model::tokens;
use snackpilot_model::{
    category_heading, day_key_from_epoch_ms, format_day_short, menu_key, today_key, AppModel, State,
};

/// The compact Menüs screen below the shell heading (task 1.3): the Day Tray
/// day navigator, the neutral refresh row, the inline error, and the swipeable
/// dish list. The pending dock is placed by the shell, which owns its measured
/// height above platform navigation; nothing here adds a second inset.
/// @req REQ-SCR-01 @req REQ-UI-15
#[allow(clippy::too_many_arguments)]
pub fn menus_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    offset: f32,
    page_width: f32,
    scroll: &ScrollHandle,
    days_scroll: &ScrollHandle,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if menus_gate(
        state.gourmet_authenticated,
        state.launch_settled,
        has_cached_menus(state),
    ) {
        return auth_gate(theme);
    }

    let mut column = div().size_full().flex().flex_col();

    // The screen's first row is the day navigator: no page title above it —
    // the tab bar already names the destination.
    let mut day_row = div().w_full().flex().items_center();
    if let Some(snapshot) = &state.menu_snapshot {
        if !snapshot.available_dates.is_empty() {
            let dates = &snapshot.available_dates;
            let selected = state.selected_day.clone();
            day_row = day_row.child(div().w_full().child(mobile_day_strip(
                dates,
                &day_facts(state, dates),
                selected.as_deref(),
                &today_key(),
                days_scroll,
                theme,
                day_select_handler(cx),
            )));
        }
    }
    column = column.child(day_row);

    // The quiet line is the only progress a cached Menüs shows while a
    // background refresh keeps its cards in place.
    if state.order_progress.is_none()
        && state
            .menu_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.refreshing)
    {
        column = column.child(refresh_line(theme));
    }
    let host = state.host_kind;
    if let Some(error) = &state.error_text {
        column = column.child(mobile_error_banner(
            &error.message,
            retry_handler(host, cx),
            theme,
        ));
    }

    let busy = state.busy;
    let (dates, day) = match (&state.menu_snapshot, state.selected_day.as_deref()) {
        (Some(snapshot), Some(day)) if !snapshot.available_dates.is_empty() => {
            (&snapshot.available_dates, day.to_string())
        }
        _ => {
            // Nothing to page between yet: the plain list, same as before.
            let content = match state.selected_day.as_deref() {
                Some(day) => menu_list(state, model, day, busy, theme, Some(scroll), cx),
                None => empty_state(theme),
            };
            return column
                .child(div().flex_1().min_h_0().child(content))
                .into_any_element();
        }
    };
    let (prev, next) = neighbours(dates, &day);
    let can_back = prev.is_some();
    let can_forward = next.is_some();
    let reduce_motion = cx.reduce_motion();
    let travel = carousel_travel(
        swipe_offset(offset, can_back, can_forward, reduce_motion),
        page_width,
    );

    // The day carousel: the day before, the day on screen and the day after,
    // side by side in one clipped row. Dragging slides the row, so the next day
    // comes in under the finger instead of the list just shifting into a blank
    // edge; the release moves the selection by one, and the neighbour that was
    // arriving is centred — the same pixels, so there is no jump.
    // Widths are percentages of the clip, so the carousel does not depend on a
    // measured pixel width: the row is three pages wide and starts one page to
    // the left, which puts the day on screen exactly in view.
    let mut row = div()
        .flex()
        .flex_row()
        .w(relative(3.))
        .h_full()
        .ml(relative(-1.));
    for (slot, other) in [
        (0, prev.as_deref()),
        (1, Some(day.as_str())),
        (2, next.as_deref()),
    ] {
        let pane = match other {
            Some(other_day) => {
                let tracked = (slot == 1).then_some(scroll);
                menu_list(state, model, other_day, busy, theme, tracked, cx)
            }
            None => div().into_any_element(),
        };
        row = row.child(
            div()
                .w(relative(1. / 3.))
                .opacity(page_fade(travel, slot, page_width))
                .h_full()
                .min_h(px(0.))
                .flex_shrink_0()
                .overflow_hidden()
                .child(pane),
        );
    }
    let content = div()
        .size_full()
        .overflow_hidden()
        .child(div().size_full().ml(px(travel)).child(row))
        .into_any_element();

    // The swipe surface takes only the height left under the day navigator;
    // a size_full child there pushed the list under the shell's bottom edge and
    // clipped the last row.
    column
        .child(div().flex_1().min_h_0().child(swipe_days(
            "menus-swipe",
            content,
            swipe_handlers(can_back, can_forward, page_width, cx),
        )))
        .into_any_element()
}

/// The days either side of `day` in the published week, for the carousel.
pub fn neighbours(dates: &[String], day: &str) -> (Option<String>, Option<String>) {
    match dates.iter().position(|d| d == day) {
        Some(index) => (
            index.checked_sub(1).and_then(|i| dates.get(i)).cloned(),
            dates.get(index + 1).cloned(),
        ),
        None => (None, None),
    }
}

/// One page's opacity as the carousel slides. A page is solid where it belongs
/// and fades exactly as far as it is out of place, so the two pages of a
/// transition always add up to one page of ink: the arriving day reads as a
/// fade-in, and nothing pops when the release makes it the middle pane.
pub fn page_fade(travel: f32, slot: usize, width: f32) -> f32 {
    let width = width.max(1.);
    let offset = travel + (slot as f32 - 1.) * width;
    (1. - offset.abs() / width).clamp(0., 1.)
}

/// How far the carousel row may travel: exactly one page either way, so the
/// neighbour can arrive fully but the row never shows the page beyond it.
pub fn carousel_travel(offset: f32, width: f32) -> f32 {
    offset.clamp(-width.max(1.), width.max(1.))
}

/// What each available day carries, in the strip's order: whether an order
/// stands for it, whether dishes are marked for it, and whether the app has
/// ever shown it. A day nobody opened is how a newly published week announces
/// itself.
fn day_facts(state: &State, dates: &[String]) -> Vec<DayFacts> {
    let ordered: Vec<String> = state
        .orders_split
        .as_ref()
        .map(|split| {
            split
                .upcoming
                .iter()
                .chain(split.past.iter())
                .map(|order| day_key_from_epoch_ms(order.date_epoch_ms))
                .collect()
        })
        .unwrap_or_default();
    // Pending keys are `id|day`; the day part says which chip carries a mark.
    let marked: Vec<&str> = state
        .menu_snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .pending_orders
                .iter()
                .filter_map(|key| key.split_once('|').map(|(_, day)| day))
                .collect()
        })
        .unwrap_or_default();
    dates
        .iter()
        .map(|day| DayFacts {
            ordered: ordered.iter().any(|o| o == day),
            marked: marked.iter().any(|m| m == day),
            visited: state.visited_days.iter().any(|seen| seen == day),
        })
        .collect()
}

/// Tapping a chip selects that day; nothing is refetched and no request is
/// added, exactly as the incumbent shape did.
fn day_select_handler(cx: &mut Context<AppShell>) -> DaySelectHandler {
    let shell = cx.entity().clone();
    Rc::new(move |day: &str, _: &mut Window, cx: &mut App| {
        let day = day.to_string();
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.select_day(&day);
            }
            cx.notify();
        });
    })
}

/// The dock's content for the current state, under exactly the incumbent
/// pending/progress conditions, with the label taken verbatim from the model.
fn pending_dock(state: &State, model: Option<&Arc<AppModel>>) -> Option<DockState> {
    if !state.gourmet_authenticated {
        return None;
    }
    let model = model?;
    let pending = state
        .menu_snapshot
        .as_ref()
        .map(|s| s.pending_orders.len() + s.pending_cancellations.len())
        .unwrap_or(0);
    dock_state(pending, state.order_progress.as_deref(), model.fab_label())
}

/// The height the shell reserves for the dock above navigation: the dock
/// recipe measured at the Day Tray column width (the label shaped at the
/// current metrics), `0` when no dock is present.
pub(crate) fn menus_dock_height(
    state: &State,
    model: Option<&Arc<AppModel>>,
    column_width: f32,
    theme: &AppTheme,
    text_system: &WindowTextSystem,
) -> f32 {
    pending_dock(state, model)
        .map(|dock| dock_height(&dock, column_width, theme, text_system))
        .unwrap_or(0.)
}

/// The compact pending dock, painted by the shell in the band it reserved
/// above platform navigation (8 px clear). It submits the same all-days set as
/// the incumbent FAB through the shell's one synchronous gate; during an
/// operation it shows the existing phase and offers no repeat action.
/// @req REQ-SCR-01 @req REQ-UI-15 @req REQ-UI-16
pub(crate) fn menus_dock(
    state: &State,
    model: Option<&Arc<AppModel>>,
    column_width: f32,
    measured_height: f32,
    navigation_top: f32,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Option<AnyElement> {
    let dock = pending_dock(state, model)?;
    let handler = dock
        .is_action()
        .then(|| submit_handler(state.host_kind, cx));
    Some(
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(dock_bottom(navigation_top)))
            .flex()
            .justify_center()
            .child(div().w(px(column_width)).child(mobile_dock(
                dock,
                measured_height,
                handler,
                theme,
            )))
            .into_any_element(),
    )
}

/// Grouped, scrollable cards for the selected day. The shell owns the column
/// gutter and the measured bottom reservation, so this list adds neither: no
/// legacy 16/100 px inset remains on the Menüs screen.
/// @req REQ-SCR-01 @req REQ-UI-15
fn menu_list(
    state: &State,
    model: Option<&Arc<AppModel>>,
    day: &str,
    busy: bool,
    theme: &AppTheme,
    scroll: Option<&ScrollHandle>,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(model) = model else {
        return empty_state(theme);
    };

    let groups = model.categories_for_day(day);
    if groups.is_empty() {
        let cached = state
            .menu_snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.items.is_empty());
        return if busy && !cached {
            loading_state(theme)
        } else {
            empty_state(theme)
        };
    }

    let (pending_orders, pending_cancellations) = state
        .menu_snapshot
        .as_ref()
        .map(|s| (s.pending_orders.clone(), s.pending_cancellations.clone()))
        .unwrap_or_default();

    let mut list = div()
        .id(SharedString::from(format!("menus-list-{day}")))
        .w_full()
        .size_full()
        .flex()
        .flex_col()
        .pt(px(theme.scale.space_4))
        .overflow_y_scroll();
    if let Some(scroll) = scroll {
        list = list.track_scroll(scroll);
    }

    for (category, items) in groups {
        list = list.child(group(
            category,
            items,
            &pending_orders,
            &pending_cancellations,
            model,
            theme,
            cx,
        ));
    }
    list.into_any_element()
}

/// One category group: the quiet metadata label 8 px above its rows and 16 px
/// after the group. The heading carries the MENÜ label, so the rows never
/// repeat it (sample M2); SUPPE & SALAT stays suppressed by the model.
/// @req REQ-SCR-01 @req REQ-UI-15
fn group(
    category: MenuCategory,
    items: Vec<MenuItem>,
    pending_orders: &[String],
    pending_cancellations: &[String],
    model: &Arc<AppModel>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Div {
    let mut group = div().w_full().flex().flex_col().mb(px(theme.scale.space_4));
    if let Some(heading) = category_heading(category) {
        group = group.child(
            div()
                .mb(px(theme.scale.space_2))
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(heading),
        );
    }
    for (index, item) in items.into_iter().enumerate() {
        if index > 0 {
            group = group.child(div().h(px(1.)).bg(theme.border));
        }
        let key = format!("{}|{}", item.id, item.day);
        let state = MenuCardState {
            selected: pending_orders.contains(&key),
            pending_cancel: pending_cancellations.contains(&key),
            ordered: model.is_ordered(&item),
            available: item.available,
            cutoff: model.is_cutoff(&item.day),
        };
        let handler = mobile_menu_tappable(&state).then(|| tap_handler(cx, item.clone()));
        group = group.child(menu_card(&item, state, handler, theme));
    }
    group
}

const EMPTY_COPY: (&str, &str) = (
    "Keine Menüs für diesen Tag",
    "Wähle mit den Pfeilen einen anderen Tag",
);

const AUTH_COPY: (&str, &str) = (
    "Nicht angemeldet",
    "Gehe zu Einstellungen, um Zugangsdaten einzugeben",
);

fn empty_copy() -> (&'static str, &'static str) {
    EMPTY_COPY
}

fn auth_copy() -> (&'static str, &'static str) {
    AUTH_COPY
}

/// One centred two-line empty state, no card.
fn empty_state(theme: &AppTheme) -> AnyElement {
    let (fact, hint) = empty_copy();
    state_message(fact, hint, theme)
}

/// Whether the Menüs screen shows the credential gate.
///
/// A launch signs in with the credentials saved on the device, and that takes
/// as long as the store takes to answer. The week is already on disk, so the
/// screen shows it and lets the sign-in run behind: the gate is for an
/// unauthenticated app that has nothing to show, or one whose restore has
/// settled without a session.
pub fn menus_gate(authenticated: bool, launch_settled: bool, has_cached_items: bool) -> bool {
    !authenticated && (launch_settled || !has_cached_items)
}

/// Whether the cached week has anything to paint.
fn has_cached_menus(state: &State) -> bool {
    state
        .menu_snapshot
        .as_ref()
        .is_some_and(|snapshot| !snapshot.items.is_empty())
}

/// The initial load: one quiet centred 20 px spinner and nothing else.
fn loading_state(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(theme.scale.space_4))
        .child(spinner(theme.text_secondary, theme.scale.icon))
        .into_any_element()
}

/// The cached refresh keeps the day and rows: a 2 px progress bar over the
/// existing `Aktualisiere...` metadata line, never a decorative card.
fn refresh_line(theme: &AppTheme) -> AnyElement {
    div()
        .w_full()
        .px(px(theme.scale.space_4))
        .py(px(theme.scale.space_2))
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_1))
        .child(refresh_bar(theme, "menus-refresh-bar"))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child("Aktualisiere..."),
        )
        .into_any_element()
}

fn auth_gate(theme: &AppTheme) -> AnyElement {
    let (fact, hint) = auth_copy();
    state_message(fact, hint, theme)
}

/// The compact inline error: one centred metadata line with the exact store
/// message, and the existing `Erneut versuchen` target at the 44 px floor.
/// @req REQ-SCR-01 @req REQ-UI-16
fn mobile_error_banner(message: &str, retry: ClickHandler, theme: &AppTheme) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(theme.scale.space_2))
        .p(px(theme.scale.space_4))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .text_center()
                .child(message.to_string()),
        )
        .child(
            div()
                .id("menus-retry")
                .min_h(px(theme.scale.target))
                .px(px(theme.scale.space_2))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .on_click(move |_, window, cx| retry(window, cx))
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.accent)
                        .child("Erneut versuchen"),
                ),
        )
        .into_any_element()
}

fn refresh_banner(theme: &AppTheme) -> AnyElement {
    div()
        .mx(px(16.))
        .mt(px(8.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(refresh_bar(theme, "desktop-refresh-bar"))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::NORMAL)
                .text_color(theme.accent)
                .child("Aktualisiere..."),
        )
        .into_any_element()
}

fn progress_banner(progress: &str, theme: &AppTheme) -> AnyElement {
    let r = theme.button_primary();
    theme::apply(
        div()
            .mx(px(16.))
            .mt(px(8.))
            .py(px(10.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(spinner(theme.on_accent, 12.))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.on_accent)
                    .child(progress.to_string()),
            ),
        &r,
    )
    .into_any_element()
}

fn error_banner(message: &str, retry: ClickHandler, theme: &AppTheme) -> AnyElement {
    let r = theme.tinted_banner(theme.error_surface);
    theme::apply(
        div()
            .mx(px(16.))
            .mt(px(8.))
            .p(px(12.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.error)
                    .child(message.to_string()),
            )
            .child(
                div()
                    .id("menus-retry")
                    .cursor_pointer()
                    .on_click(move |_, window, cx| retry(window, cx))
                    .child(
                        div()
                            .text_size(px(theme.scale.meta))
                            .line_height(px(theme.scale.meta_line))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme.error)
                            .child("Erneut versuchen"),
                    ),
            ),
        &r,
    )
    .into_any_element()
}

fn retry_handler(host: HostKind, cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            run_blocking(host, cx, move || model.refresh_menus());
            cx.notify();
        });
    })
}

/// The all-days submit of the narrow shell. Off-thread on every desktop host,
/// the narrow window included, under the shell's synchronous gate so a double
/// activation cannot submit twice before the model reports `busy`.
fn submit_handler(host: HostKind, cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            let state = model.state();
            let Some(ticket) = s
                .mutation_gate
                .claim(state.busy, state.order_progress.is_some())
            else {
                return;
            };
            run_blocking(host, cx, move || {
                let _ticket = ticket;
                model.submit_orders()
            });
            cx.notify();
        });
    })
}

fn tap_handler(cx: &mut Context<AppShell>, item: MenuItem) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.toggle(&item);
                cx.notify();
            }
        });
    })
}

/// Drag handlers. The gesture lives for one touch: `Started` begins it,
/// `Moved` travels, `Ended` decides. Only deltas that move further across than
/// down count, so the list's own vertical scrolling never moves the day.
///
/// The phase gate is what makes this work on a device: after the finger lifts,
/// the host keeps replaying the fling as further `Moved` deltas (its momentum
/// scroller runs on the display link). Those are not the gesture any more —
/// counting them switched the day a second time, or left the content shifted
/// with no release left to clear it.
fn swipe_handlers(
    can_back: bool,
    can_forward: bool,
    page_width: f32,
    cx: &mut Context<AppShell>,
) -> SwipeHandlers {
    let shell = cx.entity().clone();
    let scrolled = Rc::new(move |e: &ScrollWheelEvent, _: &mut Window, cx: &mut App| {
        let (dx, dy) = crate::components::swipe::drag_delta(e);
        let phase = e.touch_phase;
        shell.update(cx, |s, cx| match phase {
            TouchPhase::Started => {
                s.swipe_live = true;
                s.swipe_claimed = false;
                s.swipe_dx = 0.;
                s.swipe_settle = 0.; // a new touch takes the page over
                swipe_travel(s, dx, dy, cx);
            }
            TouchPhase::Moved => swipe_travel(s, dx, dy, cx),
            TouchPhase::Ended => swipe_release(s, can_back, can_forward, page_width, cx),
            TouchPhase::Cancelled => swipe_clear(s, cx),
        });
    });

    let shell = cx.entity().clone();
    let up = Rc::new(move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
        // A host that sends no `Ended` still ends the gesture here.
        shell.update(cx, |s, cx| {
            swipe_release(s, can_back, can_forward, page_width, cx)
        });
    });

    SwipeHandlers { scrolled, up }
}

/// One delta of the live gesture: travel the content and claim it once the
/// threshold passes.
fn swipe_travel(s: &mut AppShell, dx: f32, dy: f32, cx: &mut Context<AppShell>) {
    if !s.swipe_live || !is_swipe_delta(dx, dy) {
        return;
    }
    s.swipe_dx = add_travel(s.swipe_dx, dx);
    let claimed = !s.swipe_claimed && claims_horizontal(s.swipe_dx, 0.);
    s.swipe_claimed = s.swipe_claimed || claimed;
    cx.notify();
}

/// The finger lifted: switch the day when the drag was far enough, then finish
/// the page.
///
/// The switch is where a carousel can cheat nicely. The row holds the day
/// before, the day on screen and the day after; switching mid-drag changes
/// which pane is the middle one, so the travel that keeps the *incoming* day at
/// the pixel it already had is a known number (`settle_after`). Leaving that
/// travel in place and easing it to zero finishes the page: the day that was
/// sliding in comes to rest centred instead of jumping there.
fn swipe_release(
    s: &mut AppShell,
    can_back: bool,
    can_forward: bool,
    page_width: f32,
    cx: &mut Context<AppShell>,
) {
    if !std::mem::take(&mut s.swipe_live) {
        return;
    }
    let claimed = std::mem::take(&mut s.swipe_claimed);
    let dx = std::mem::take(&mut s.swipe_dx);
    let mut settle = dx;
    if claimed {
        if let Some(model) = &s.model {
            let day = swipe_day(dx, can_back, can_forward);
            log::info!("swipe: {dx} px released, day {day:?}");
            match day {
                Some(-1) => {
                    settle = settle_after(dx, -1, page_width);
                    model.prev_day();
                }
                Some(1) => {
                    settle = settle_after(dx, 1, page_width);
                    model.next_day();
                }
                _ => {}
            }
        }
    }
    if settle.abs() > 0.5 && !cx.reduce_motion() {
        s.swipe_settle = settle;
        ease_page(cx);
    } else {
        // No page to finish: the row returns at once.
        s.swipe_settle = 0.;
    }
    cx.notify();
}

/// The row travel that leaves the incoming day exactly where it is on screen
/// when it becomes the middle pane, so the page finish starts without a jump.
pub fn settle_after(dx: f32, day: i32, width: f32) -> f32 {
    let width = width.max(1.);
    match day {
        -1 => dx - width,
        1 => dx + width,
        _ => dx,
    }
}

/// Ease the page finish to rest: the carousel has no physics to model, only a
/// page that has to arrive.
fn ease_page(cx: &mut Context<AppShell>) {
    let shell = cx.entity().clone();
    cx.spawn(
        async move |_: WeakEntity<AppShell>, cx: &mut AsyncApp| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(16))
                .await;
            let more = shell.update(cx, |s, cx| {
                s.swipe_settle *= 0.68;
                if s.swipe_settle.abs() < 0.5 {
                    s.swipe_settle = 0.;
                    false
                } else {
                    cx.notify();
                    true
                }
            });
            if !more {
                break;
            }
        },
    )
    .detach();
}

/// The system took the touch: drop the gesture without switching.
fn swipe_clear(s: &mut AppShell, cx: &mut Context<AppShell>) {
    s.swipe_live = false;
    s.swipe_claimed = false;
    s.swipe_dx = 0.;
    s.swipe_settle = 0.;
    cx.notify();
}

/// The wide-desktop Menüs workspace (task 1.3): the shell's DayNavigator sits
/// above; this owns banners, the grouped preview list, the complete dish
/// detail and the all-days pending strip. Row activation only previews, and
/// the detail's explicit action is the incumbent model toggle.
/// @req REQ-SCR-01 @req REQ-UI-08 @req REQ-UI-12
pub fn desktop_menus_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    mutation_claimed: bool,
    list_focus: &RecordListFocus,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if !state.gourmet_authenticated {
        return auth_gate(theme);
    }
    let geometry = model
        .map(|model| model.desktop_geometry())
        .unwrap_or_else(|| desktop_geometry(HostKind::Desktop, 0.));

    let mut column = div().flex_1().min_h_0().flex().flex_col();
    if state.order_progress.is_none() {
        if let Some(snapshot) = &state.menu_snapshot {
            if snapshot.refreshing {
                column = column.child(refresh_banner(theme));
            }
        }
    }
    if let Some(progress) = &state.order_progress {
        column = column.child(progress_banner(progress, theme));
    }
    let host = state.host_kind;
    if let Some(error) = &state.error_text {
        column = column.child(error_banner(&error.message, retry_handler(host, cx), theme));
    }

    let blocked =
        mutation_claimed || desktop_mutation_blocked(state.busy, state.order_progress.is_some());
    column = column.child(split_workspace_scrolled(
        desktop_menu_master(state, model, blocked, list_focus, theme, cx),
        desktop_menu_detail_pane(state, model, blocked, theme, cx),
        &geometry,
        theme,
        &list_focus.scroll(),
        list_focus.detail(),
    ));
    if !blocked {
        if let Some(strip) = desktop_submit_strip(state, model, theme, cx) {
            column = column.child(strip);
        }
    }
    column.into_any_element()
}

/// Grouped master list with the incumbent group order and SUPPE & SALAT
/// heading suppression. Selection is a preview; it never toggles.
fn desktop_menu_master(
    state: &State,
    model: Option<&Arc<AppModel>>,
    blocked: bool,
    list_focus: &RecordListFocus,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(model) = model else {
        list_focus.retain(&[]);
        return desktop_empty_master(theme);
    };
    let Some(day) = state.selected_day.as_deref() else {
        list_focus.retain(&[]);
        return desktop_empty_master(theme);
    };
    let groups = model.categories_for_day(day);
    if groups.is_empty() {
        list_focus.retain(&[]);
        let loading = state
            .menu_snapshot
            .as_ref()
            .map_or(true, |snapshot| snapshot.items.is_empty())
            && (state.busy || state.menu_snapshot.as_ref().is_some_and(|s| s.loading));
        return if loading {
            desktop_loading_state(theme)
        } else {
            desktop_empty_master(theme)
        };
    }

    let (pending_orders, pending_cancellations) = state
        .menu_snapshot
        .as_ref()
        .map(|s| (s.pending_orders.clone(), s.pending_cancellations.clone()))
        .unwrap_or_default();
    let preview = state.preview.clone();
    // The visible row order the roving cursor moves through; every row is
    // listed, including unavailable ones, because they are all visible.
    let ids: Vec<String> = groups
        .iter()
        .flat_map(|(_, items)| items.iter().map(menu_key))
        .collect();
    // Only the rows in this render stay focusable or restorable: a day change
    // or refresh drops the handles of the previous set.
    list_focus.retain(&ids);
    // Whether the previewed dish's detail offers an enabled action; Enter
    // then lands on that control instead of the read-only region.
    let detail_control = preview.as_deref().is_some_and(|key| {
        !blocked
            && groups
                .iter()
                .flat_map(|(_, items)| items.iter())
                .find(|item| menu_key(item) == key)
                .is_some_and(|item| {
                    desktop_menu_action(&desktop_row_state(
                        item,
                        model,
                        &pending_orders,
                        &pending_cancellations,
                        true,
                    ))
                    .is_some()
                })
    });

    // One tab stop for the whole list: Up/Down/Home/End move the row cursor,
    // preview the stable identity and reveal the row. Enter hands focus to the
    // detail, no mutation and no wrap.
    let on_key = {
        let focus = list_focus.clone();
        let model = Arc::clone(model);
        let ids = ids.clone();
        move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if event.keystroke.key.as_str() == "enter" {
                focus.enter(detail_entry(detail_control), window, cx);
                cx.stop_propagation();
                return;
            }
            let Some(id) = focus.step(&ids, event.keystroke.key.as_str(), window, cx) else {
                return;
            };
            model.open_preview(ListKind::Menus, &id);
            model.set_cursor(ListKind::Menus, Some(id));
            cx.stop_propagation();
        }
    };
    let mut list = div()
        .id("desktop-menu-list")
        .flex()
        .flex_col()
        .tab_index(0)
        .track_focus(list_focus.container())
        .focus(theme::desktop_focus(theme.accent))
        .on_key_down(on_key);
    for (category, items) in groups {
        if let Some(heading) = category_heading(category) {
            list = list.child(section_label(heading, theme));
        }
        for item in items {
            let key = menu_key(&item);
            let selected = preview.as_deref() == Some(key.as_str());
            let row = desktop_row_state(
                &item,
                model,
                &pending_orders,
                &pending_cancellations,
                selected,
            );
            let handle = list_focus.row(&key, cx);
            let mut cells = vec![menu_primary_cell(&item, row, theme)];
            if let Some((label, ink, _)) = desktop_menu_status(&row, theme) {
                cells.push(record_status_cell(label, ink, theme));
            }
            cells.push(record_cell_right(item.price.clone(), 80., theme));
            list = list.child(record_row_focusable(
                SharedString::from(format!("desktop-menu-{key}")),
                cells,
                selected,
                Some(desktop_preview_handler(cx, key)),
                false,
                None,
                Some(&handle),
                theme,
            ));
        }
    }
    list.into_any_element()
}

fn desktop_empty_master(theme: &AppTheme) -> AnyElement {
    let (fact, hint) = empty_copy();
    state_message(fact, hint, theme)
}

/// The menu row's primary cell: the dish in `BODY`, struck while its
/// cancellation is pending. The category is the group heading above, so the
/// row never repeats it. Ink is inherited from the row, so the selected
/// row paints it in the accent.
fn menu_primary_cell(item: &MenuItem, state: DesktopMenuRowState, theme: &AppTheme) -> AnyElement {
    let title = div()
        .flex_1()
        .min_w_0()
        .text_size(px(theme.scale.body))
        .line_height(px(theme.scale.body_line))
        .font_weight(FontWeight::SEMIBOLD)
        .text_ellipsis()
        .whitespace_nowrap()
        .child(crate::components::menu_card::dish_label(item));
    let title = if state.pending_cancel {
        title.line_through()
    } else {
        title
    };
    title.into_any_element()
}

/// The detail pane: the selected row's complete dish, or the prompt.
fn desktop_menu_detail_pane(
    state: &State,
    model: Option<&Arc<AppModel>>,
    blocked: bool,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let (Some(model), Some(preview)) = (model, state.preview.as_deref()) else {
        return detail_prompt("Menü auswählen", theme);
    };
    let Some(snapshot) = &state.menu_snapshot else {
        return detail_prompt("Menü auswählen", theme);
    };
    let Some(item) = snapshot.items.iter().find(|item| menu_key(item) == preview) else {
        return detail_prompt("Menü auswählen", theme);
    };

    let (pending_orders, pending_cancellations) = (
        snapshot.pending_orders.clone(),
        snapshot.pending_cancellations.clone(),
    );
    let row = desktop_row_state(item, model, &pending_orders, &pending_cancellations, true);
    desktop_menu_detail(
        item,
        row,
        blocked,
        desktop_toggle_handler(cx, item.clone()),
        theme,
        state.content_width,
        cx,
    )
}

/// One complete dish (task 1.3 brief): formatted day, title, full dish,
/// allergens only when present, original price string, status and the
/// explicit toggle. `blocked` only swaps the control ink; the handler is
/// guarded too, so a duplicate activation cannot dispatch.
/// @req REQ-SCR-01 @req REQ-UI-12 @req REQ-UI-13
pub(crate) fn desktop_menu_detail(
    item: &MenuItem,
    state: DesktopMenuRowState,
    blocked: bool,
    on_action: ClickHandler,
    theme: &AppTheme,
    content_width: f32,
    cx: &App,
) -> AnyElement {
    let pad = if content_width >= 1024. {
        theme.scale.space_5
    } else {
        theme.scale.space_4
    };
    let dish = if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    };
    let mut detail = div()
        .p(px(pad))
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(format_day_short(&item.day)),
        )
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(dish),
        );

    let allergens = item
        .allergens
        .iter()
        .filter(|allergen| !allergen.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !allergens.is_empty() {
        detail = detail.child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(format!("Allergene: {allergens}")),
        );
    }

    detail = detail.child(
        div()
            .text_size(px(theme.scale.body))
            .line_height(px(theme.scale.body_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.text_primary)
            .child(item.price.clone()),
    );
    if let Some((label, ink, fill)) = desktop_menu_status(&state, theme) {
        detail = detail.child(desktop_status_pill(label, ink, fill, theme));
    }
    if let Some((label, kind)) = desktop_menu_action(&state) {
        let control = if blocked {
            ControlState::Disabled
        } else {
            ControlState::Rest
        };
        detail = detail.child(desktop_action_state(
            format!("desktop-menu-action-{}", menu_key(item)),
            label,
            kind,
            control,
            theme,
            cx,
            move |_, window, cx| on_action(window, cx),
        ));
    }
    detail.into_any_element()
}

fn desktop_status_pill(label: &'static str, ink: Hsla, fill: Hsla, theme: &AppTheme) -> AnyElement {
    div()
        .flex()
        .child(
            div()
                .px(px(theme.scale.space_2))
                .py(px(theme.scale.space_1))
                .rounded(px(theme.scale.radius_chip))
                .bg(fill)
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .child(label),
        )
        .into_any_element()
}

/// The all-days pending strip: one trailing primary action with the exact
/// incumbent count label. It exists only while pending changes wait and no
/// operation runs, so the progress banner takes its place during submission.
fn desktop_submit_strip(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Option<AnyElement> {
    let (model, snapshot) = (model?, state.menu_snapshot.as_ref()?);
    let pending = snapshot.pending_orders.len() + snapshot.pending_cancellations.len();
    if pending == 0 || state.order_progress.is_some() || state.busy {
        return None;
    }
    Some(action_strip(
        vec![div()
            .flex()
            .min_w_0()
            .max_w(relative(1.))
            .ml_auto()
            .child(desktop_submit_action(
                model.fab_label(),
                theme,
                desktop_submit_handler(cx),
            ))
            .into_any_element()],
        theme,
    ))
}

/// The batch submit action, wrapped in a growing div so the mixed label
/// (`Änderungen bestätigen (n)`) wraps at 720 instead of clipping. The kit
/// button is not used here because it never wraps.
pub(crate) fn desktop_submit_action(
    label: String,
    theme: &AppTheme,
    on_click: ClickHandler,
) -> AnyElement {
    let button = div()
        .min_h(px(tokens::ACTION_HEIGHT))
        .min_w_0()
        .px(px(theme.scale.space_4))
        .py(px(theme.scale.space_2))
        .rounded(px(theme.scale.radius_chip))
        .bg(theme.accent)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(theme.pressed(theme.accent)))
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.on_accent)
                .text_center()
                .child(label),
        );
    theme::focus_frame(button, ControlState::Rest, theme.scale.radius_chip, theme)
        .id("desktop-menus-submit")
        .tab_index(0)
        .on_click(move |_, window, cx| on_click(window, cx))
        .into_any_element()
}

fn desktop_row_state(
    item: &MenuItem,
    model: &Arc<AppModel>,
    pending_orders: &[String],
    pending_cancellations: &[String],
    selected: bool,
) -> DesktopMenuRowState {
    let key = menu_key(item);
    DesktopMenuRowState {
        selected,
        pending_order: pending_orders.contains(&key),
        pending_cancel: pending_cancellations.contains(&key),
        ordered: model.is_ordered(item),
        available: item.available,
        cutoff: model.is_cutoff(&item.day),
    }
}

/// Row activation: preview plus cursor, never toggle, submit or refresh.
fn desktop_preview_handler(cx: &mut Context<AppShell>, key: String) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.open_preview(ListKind::Menus, &key);
                model.set_cursor(ListKind::Menus, Some(key.clone()));
                cx.notify();
            }
        });
    })
}

/// The explicit detail action: the incumbent model toggle, guarded so a
/// pending submit or another running operation cannot be interleaved.
fn desktop_toggle_handler(cx: &mut Context<AppShell>, item: MenuItem) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = &s.model else {
                return;
            };
            let state = model.state();
            if s.mutation_gate.is_claimed()
                || desktop_mutation_blocked(state.busy, state.order_progress.is_some())
            {
                return;
            }
            model.toggle(&item);
            cx.notify();
        });
    })
}

/// Batch submission runs off the render thread (the model blocks on its own
/// runtime) so the progress banner can paint. The shell claims the gate
/// synchronously before the spawn: the model's `busy` only arrives after the
/// spawn, so without the claim a double activation could submit twice.
fn desktop_submit_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            let state = model.state();
            let Some(ticket) = s
                .mutation_gate
                .claim(state.busy, state.order_progress.is_some())
            else {
                return;
            };
            cx.background_executor()
                .spawn(async move {
                    let _ticket = ticket;
                    model.submit_orders()
                })
                .detach();
            cx.notify();
        });
    })
}

#[cfg(test)]
mod mobile_tests {
    use super::{auth_copy, empty_copy};

    /// The empty and auth states are two-line copy: the fact first, then the
    /// next step. @req REQ-SCR-01 @req REQ-UI-15
    #[test]
    fn empty_and_auth_states_state_fact_then_next_step() {
        assert_eq!(
            empty_copy(),
            (
                "Keine Menüs für diesen Tag",
                "Wähle mit den Pfeilen einen anderen Tag"
            )
        );
        assert_eq!(
            auth_copy(),
            (
                "Nicht angemeldet",
                "Gehe zu Einstellungen, um Zugangsdaten einzugeben"
            )
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{carousel_travel, menus_gate, neighbours, page_fade, settle_after};

    /// The two pages of a drag always add up to one page of ink, and the switch
    /// moves the middle without a pop: the opacity the arriving page had is the
    /// opacity the new middle page has on the same pixel.
    /// @req REQ-SCR-01
    #[test]
    fn the_carousel_cross_fades_without_a_pop() {
        let width = 360.;
        assert_eq!(page_fade(0., 1, width), 1., "the day in place is solid");
        assert_eq!(page_fade(0., 0, width), 0.);
        assert_eq!(page_fade(0., 2, width), 0.);

        let travel = -90.;
        assert_eq!(page_fade(travel, 1, width), 0.75);
        assert_eq!(page_fade(travel, 2, width), 0.25);
        assert!(
            (page_fade(travel, 1, width) + page_fade(travel, 2, width) - 1.).abs() < 1e-6,
            "the transition never shows more or less than one page"
        );

        // Released to the next day: the row keeps the travel that held the
        // arriving page in place, and that page is now the middle one.
        let settle = settle_after(travel, 1, width);
        assert_eq!(
            page_fade(settle, 1, width),
            page_fade(travel, 2, width),
            "no brightness step at the switch"
        );
    }

    /// The page finish keeps the incoming day on the same pixel: switching mid
    /// drag must not move the row, only restart which pane is the middle one.
    /// @req REQ-SCR-01
    #[test]
    fn the_page_finish_keeps_the_incoming_day_in_place() {
        let width = 390.;
        // Dragging left by 50 px: the next day is visible in the right 50 px.
        // After the switch it must still be there, then slide left to centre.
        assert_eq!(settle_after(-50., 1, width), -50. + width);
        // Fully dragged: nothing left to animate.
        assert_eq!(settle_after(-width, 1, width), 0.);
        // Dragging right: the previous day is visible in the left strip.
        assert_eq!(settle_after(50., -1, width), 50. - width);
        assert_eq!(settle_after(width, -1, width), 0.);
        // No switch: the row returns from where the finger left it.
        assert_eq!(settle_after(30., 0, width), 30.);
        assert_eq!(settle_after(-30., 0, width), -30.);
    }

    /// The carousel pages between published days and stops at the ends.
    /// @req REQ-SCR-01
    #[test]
    fn the_carousel_pages_between_published_days_only() {
        let dates: Vec<String> = ["2026-11-16", "2026-11-17", "2026-11-18"]
            .iter()
            .map(|d| d.to_string())
            .collect();
        let (prev, next) = neighbours(&dates, "2026-11-17");
        assert_eq!(prev.as_deref(), Some("2026-11-16"));
        assert_eq!(next.as_deref(), Some("2026-11-18"));

        let (prev, next) = neighbours(&dates, "2026-11-16");
        assert_eq!(prev, None, "the first day has nothing before it");
        assert_eq!(next.as_deref(), Some("2026-11-17"));

        let (prev, next) = neighbours(&dates, "2026-11-18");
        assert_eq!(prev.as_deref(), Some("2026-11-17"));
        assert_eq!(next, None, "the last day has nothing after it");

        assert_eq!(
            neighbours(&dates, "2026-01-01"),
            (None, None),
            "a day outside the week pages nowhere"
        );
    }

    /// Travel stops at exactly one page, so the neighbour can arrive fully and
    /// never the page beyond it.
    /// @req REQ-SCR-01
    #[test]
    fn carousel_travel_is_one_page() {
        assert_eq!(carousel_travel(40., 390.), 40.);
        assert_eq!(carousel_travel(-2000., 390.), -390.);
        assert_eq!(carousel_travel(2000., 390.), 390.);
        assert_eq!(carousel_travel(10., 0.), 10., "a zero width cannot divide");
    }

    /// The cached week is what a launch shows: the gate waits for the launch
    /// order to settle, so a slow sign-in never blanks the screen.
    /// @req REQ-SCR-01
    #[test]
    fn a_launch_shows_the_cached_week_while_it_signs_in() {
        assert!(!menus_gate(true, false, true), "signed in: never a gate");
        assert!(
            !menus_gate(false, false, true),
            "a restore with cache on disk shows the week"
        );
        assert!(
            menus_gate(false, true, true),
            "the restore settled without a session: the gate asks for one"
        );
        assert!(
            menus_gate(false, false, false),
            "nothing cached leaves nothing to paint but the gate"
        );
    }
}
