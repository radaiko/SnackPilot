//! Billing tab — docs/requirements/04-ui-ux.md §3.3 and §4.7.
//!
//! Month tabs, source chips, summary bar, newest-first `BillCard` list, the
//! empty/loading/error states and pull-to-refresh. All data comes from the
//! model's merged entries and totals; the selected month and source live in
//! `AppModel` state.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::Styled;

use crate::app::{dispatch_off_thread, AppShell};
use crate::components::bill_card::{
    bill_card, desktop_bill_detail, desktop_bill_header, desktop_bill_row,
};
use crate::components::chip::desktop_chip;
use crate::components::pull_refresh::pull_refresh;
use crate::components::record_table::{detail_entry, RecordListFocus};
use crate::components::spinner::spinner;
use crate::components::workspace::{detail_prompt, split_workspace_scrolled, state_message};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MonthOption;
use snackpilot_model::desktop::{ControlState, LayoutMode};
use snackpilot_model::navigation::{ListKind, Screen};
use snackpilot_model::{format_eur, AppModel, BillingEntry, BillingSource, BillingTotals, State};

const EMPTY_COPY: (&str, &str) = (
    "Keine Belege in diesem Zeitraum",
    "Wähle einen anderen Monat oder eine andere Quelle",
);

fn empty_copy() -> (&'static str, &'static str) {
    EMPTY_COPY
}

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

    let mut content = div().w_full().flex_1().min_h(px(0.)).flex().flex_col();
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
                    .child(spinner(theme.accent, 36.)),
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
            .w_full()
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
            let (fact, hint) = empty_copy();
            list = list.child(div().flex_1().child(state_message(fact, hint, theme)));
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
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child("Anmeldung erforderlich"),
        )
        .into_any_element()
}

/// One month choice activation (compact task 1.5).
pub(crate) type MonthSelect = Rc<dyn Fn(u8, &mut Window, &mut App)>;

/// One flat option chip: 44 px high, 8 radius, 12 side padding and the accent
/// tint and boundary when selected. The source chips carry the 20 px check; the
/// three equal month chips keep their whole third for the full German label,
/// with the boundary and primary-role word carrying the selection.
fn option_chip(label: &str, selected: bool, equal: bool, theme: &AppTheme) -> Div {
    let (fill, border, ink) = if selected {
        // The accent on its own tint reaches only 4.44:1 in light: the word
        // takes the primary text role, the boundary keeps the accent.
        (theme.accent_surface, theme.accent, theme.text_primary)
    } else {
        (theme.surface, theme.border, theme.text_secondary)
    };
    let (size, line) = if equal {
        (theme.scale.meta, theme.scale.meta_line)
    } else {
        (theme.scale.body, theme.scale.body_line)
    };
    let mut chip = div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(theme.scale.space_2))
        .min_h(px(theme.scale.target))
        .px(px(theme.scale.space_3))
        .py(px(theme.scale.space_2))
        .rounded(px(theme.scale.radius_chip))
        .bg(fill)
        .border(px(1.))
        .border_color(border);
    if equal {
        chip = chip.flex_1().min_w(px(0.));
    } else if selected {
        chip = chip.child(
            Icon::new(IconName::Check)
                .with_size(px(theme.scale.icon))
                .text_color(ink),
        );
    }
    chip.child(
        div()
            .min_w(px(0.))
            .text_size(px(size))
            .line_height(px(line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_center()
            .text_color(ink)
            .child(label.to_string()),
    )
}

/// The three month choices: three equal 44 px chips in the incumbent full
/// German form (a second line is allowed, never truncation), selected by the
/// accent tint and boundary. The controls stay painted while a fetch runs.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub(crate) fn mobile_month_row(
    options: &[MonthOption],
    selected: u8,
    on_select: MonthSelect,
    theme: &AppTheme,
) -> Div {
    let mut row = div()
        .flex()
        .w_full()
        .gap(px(theme.scale.space_2))
        .py(px(theme.scale.space_2));
    for option in options {
        let offset = option.offset;
        let on = Rc::clone(&on_select);
        let ink = if offset == selected {
            theme.accent
        } else {
            theme.text_secondary
        };
        let hover = theme.pressed(if offset == selected {
            theme.accent_surface
        } else {
            theme.surface
        });
        row = row.child(
            option_chip(&option.label, offset == selected, true, theme)
                .id(SharedString::from(format!("billing-month-{offset}")))
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .focus(theme::desktop_focus(ink))
                .on_click(move |_, window, cx| on(offset, window, cx)),
        );
    }
    row
}

/// One source filter activation (compact task 1.5).
pub(crate) type SourceSelect = Rc<dyn Fn(BillingSource, &mut Window, &mut App)>;

/// The `Alle`/`Kantine`/`Automaten` options: 44 px chips with the 20 px check
/// on the selected one, wrapping when the text scale demands it.
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
        .gap(px(theme.scale.space_2))
        .py(px(theme.scale.space_2));
    for (source, label) in [
        (BillingSource::All, "Alle"),
        (BillingSource::Gourmet, "Kantine"),
        (BillingSource::Ventopay, "Automaten"),
    ] {
        let on = Rc::clone(&on_select);
        let is_selected = selected == source;
        let hover = theme.pressed(if is_selected {
            theme.accent_surface
        } else {
            theme.surface
        });
        row = row.child(
            option_chip(label, is_selected, false, theme)
                .id(SharedString::from(format!("billing-chip-{label}")))
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .focus(theme::desktop_focus(theme.accent))
                .on_click(move |_, window, cx| on(source, window, cx)),
        );
    }
    row
}

/// The neutral cached-refresh feedback: a 16 px spinner with the incumbent
/// `Aktualisiere...` label, instead of a blocking overlay.
fn refresh_row(theme: &AppTheme) -> AnyElement {
    let ink = theme.text_secondary;
    div()
        .py(px(theme.scale.space_2))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(theme.scale.space_2))
        .child(spinner(ink, theme.scale.icon))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(ink)
                .child("Aktualisiere..."),
        )
        .into_any_element()
}

fn mobile_error_banner(message: &str, theme: &AppTheme) -> AnyElement {
    let fill = theme.error_surface;
    let ink = theme.error;
    let r = theme.tinted_banner(fill);
    theme::apply(
        div()
            .mb(px(theme.scale.space_2))
            .p(px(theme.scale.space_4))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
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

/// The one summary card: `Gesamt` in the metadata role, the formatted total in
/// the title role beneath it, then the `Belege` count and the conditional
/// Kantine `Zuschuss` as metadata pairs — never three equal metric columns.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_summary_surface(
    entries: &[BillingEntry],
    totals: &BillingTotals,
    source: BillingSource,
    theme: &AppTheme,
) -> AnyElement {
    let m = &theme.scale;
    let (gesamt, pairs) = mobile_summary(entries.len(), totals, source);
    let mut line = div().flex().flex_wrap().items_baseline().gap(px(m.space_4));
    for (label, value) in pairs {
        line = line.child(
            div()
                .flex()
                .items_baseline()
                .gap(px(m.space_2))
                .child(
                    div()
                        .text_size(px(m.meta))
                        .line_height(px(m.meta_line))
                        .text_color(theme.text_secondary)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(m.meta))
                        .line_height(px(m.meta_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
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
                    .gap(px(m.space_2))
                    .child(
                        div()
                            .text_size(px(m.meta))
                            .line_height(px(m.meta_line))
                            .text_color(theme.text_secondary)
                            .child("Gesamt"),
                    )
                    .child(
                        div()
                            .text_size(px(m.title))
                            .line_height(px(m.title_line))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(gesamt),
                    ),
            )
            .child(line),
        &theme.card_surface(),
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
    let border = theme.border;
    let bg = theme.surface;
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
                .min_h(px(snackpilot_model::tokens::TARGET))
                .py(px(theme.scale.space_3))
                .cursor_pointer()
                .border_b_3()
                .border_color(if selected {
                    theme.accent
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .on_click(move |_, window, cx| action(window, cx))
                .child(
                    div()
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if selected {
                            theme.accent
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
        .px(px(theme.scale.space_4))
        .py(px(theme.scale.space_2));
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
    let r = theme.tinted_banner(theme.error_surface);
    theme::apply(
        div()
            .mx(px(theme.scale.space_4))
            .mt(px(theme.scale.space_2))
            .p(px(theme.scale.space_3))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.error)
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
                    .child(spinner(theme.accent, 36.)),
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
            .gap(px(theme.scale.space_1))
            .child(
                div()
                    .text_size(px(theme.scale.meta))
                    .line_height(px(theme.scale.meta_line))
                    .text_color(theme.text_secondary)
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(theme.scale.body))
                    .line_height(px(theme.scale.body_line))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(value),
            )
    };
    let mut line = div()
        .px(px(theme.scale.space_4))
        .py(px(theme.scale.space_2))
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.scale.space_4))
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
        line = line.child(item("Zuschuss", format_eur(totals.zuschuss), theme.success));
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
        .focus(theme::desktop_focus(theme.accent))
        .on_key_down(on_key)
        .child(desktop_bill_header(compact, theme));
    if entries.is_empty() {
        let (fact, hint) = empty_copy();
        list = list.child(div().w_full().child(state_message(fact, hint, theme)));
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
        empty_copy, mobile_summary, BillingView,
    };
    use crate::theme::AppTheme;
    use gpui_kit::px;
    use snackpilot_model::navigation::Screen;
    use snackpilot_model::tokens;
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
    /// The flat Abrechnung controls (REQ-SCR-03/REQ-UI-15): months are three
    /// equal 44 px chips, sources carry the 20 px check on the selected one,
    /// and the summary is one 12 radius card whose total is the title role with
    /// the incumbent formatter and the conditional subsidy pair.
    /// @req REQ-SCR-03 @req REQ-UI-15
    #[test]
    fn mobile_billing_controls_use_chips_and_one_summary_card() {
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.target, tokens::TARGET);
        assert_eq!(theme.scale.target, 44.);
        assert_eq!(theme.scale.icon, tokens::ICON);
        assert_eq!(theme.scale.icon, 20.);
        let card = theme.card_surface();
        assert_eq!(card.radius, px(tokens::RADIUS_CARD));
        assert_eq!(card.border.map(|(w, _)| w), Some(px(1.)));
        assert_eq!(theme.scale.title, tokens::TITLE.size);
        assert_eq!(theme.scale.title, 28.);

        let totals = BillingTotals {
            gesamt: 12.5,
            belege: 3,
            zuschuss: 1.25,
        };
        let (gesamt, pairs) = mobile_summary(3, &totals, BillingSource::All);
        assert_eq!(gesamt, format_eur(totals.gesamt));
        assert_eq!(
            pairs,
            vec![
                ("Belege", "3".to_string()),
                ("Zuschuss", format_eur(totals.zuschuss))
            ]
        );
    }

    /// The empty month states the fact and then the next step, so the tab is
    /// informative instead of one grey sentence.
    /// @req REQ-SCR-03
    #[test]
    fn empty_billing_states_fact_then_next_step() {
        assert_eq!(
            empty_copy(),
            (
                "Keine Belege in diesem Zeitraum",
                "Wähle einen anderen Monat oder eine andere Quelle"
            )
        );
    }
}
