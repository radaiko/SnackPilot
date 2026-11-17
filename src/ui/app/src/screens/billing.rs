//! Billing tab — docs/requirements/04-ui-ux.md §3.3 and §4.7.
//!
//! One compact filter band (month chips over source segments), one summary
//! surface, the newest-first receipt list in one shared card, the
//! empty/loading/error states and the month reload. All data comes from the
//! model's merged entries and totals; the selected month and source live in
//! `AppModel` state.

use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::Styled;

use crate::app::{AppShell, CarouselPage};
use crate::components::bill_card::{
    bill_list, desktop_bill_detail, desktop_bill_header, desktop_bill_row,
};
use crate::components::carousel::{page_fade, painted_travel, settle_after, Carousel};
use crate::components::chip::desktop_chip;
use crate::components::record_table::{detail_entry, RecordListFocus};
#[cfg(test)]
use crate::components::segment_tabs::mobile_segment_height;
use crate::components::segment_tabs::segment_options;
use crate::components::spinner::spinner;
use crate::components::swipe::{swipe_day, swipe_surface, SwipeHandlers};
use crate::components::workspace::{detail_prompt, split_workspace_scrolled, state_message};
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::MonthOption;
use snackpilot_model::desktop::{ControlState, LayoutMode};
use snackpilot_model::navigation::ListKind;
use snackpilot_model::{format_eur, AppModel, BillingEntry, BillingSource, BillingTotals, State};

const EMPTY_COPY: (&str, &str) = (
    "Keine Belege in diesem Zeitraum",
    "Wähle einen anderen Monat oder eine andere Quelle",
);

fn empty_copy() -> (&'static str, &'static str) {
    EMPTY_COPY
}

/// The compact Billing tab content (task 1.5): shell-owned heading and gutters,
/// one compact filter band (short month chips over the source segments), the
/// inline error, the neutral cached-refresh row, one summary surface and the
/// scrolling receipts in one shared card.
/// The filters stay painted while a fetch runs, a published month that is not
/// the selected context is withheld instead of painted under the new label,
/// and the legacy 16/100 px screen padding is gone so the shell's measured
/// reservation owns the scroll-end clearance.
/// @req REQ-SCR-03 @req REQ-UI-15 @req REQ-UI-16
pub fn billing_tab(
    state: &State,
    model: Option<&Arc<AppModel>>,
    theme: &AppTheme,
    page: Carousel,
    page_width: f32,
    scroll: &ScrollHandle,
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
    let view = billing_view(state.billing_loading, cached);
    // Host-gated dispatch: a narrow desktop window projects as the phone shell
    // but still runs its blocking billing work off the render thread; only a
    // phone/tablet host stays synchronous.

    // One flex child of the shell's column: the one compact filter band (month
    // chips over the source segments) stays outside the record scroll, and the
    // column takes exactly the height left under the content top. The month
    // chips scroll at the band's full width — a chip cut at the screen edge
    // reads as "there is more" — and the tab's refresh action sits on the
    // source row below, where the row has room for it without cutting a chip.
    let mut column = div().flex_1().min_h(px(0.)).flex().flex_col();
    let month_band = div().w_full().child(mobile_month_row(
        &state.month_options,
        state.selected_offset,
        month_select(model, cx),
        theme,
    ));
    column = column.child(month_band).child(mobile_source_row(
        state.billing_source,
        source_select(model, cx),
        theme,
    ));

    // One page per month, three in a row: the month before, the one on screen
    // and the month after. The drag slides the row and the release moves the
    // selection by one, exactly like the day carousel on Menüs — the pages fade
    // with their distance from centre, so a transition always shows one page.
    let options = &state.month_options;
    let index = options
        .iter()
        .position(|option| option.offset == state.selected_offset)
        .unwrap_or(0);
    let newer = index
        .checked_sub(1)
        .and_then(|i| options.get(i))
        .map(|option| option.offset);
    let older = options.get(index + 1).map(|option| option.offset);
    let reduce_motion = cx.reduce_motion();
    let travel = painted_travel(
        &page,
        newer.is_some(),
        older.is_some(),
        reduce_motion,
        page_width,
    );

    let page = |month: u8, tracked: Option<&ScrollHandle>| -> AnyElement {
        let selected = month == state.selected_offset;
        let (entries, totals) = if selected {
            (entries.clone(), totals.clone())
        } else {
            (
                model.billing_entries_for(month),
                model.billing_totals_for(month),
            )
        };
        let has_entries = !entries.is_empty() && (!selected || billing_context_matches(state));
        let view = if selected { view } else { BillingView::Ready };
        let mut page = div().w_full().h_full().flex().flex_col();
        if selected {
            if let Some(error) = model.billing_error() {
                page = page.child(mobile_error_banner(&error.message, theme));
            }
        }
        // Progress is the shell's slim bar at the top of the column, exactly as
        // on Menüs: the page draws none of its own, and a month without a cache
        // stays blank under the filters instead of spinning.
        if view == BillingView::Initial {
            return page.into_any_element();
        }
        if has_entries {
            page = page.child(mobile_summary_surface(
                &entries,
                &totals,
                state.billing_source,
                theme,
            ));
        }
        let mut list = div()
            .id(SharedString::from(format!("billing-list-{month}")))
            .w_full()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .overflow_y_scroll();
        if let Some(scroll) = tracked {
            list = list.track_scroll(scroll);
        }
        if has_entries {
            list = list.child(bill_list(&entries, theme));
        } else {
            let (fact, hint) = empty_copy();
            list = list.child(div().flex_1().child(state_message(fact, hint, theme)));
        }
        page.child(list).into_any_element()
    };

    // Widths are percentages of the clip, so the carousel does not depend on a
    // measured pixel width: the row is three pages wide and starts one page to
    // the left, which puts the month on screen exactly in view.
    let mut row = div()
        .flex()
        .flex_row()
        .w(relative(3.))
        .h_full()
        .ml(relative(-1.));
    for (slot, month) in [(0, newer), (1, Some(state.selected_offset)), (2, older)] {
        let pane = match month {
            Some(month) => page(month, (slot == 1).then_some(scroll)),
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

    let carousel = div()
        .size_full()
        .overflow_hidden()
        .child(div().size_full().ml(px(travel)).child(row))
        .into_any_element();

    column
        .child(div().flex_1().min_h(px(0.)).child(swipe_surface(
            "billing-swipe",
            carousel,
            month_swipe_handlers(newer, older, page_width, cx),
        )))
        .into_any_element()
}

/// Drag handlers for the month carousel: the same gesture as the day strip, one
/// page per month, driven by the shell's billing carousel.
fn month_swipe_handlers(
    newer: Option<u8>,
    older: Option<u8>,
    page_width: f32,
    cx: &mut Context<AppShell>,
) -> SwipeHandlers {
    let shell = cx.entity().clone();
    let scrolled = Rc::new(move |e: &ScrollWheelEvent, _: &mut Window, cx: &mut App| {
        let (dx, dy) = crate::components::swipe::drag_delta(e);
        let phase = e.touch_phase;
        shell.update(cx, |s, cx| match phase {
            TouchPhase::Started => {
                s.months_page.begin();
                month_travel(s, dx, dy, cx);
            }
            TouchPhase::Moved => month_travel(s, dx, dy, cx),
            TouchPhase::Ended => month_release(s, newer, older, page_width, cx),
            TouchPhase::Cancelled => {
                s.months_page.clear();
                cx.notify();
            }
        });
    });

    let shell = cx.entity().clone();
    let up = Rc::new(move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
        // A host that sends no `Ended` still ends the gesture here.
        shell.update(cx, |s, cx| month_release(s, newer, older, page_width, cx));
    });

    SwipeHandlers { scrolled, up }
}

fn month_travel(s: &mut AppShell, dx: f32, dy: f32, cx: &mut Context<AppShell>) {
    if s.months_page.travel(dx, dy) {
        cx.notify();
    }
}

/// The finger lifted: switch month when the drag was far enough, then finish
/// the page. Dragging right goes to the newer month, like the strip's left
/// chevron; dragging left goes back.
fn month_release(
    s: &mut AppShell,
    newer: Option<u8>,
    older: Option<u8>,
    page_width: f32,
    cx: &mut Context<AppShell>,
) {
    // The finish starts from the pixel the row is painted at: that is the
    // damped travel at an edge, and it includes any finish that was still
    // easing when the touch began, so the release never jumps.
    let painted = painted_travel(
        &s.months_page,
        newer.is_some(),
        older.is_some(),
        cx.reduce_motion(),
        page_width,
    );
    let Some(dx) = s.months_page.release() else {
        cx.notify();
        return;
    };
    let mut settle = painted;
    let mut switch = None;
    if let Some(model) = &s.model {
        match swipe_day(dx, newer.is_some(), older.is_some()) {
            Some(-1) => {
                settle = settle_after(painted, -1, page_width);
                switch = newer.map(|offset| (Arc::clone(model), offset));
            }
            Some(1) => {
                settle = settle_after(painted, 1, page_width);
                switch = older.map(|offset| (Arc::clone(model), offset));
            }
            _ => {}
        }
    }
    if let Some((model, offset)) = switch {
        // Off the render thread: the month paints from its cache at once and
        // the request behind it fills in while the page finishes.
        cx.background_executor()
            .spawn(async move { model.select_month(offset) })
            .detach();
    }
    s.months_page.finish(settle, cx.reduce_motion());
    if s.months_page.settle != 0. {
        s.ease_page(CarouselPage::Months, cx);
    }
    cx.notify();
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

/// The compact month chip's visible height: one single-line row instead of
/// two-line tiles.
/// The month stepper's row height: the 44 px arrow target plus the surface's
/// own 16 px padding.
pub const MOBILE_MONTH_TARGET_HEIGHT: f32 = 44.;

const MONTHS_SHORT: [&str; 12] = [
    "Jän.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];

/// The month chip's single-line German short form, built from the existing
/// month key (`2026-11` → `Nov. 2026`, `2026-09` → `Sept. 2026`). An
/// unparseable key falls back to itself.
/// @req REQ-SCR-03
pub(crate) fn month_chip_label(key: &str) -> String {
    let (year, month) = match key.split_once('-') {
        Some((year, month)) => (year, month),
        None => return key.to_string(),
    };
    let index = match month.parse::<usize>() {
        Ok(month) if (1..=12).contains(&month) => month - 1,
        _ => return key.to_string(),
    };
    format!("{} {year}", MONTHS_SHORT[index])
}

/// The whole compact filter band: the month chips' 44 px target, the source
/// segment control's action-height target and the band's bottom air.
/// @req REQ-SCR-03 @req REQ-UI-15
#[cfg(test)]
pub(crate) fn mobile_filter_band_height(theme: &AppTheme) -> f32 {
    MOBILE_MONTH_TARGET_HEIGHT
        + theme.scale.space_4
        + mobile_segment_height(theme)
        + theme.scale.space_2
}

/// The month stepper: one surface with a back and a forward arrow around the
/// month, shaped exactly like the Menüs day navigator. Chips in a scroller
/// always cut the third month in half and hide how far back the list goes; a
/// stepper says "one month at a time", which is how a billing screen is read.
/// Back walks into the past, forward toward the current month, and the
/// shortcut home appears only when you are not on the current month.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_month_row(
    options: &[MonthOption],
    selected: u8,
    on_select: MonthSelect,
    theme: &AppTheme,
) -> AnyElement {
    let index = options
        .iter()
        .position(|option| option.offset == selected)
        .unwrap_or(0);
    let newer = index.checked_sub(1).and_then(|i| options.get(i));
    let older = options.get(index + 1);

    let arrow = |id: &'static str, icon: IconName, target: Option<u8>| {
        let ink = if target.is_some() {
            theme.text_primary
        } else {
            theme.text_tertiary
        };
        let button = div()
            .w(px(theme.scale.target))
            .h(px(theme.scale.target))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(theme.scale.radius_chip))
            .child(
                Icon::new(icon)
                    .with_size(px(theme.scale.icon))
                    .text_color(ink),
            )
            .id(SharedString::from(id))
            .tab_index(0)
            .focus(theme::desktop_focus(theme.accent));
        match target {
            Some(offset) => {
                let on = Rc::clone(&on_select);
                button
                    .cursor_pointer()
                    .hover(move |s| s.bg(theme.surface_alt))
                    .on_click(move |_, window, cx| on(offset, window, cx))
            }
            None => button,
        }
        .into_any_element()
    };

    let label = options
        .get(index)
        .map(|option| month_chip_label(&option.key))
        .unwrap_or_default();
    let row = div()
        .w_full()
        .h(px(MOBILE_MONTH_TARGET_HEIGHT + theme.scale.space_4))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(theme.scale.space_2))
        .px(px(theme.scale.space_2))
        .child(arrow(
            "billing-month-newer",
            IconName::ChevronLeft,
            newer.map(|option| option.offset),
        ))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .items_center()
                .justify_center()
                .whitespace_nowrap()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(label),
        )
        .child(arrow(
            "billing-month-older",
            IconName::ChevronRight,
            older.map(|option| option.offset),
        ))
        .border_b_1()
        .border_color(theme.border);

    let mut column = div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2));
    column = column.child(row);
    column.into_any_element()
}

/// One source filter activation (compact task 1.5).
pub(crate) type SourceSelect = Rc<dyn Fn(BillingSource, &mut Window, &mut App)>;

/// The `Alle`/`Kantine`/`Automaten` options as the same contained segment
/// control the Bestellungen tabs use, so the app has one selected-state
/// language.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_source_row(
    selected: BillingSource,
    on_select: SourceSelect,
    theme: &AppTheme,
) -> Div {
    div()
        .w_full()
        .pb(px(theme.scale.space_2))
        .child(segment_options(
            [
                ("billing-chip-Alle", "Alle".to_string(), BillingSource::All),
                (
                    "billing-chip-Kantine",
                    "Kantine".to_string(),
                    BillingSource::Gourmet,
                ),
                (
                    "billing-chip-Automaten",
                    "Automaten".to_string(),
                    BillingSource::Ventopay,
                ),
            ],
            selected,
            on_select,
            theme,
        ))
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

/// The one compact summary surface's content: the formatted `Gesamt`, then one
/// metadata line with the `Belege` count and the conditional Kantine
/// `Zuschuss`, exactly the incumbent filtered totals and formatter (no new
/// metric or calculation).
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_summary(
    entries: usize,
    totals: &BillingTotals,
    source: BillingSource,
) -> (String, String) {
    let mut meta = format!("{entries} Belege");
    if totals.zuschuss > 0. && source != BillingSource::Ventopay {
        meta.push_str(&format!(" · Zuschuss {}", format_eur(totals.zuschuss)));
    }
    (format_eur(totals.gesamt), meta)
}

/// The one summary card: `Gesamt` and the amount on one line (the amount in
/// the title role), with one metadata line beneath — never three equal metric
/// columns and never a second crowded label/value row.
/// @req REQ-SCR-03 @req REQ-UI-15
pub(crate) fn mobile_summary_surface(
    entries: &[BillingEntry],
    totals: &BillingTotals,
    source: BillingSource,
    theme: &AppTheme,
) -> AnyElement {
    let m = &theme.scale;
    let (gesamt, meta) = mobile_summary(entries.len(), totals, source);
    div()
        .mb(px(m.space_2))
        .px(px(m.space_4))
        .py(px(m.space_3))
        .flex()
        .flex_col()
        .gap(px(m.space_1))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
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
                        .flex_shrink_0()
                        .whitespace_nowrap()
                        .text_size(px(m.title))
                        .line_height(px(m.title_line))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text_primary)
                        .child(gesamt),
                ),
        )
        .child(
            div()
                .text_size(px(m.meta))
                .line_height(px(m.meta_line))
                .text_color(theme.text_secondary)
                .child(meta),
        )
        .into_any_element()
}

fn month_select(model: &Arc<AppModel>, cx: &mut Context<AppShell>) -> MonthSelect {
    // The model is already in hand: reading the shell here would hit the
    // entity while the render that calls this projection still updates it.
    let model = Arc::clone(model);
    let shell = cx.entity().clone();
    Rc::new(move |offset: u8, _: &mut Window, cx: &mut App| {
        let model = Arc::clone(&model);
        dispatch_billing_action(cx, move || model.select_month(offset));
        let _ = shell.update(cx, |_, cx| cx.notify());
    })
}

fn source_select(model: &Arc<AppModel>, cx: &mut Context<AppShell>) -> SourceSelect {
    let model = Arc::clone(model);
    let shell = cx.entity().clone();
    Rc::new(move |source: BillingSource, _: &mut Window, cx: &mut App| {
        let model = Arc::clone(&model);
        dispatch_billing_action(cx, move || model.set_billing_source(source));
        let _ = shell.update(cx, |_, cx| cx.notify());
    })
}

/// The desktop month control is the same row the phone uses: one month with a
/// ‹ › either side. A segmented row of chips was fine for exactly three
/// months, but the app now offers every month it still has on disk, and that
/// count grows with use.
fn month_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
    cx: &mut Context<AppShell>,
) -> Div {
    let model = Arc::clone(model);
    let shell = cx.entity().clone();
    let on_select: MonthSelect = Rc::new(move |offset: u8, _: &mut Window, cx: &mut App| {
        let model = Arc::clone(&model);
        dispatch_billing_action(cx, move || model.select_month(offset));
        let _ = shell.update(cx, |_, cx| cx.notify());
    });
    div().w_full().child(mobile_month_row(
        &state.month_options,
        state.selected_offset,
        on_select,
        theme,
    ))
}

fn desktop_chips_row(
    state: &State,
    model: &Arc<AppModel>,
    theme: &AppTheme,
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
            dispatch_billing_action(cx, {
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

/// Run one billing model action off the render thread, so the cached rows keep
/// painting while it runs.
/// Abrechnung's work is one or two network fetches. It runs on the background
/// executor on every host: switching a month used to run inline on the phone,
/// which froze the screen for as long as Gourmet and Ventopay took to answer.
/// The selection is published before the fetch starts, so the chip answers the
/// tap immediately and the rows keep their place until the new ones arrive.
fn dispatch_billing_action(cx: &mut App, action: impl FnOnce() + Send + 'static) {
    cx.background_executor()
        .spawn(async move { action() })
        .detach();
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

    let mut column = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(month_row(state, model, theme, cx))
        .child(desktop_chips_row(state, model, theme, cx));
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
        billing_context_cached, billing_context_matches, billing_view, empty_copy,
        mobile_filter_band_height, mobile_summary, month_chip_label, BillingView,
        MOBILE_MONTH_TARGET_HEIGHT,
    };
    use crate::theme::AppTheme;
    use gpui_kit::px;
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

        // A cached refresh keeps its context: the shell's slim bar is the
        // progress and the records stay on screen.
        assert_eq!(billing_view(true, true), BillingView::Refresh);

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
        let mut meta = format!("{} Belege", entries.len());
        if totals.zuschuss > 0. {
            meta.push_str(&format!(" · Zuschuss {}", format_eur(totals.zuschuss)));
        }
        assert_eq!(
            mobile_summary(entries.len(), &totals, BillingSource::All),
            (format_eur(totals.gesamt), meta)
        );

        // The subsidy stays conditional: it belongs to the Kantine side only.
        let with_subsidy = BillingTotals {
            gesamt: 12.5,
            belege: 3,
            zuschuss: 1.25,
        };
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::All),
            (
                "€ 12,50".to_string(),
                "3 Belege · Zuschuss € 1,25".to_string()
            )
        );
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::Gourmet).1,
            "3 Belege · Zuschuss € 1,25"
        );
        assert_eq!(
            mobile_summary(3, &with_subsidy, BillingSource::Ventopay).1,
            "3 Belege",
            "the Automaten filter shows no Kantine subsidy"
        );
        let no_subsidy = BillingTotals {
            zuschuss: 0.,
            ..with_subsidy
        };
        assert_eq!(
            mobile_summary(0, &no_subsidy, BillingSource::All),
            ("€ 12,50".to_string(), "0 Belege".to_string()),
            "no zero-value subsidy line is painted"
        );
    }

    /// The two filters are one compact band: the month row is a single-line
    /// short form built from the existing month key, September keeps its four
    /// letters, and the whole band stays within 100 px.
    /// @req REQ-SCR-03 @req REQ-UI-15
    #[test]
    fn month_chips_use_single_line_short_labels() {
        assert_eq!(month_chip_label("2026-11"), "Nov. 2026");
        assert_eq!(month_chip_label("2026-09"), "Sept. 2026");
        assert_eq!(month_chip_label("2026-03"), "März 2026");
        for key in ["2026-11", "2026-09", "2026-03"] {
            let label = month_chip_label(key);
            assert!(
                !label.contains('\n') && !label.contains('\r'),
                "{key} paints on one line: {label}"
            );
        }
        assert_eq!(
            month_chip_label("not-a-key"),
            "not-a-key",
            "an unparseable key falls back to itself instead of inventing a month"
        );
        assert_eq!(
            MOBILE_MONTH_TARGET_HEIGHT,
            tokens::TARGET,
            "the stepper's arrows keep the 44 px phone tap target"
        );
        assert!(
            mobile_filter_band_height(&AppTheme::build(false)) <= 120.,
            "the month stepper and source segments fit one compact band"
        );
    }
    /// The compact Abrechnung controls share one selected-state language: the
    /// months are one short-label row, the sources are the segment control the
    /// Bestellungen tabs use, and the summary is one 12 radius card whose
    /// amount is the title role over one metadata line.
    /// @req REQ-SCR-03 @req REQ-UI-15
    #[test]
    fn mobile_billing_controls_are_compact_and_share_one_summary_card() {
        let theme = AppTheme::build(false);
        assert_eq!(theme.scale.target, tokens::TARGET);
        assert_eq!(theme.scale.target, 44.);
        assert_eq!(theme.scale.icon, tokens::ICON);
        assert_eq!(theme.scale.icon, 20.);
        let card = theme.card_surface();
        assert_eq!(card.radius, px(tokens::RADIUS_CARD));
        assert_eq!(card.border, None, "flat: the surface carries no frame");
        assert_eq!(theme.scale.title, tokens::TITLE.size);
        assert_eq!(theme.scale.title, tokens::TITLE.size);

        let totals = BillingTotals {
            gesamt: 12.5,
            belege: 3,
            zuschuss: 1.25,
        };
        let (gesamt, meta) = mobile_summary(3, &totals, BillingSource::All);
        assert_eq!(gesamt, format_eur(totals.gesamt));
        assert_eq!(meta, format!("3 Belege · Zuschuss {}", format_eur(1.25)));
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
