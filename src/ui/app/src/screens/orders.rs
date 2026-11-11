//! Orders tab — docs/requirements/04-ui-ux.md §3.2 and §4.6.
//!
//! Segment tabs, the unconfirmed banner with its plural text, the confirm
//! banner action, `OrderItem` cards with the dish lookup and cancel flow, the
//! multi-select batch cancel and the empty/error states. All state comes from
//! `AppModel`, including the active segment (`State::orders_past`).

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;

use crate::app::AppShell;
use crate::components::order_item::order_item;
use crate::components::segment_tabs::{segment_tabs, Segment};
use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::OrderedMenu;
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
