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

use crate::app::{desktop_mutation_blocked, run_blocking, AppShell};
use crate::components::button::desktop_action_state;
use crate::components::fab::fab;
use crate::components::menu_card::{
    desktop_menu_action, desktop_menu_row, desktop_menu_status, menu_card, DesktopMenuRowState,
    MenuCardState,
};
use crate::components::record_table::{detail_entry, RecordListFocus};
use crate::components::spinner::spinner;
use crate::components::swipe::{swipe_days, SwipeHandlers, CLAIM_PX, EDGE_DAMPING, SWITCH_PX};
use crate::components::workspace::{action_strip, detail_prompt, split_workspace_scrolled};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::{MenuCategory, MenuItem};
use snackpilot_model::desktop::{desktop_geometry, ControlState, HostKind, TARGET_MIN};
use snackpilot_model::navigation::ListKind;
use snackpilot_model::{category_heading, format_day_short, menu_key, AppModel, State};

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
    let host = state.host_kind;
    if let Some(error) = &state.error_text {
        column = column.child(error_banner(&error.message, retry_handler(host, cx), theme));
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
            column = column.child(fab(model.fab_label(), submit_handler(host, cx), theme));
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

/// Drag handlers: claim past 10 px horizontal, damp at the edges, switch at
/// 50 px on release.
fn swipe_handlers(can_back: bool, can_forward: bool, cx: &mut Context<AppShell>) -> SwipeHandlers {
    let shell = cx.entity().clone();
    let down = Rc::new(move |e: &MouseDownEvent, _: &mut Window, cx: &mut App| {
        let x = f32::from(e.position.x);
        let y = f32::from(e.position.y);
        shell.update(cx, |s, _| {
            s.swipe_start = Some(x);
            s.swipe_start_y = Some(y);
            s.swipe_claimed = false;
        });
    });

    let shell = cx.entity().clone();
    let moved = Rc::new(move |e: &MouseMoveEvent, _: &mut Window, cx: &mut App| {
        if e.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let x = f32::from(e.position.x);
        let y = f32::from(e.position.y);
        shell.update(cx, |s, cx| {
            let (Some(start), Some(start_y)) = (s.swipe_start, s.swipe_start_y) else {
                return;
            };
            let dx = x - start;
            let dy = y - start_y;
            if !s.swipe_claimed {
                // Claim only when horizontal movement dominates (§3.1).
                if dx.abs() <= CLAIM_PX || dx.abs() <= dy.abs() {
                    return;
                }
                s.swipe_claimed = true;
            }
            let mut offset = dx;
            if (offset > 0. && !can_back) || (offset < 0. && !can_forward) {
                offset *= EDGE_DAMPING;
            }
            s.swipe_dx = offset;
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
            let claimed = std::mem::take(&mut s.swipe_claimed);
            s.swipe_start_y = None;
            let dx = x - start;
            s.swipe_dx = 0.;
            if claimed {
                if let Some(model) = &s.model {
                    if dx > SWITCH_PX && can_back {
                        model.prev_day();
                    } else if dx < -SWITCH_PX && can_forward {
                        model.next_day();
                    }
                }
            }
            cx.notify();
        });
    });

    SwipeHandlers { down, moved, up }
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
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.primary, 36.))
                .into_any_element()
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
        .focus(theme::desktop_focus(theme.desktop.focus))
        .on_key_down(on_key);
    for (category, items) in groups {
        if let Some(heading) = category_heading(category) {
            list = list.child(
                div()
                    .px(px(16.))
                    .pt(px(16.))
                    .pb(px(8.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.desktop.ink)
                    .child(heading),
            );
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
            list = list.child(desktop_menu_row(
                &item,
                row,
                desktop_preview_handler(cx, key),
                Some(&handle),
                theme,
            ));
        }
    }
    list.into_any_element()
}

fn desktop_empty_master(theme: &AppTheme) -> AnyElement {
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
                .child("Keine Menüs verfügbar"),
        )
        .into_any_element()
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
    let pad = if content_width >= 1024. { 24. } else { 16. };
    let dish = if item.subtitle.trim().is_empty() {
        item.title.clone()
    } else {
        item.subtitle.clone()
    };
    let mut detail = div()
        .p(px(pad))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format_day_short(&item.day)),
        )
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.desktop.ink)
                .child(item.title.clone()),
        )
        .child(
            div()
                .text_size(px(16.))
                .line_height(px(24.))
                .text_color(theme.desktop.ink)
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
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .child(format!("Allergene: {allergens}")),
        );
    }

    detail = detail.child(
        div()
            .text_size(px(16.))
            .line_height(px(24.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.desktop.ink)
            .child(item.price.clone()),
    );
    if let Some((label, ink, fill)) = desktop_menu_status(&state, theme) {
        detail = detail.child(desktop_status_pill(label, ink, fill));
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

fn desktop_status_pill(label: &'static str, ink: Hsla, fill: Hsla) -> AnyElement {
    div()
        .flex()
        .child(
            div()
                .px(px(8.))
                .py(px(2.))
                .rounded(px(4.))
                .bg(fill)
                .text_size(px(12.))
                .line_height(px(16.))
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
    let action = theme.desktop.action;
    let button = div()
        .min_h(px(TARGET_MIN))
        .min_w_0()
        .px(px(16.))
        .py(px(8.))
        .rounded(px(8.))
        .bg(action.fill)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(action.hover))
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(20.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(action.ink)
                .text_center()
                .child(label),
        );
    theme::focus_frame(button, ControlState::Rest, 8., theme)
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
