//! Billing tab — docs/requirements/04-ui-ux.md §3.3 and §4.7.
//!
//! Month tabs, source chips, summary bar, newest-first `BillCard` list, the
//! empty/loading/error states and pull-to-refresh. All data comes from the
//! model's merged entries and totals; the selected month and source live in
//! `AppModel` state.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;

use crate::app::AppShell;
use crate::components::bill_card::bill_card;
use crate::components::pull_refresh::pull_refresh;
use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_model::{format_eur, AppModel, BillingEntry, BillingSource, BillingTotals, State};

/// The Billing tab content.
pub fn billing_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    if !state.gourmet_authenticated && !state.ventopay_authenticated {
        return gate(theme);
    }
    let Some(model) = model else {
        return gate(theme);
    };

    let entries = model.merged_entries();
    let totals = model.billing_totals();

    let mut column = div().size_full().flex().flex_col();
    column = column.child(month_row(state, model, theme, cx));

    let mut content = div().flex_1().flex().flex_col();

    if let Some(error) = model.billing_error() {
        content = content.child(error_banner(&error.message, theme));
    }

    if state.billing_loading && entries.is_empty() {
        content = content.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.primary, 36.)),
        );
    } else {
        content = content.child(chips_row(state, model, theme, cx));
        if !entries.is_empty() {
            content = content.child(summary(&entries, &totals, state.billing_source, theme));
        }
        let mut list = div()
            .id("billing-list")
            .flex_1()
            .flex()
            .flex_col()
            .p(px(16.))
            .pb(px(100.))
            .overflow_y_scroll();
        if entries.is_empty() {
            list = list.child(
                div().flex_1().flex().items_center().justify_center().child(
                    div()
                        .text_size(px(16.))
                        .text_color(theme.text_tertiary)
                        .child("Keine Abrechnungsdaten für diesen Monat"),
                ),
            );
        } else {
            for entry in &entries {
                list = list.child(bill_card(entry, theme));
            }
        }
        content = content.child(list);
    }

    column = column.child(pull_refresh(
        "billing-pull",
        content.into_any_element(),
        reload_handler(cx),
    ));
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

fn month_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Div {
    let border = if theme.flat {
        theme.border
    } else {
        theme.glass_highlight
    };
    let bg = if theme.flat {
        theme.surface
    } else {
        theme.glass_surface
    };
    let mut row = div()
        .flex()
        .w_full()
        .bg(bg)
        .border_b_1()
        .border_color(border);
    for option in state.month_options.iter() {
        let offset = option.offset;
        let selected = state.selected_offset == offset;
        let shell = cx.entity().clone();
        let model = Arc::clone(model);
        let action: ClickHandler = Rc::new(move |_: &mut Window, cx: &mut App| {
            model.select_month(offset);
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        row = row.child(
            div()
                .id(SharedString::from(format!("billing-month-{offset}")))
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .py(px(14.))
                .cursor_pointer()
                .border_b_3()
                .border_color(if selected {
                    theme.primary
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .on_click(move |_, window, cx| action(window, cx))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if selected {
                            theme.primary
                        } else {
                            theme.text_tertiary
                        })
                        .child(option.label.clone()),
                ),
        );
    }
    row
}

fn chips_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(16.))
        .py(px(8.));
    for (source, label) in [
        (BillingSource::All, "Alle"),
        (BillingSource::Gourmet, "Kantine"),
        (BillingSource::Ventopay, "Automaten"),
    ] {
        let selected = state.billing_source == source;
        let shell = cx.entity().clone();
        let model = Arc::clone(model);
        let action: ClickHandler = Rc::new(move |_: &mut Window, cx: &mut App| {
            model.set_billing_source(source);
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        row = row.child(
            div()
                .id(SharedString::from(format!("billing-chip-{label}")))
                .px(px(14.))
                .py(px(6.))
                .rounded(px(20.))
                .border_1()
                .border_color(if selected {
                    theme.primary
                } else if theme.flat {
                    theme.border
                } else {
                    theme.glass_highlight
                })
                .bg(if selected {
                    if theme.flat {
                        theme.primary_surface
                    } else {
                        theme.glass_primary
                    }
                } else if theme.flat {
                    theme.surface
                } else {
                    theme.glass_surface
                })
                .cursor_pointer()
                .on_click(move |_, window, cx| action(window, cx))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if selected {
                            theme.primary
                        } else {
                            theme.text_tertiary
                        })
                        .child(label),
                ),
        );
    }
    row
}

fn summary(
    entries: &[BillingEntry],
    totals: &BillingTotals,
    source: BillingSource,
    theme: &AppTheme,
) -> AnyElement {
    let show_subsidy = totals.zuschuss > 0. && source != BillingSource::Ventopay;
    let column = |label: &'static str, value: String, color: Hsla| {
        div()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_tertiary)
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(color)
                    .child(value),
            )
    };
    let bar = div()
        .mx(px(16.))
        .mt(px(4.))
        .py(px(12.))
        .px(px(16.))
        .flex()
        .justify_around();
    let mut bar = theme::apply(bar, &theme.banner_surface());
    bar = bar
        .child(column(
            "Gesamt",
            format_eur(totals.gesamt),
            theme.text_primary,
        ))
        .child(column(
            "Belege",
            entries.len().to_string(),
            theme.text_primary,
        ));
    if show_subsidy {
        bar = bar.child(column(
            "Zuschuss",
            format_eur(totals.zuschuss),
            theme.success,
        ));
    }
    bar.into_any_element()
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

fn reload_handler(cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.reload_billing();
                cx.notify();
            }
        });
    })
}
