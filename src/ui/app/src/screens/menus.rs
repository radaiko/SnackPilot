//! Menus tab — docs/requirements/04-ui-ux.md §3.1.
//!
//! The DayNavigator is rendered by the shell above; this module owns the
//! swipeable area: refresh/progress/error banners, the grouped `MenuCard`
//! list, the empty and auth states, the FAB and the 50 px day swipe. Every
//! value comes from `AppModel`; the only view-local state is the transient
//! drag offset stored on [`AppShell`].

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;

use crate::app::AppShell;
use crate::components::fab::fab;
use crate::components::menu_card::{menu_card, MenuCardState};
use crate::components::spinner::spinner;
use crate::components::swipe::{swipe_days, SwipeHandlers, CLAIM_PX, EDGE_DAMPING, SWITCH_PX};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::{MenuCategory, MenuItem};
use snackpilot_model::{category_heading, AppModel, State};

/// The Menus tab content below the DayNavigator.
pub fn menus_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    offset: f32,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if !state.gourmet_authenticated {
        return auth_gate(theme);
    }

    let mut column = div().size_full().flex().flex_col();

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
    if let Some(error) = &state.error_text {
        column = column.child(error_banner(&error.message, retry_handler(cx), theme));
    }

    let busy = state.busy;
    let content = menu_list(state, model, busy, theme, cx);

    let (can_back, can_forward, offset) = match &state.menu_snapshot {
        Some(snapshot) if !snapshot.available_dates.is_empty() => {
            let day = state.selected_day.as_deref().unwrap_or_default();
            let can_back = snapshot.available_dates.first().is_some_and(|d| d != day);
            let can_forward = snapshot.available_dates.last().is_some_and(|d| d != day);
            (can_back, can_forward, offset)
        }
        _ => (false, false, 0.),
    };

    column = column.child(swipe_days(
        "menus-swipe",
        content,
        offset,
        swipe_handlers(can_back, can_forward, cx),
    ));

    if let (Some(model), Some(snapshot)) = (model, &state.menu_snapshot) {
        let pending = snapshot.pending_orders.len() + snapshot.pending_cancellations.len();
        if pending > 0 && state.order_progress.is_none() {
            column = column.child(fab(model.fab_label(), submit_handler(cx), theme));
        }
    }

    column.into_any_element()
}

/// Grouped, scrollable cards for the selected day.
fn menu_list(
    state: &State,
    model: Option<&Arc<AppModel>>,
    busy: bool,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(model) = model else {
        return empty_state(theme);
    };
    let Some(day) = state.selected_day.as_deref() else {
        return empty_state(theme);
    };

    let groups = model.categories_for_day(day);
    if groups.is_empty() {
        return if busy {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.primary, 36.))
                .into_any_element()
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
        .id("menus-list")
        .size_full()
        .flex()
        .flex_col()
        .p(px(16.))
        .pb(px(100.))
        .overflow_y_scroll();

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

fn group(
    category: MenuCategory,
    items: Vec<MenuItem>,
    pending_orders: &[String],
    pending_cancellations: &[String],
    model: &Arc<AppModel>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Div {
    let mut group = div().flex().flex_col().mb(px(16.));
    if let Some(heading) = category_heading(category) {
        group = group.child(
            div()
                .mb(px(8.))
                .pl(px(4.))
                .text_size(px(22.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.primary)
                .child(heading),
        );
    }
    for item in items {
        let key = format!("{}|{}", item.id, item.day);
        let state = MenuCardState {
            selected: pending_orders.contains(&key),
            pending_cancel: pending_cancellations.contains(&key),
            ordered: model.is_ordered(&item),
            available: item.available,
            cutoff: model.is_cutoff(&item.day),
        };
        let tappable = state.ordered || (item.available && !state.cutoff) || state.selected;
        let handler = tappable.then(|| tap_handler(cx, item.clone()));
        group = group.child(menu_card(&item, state, handler, theme));
    }
    group
}

fn empty_state(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme.text_tertiary)
                .child("Keine Menüs verfügbar"),
        )
        .into_any_element()
}

fn auth_gate(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.error)
                .child("Nicht angemeldet"),
        )
        .child(
            div()
                .mt(px(8.))
                .text_size(px(14.))
                .text_color(theme.text_tertiary)
                .child("Gehe zu Einstellungen, um Zugangsdaten einzugeben"),
        )
        .into_any_element()
}

fn refresh_banner(theme: &AppTheme) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_primary);
    theme::apply(
        div()
            .mx(px(16.))
            .mt(px(8.))
            .py(px(8.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .child(spinner(theme.primary, 12.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.primary)
                    .child("Aktualisiere..."),
            ),
        &r,
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
            .child(spinner(theme.on_primary, 12.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.on_primary)
                    .child(progress.to_string()),
            ),
        &r,
    )
    .into_any_element()
}

fn error_banner(message: &str, retry: ClickHandler, theme: &AppTheme) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_error);
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
                    .text_size(px(13.))
                    .text_color(theme.error_text)
                    .child(message.to_string()),
            )
            .child(
                div()
                    .id("menus-retry")
                    .cursor_pointer()
                    .on_click(move |_, window, cx| retry(window, cx))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.error)
                            .child("Erneut versuchen"),
                    ),
            ),
        &r,
    )
    .into_any_element()
}

fn retry_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.refresh_menus();
                cx.notify();
            }
        });
    })
}

fn submit_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.submit_orders();
                cx.notify();
            }
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

/// Drag handlers: claim past 10 px horizontal, damp at the edges, switch at
/// 50 px on release.
fn swipe_handlers(can_back: bool, can_forward: bool, cx: &mut Context<AppShell>) -> SwipeHandlers {
    let shell = cx.entity().clone();
    let down = Rc::new(move |e: &MouseDownEvent, _: &mut Window, cx: &mut App| {
        let x = f32::from(e.position.x);
        shell.update(cx, |s, _| s.swipe_start = Some(x));
    });

    let shell = cx.entity().clone();
    let moved = Rc::new(move |e: &MouseMoveEvent, _: &mut Window, cx: &mut App| {
        if e.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let x = f32::from(e.position.x);
        shell.update(cx, |s, cx| {
            let Some(start) = s.swipe_start else {
                return;
            };
            let mut dx = x - start;
            if !dx.is_finite() || dx.abs() <= CLAIM_PX {
                return;
            }
            if (dx > 0. && !can_back) || (dx < 0. && !can_forward) {
                dx *= EDGE_DAMPING;
            }
            s.swipe_dx = dx;
            cx.notify();
        });
    });

    let shell = cx.entity().clone();
    let up = Rc::new(move |e: &MouseUpEvent, _: &mut Window, cx: &mut App| {
        let x = f32::from(e.position.x);
        shell.update(cx, |s, cx| {
            let Some(start) = s.swipe_start.take() else {
                return;
            };
            let dx = x - start;
            s.swipe_dx = 0.;
            if let Some(model) = &s.model {
                if dx > SWITCH_PX && can_back {
                    model.prev_day();
                } else if dx < -SWITCH_PX && can_forward {
                    model.next_day();
                }
            }
            cx.notify();
        });
    });

    SwipeHandlers { down, moved, up }
}
