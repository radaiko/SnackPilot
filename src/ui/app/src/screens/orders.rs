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
    desktop_mutation_blocked, order_cancel_state, order_row_effect, AppShell, OrderRowEffect,
};
use crate::components::button::{desktop_action_state, ButtonKind};
use crate::components::order_item::{
    desktop_order_detail, desktop_order_header, desktop_order_row, order_description, order_item,
    DesktopOrderRowState,
};
use crate::components::segment_tabs::{desktop_segment_toolbar, segment_tabs, Segment};
use crate::components::spinner::spinner;
use crate::components::workspace::{action_strip, detail_prompt, split_workspace};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::OrderedMenu;
use snackpilot_model::desktop::{ControlState, LayoutMode};
use snackpilot_model::navigation::ListKind;
use snackpilot_model::{AppModel, State};

/// The Orders tab content.
pub fn orders_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    past: bool,
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
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.primary, 36.))
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

    let mut column = div().size_full().flex().flex_col();
    column = column.child(segment_tabs(
        active,
        split.upcoming.len(),
        split.past.len(),
        select_segment,
        theme,
    ));

    if state.order_progress.is_none() {
        if let Some(banner) = model.unconfirmed_banner_text() {
            if !past {
                column = column.child(confirm_banner(
                    &banner,
                    state.busy,
                    confirm_handler(cx),
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
        .flex_1()
        .flex()
        .flex_col()
        .p(px(16.))
        .pb(px(100.))
        .overflow_y_scroll();

    if orders.is_empty() {
        list = list.child(
            div().flex_1().flex().items_center().justify_center().child(
                div()
                    .text_size(px(16.))
                    .text_color(theme.text_tertiary)
                    .child(if past {
                        "Keine vergangenen Bestellungen"
                    } else {
                        "Keine kommenden Bestellungen"
                    }),
            ),
        );
    } else {
        for order in orders {
            let cancelling = state.cancelling_id.as_deref() == Some(order.position_id.as_str());
            let can_cancel = !past && state.cancelling_id.is_none();
            let cutoff = model.is_cancellation_cutoff(order);
            let dish = model.dish(order);
            let handler = cancel_handler(cx, order.clone());
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

    column = column.child(list);

    // Batch-cancel bar while multi-select mode is active.
    if state.order_selection_mode {
        column = column.child(selection_bar(state, cx, theme));
    } else if !past {
        let handler = begin_selection_handler(cx);
        column = column.child(
            div().flex().justify_end().child(
                div()
                    .id("orders-select")
                    .px(px(16.))
                    .py(px(8.))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| handler(window, cx))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.primary)
                            .child("Auswählen"),
                    ),
            ),
        );
    }

    column.into_any_element()
}

fn gate(theme: &AppTheme) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme.text_tertiary)
                .child("Anmeldung erforderlich"),
        )
        .into_any_element()
}

fn confirm_banner(
    text: &str,
    busy: bool,
    on_confirm: ClickHandler,
    theme: &AppTheme,
) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_warning);
    let r_button = theme.button_primary();
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
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.warning_text)
                    .child(text.to_string()),
            )
            .child(
                theme::apply(div().px(px(20.)).py(px(8.)), &r_button)
                    .id("orders-confirm")
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_confirm(window, cx))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.on_primary)
                            .child(if busy { "…" } else { "Bestätigen" }),
                    ),
            ),
        &r,
    )
    .into_any_element()
}

fn error_banner(message: &str, theme: &AppTheme) -> AnyElement {
    let r = theme.tinted_banner(theme.glass_error);
    theme::apply(
        div().mx(px(16.)).mt(px(8.)).p(px(12.)).child(
            div()
                .text_size(px(13.))
                .text_color(theme.error_text)
                .child(message.to_string()),
        ),
        &r,
    )
    .into_any_element()
}

fn selection_bar(state: &State, cx: &mut Context<AppShell>, theme: &AppTheme) -> AnyElement {
    let cancel = cancel_selected_handler(cx);
    let done = end_selection_handler(cx);
    let count = state.selected_order_ids.len();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .px(px(16.))
        .py(px(8.))
        .bg(theme.surface)
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .id("orders-selection-cancel")
                .px(px(16.))
                .py(px(8.))
                .rounded(px(8.))
                .bg(theme.error)
                .cursor_pointer()
                .on_click(move |_, window, cx| cancel(window, cx))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.on_primary)
                        .child(format!("Stornieren ({count})")),
                ),
        )
        .child(
            div()
                .id("orders-selection-done")
                .px(px(16.))
                .py(px(8.))
                .cursor_pointer()
                .on_click(move |_, window, cx| done(window, cx))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.primary)
                        .child("Fertig"),
                ),
        )
        .into_any_element()
}

fn confirm_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.confirm_orders();
                cx.notify();
            }
        });
    })
}

fn cancel_handler(cx: &mut Context<AppShell>, order: OrderedMenu) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        let order = order.clone();
        shell.update(cx, |s, cx| {
            let model = s.model.clone();
            let on_confirm = {
                let model = model.clone();
                let id = order.position_id.clone();
                Rc::new(move |_: &mut Window, _: &mut App| {
                    if let Some(model) = &model {
                        model.cancel_order(&id);
                    }
                })
            };
            let message = format!("\"{}\" stornieren?", order.title);
            s.set_dialog(
                crate::components::dialog::Dialog::Confirm {
                    title: "Bestellung stornieren".into(),
                    message: message.into(),
                    confirm_label: Some("Stornieren"),
                    cancel_label: Some("Behalten"),
                    on_confirm,
                },
                cx,
            );
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

fn cancel_selected_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.cancel_selected_orders();
                cx.notify();
            }
        });
    })
}

/// The wide-desktop Bestellungen workspace (task 1.4): segment toolbar with
/// the batch entry, the unconfirmed/error banners, the record table and the
/// order detail. A row body only previews; the 44 px check column is the only
/// way into the batch set, so no activation can cancel or approve an order.
/// @req REQ-SCR-02 @req REQ-UI-08 @req REQ-UI-12
pub fn desktop_orders_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    layout: LayoutMode,
    mutation_claimed: bool,
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
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.primary, 36.))
                .into_any_element()
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
            column = column.child(confirm_banner(
                &banner,
                state.busy,
                confirm_handler(cx),
                theme,
            ));
        }
    }
    if let Some(error) = &state.orders_error {
        column = column.child(error_banner(&error.message, theme));
    }

    column = column.child(split_workspace(
        desktop_order_master(state, model, compact, theme, cx),
        desktop_order_detail_pane(state, model, mutation_claimed, theme, cx),
        &geometry,
        theme,
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
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let Some(split) = &state.orders_split else {
        return desktop_empty_master(state.orders_past, theme);
    };
    let orders: &[OrderedMenu] = if state.orders_past {
        &split.past
    } else {
        &split.upcoming
    };
    let mut list = div().flex().flex_col().child(desktop_order_header(
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
        let description = order_description(model.dish(order), order);
        let preview = desktop_preview_handler(cx, order.position_id.clone());
        let check = state
            .order_selection_mode
            .then(|| desktop_check_handler(cx, order.position_id.clone()));
        list = list.child(desktop_order_row(
            order,
            &description,
            state_row,
            compact,
            preview,
            check,
            theme,
        ));
    }
    list.into_any_element()
}

fn desktop_empty_master(past: bool, theme: &AppTheme) -> AnyElement {
    div()
        .w_full()
        .p(px(16.))
        .pt(px(32.))
        .flex()
        .justify_center()
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(theme.text_secondary)
                .text_center()
                .child(if past {
                    "Keine vergangenen Bestellungen"
                } else {
                    "Keine kommenden Bestellungen"
                }),
        )
        .into_any_element()
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
            if order_row_effect(true, state.order_selection_mode) == OrderRowEffect::ToggleCheck {
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
            let previous_ids = desktop_segment_ids(&state);
            let on_confirm: ClickHandler = {
                let shell = cx.entity().clone();
                let model = Arc::clone(&model);
                let id = order.position_id.clone();
                Rc::new(move |_: &mut Window, cx: &mut App| {
                    let shell = shell.clone();
                    let model = Arc::clone(&model);
                    let id = id.clone();
                    let previous_ids = previous_ids.clone();
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
                                dispatched.cancel_order(&id);
                                let state = dispatched.state();
                                let surviving_ids = desktop_segment_ids(&state);
                                dispatched.reconcile_list(
                                    ListKind::Orders,
                                    &previous_ids,
                                    &surviving_ids,
                                );
                            })
                            .detach();
                        cx.notify();
                    });
                })
            };
            let message = format!("\"{}\" stornieren?", order.title);
            s.set_dialog(
                crate::components::dialog::Dialog::Confirm {
                    title: "Bestellung stornieren".into(),
                    message: message.into(),
                    confirm_label: Some("Stornieren"),
                    cancel_label: Some("Behalten"),
                    on_confirm,
                },
                cx,
            );
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
            let previous_ids = desktop_segment_ids(&state);
            let dispatched = Arc::clone(&model);
            cx.background_executor()
                .spawn(async move {
                    let _ticket = ticket;
                    dispatched.cancel_selected_orders();
                    let state = dispatched.state();
                    let surviving_ids = desktop_segment_ids(&state);
                    dispatched.reconcile_list(ListKind::Orders, &previous_ids, &surviving_ids);
                })
                .detach();
            cx.notify();
        });
    })
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
mod desktop_tests {
    use super::cutoff_repaint_due;
    use snackpilot_model::desktop::LayoutMode;
    use std::time::Duration;

    /// @req REQ-SCR-02
    #[test]
    fn cutoff_repaint_runs_only_on_desktop_cadence() {
        assert!(cutoff_repaint_due(
            LayoutMode::DesktopCompact,
            Duration::from_secs(30)
        ));
        assert!(cutoff_repaint_due(
            LayoutMode::DesktopExpanded,
            Duration::from_secs(31)
        ));
        assert!(!cutoff_repaint_due(
            LayoutMode::DesktopExpanded,
            Duration::from_secs(29)
        ));
        assert!(!cutoff_repaint_due(
            LayoutMode::Phone,
            Duration::from_secs(30)
        ));
    }
}
