//! Billing tab — docs/requirements/04-ui-ux.md §3.3 and §4.7.
//!
//! Month tabs, source chips, summary bar, newest-first `BillCard` list, the
//! empty/loading/error states and pull-to-refresh. All data comes from the
//! model's merged entries and totals; the selected month and source live in
//! `AppModel` state.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;

use crate::app::{dispatch_off_thread, AppShell};
use crate::components::bill_card::{
    bill_card, desktop_bill_detail, desktop_bill_header, desktop_bill_row,
};
use crate::components::chip;
use crate::components::chip::desktop_chip;
use crate::components::pull_refresh::pull_refresh;
use crate::components::record_table::{detail_entry, RecordListFocus};
use crate::components::spinner::spinner;
use crate::components::workspace::{detail_prompt, split_workspace_scrolled};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MonthOption;
use snackpilot_model::desktop::{ControlState, LayoutMode};
use snackpilot_model::navigation::{ListKind, Screen};
use snackpilot_model::{format_eur, AppModel, BillingEntry, BillingSource, BillingTotals, State};

/// The compact Billing tab content (task 1.5): shell-owned heading and gutters,
/// the three month choices, the wrapping source options, the inline error, the
/// neutral cached-refresh row, one summary surface and the scrolling receipts.
/// The filters stay painted while a fetch runs, a published month that is not
/// the selected context is withheld instead of painted under the new label,
/// and the legacy 16/100 px screen padding is gone so the shell's measured
/// reservation owns the scroll-end clearance.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
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
    // The context guard: only the selected month's published rows reach the
    // screen, so a month switch still in flight never paints the previous
    // month under the new label (design A17/§Abrechnung states).
    let cached = billing_context_cached(state);
    let has_entries = billing_context_matches(state) && !entries.is_empty();
    let view = billing_view(state.billing_loading, cached);
    // Host-gated dispatch: a narrow desktop window projects as the phone shell
    // but still runs its blocking billing work off the render thread; only a
    // phone/tablet host stays synchronous.
    let off_thread = dispatch_off_thread(state.host_kind);

    // One flex child of the shell's column: the month row and the filters stay
    // outside the record scroll, and the column takes exactly the height left
    // under the shell heading.
    let mut column = div().flex_1().min_h(px(0.)).flex().flex_col();
    column = column.child(mobile_month_row(
        &state.month_options,
        state.selected_offset,
        month_select(model, off_thread, cx),
        theme,
    ));

    let mut content = div().flex_1().min_h(px(0.)).flex().flex_col();
    content = content.child(mobile_source_row(
        state.billing_source,
        source_select(model, off_thread, cx),
        theme,
    ));

    if let Some(error) = model.billing_error() {
        content = content.child(mobile_error_banner(&error.message, theme));
    }

    match view {
        BillingView::Initial => {
            content = content.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(spinner(theme.primary, 36.)),
            );
        }
        BillingView::Refresh => {
            content = content.child(refresh_row(theme));
        }
        BillingView::Ready => {}
    }

    if view != BillingView::Initial {
        if has_entries {
            content = content.child(mobile_summary_surface(
                &entries,
                &totals,
                state.billing_source,
                theme,
            ));
        }
        let mut list = div()
            .id("billing-list")
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .overflow_y_scroll();
        if has_entries {
            for entry in &entries {
                list = list.child(bill_card(entry, theme));
            }
        } else {
            list = list.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(theme.mobile.space_4))
                    .child(
                        div()
                            .text_size(px(theme.mobile.body))
                            .line_height(px(theme.mobile.body_line))
                            .text_center()
                            .text_color(theme.mobile_secondary_on(theme.mobile_card_fill()))
                            .child("Keine Abrechnungsdaten für diesen Monat"),
                    ),
            );
        }
        content = content.child(list);
    }

    column = column.child(pull_refresh(
        "billing-pull",
        content.into_any_element(),
        reload_handler(off_thread, cx),
    ));
    column.into_any_element()
}

/// The compact Abrechnung loading projection (design A17): a fetch without a
/// cache for the selected context is the quiet initial spinner, a fetch over
/// that cache keeps the records and shows the neutral refresh row, and
/// everything else is ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BillingView {
    Initial,
    Refresh,
    Ready,
}

/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn billing_view(loading: bool, cached: bool) -> BillingView {
    match (loading, cached) {
        (true, false) => BillingView::Initial,
        (true, true) => BillingView::Refresh,
        (false, _) => BillingView::Ready,
    }
}

/// The selected month's key, from the existing month options.
fn billing_selected_key(state: &State) -> Option<&str> {
    state
        .month_options
        .iter()
        .find(|option| option.offset == state.selected_offset)
        .map(|option| option.key.as_str())
}

/// Whether every published source month belongs to the selected month. A
/// missing source month (not fetched, failed, signed out) is not a mismatch;
/// a present month with another key is, and its rows are never painted under
/// the new label.
/// @req REQ-SCR-03
pub(crate) fn billing_context_matches(state: &State) -> bool {
    let Some(key) = billing_selected_key(state) else {
        return false;
    };
    state
        .gourmet_month
        .as_ref()
        .is_none_or(|month| month.month_key == key)
        && state
            .ventopay_month
            .as_ref()
            .is_none_or(|month| month.month_key == key)
}

/// Whether the selected context itself is already cached: its identity matches
/// and at least one source month was published for it (possibly empty).
/// @req REQ-SCR-03
pub(crate) fn billing_context_cached(state: &State) -> bool {
    billing_context_matches(state)
        && (state.gourmet_month.is_some() || state.ventopay_month.is_some())
}

/// The compact Abrechnung surface owns its own progress feedback (design A17):
/// the quiet centred spinner36 without an elevated card while no cache exists,
/// the neutral 16 px row over a cached context — both inside the month/source
/// controls the design keeps present during loading. The shell therefore never
/// covers this surface with the blocking loading overlay; every other surface
/// keeps the incumbent overlay.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn billing_owns_progress(top: Screen, loading: bool) -> bool {
    top == Screen::Billing && loading
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

/// One month choice activation (compact task 1.5).
pub(crate) type MonthSelect = Rc<dyn Fn(u8, &mut Window, &mut App)>;

/// The three month choices: equal min-48 targets, body-16 labels in the
/// incumbent full German form (a second line is allowed, never truncation),
/// the accent rule and readable accent ink on the selected one. The controls
/// stay painted and usable while a fetch runs.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub(crate) fn mobile_month_row(
    options: &[MonthOption],
    selected: u8,
    on_select: MonthSelect,
    theme: &AppTheme,
) -> Div {
    let card = theme.mobile_card_fill();
    // Three equal thirds: the row never wraps, the label may take a second
    // line inside its own target, and no part of it is truncated.
    let mut row = div().flex().w_full();
    for option in options {
        let offset = option.offset;
        let is_selected = offset == selected;
        let on = Rc::clone(&on_select);
        row = row.child(
            div()
                .id(SharedString::from(format!("billing-month-{offset}")))
                .flex_1()
                .min_w_0()
                .min_h(px(theme.mobile.action_height))
                .px(px(theme.mobile.space_2))
                .py(px(theme.mobile.space_2))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .border_b(px(3.))
                .border_color(if is_selected {
                    theme.primary
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .on_click(move |_, window, cx| on(offset, window, cx))
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .text_size(px(theme.mobile.body))
                        .line_height(px(theme.mobile.body_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_center()
                        .text_color(if is_selected {
                            theme.mobile_ink_on(card, theme.primary)
                        } else {
                            theme.mobile_secondary_on(card)
                        })
                        .child(option.label.clone()),
                ),
        );
    }
    row
}

/// One source filter activation (compact task 1.5).
pub(crate) type SourceSelect = Rc<dyn Fn(BillingSource, &mut Window, &mut App)>;

/// The `Alle`/`Kantine`/`Automaten` options: the shared mobile option recipe
/// (min-44 target, 12 px side padding, r8, check 20 plus tint when selected),
/// wrapping when the text scale demands it.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub(crate) fn mobile_source_row(
    selected: BillingSource,
    on_select: SourceSelect,
    theme: &AppTheme,
) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.mobile.space_2))
        .py(px(theme.mobile.space_2));
    for (source, label) in [
        (BillingSource::All, "Alle"),
        (BillingSource::Gourmet, "Kantine"),
        (BillingSource::Ventopay, "Automaten"),
    ] {
        let on = Rc::clone(&on_select);
        row = row.child(
            chip::chip(label, selected == source, theme)
                .id(SharedString::from(format!("billing-chip-{label}")))
                .cursor_pointer()
                .on_click(move |_, window, cx| on(source, window, cx)),
        );
    }
    row
}

/// The neutral cached-refresh feedback: a 16 px spinner with the incumbent
/// `Aktualisiere...` label, instead of a blocking overlay.
fn refresh_row(theme: &AppTheme) -> AnyElement {
    let ink = theme.mobile_secondary_on(theme.mobile_card_fill());
    div()
        .px(px(theme.mobile.space_4))
        .py(px(theme.mobile.space_2))
        .flex()
        .items_center()
        .gap(px(theme.mobile.space_2))
        .child(spinner(ink, 16.))
        .child(
            div()
                .text_size(px(theme.mobile.meta))
                .line_height(px(theme.mobile.meta_line))
                .text_color(ink)
                .child("Aktualisiere..."),
        )
        .into_any_element()
}

fn mobile_error_banner(message: &str, theme: &AppTheme) -> AnyElement {
    let fill = if theme.flat {
        theme.error_surface
    } else {
        theme.glass_error
    };
    let ink = theme.mobile_status_on(fill, "error");
    let r = theme.tinted_banner(fill);
    theme::apply(
        div()
            .mb(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_3))
            .child(
                div()
                    .text_size(px(theme.mobile.meta))
                    .line_height(px(theme.mobile.meta_line))
                    .text_color(ink)
                    .child(message.to_string()),
            ),
        &r,
    )
    .into_any_element()
}

/// The one compact summary surface's content: the formatted `Gesamt` first,
/// then the `Belege` count and the conditional Kantine `Zuschuss`, exactly the
/// incumbent filtered totals and formatter (no new metric or calculation).
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_summary(
    entries: usize,
    totals: &BillingTotals,
    source: BillingSource,
) -> (String, Vec<(&'static str, String)>) {
    let mut pairs = vec![("Belege", entries.to_string())];
    if totals.zuschuss > 0. && source != BillingSource::Ventopay {
        pairs.push(("Zuschuss", format_eur(totals.zuschuss)));
    }
    (format_eur(totals.gesamt), pairs)
}

/// The summary surface: `banner_surface` with padding16, `Gesamt` in metadata
/// above the body value at the leading edge, then one wrapping metadata line
/// of the remaining pairs 16 px apart — never three equal metric columns.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_summary_surface(
    entries: &[BillingEntry],
    totals: &BillingTotals,
    source: BillingSource,
    theme: &AppTheme,
) -> AnyElement {
    let m = &theme.mobile;
    // The banner fill is the surface the ink is measured on.
    let plan = theme.banner_surface();
    let fill = plan.bg;
    let meta = theme.mobile_secondary_on(fill);
    let (gesamt, pairs) = mobile_summary(entries.len(), totals, source);
    let mut line = div().flex().flex_wrap().items_baseline().gap(px(m.space_4));
    for (label, value) in pairs {
        let ink = if label == "Zuschuss" {
            theme.mobile_status_on(fill, "success")
        } else {
            theme.text_primary
        };
        line = line.child(
            div()
                .flex()
                .items_baseline()
                .gap(px(m.space_1))
                .child(
                    div()
                        .text_size(px(m.meta))
                        .line_height(px(m.meta_line))
                        .text_color(meta)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(m.meta))
                        .line_height(px(m.meta_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink)
                        .child(value),
                ),
        );
    }
    theme::apply(
        div()
            .mb(px(m.space_2))
            .p(px(m.space_4))
            .flex()
            .flex_col()
            .gap(px(m.space_3))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(m.meta))
                            .line_height(px(m.meta_line))
                            .text_color(meta)
                            .child("Gesamt"),
                    )
                    .child(
                        div()
                            .text_size(px(m.body))
                            .line_height(px(m.body_line))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(gesamt),
                    ),
            )
            .child(line),
        &plan,
    )
    .into_any_element()
}

fn month_select(
    model: &Arc<AppModel>,
    off_thread: bool,
    cx: &mut Context<AppShell>,
) -> MonthSelect {
    // The model is already in hand: reading the shell here would hit the
    // entity while the render that calls this projection still updates it.
    let model = Arc::clone(model);
    let shell = cx.entity().clone();
    Rc::new(move |offset: u8, _: &mut Window, cx: &mut App| {
        let model = Arc::clone(&model);
        dispatch_billing_action(off_thread, cx, move || model.select_month(offset));
        let _ = shell.update(cx, |_, cx| cx.notify());
    })
}

fn source_select(
    model: &Arc<AppModel>,
    off_thread: bool,
    cx: &mut Context<AppShell>,
) -> SourceSelect {
    let model = Arc::clone(model);
    let shell = cx.entity().clone();
    Rc::new(move |source: BillingSource, _: &mut Window, cx: &mut App| {
        let model = Arc::clone(&model);
        dispatch_billing_action(off_thread, cx, move || model.set_billing_source(source));
        let _ = shell.update(cx, |_, cx| cx.notify());
    })
}

fn month_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
    off_thread: bool,
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
            dispatch_billing_action(off_thread, cx, {
                let model = Arc::clone(&model);
                move || model.select_month(offset)
            });
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

fn desktop_chips_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
    off_thread: bool,
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
            dispatch_billing_action(off_thread, cx, {
                let model = Arc::clone(&model);
                move || model.set_billing_source(source)
            });
            let _ = shell.update(cx, |_, cx| cx.notify());
        });
        let chip_state = if selected {
            ControlState::Selected
        } else {
            ControlState::Rest
        };
        row = row.child(
            desktop_chip(None, label, chip_state, theme)
                .id(SharedString::from(format!("billing-chip-{label}")))
                .cursor_pointer()
                .tab_index(0)
                .on_click(move |_, window, cx| action(window, cx)),
        );
    }
    row
}

/// Run one billing model action on the host's thread: every desktop host
/// dispatches off the render thread so the cached rows keep painting, the
/// narrow window included (the rail is irrelevant), while a phone or tablet
/// host keeps the incumbent synchronous path.
fn dispatch_billing_action(off_thread: bool, cx: &mut App, action: impl FnOnce() + Send + 'static) {
    if off_thread {
        cx.background_executor()
            .spawn(async move { action() })
            .detach();
    } else {
        action();
    }
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

fn reload_handler(off_thread: bool, cx: &mut Context<AppShell>) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            let Some(model) = s.model.clone() else {
                return;
            };
            dispatch_billing_action(off_thread, cx, move || model.reload_billing());
            cx.notify();
        });
    })
}

/// The wide-desktop Abrechnung workspace (task 1.4): month toolbar, source
/// chips, one compact summary line, the receipt table and its detail. Month
/// and source changes use the incumbent model actions, which clear the
/// preview; the totals follow the selected context.
/// @req REQ-SCR-03 @req REQ-UI-08 @req REQ-UI-12
pub fn desktop_billing_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    layout: LayoutMode,
    list_focus: &RecordListFocus,
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
    let geometry = model.desktop_geometry();
    let compact = layout == LayoutMode::DesktopCompact;
    let off_thread = dispatch_off_thread(state.host_kind);

    let mut column = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(month_row(state, model, theme, off_thread, cx))
        .child(desktop_chips_row(state, model, theme, off_thread, cx));
    if !entries.is_empty() {
        column = column.child(desktop_summary_line(
            &entries,
            &totals,
            state.billing_source,
            theme,
        ));
    }
    if let Some(error) = model.billing_error() {
        column = column.child(error_banner(&error.message, theme));
    }
    if state.billing_loading && entries.is_empty() {
        return column
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(spinner(theme.primary, 36.)),
            )
            .into_any_element();
    }
    column
        .child(split_workspace_scrolled(
            desktop_receipt_master(state, model, &entries, compact, list_focus, theme, cx),
            desktop_receipt_detail_pane(state, &entries, theme),
            &geometry,
            theme,
            &list_focus.scroll(),
            list_focus.detail(),
        ))
        .into_any_element()
}

/// One compact wrapping summary line (`Gesamt`, `Belege`, conditional
/// `Zuschuss`), never separate metric cards.
/// @req REQ-SCR-03
pub(crate) fn desktop_summary_line(
    entries: &[BillingEntry],
    totals: &BillingTotals,
    source: BillingSource,
    theme: &AppTheme,
) -> AnyElement {
    let show_subsidy = totals.zuschuss > 0. && source != BillingSource::Ventopay;
    let item = |label: &'static str, value: String, ink: Hsla| {
        div()
            .flex()
            .items_baseline()
            .gap(px(4.))
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .text_color(theme.text_secondary)
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(value),
            )
    };
    let mut line = div()
        .px(px(16.))
        .py(px(8.))
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(16.))
        .child(item(
            "Gesamt",
            format_eur(totals.gesamt),
            theme.text_primary,
        ))
        .child(item(
            "Belege",
            entries.len().to_string(),
            theme.text_primary,
        ));
    if show_subsidy {
        line = line.child(item(
            "Zuschuss",
            format_eur(totals.zuschuss),
            theme.success_text,
        ));
    }
    line.into_any_element()
}

fn desktop_receipt_master(
    state: &State,
    model: &Arc<AppModel>,
    entries: &[BillingEntry],
    compact: bool,
    list_focus: &RecordListFocus,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    // One tab stop for the receipt table: the row cursor previews the stable
    // receipt ID in the incumbent newest-first order and reveals the row.
    // A receipt is read-only, so Enter hands focus to the detail's scroll
    // region itself (record_table::detail_entry).
    let ids: Vec<String> = entries.iter().map(|entry| entry.id.clone()).collect();
    // Only this render's receipts keep a handle: a month or source change
    // drops the old rows from the restore set (REQ-UI-11/14).
    list_focus.retain(&ids);
    let on_key = {
        let focus = list_focus.clone();
        let model = Arc::clone(model);
        move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if event.keystroke.key.as_str() == "enter" {
                focus.enter(detail_entry(false), window, cx);
                cx.stop_propagation();
                return;
            }
            let Some(id) = focus.step(&ids, event.keystroke.key.as_str(), window, cx) else {
                return;
            };
            model.open_preview(ListKind::Receipts, &id);
            model.set_cursor(ListKind::Receipts, Some(id));
            cx.stop_propagation();
        }
    };
    let mut list = div()
        .id("desktop-billing-list")
        .flex()
        .flex_col()
        .tab_index(0)
        .track_focus(list_focus.container())
        .focus(theme::desktop_focus(theme.desktop.focus))
        .on_key_down(on_key)
        .child(desktop_bill_header(compact, theme));
    if entries.is_empty() {
        list = list.child(
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
                        .child("Keine Abrechnungsdaten für diesen Monat"),
                ),
        );
        return list.into_any_element();
    }
    for entry in entries {
        let selected = state.receipt_preview.as_deref() == Some(entry.id.as_str());
        let preview = desktop_receipt_preview_handler(cx, entry.id.clone());
        let handle = list_focus.row(&entry.id, cx);
        list = list.child(desktop_bill_row(
            entry,
            selected,
            compact,
            preview,
            Some(&handle),
            theme,
        ));
    }
    list.into_any_element()
}

fn desktop_receipt_detail_pane(
    state: &State,
    entries: &[BillingEntry],
    theme: &AppTheme,
) -> AnyElement {
    let Some(preview) = state.receipt_preview.as_deref() else {
        return detail_prompt("Beleg auswählen", theme);
    };
    let Some(entry) = entries.iter().find(|entry| entry.id == preview) else {
        return detail_prompt("Beleg auswählen", theme);
    };
    desktop_bill_detail(entry, state.content_width < 1024., theme)
}

fn desktop_receipt_preview_handler(cx: &mut Context<AppShell>, id: String) -> ClickHandler {
    let shell = cx.entity().clone();
    Rc::new(move |_: &mut Window, cx: &mut App| {
        shell.update(cx, |s, cx| {
            if let Some(model) = &s.model {
                model.open_preview(ListKind::Receipts, &id);
                model.set_cursor(ListKind::Receipts, Some(id.clone()));
                cx.notify();
            }
        });
    })
}

#[cfg(test)]
mod mobile_tests {
    use super::{
        billing_context_cached, billing_context_matches, billing_owns_progress, billing_view,
        mobile_summary, BillingView,
    };
    use snackpilot_model::navigation::Screen;
    use snackpilot_model::{format_eur, AppModel, BillingSource, BillingTotals, FakePlatform};
    use std::sync::Arc;

    fn demo(tag: &str) -> Arc<AppModel> {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-app-billing-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let model = AppModel::new(dir, Arc::new(FakePlatform::new())).unwrap();
        model.login("demo", "demo1234!").unwrap();
        Arc::new(model)
    }

    /// The compact Abrechnung screen (task 1.5) keeps the month/source
    /// controls while loading, withholds a published month that is not the
    /// selected context, and shows a cached refresh as the neutral row rather
    /// than the initial spinner or the shell's blocking overlay. Its one
    /// summary shows the incumbent filtered totals and the subsidy only while
    /// the Kantine side is shown.
    /// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
    #[test]
    fn mobile_billing_controls_survive_loading_and_context_change() {
        let model = demo("controls");
        model.select_month(1);
        let state = model.state();
        assert!(
            billing_context_matches(&state),
            "a settled fetch publishes the selected month"
        );
        assert!(
            billing_context_cached(&state),
            "the settled month is the selected-context cache"
        );

        // A cached refresh keeps its context: the neutral row is the only
        // progress feedback and the shell leaves the records on screen. The
        // initial load with no cache is the screen's own quiet spinner, never
        // the shell's blocking card over the still-present filters.
        assert_eq!(billing_view(true, true), BillingView::Refresh);
        assert!(
            billing_owns_progress(Screen::Billing, true),
            "the Abrechnung surface paints its own progress, not the blocking card"
        );
        assert!(!billing_owns_progress(Screen::Billing, false));
        assert!(
            !billing_owns_progress(Screen::Orders, true),
            "only the Abrechnung surface owns its progress"
        );
        assert!(!billing_owns_progress(Screen::Menus, true));

        // A month switch still in flight publishes the previous month's rows.
        // They are identifiable by the existing month identity, so the screen
        // withholds them instead of painting them under the new label.
        let mut stale = state.clone();
        let next_key = stale
            .month_options
            .iter()
            .find(|option| option.offset == 2)
            .expect("the second past month option")
            .key
            .clone();
        stale.selected_offset = 2;
        assert_ne!(
            stale
                .gourmet_month
                .as_ref()
                .map(|month| month.month_key.as_str()),
            Some(next_key.as_str()),
            "the stale rows still carry the previous month's identity"
        );
        assert!(
            !billing_context_matches(&stale),
            "the previous month's rows are not the selected context"
        );
        assert!(!billing_context_cached(&stale));
        assert_eq!(
            billing_view(true, false),
            BillingView::Initial,
            "a context-less load is the quiet initial spinner, never stale rows"
        );
        assert_eq!(billing_view(false, true), BillingView::Ready);
        assert_eq!(billing_view(false, false), BillingView::Ready);

        // The summary is the incumbent filtered total, not a new calculation.
        let totals = model.billing_totals();
        let entries = model.merged_entries();
        let mut pairs = vec![("Belege", entries.len().to_string())];
        if totals.zuschuss > 0. {
            pairs.push(("Zuschuss", format_eur(totals.zuschuss)));
        }
        assert_eq!(
            mobile_summary(entries.len(), &totals, BillingSource::All),
            (format_eur(totals.gesamt), pairs)
        );

        // The subsidy stays conditional: it belongs to the Kantine side only.
        let with_subsidy = BillingTotals {
            gesamt: 12.5,
            belege: 3,
            zuschuss: 1.25,
        };
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::All).1,
            vec![
                ("Belege", "3".to_string()),
                ("Zuschuss", "€ 1,25".to_string())
            ]
        );
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::Gourmet).1,
            vec![
                ("Belege", "3".to_string()),
                ("Zuschuss", "€ 1,25".to_string())
            ]
        );
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::Ventopay).1,
            vec![("Belege", "3".to_string())],
            "the Automaten filter shows no Kantine subsidy"
        );
        let no_subsidy = BillingTotals {
            zuschuss: 0.,
            ..with_subsidy
        };
        assert_eq!(
            mobile_summary(0, &no_subsidy, BillingSource::All),
            ("€ 12,50".to_string(), vec![("Belege", "0".to_string())]),
            "no zero-value subsidy line is painted"
        );
    }
}
