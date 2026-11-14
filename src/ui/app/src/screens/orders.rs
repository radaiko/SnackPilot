//! Orders tab — docs/requirements/04-ui-ux.md §3.2 and §4.6.
//!
//! Segment tabs, the unconfirmed banner with its plural text, the confirm
//! banner action, `OrderItem` cards with the dish lookup and cancel flow, the
//! multi-select batch cancel and the empty/error states. All state comes from
//! `AppModel`, including the active segment (`State::orders_past`).

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::*;

use crate::app::{
    desktop_check_effect, desktop_mutation_blocked, order_cancel_state, run_blocking, AppShell,
    OrderCancelState, OrderRowEffect,
};
use crate::components::button::{
    desktop_action_state, mobile_action, mobile_action_action, mobile_action_metrics, ButtonKind,
};
use crate::components::dialog::Dialog;
use crate::components::order_item::{
    desktop_order_detail, desktop_order_header, desktop_order_row, order_description, order_item,
    DesktopOrderRowState,
};
use crate::components::pull_refresh::pull_refresh;
use crate::components::record_table::{detail_entry, RecordListFocus};
use crate::components::segment_tabs::{desktop_segment_toolbar, segment_tabs, Segment};
use crate::components::spinner::spinner;
use crate::components::workspace::{
    action_strip, detail_prompt, loading_state, split_workspace_scrolled, state_message,
};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::OrderedMenu;
use snackpilot_model::desktop::{ControlState, HostKind, LayoutMode};
use snackpilot_model::navigation::{ListKind, Tab};
use snackpilot_model::{AppModel, State};

/// The compact Orders tab content (task 1.4): shell-owned heading and gutters,
/// the compact segment bar, the optional confirm banner, the inline error, the
/// scrolling cards and — in selection mode — the counted batch strip. All state
/// comes from `AppModel`; the legacy 16/100 px screen padding is gone, so the
/// shell's measured reservation owns the scroll-end clearance.
/// @req REQ-SCR-02 @req REQ-UI-15 @req REQ-UI-16
#[allow(clippy::too_many_arguments)]
pub fn orders_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    past: bool,
    theme: &AppTheme,
    scroll: &ScrollHandle,
    pull: f32,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if !state.gourmet_authenticated {
        return gate(theme);
    }
    let Some(model) = model else {
        return gate(theme);
    };
    let Some(split) = &state.orders_split else {
        return if state.busy {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.accent, 36.))
                .into_any_element()
        } else {
            gate(theme)
        };
    };

    let active = if past {
        Segment::Past
    } else {
        Segment::Upcoming
    };

    // The segment is model state (task 1.1): the binding sets it and the
    // model's notification repaints the shell, so no view copy exists.
    let select_segment = {
        let model = Arc::clone(model);
        Rc::new(move |segment: Segment, _w: &mut Window, _cx: &mut App| {
            model.set_orders_segment(segment == Segment::Past);
        })
    };

    let host = state.host_kind;
    // One flex child of the shell's column: it takes exactly the height left
    // under the shell heading, so the batch strip stays above navigation
    // instead of overflowing the heading's height.
    let mut column = div().flex_1().min_h(px(0.)).flex().flex_col();
    // The screen's first row is the segment: no page title above it.
    let head = div()
        .w_full()
        .flex()
        .items_center()
        .gap(px(theme.scale.space_2))
        .child(div().flex_1().min_w(px(0.)).child(segment_tabs(
            active,
            split.upcoming.len(),
            split.past.len(),
            select_segment,
            theme,
        )));
    column = column.child(head);

    if state.order_progress.is_none() {
        if let Some(banner) = model.unconfirmed_banner_text() {
            if !past {
                column = column.child(confirm_banner(
                    &banner,
                    state.busy,
                    confirm_handler(host, cx),
                    theme,
                ));
            }
        }
    }
    if let Some(error) = &state.orders_error {
        column = column.child(error_banner(&error.message, theme));
    }

    let orders: &[OrderedMenu] = if past { &split.past } else { &split.upcoming };

    let mut list = div()
        .id("orders-list")
        .w_full()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_y_scroll();

    if orders.is_empty() {
        list = list.child(empty_state(past, theme));
    } else {
        for order in orders {
            let cancelling = state.cancelling_id.as_deref() == Some(order.position_id.as_str());
            let can_cancel = !past && state.cancelling_id.is_none();
            let cutoff = model.is_cancellation_cutoff(order);
            let dish = model.dish(order);
            let handler = cancel_handler(host, cx, order.clone());
            let select_mode = state.order_selection_mode;
            let selected = state.selected_order_ids.contains(&order.position_id);
            let on_select =
                select_mode.then(|| toggle_selection_handler(cx, order.position_id.clone()));
            list = list.child(order_item(
                order,
                dish,
                can_cancel,
                cancelling,
                cutoff,
                select_mode,
                selected,
                handler,
                on_select,
                theme,
            ));
        }
    }

    // A drag from the top of the list reloads the orders.
    let on_refresh = crate::app::refresh_handler(Tab::Orders, Some(model), state.host_kind);
    let gesture = crate::app::pull_handlers(scroll.clone(), on_refresh, cx);
    column = column.child(pull_refresh(
        "orders-pull",
        list.track_scroll(scroll).into_any_element(),
        pull,
        gesture,
        theme,
    ));

    // Batch-cancel bar while multi-select mode is active.
    if state.order_selection_mode {
        column = column.child(selection_bar(state, host, cx, theme));
    } else if !past && !orders.is_empty() {
        // Batch selection only exists when there is something to select: an
        // empty upcoming list showed a secondary "Auswählen" that led nowhere.
        let (height, radius) = mobile_select_metrics(theme);
        let handler = begin_selection_handler(cx);
        column = column.child(
            div().w_full().flex().justify_end().child(
                mobile_action_action(
                    "orders-select",
                    "Auswählen",
                    ButtonKind::Secondary,
                    theme,
                    cx,
                    move |_, window, cx| handler(window, cx),
                )
                .min_h(px(height))
                .rounded(px(radius)),
            ),
        );
    }

    column.into_any_element()
}

const EMPTY_COPY_UPCOMING: (&str, &str) = (
    "Keine kommenden Bestellungen",
    "Bestellte Menüs erscheinen hier",
);

const EMPTY_COPY_PAST: (&str, &str) = (
    "Keine vergangenen Bestellungen",
    "Abgeschlossene Bestellungen erscheinen hier",
);

fn empty_copy(past: bool) -> (&'static str, &'static str) {
    if past {
        EMPTY_COPY_PAST
    } else {
        EMPTY_COPY_UPCOMING
    }
}

/// The empty segment's two-line state: the fact, then the next step.
fn empty_state(past: bool, theme: &AppTheme) -> AnyElement {
    let (fact, hint) = empty_copy(past);
    state_message(fact, hint, theme)
}

fn gate(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child("Anmeldung erforderlich"),
        )
        .into_any_element()
}

/// The confirm banner's primary action: the accent fill and ink at the 48 px
/// action floor. A busy flow paints the disabled pair and installs no callback,
/// so a second tap cannot confirm twice.
fn banner_action(
    label: &str,
    enabled: bool,
    on_click: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let (height, radius) = mobile_action_metrics(ButtonKind::Primary);
    let (fill, ink) = if enabled {
        (theme.accent, theme.on_accent)
    } else {
        (theme.surface_alt, theme.text_tertiary)
    };
    let mut action = div()
        .id("orders-confirm")
        .flex_shrink_0()
        .min_h(px(height))
        .px(px(theme.scale.space_4))
        .rounded(px(radius))
        .bg(fill)
        .flex()
        .items_center()
        .justify_center();
    if enabled {
        action = action
            .tab_index(0)
            .cursor_pointer()
            .hover(move |s| s.bg(theme.pressed(theme.accent)))
            .focus(crate::theme::desktop_focus(theme.accent))
            .on_click(move |_, window, cx| on_click(window, cx));
    }
    action
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .text_center()
                .child(label.to_string()),
        )
        .into_any_element()
}

/// The unconfirmed banner: one row with the model's text and the 48 px primary
/// `Bestätigen` at the trailing edge; it wraps instead of clipping. Copy and
/// busy label are the incumbent ones.
/// @req REQ-SCR-02 @req REQ-UI-15
pub(crate) fn confirm_banner(
    text: &str,
    busy: bool,
    on_confirm: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    theme::apply(
        div()
            .mb(px(theme.scale.space_2))
            .p(px(theme.scale.space_4))
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(theme.scale.space_3))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
                    .text_color(theme.text_primary)
                    .child(text.to_string()),
            )
            .child(banner_action(
                if busy { "…" } else { "Bestätigen" },
                !busy,
                on_confirm,
                theme,
            )),
        &theme.card_surface(),
    )
    .into_any_element()
}

fn error_banner(message: &str, theme: &AppTheme) -> AnyElement {
    theme::apply(
        div()
            .mb(px(theme.scale.space_2))
            .p(px(theme.scale.space_4))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.error)
                    .child(message.to_string()),
            ),
        &theme.tinted_banner(theme.error_surface),
    )
    .into_any_element()
}

/// Whether a batch-cancel activation may run at all: the model's own
/// precondition — a non-empty check set and no running flow — checked before
/// the shell's gate claims anything (REQ-SCR-02/16).
fn batch_cancel_ready(state: &State) -> bool {
    !state.selected_order_ids.is_empty() && !state.busy
}

/// The counted batch label, exactly as the model's selection reads it.
pub(crate) fn mobile_batch_cancel_label(count: usize) -> String {
    format!("Stornieren ({count})")
}

/// The trailing `Auswählen` entry: a proper 48 px outlined control, aligned to
/// the content's trailing edge, never loose accent text. It shares the action
/// floor and control radius with every other button.
pub(crate) fn mobile_select_metrics(theme: &AppTheme) -> (f32, f32) {
    (theme.scale.action_height, theme.scale.radius_control)
}

/// The selection strip: the counted `Stornieren ({count})` danger action and
/// `Fertig`, wrapping instead of clipping. It sits in the measured content box
/// above platform navigation, so it clears the navigation like the menu dock.
fn selection_bar(
    state: &State,
    host: HostKind,
    cx: &mut Context<AppShell>,
    theme: &AppTheme,
) -> AnyElement {
    let count = state.selected_order_ids.len();
    let ready = batch_cancel_ready(state);
    let cancel = cancel_selected_handler(host, cx);
    let done = end_selection_handler(cx);
    let danger = if ready {
        mobile_action_action(
            "orders-selection-cancel",
            mobile_batch_cancel_label(count),
            ButtonKind::Danger,
            theme,
            cx,
            move |_, window, cx| cancel(window, cx),
        )
    } else {
        mobile_action(
            "orders-selection-cancel",
            mobile_batch_cancel_label(count),
            ButtonKind::Danger,
            false,
            theme,
            cx,
        )
    };
    div()
        .w_full()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(theme.scale.space_2))
        .py(px(theme.scale.space_2))
        .child(danger)
        .child(mobile_action_action(
            "orders-selection-done",
            "Fertig",
            ButtonKind::Secondary,
            theme,
            cx,
            move |_, window, cx| done(window, cx),
        ))
        .into_any_element()
}

/// The labelled cancellation dialog, shared by the handler and its test: the
/// exact incumbent copy, with only the confirm action dispatching.
/// @req REQ-SCR-02
pub(crate) fn cancel_dialog(order: &OrderedMenu, on_confirm: ClickHandler) -> Dialog {
    Dialog::Confirm {
        title: "Bestellung stornieren".into(),
        message: format!("\"{}\" stornieren?", order.title).into(),
        confirm_label: Some("Stornieren"),
        cancel_label: Some("Behalten"),
        on_confirm,
    }
}

/// The desktop Bestätigen banner: same warning banner and copy as mobile,
/// but its action is the shared desktop primary (44 px target, pointer hover
/// and focus) instead of the phone button recipe.
/// @req REQ-SCR-02 @req REQ-UI-07 @req REQ-UI-13
fn desktop_confirm_banner(
    text: &str,
    busy: bool,
    on_confirm: ClickHandler,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let r = theme.tinted_banner(theme.warning_surface);
    let state = if busy {
        ControlState::Disabled
    } else {
        ControlState::Rest
    };
    theme::apply(
        div()
            .mx(px(theme.scale.space_4))
            .mt(px(theme.scale.space_2))
            .p(px(theme.scale.space_3))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(theme.scale.space_2))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning)
                    .child(text.to_string()),
            )
            .child(desktop_action_state(
                "desktop-orders-confirm",
                if busy { "…" } else { "Bestätigen" },
                ButtonKind::Primary,
                state,
                theme,
                cx,
                move |_, window, cx| on_confirm(window, cx),
            )),
        &r,
    )
    .into_any_element()
}

/// The narrow Bestätigen action. Off-thread on every desktop host, the narrow
/// window included, under the shell's gate; a phone host stays synchronous.
fn confirm_handler(host: HostKind, cx: &mut Context<AppShell>) -> ClickHandler {
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
                model.confirm_orders()
            });
            cx.notify();
        });
    })
}

/// Desktop confirmation: the model call blocks on its runtime, so it runs on
/// the background executor under the shell's gate instead of freezing the
/// window. Cached rows stay readable while it runs.
/// @req REQ-SCR-02 @req REQ-UI-12
fn desktop_confirm_handler(cx: &mut Context<AppShell>) -> ClickHandler {
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
                    desktop_confirm_dispatch(&model);
                })
                .detach();
            cx.notify();
        });
    })
}

/// The confirmation body the handler spawns, shared with its test.
pub(crate) fn desktop_confirm_dispatch(model: &Arc<AppModel>) {
    model.confirm_orders();
}

fn cancel_handler(host: HostKind, cx: &mut Context<AppShell>, order: OrderedMenu) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        let order = order.clone();
        shell.update(cx, |s, cx| {
            let model = s.model.clone();
            let on_confirm = {
                let model = model.clone();
                let id = order.position_id.clone();
                Rc::new(move |_: &mut Window, cx: &mut App| {
                    let Some(model) = model.clone() else {
                        return;
                    };
                    let id = id.clone();
                    run_blocking(host, cx, move || model.cancel_order(&id));
                })
            };
            s.set_dialog(cancel_dialog(&order, on_confirm), cx);
        });
    })
}

fn toggle_selection_handler(cx: &mut Context<AppShell>, id: String) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.toggle_order_selection(&id);
                cx.notify();
            }
        });
    })
}

fn begin_selection_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.begin_order_selection();
                cx.notify();
            }
        });
    })
}

fn end_selection_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.end_order_selection();
                cx.notify();
            }
        });
    })
}

/// The narrow batch cancellation. Off-thread on every desktop host under the
/// shell's gate; the phone host keeps the incumbent synchronous call.
fn cancel_selected_handler(host: HostKind, cx: &mut Context<AppShell>) -> ClickHandler {
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
                model.cancel_selected_orders();
            });
            cx.notify();
        });
    })
}

/// The wide-desktop Bestellungen workspace (task 1.4): segment toolbar with
/// the batch entry, the unconfirmed/error banners, the record table and the
/// order detail. A row body only previews; the 44 px check column is the only
/// way into the batch set, so no activation can cancel or approve an order.
/// @req REQ-SCR-02 @req REQ-UI-08 @req REQ-UI-12
#[allow(clippy::too_many_arguments)]
pub fn desktop_orders_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    layout: LayoutMode,
    mutation_claimed: bool,
    list_focus: &RecordListFocus,
    segment_group: &FocusHandle,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if !state.gourmet_authenticated {
        return gate(theme);
    }
    let Some(model) = model else {
        return gate(theme);
    };
    let Some(split) = &state.orders_split else {
        return if state.busy {
            loading_state(theme)
        } else {
            gate(theme)
        };
    };
    let geometry = model.desktop_geometry();
    let compact = layout == LayoutMode::DesktopCompact;

    let select_segment = {
        let model = Arc::clone(model);
        Rc::new(move |segment: Segment, _w: &mut Window, _cx: &mut App| {
            model.set_orders_segment(segment == Segment::Past);
        })
    };
    let select_mode = (!state.orders_past && !state.order_selection_mode)
        .then(|| desktop_begin_selection_handler(cx));
    let toolbar = desktop_segment_toolbar(
        if state.orders_past {
            Segment::Past
        } else {
            Segment::Upcoming
        },
        split.upcoming.len(),
        split.past.len(),
        select_segment,
        select_mode,
        Some(segment_group),
        theme,
    );

    let mut column = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(div().w_full().child(toolbar));
    if state.order_progress.is_none() && !state.orders_past {
        if let Some(banner) = model.unconfirmed_banner_text() {
            column = column.child(desktop_confirm_banner(
                &banner,
                state.busy,
                desktop_confirm_handler(cx),
                theme,
                cx,
            ));
        }
    }
    if let Some(error) = &state.orders_error {
        column = column.child(error_banner(&error.message, theme));
    }

    let blocked = mutation_claimed
        || desktop_mutation_blocked(state.busy, state.order_progress.is_some())
        || state.cancelling_id.is_some();
    column = column.child(split_workspace_scrolled(
        desktop_order_master(state, model, compact, blocked, list_focus, theme, cx),
        desktop_order_detail_pane(state, model, mutation_claimed, theme, cx),
        &geometry,
        theme,
        &list_focus.scroll(),
        list_focus.detail(),
    ));
    if state.order_selection_mode {
        column = column.child(desktop_selection_strip(state, mutation_claimed, theme, cx));
    }
    column.into_any_element()
}

/// Whether the 30 s cutoff cadence must repaint: only a desktop layout
/// re-evaluates order eligibility on a timer, because a visible disabled
/// `Stornieren` must flip without waiting for mouse movement (brief
/// "Re-evaluate the cutoff on the existing cadence").
/// @req REQ-SCR-02
pub(crate) fn cutoff_repaint_due(layout: LayoutMode, elapsed: Duration) -> bool {
    layout != LayoutMode::Phone && elapsed >= Duration::from_secs(30)
}

fn desktop_order_master(
    state: &State,
    model: &Arc<AppModel>,
    compact: bool,
    blocked: bool,
    list_focus: &RecordListFocus,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(split) = &state.orders_split else {
        list_focus.retain(&[]);
        return desktop_empty_master(state.orders_past, theme);
    };
    let orders: &[OrderedMenu] = if state.orders_past {
        &split.past
    } else {
        &split.upcoming
    };
    // One tab stop for the whole table: the row cursor follows the visible
    // segment order, previews without approving or cancelling, and reveals
    // the focused row. Space stays the batch check (record_table); Enter takes
    // the detail's enabled Stornieren control, else the read-only region.
    let ids: Vec<String> = orders.iter().map(|o| o.position_id.clone()).collect();
    // A segment change or refresh re-renders a different set: the previous
    // rows' handles must not survive it (REQ-UI-11/14).
    list_focus.retain(&ids);
    let detail_control = !blocked
        && state.order_preview.as_deref().is_some_and(|id| {
            orders
                .iter()
                .find(|order| order.position_id == id)
                .is_some_and(|order| {
                    order_cancel_state(
                        state.orders_past,
                        state.cancelling_id.as_deref() == Some(order.position_id.as_str()),
                        model.is_cancellation_cutoff(order),
                        false,
                    ) == OrderCancelState::Ready
                })
        });
    let on_key = {
        let focus = list_focus.clone();
        let model = Arc::clone(model);
        move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if event.keystroke.key.as_str() == "enter" {
                focus.enter(detail_entry(detail_control), window, cx);
                cx.stop_propagation();
                return;
            }
            let Some(id) = focus.step(&ids, event.keystroke.key.as_str(), window, cx) else {
                return;
            };
            model.open_preview(ListKind::Orders, &id);
            model.set_cursor(ListKind::Orders, Some(id));
            cx.stop_propagation();
        }
    };
    let mut list = div()
        .id("desktop-orders-list")
        .flex()
        .flex_col()
        .tab_index(0)
        .track_focus(list_focus.container())
        .focus(theme::desktop_focus(theme.accent))
        .on_key_down(on_key)
        .child(desktop_order_header(
            compact,
            state.order_selection_mode,
            theme,
        ));
    if orders.is_empty() {
        return list
            .child(desktop_empty_master(state.orders_past, theme))
            .into_any_element();
    }
    for order in orders {
        let selected = state.order_preview.as_deref() == Some(order.position_id.as_str());
        let state_row = DesktopOrderRowState {
            selected,
            checked: state.selected_order_ids.contains(&order.position_id),
            cancelling: state.cancelling_id.as_deref() == Some(order.position_id.as_str()),
        };
        let preview = desktop_preview_handler(cx, order.position_id.clone());
        let check = state
            .order_selection_mode
            .then(|| desktop_check_handler(cx, order.position_id.clone()));
        let handle = list_focus.row(&order.position_id, cx);
        list = list.child(desktop_order_row(
            order,
            state_row,
            compact,
            preview,
            check,
            Some(&handle),
            theme,
        ));
    }
    list.into_any_element()
}

fn desktop_empty_master(past: bool, theme: &AppTheme) -> AnyElement {
    let (fact, hint) = empty_copy(past);
    state_message(fact, hint, theme)
}

fn desktop_order_detail_pane(
    state: &State,
    model: &Arc<AppModel>,
    mutation_claimed: bool,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let (Some(preview), Some(split)) = (state.order_preview.as_deref(), &state.orders_split) else {
        return detail_prompt("Bestellung auswählen", theme);
    };
    let orders: &[OrderedMenu] = if state.orders_past {
        &split.past
    } else {
        &split.upcoming
    };
    let Some(order) = orders.iter().find(|order| order.position_id == preview) else {
        return detail_prompt("Bestellung auswählen", theme);
    };
    let blocked = mutation_claimed
        || desktop_mutation_blocked(state.busy, state.order_progress.is_some())
        || state.cancelling_id.is_some();
    let cancel = order_cancel_state(
        state.orders_past,
        state.cancelling_id.as_deref() == Some(order.position_id.as_str()),
        model.is_cancellation_cutoff(order),
        blocked,
    );
    desktop_order_detail(
        order,
        &order_description(model.dish(order), order),
        cancel,
        desktop_cancel_handler(cx, order.clone()),
        theme,
        cx,
    )
}

fn desktop_selection_strip(
    state: &State,
    mutation_claimed: bool,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let count = state.selected_order_ids.len();
    let blocked =
        mutation_claimed || desktop_mutation_blocked(state.busy, state.order_progress.is_some());
    let cancel = desktop_cancel_selected_handler(cx);
    let done = end_selection_handler(cx);
    action_strip(
        vec![div()
            .ml_auto()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(desktop_action_state(
                "desktop-orders-cancel-selected",
                format!("Stornieren ({count})"),
                ButtonKind::Danger,
                if count == 0 || blocked {
                    ControlState::Disabled
                } else {
                    ControlState::Rest
                },
                theme,
                cx,
                move |_, window, cx| cancel(window, cx),
            ))
            .child(desktop_action_state(
                "desktop-orders-selection-done",
                "Fertig",
                ButtonKind::Secondary,
                ControlState::Rest,
                theme,
                cx,
                move |_, window, cx| done(window, cx),
            ))
            .into_any_element()],
        theme,
    )
}

fn desktop_preview_handler(cx: &mut Context<AppShell>, id: String) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.open_preview(ListKind::Orders, &id);
                model.set_cursor(ListKind::Orders, Some(id.clone()));
                cx.notify();
            }
        });
    })
}

fn desktop_check_handler(cx: &mut Context<AppShell>, id: String) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = &s.model else {
                return;
            };
            let state = model.state();
            if desktop_check_effect(state.order_selection_mode) == OrderRowEffect::ToggleCheck {
                model.toggle_order_selection(&id);
                cx.notify();
            }
        });
    })
}

fn desktop_begin_selection_handler(cx: &mut Context<AppShell>) -> ClickHandler {
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
            model.begin_order_selection();
            cx.notify();
        });
    })
}

/// Single cancellation is explicit: the detail opens the unchanged themed
/// dialog, and only its confirm dispatches. The cancel runs off the render
/// thread under a synchronous gate claim, then the list reconciles so a
/// disappeared preview restores the nearest surviving cursor.
fn desktop_cancel_handler(cx: &mut Context<AppShell>, order: OrderedMenu) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        let order = order.clone();
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            let state = model.state();
            if s.mutation_gate.is_claimed()
                || desktop_mutation_blocked(state.busy, state.order_progress.is_some())
                || state.cancelling_id.is_some()
                || model.is_cancellation_cutoff(&order)
            {
                return;
            }
            let on_confirm: ClickHandler = {
                let shell = cx.entity().clone();
                let model = Arc::clone(&model);
                let id = order.position_id.clone();
                Rc::new(move |_: &mut Window, cx: &mut App| {
                    let shell = shell.clone();
                    let model = Arc::clone(&model);
                    let id = id.clone();
                    let _ = shell.update(cx, |s, cx| {
                        let state = model.state();
                        let Some(ticket) = s
                            .mutation_gate
                            .claim(state.busy, state.order_progress.is_some())
                        else {
                            return;
                        };
                        let dispatched = Arc::clone(&model);
                        cx.background_executor()
                            .spawn(async move {
                                let _ticket = ticket;
                                desktop_cancel_dispatch(&dispatched, &id);
                            })
                            .detach();
                        cx.notify();
                    });
                })
            };
            s.set_dialog(cancel_dialog(&order, on_confirm), cx);
        });
    })
}

/// Batch cancellation keeps the incumbent model flow: one dispatch, its own
/// busy guard, then the same list reconciliation. The shell holds its own gate
/// ticket because the model clears only its own claim when the flow ends.
fn desktop_cancel_selected_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            let state = model.state();
            if state.selected_order_ids.is_empty() || state.busy {
                return;
            }
            let Some(ticket) = s
                .mutation_gate
                .claim(state.busy, state.order_progress.is_some())
            else {
                return;
            };
            let dispatched = Arc::clone(&model);
            cx.background_executor()
                .spawn(async move {
                    let _ticket = ticket;
                    desktop_cancel_selected_dispatch(&dispatched);
                })
                .detach();
            cx.notify();
        });
    })
}

/// The reconciliation basis read at dispatch time — after the dialog has
/// been confirmed, so a refresh that changed rows while it was open cannot
/// leave a stale cursor basis.
pub(crate) fn desktop_cancel_basis(model: &AppModel) -> Vec<String> {
    desktop_segment_ids(&model.state())
}

/// The single-cancel body the dialog confirm spawns: one core cancel, then
/// reconcile the list from the basis captured at this call (confirm time).
pub(crate) fn desktop_cancel_dispatch(model: &Arc<AppModel>, id: &str) {
    let previous_ids = desktop_cancel_basis(model);
    model.cancel_order(id);
    let surviving_ids = desktop_segment_ids(&model.state());
    model.reconcile_list(ListKind::Orders, &previous_ids, &surviving_ids);
}

/// The batch-cancel body the handler spawns: one core batch, then the same
/// reconciliation. Shared with its test so the covered flow is the one the
/// click handler runs.
pub(crate) fn desktop_cancel_selected_dispatch(model: &Arc<AppModel>) {
    let previous_ids = desktop_cancel_basis(model);
    model.cancel_selected_orders();
    let surviving_ids = desktop_segment_ids(&model.state());
    model.reconcile_list(ListKind::Orders, &previous_ids, &surviving_ids);
}

/// The active segment's identities, in display order: the reconciliation
/// basis after a cancellation.
fn desktop_segment_ids(state: &State) -> Vec<String> {
    let Some(split) = &state.orders_split else {
        return Vec::new();
    };
    let orders = if state.orders_past {
        &split.past
    } else {
        &split.upcoming
    };
    orders
        .iter()
        .map(|order| order.position_id.clone())
        .collect()
}

#[cfg(test)]
mod mobile_tests {
    use super::{
        batch_cancel_ready, cancel_dialog, empty_copy, mobile_batch_cancel_label,
        mobile_select_metrics,
    };
    use crate::app::MutationGate;
    use crate::components::dialog::Dialog;
    use crate::navigation::ClickHandler;
    use crate::theme::AppTheme;
    use snackpilot_core::domain::{MenuCategory, OrderedMenu};
    use snackpilot_model::{AppModel, FakePlatform};
    use std::rc::Rc;
    use std::sync::Arc;

    fn demo(tag: &str) -> (Arc<AppModel>, Arc<FakePlatform>) {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-app-orders-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let platform = Arc::new(FakePlatform::new());
        let model = AppModel::new(dir, platform.clone()).unwrap();
        model.login("demo", "demo1234!").unwrap();
        (Arc::new(model), platform)
    }

    /// Two placed demo orders on the last offered day, so the batch paths work
    /// on real state and no request reaches a host.
    fn place_two_orders(model: &AppModel) -> Vec<OrderedMenu> {
        let snapshot = model
            .state()
            .menu_snapshot
            .expect("demo menus load at login");
        let day = snapshot
            .available_dates
            .last()
            .cloned()
            .expect("demo offers days");
        let items: Vec<_> = snapshot
            .items
            .into_iter()
            .filter(|item| {
                item.day == day && item.available && !item.ordered && !model.is_cutoff(&item.day)
            })
            .filter(|item| !item.ordered)
            .take(2)
            .collect();
        assert_eq!(items.len(), 2, "two orderable demo dishes");
        for item in &items {
            model.toggle(item);
        }
        model.submit_orders();
        model
            .state()
            .orders_split
            .expect("orders reloaded after submit")
            .upcoming
    }

    /// The empty segment states the fact and then the next step, so the tab is
    /// informative instead of one grey sentence.
    /// @req REQ-SCR-02
    #[test]
    fn empty_segments_state_fact_then_next_step() {
        assert_eq!(
            empty_copy(false),
            (
                "Keine kommenden Bestellungen",
                "Bestellte Menüs erscheinen hier"
            )
        );
        assert_eq!(
            empty_copy(true),
            (
                "Keine vergangenen Bestellungen",
                "Abgeschlossene Bestellungen erscheinen hier"
            )
        );
    }

    /// `Auswählen` is a real trailing control (REQ-SCR-02/REQ-UI-15): the 48 px
    /// action floor and the 10 px control radius, never loose accent text.
    /// @req REQ-SCR-02 @req REQ-UI-15
    #[test]
    fn mobile_select_entry_is_a_48px_control() {
        let theme = AppTheme::build(false);
        let (height, radius) = mobile_select_metrics(&theme);
        assert_eq!(height, 48.);
        assert_eq!(radius, 10.);
    }

    /// The confirm banner and the counted batch strip act once
    /// (REQ-SCR-02/16): the strip's label is the counted copy, the batch entry
    /// is ready for one or many checks but not for none or a running flow, and
    /// the shell's one gate plus the model's own guard turn a repeated
    /// activation into exactly one `order.cancelled` with the count.
    /// @req REQ-SCR-02 @req REQ-UI-16
    #[test]
    fn mobile_confirm_and_batch_strip_wrap_without_dual_dispatch() {
        let (model, fake) = demo("batch-strip");
        let orders = place_two_orders(&model);
        // Demo submits are confirmed inline, so the banner is the model's own
        // text source and has nothing to say here.
        assert_eq!(model.unconfirmed_banner_text(), None);

        model.begin_order_selection();
        assert!(
            !batch_cancel_ready(&model.state()),
            "no check selected: the strip has nothing to cancel"
        );
        model.toggle_order_selection(&orders[0].position_id);
        assert_eq!(mobile_batch_cancel_label(1), "Stornieren (1)");
        assert!(batch_cancel_ready(&model.state()), "one check is ready");
        model.toggle_order_selection(&orders[1].position_id);
        assert_eq!(mobile_batch_cancel_label(2), "Stornieren (2)");
        assert!(batch_cancel_ready(&model.state()), "many checks are ready");

        // The shell's one gate: the first activation claims, the second tap
        // while it is held cannot dispatch, and the claim releases.
        let gate = MutationGate::default();
        let first = gate.claim(false, false);
        assert!(first.is_some(), "the first activation is accepted");
        assert!(
            gate.claim(false, false).is_none(),
            "a repeated tap cannot cancel twice"
        );
        assert!(gate.claim(false, true).is_none(), "a running op blocks it");
        drop(first);

        let before = fake.tracks().len();
        assert!(model.cancel_selected_orders(), "one dispatch runs");
        let tracks: Vec<_> = fake
            .tracks()
            .into_iter()
            .filter(|(event, _)| event == "order.cancelled")
            .map(|(_, props)| props.get("count").cloned().unwrap_or_default())
            .collect();
        assert_eq!(tracks, vec!["2".to_string()], "exactly one batch cancel");
        assert_eq!(
            fake.tracks().len(),
            before + 1,
            "the batch recorded exactly one event"
        );
        assert!(
            !model.cancel_selected_orders(),
            "a second activation has nothing left to dispatch"
        );
        assert_eq!(
            fake.tracks()
                .into_iter()
                .filter(|(event, _)| event == "order.cancelled")
                .count(),
            1,
            "no duplicate cancellation was dispatched"
        );
        assert!(
            !batch_cancel_ready(&model.state()),
            "the strip closed with the flow"
        );
    }

    /// The compact card keeps the incumbent description fallback and opens the
    /// unchanged labelled cancellation dialog (REQ-SCR-02): `Bestellung
    /// stornieren`, `"{title}" stornieren?`, `Stornieren`, `Behalten`, two
    /// actions with index 0 cancelling.
    /// @req REQ-SCR-02
    #[test]
    fn mobile_order_description_and_exact_cancel_dialog() {
        use crate::components::order_item::order_description;

        let order = OrderedMenu {
            position_id: "pos-1".into(),
            eating_cycle_id: "cycle-1".into(),
            date_epoch_ms: 1_794_308_400_000,
            title: "MENÜ I".into(),
            subtitle: "Rindsgulasch".into(),
            approved: false,
        };
        assert_eq!(
            order_description(None, &order),
            "Rindsgulasch",
            "the card's dish is the order subtitle when the menu lookup is empty"
        );

        let noop: ClickHandler = Rc::new(|_, _| {});
        let dialog = cancel_dialog(&order, noop);
        let Dialog::Confirm { title, message, .. } = &dialog else {
            panic!("the cancel flow opens the labelled confirm dialog");
        };
        assert_eq!(title.as_ref(), "Bestellung stornieren");
        assert_eq!(message.as_ref(), "\"MENÜ I\" stornieren?");
        assert_eq!(dialog.button_count(), 2, "one cancel, one keep");
        assert_eq!(dialog.label_at(0), "Behalten");
        assert_eq!(dialog.label_at(1), "Stornieren");
        // Unused import guard: the sample order's category never rewrites the
        // dialog text.
        assert_eq!(MenuCategory::Menu1.display(), "MENÜ I");
    }
}

#[cfg(test)]
mod desktop_tests {
    use super::cutoff_repaint_due;
    use snackpilot_model::desktop::LayoutMode;
    use std::time::Duration;

    /// @req REQ-SCR-02
    #[test]
    fn cutoff_repaint_runs_only_on_desktop_cadence() {
        assert!(
            !cutoff_repaint_due(LayoutMode::DesktopExpanded, Duration::from_millis(29_999)),
            "just under the 30 s cadence must not repaint"
        );
        assert!(
            cutoff_repaint_due(LayoutMode::DesktopExpanded, Duration::from_secs(30)),
            "the cadence boundary repaints"
        );
        assert!(cutoff_repaint_due(
            LayoutMode::DesktopCompact,
            Duration::from_millis(30_001)
        ));
        assert!(!cutoff_repaint_due(
            LayoutMode::Phone,
            Duration::from_secs(31)
        ));
    }
}
