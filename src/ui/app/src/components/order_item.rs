//! Order item card — docs/requirements/04-ui-ux.md §4.6.
//!
//! Row card: date / category / dish on the left, approval badge and the cancel
//! affordance on the right. The caller passes `can_cancel`, whether this order
//! is currently being cancelled, and whether the cutoff has passed.

use gpui::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};

use crate::app::OrderCancelState;
use crate::components::button::{desktop_action_state, ButtonKind};
use crate::components::record_table::{
    record_cell_fixed, record_check_cell, record_header_blank, record_header_cells,
    record_header_fixed, record_header_flex, record_row_focusable, table_date, RowFocus,
    CHECK_COLUMN,
};
use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::{self, AppTheme};
use snackpilot_core::domain::OrderedMenu;
use snackpilot_model::desktop::ControlState;
use snackpilot_model::format_date_long;

/// Compact table date column (brief `desktop.orderDateCompact`: 80 px).
pub const ORDER_DATE_COMPACT: f32 = 80.;
/// Expanded table date column (brief `desktop.orderDateExpanded`: 104 px).
pub const ORDER_DATE_EXPANDED: f32 = 104.;
/// Expanded status column (brief `desktop.orderStatusExpanded`: 104 px).
pub const ORDER_STATUS_EXPANDED: f32 = 104.;

/// The incumbent description fallback: the menu description lookup, then the
/// order's own non-empty subtitle, then its title. Never invented.
pub fn order_description(dish: Option<String>, order: &OrderedMenu) -> String {
    dish.filter(|d| !d.is_empty())
        .or_else(|| (!order.subtitle.is_empty()).then(|| order.subtitle.clone()))
        .unwrap_or_else(|| order.title.clone())
}

/// The compact card's trailing control (task 1.4), in the incumbent
/// precedence: multi-select check, running cancellation, cancel before the
/// cutoff, the visible-but-inert target past it, and nothing on a read-only
/// past row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileOrderTrailing {
    /// Multi-select is active: the whole card toggles this reserved 44 px area.
    Check { selected: bool },
    /// This order's cancellation runs: a 16 px spinner in the same slot.
    Cancelling,
    /// Before the cutoff and with no other cancellation in flight.
    Cancel,
    /// Past the cutoff: the target stays visible and installs no handler.
    CancelDisabled,
    /// A past/read-only row: no trailing action and no implied tap.
    None,
}

impl MobileOrderTrailing {
    /// Whether this control really dispatches a cancellation.
    pub fn is_action(self) -> bool {
        matches!(self, Self::Cancel)
    }

    /// Whether the slot is the batch check rather than a cancel affordance.
    pub fn is_check(self) -> bool {
        matches!(self, Self::Check { .. })
    }
}

/// The one trailing control for a card, in the incumbent order: selection mode
/// first, then the in-flight cancellation, then the cancel target. Past rows
/// (`can_cancel == false`) carry none at all.
pub fn mobile_order_trailing(
    select_mode: bool,
    selected: bool,
    cancelling: bool,
    can_cancel: bool,
    cutoff: bool,
) -> MobileOrderTrailing {
    if select_mode {
        MobileOrderTrailing::Check { selected }
    } else if cancelling {
        MobileOrderTrailing::Cancelling
    } else if can_cancel && !cutoff {
        MobileOrderTrailing::Cancel
    } else if can_cancel {
        MobileOrderTrailing::CancelDisabled
    } else {
        MobileOrderTrailing::None
    }
}

/// A read-only card is never a button: only the incumbent selection mode makes
/// the whole card the check target.
pub fn mobile_order_card_tappable(select_mode: bool) -> bool {
    select_mode
}

/// The dish measure the card keeps before moving the trailing column under it:
/// twelve body-role characters, so a long dish never collapses into a
/// one-word column at large type.
pub const MOBILE_ORDER_DISH_CHARS: f32 = 12.;

/// Whether the status/cancel column moves below the dish instead of squeezing
/// it: exactly when the card's inner width cannot hold the readable dish
/// measure, the 44 px cancel target and the 8 px gap between them.
pub fn mobile_order_actions_below(inner_width: f32, body: f32, target: f32) -> bool {
    inner_width
        < body * MOBILE_ORDER_DISH_CHARS + target + snackpilot_model::tokens::mobile::SPACE_2
}

/// The date/category line, shared by both shapes: metadata role, separated by
/// 4 px and allowed to wrap, so the formatter's output is never abbreviated.
fn order_meta_row(order: &OrderedMenu, meta: Hsla, theme: &AppTheme) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.mobile.space_1))
        .child(
            div()
                .text_size(px(theme.mobile.meta))
                .line_height(px(theme.mobile.meta_line))
                .text_color(meta)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(theme.mobile.meta))
                .line_height(px(theme.mobile.meta_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(meta)
                .child(order.title.clone()),
        )
}

/// The status badge: the incumbent words on their semantic surface at r8 in
/// the metadata role, with a readable ink resolved on the painted fill.
fn order_status_badge(order: &OrderedMenu, cancelling: bool, theme: &AppTheme) -> AnyElement {
    let warning = cancelling || !order.approved;
    let bg = if warning {
        if theme.flat {
            theme.warning_surface
        } else {
            theme.glass_warning
        }
    } else if theme.flat {
        theme.success_surface
    } else {
        theme.glass_success
    };
    let ink = theme.mobile_status_on(bg, if warning { "warning" } else { "success" });
    div()
        .px(px(theme.mobile.space_2))
        .py(px(theme.mobile.space_1))
        .rounded(px(theme.mobile.radius_status))
        .bg(bg)
        .text_size(px(theme.mobile.meta))
        .line_height(px(theme.mobile.meta_line))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink)
        .child(order_status_label(order.approved, cancelling))
        .into_any_element()
}

/// The trailing control as painted: a separate 44x44 target with the neutral
/// rest surface and the readable danger `X`, the visible-but-inert cutoff
/// state, the in-slot 16 px spinner, or the reserved check area of selection
/// mode. Only the enabled cancellation installs a callback.
fn order_trailing_control(
    trailing: MobileOrderTrailing,
    order: &OrderedMenu,
    on_cancel: ClickHandler,
    theme: &AppTheme,
) -> Option<AnyElement> {
    let size = theme.mobile.target;
    let slot = || {
        div()
            .w(px(size))
            .h(px(size))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(theme.mobile.radius_option))
    };
    match trailing {
        MobileOrderTrailing::None => None,
        MobileOrderTrailing::Check { selected } => Some(
            slot()
                .bg(if selected {
                    theme.mobile.selection_fill
                } else {
                    theme.mobile_card_fill()
                })
                .border_1()
                .border_color(if selected {
                    theme.mobile.selection_boundary
                } else {
                    theme.border
                })
                .child(if selected {
                    Icon::new(IconName::Check)
                        .with_size(px(theme.mobile.space_5))
                        .text_color(theme.mobile.selection_ink)
                        .into_any_element()
                } else {
                    div().into_any_element()
                })
                .into_any_element(),
        ),
        MobileOrderTrailing::Cancelling => Some(
            slot()
                .child(spinner(
                    theme.mobile_danger_on(theme.mobile_card_fill()),
                    16.,
                ))
                .into_any_element(),
        ),
        MobileOrderTrailing::Cancel | MobileOrderTrailing::CancelDisabled => {
            let enabled = trailing.is_action();
            let fill = if enabled {
                theme.mobile.secondary_fill
            } else {
                theme.mobile.disabled_fill
            };
            let glyph = if enabled {
                theme.mobile_danger_on(fill)
            } else {
                theme.mobile.disabled_ink
            };
            let boundary = if enabled {
                theme.mobile.secondary_boundary
            } else {
                theme.border
            };
            let mut target = slot()
                .id(SharedString::from(format!(
                    "order-cancel-{}",
                    order.position_id
                )))
                .bg(fill)
                .border_1()
                .border_color(boundary)
                .child(
                    Icon::new(IconName::X)
                        .with_size(px(theme.mobile.space_5))
                        .text_color(glyph),
                );
            if enabled {
                target = target
                    .tab_index(0)
                    .focus(crate::theme::desktop_focus(theme.mobile.focus))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| on_cancel(window, cx));
            }
            Some(target.into_any_element())
        }
    }
}

/// One compact §4.6 card (task 1.4): 16 px padding, 8 px gaps and 8 px after
/// the card, the pinned card radii, the date/category line in the readable
/// metadata role, the description in the body role, the status badge at r8 and
/// the cancellation as a separate trailing 44x44 target. Below the readable
/// dish measure the status and cancellation move under the dish. Selection mode
/// keeps the whole-card check; a read-only row has no action at all.
/// @req REQ-SCR-02 @req REQ-UI-15 @req REQ-UI-16
#[allow(clippy::too_many_arguments)]
pub fn order_item(
    order: &OrderedMenu,
    dish: Option<String>,
    inner_width: f32,
    can_cancel: bool,
    cancelling: bool,
    cutoff: bool,
    select_mode: bool,
    selected: bool,
    on_cancel: ClickHandler,
    on_select: Option<ClickHandler>,
    theme: &AppTheme,
) -> Stateful<Div> {
    let description = order_description(dish, order);
    let card_fill = theme.mobile_card_fill();
    let meta = theme.mobile_secondary_on(card_fill);

    let mut card = theme::apply(
        div()
            .flex()
            .flex_col()
            .gap(px(theme.mobile.space_2))
            .p(px(theme.mobile.space_4))
            .mb(px(theme.mobile.space_2)),
        &theme.card_surface(),
    )
    .id(SharedString::from(format!("order-{}", order.position_id)));
    if let (true, Some(on_select)) = (mobile_order_card_tappable(select_mode), on_select) {
        card = card
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.mobile.focus))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_select(window, cx));
    }

    let trailing = mobile_order_trailing(select_mode, selected, cancelling, can_cancel, cutoff);
    let badge = order_status_badge(order, cancelling, theme);
    let target = order_trailing_control(trailing, order, on_cancel, theme);

    let dish = div()
        .flex_1()
        .min_w(px(0.))
        .text_size(px(theme.mobile.body))
        .line_height(px(theme.mobile.body_line))
        .text_color(theme.text_primary)
        .child(description);

    card = card.child(order_meta_row(order, meta, theme));
    if mobile_order_actions_below(inner_width, theme.mobile.body, theme.mobile.target) {
        // Narrow or large type: the dish keeps the full card width and the
        // status/cancel pair sits under it, trailing.
        let mut actions = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap(px(theme.mobile.space_2))
            .child(badge);
        if let Some(target) = target {
            actions = actions.child(target);
        }
        card.child(dish).child(actions)
    } else {
        let mut column = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(theme.mobile.space_2))
            .child(badge);
        if let Some(target) = target {
            column = column.child(target);
        }
        card.child(
            div()
                .flex()
                .items_start()
                .gap(px(theme.mobile.space_2))
                .child(dish)
                .child(column),
        )
    }
}

/// §4.6: the cancel affordance is interactive only before the cutoff and
/// while no other cancellation is in flight.
pub fn cancel_enabled(can_cancel: bool, cutoff: bool) -> bool {
    can_cancel && !cutoff
}

/// The row's own facts (desktop): preview selection and batch check stay
/// independent, and an in-flight cancellation replaces the status word.
#[derive(Debug, Clone, Copy, Default)]
pub struct DesktopOrderRowState {
    /// The preview cursor: tint and accent rule only, never a mutation.
    pub selected: bool,
    /// This ID is in the batch set (drawn as the 20 px checkbox).
    pub checked: bool,
    /// This order's cancel is running (`Wird storniert`).
    pub cancelling: bool,
}

/// The date column width for the table density (brief columns).
pub fn order_date_width(compact: bool) -> f32 {
    if compact {
        ORDER_DATE_COMPACT
    } else {
        ORDER_DATE_EXPANDED
    }
}

/// The one visible status word of an order row/detail: the in-flight
/// cancellation wins, then the approval state. Never hover-only.
pub fn order_status_label(approved: bool, cancelling: bool) -> &'static str {
    if cancelling {
        "Wird storniert"
    } else if approved {
        "Bestätigt"
    } else {
        "Ausstehend"
    }
}

/// The status ink matching [`order_status_label`] (success/warning semantics).
fn order_status_ink(approved: bool, cancelling: bool, theme: &AppTheme) -> Hsla {
    if cancelling || !approved {
        theme.warning_text
    } else {
        theme.success_text
    }
}

/// The compact/expanded header the order table shares with Abrechnung.
/// Compact columns are `Datum` 80 + `Bestellung`; expanded adds `Status` 104
/// (brief "Bestellungen"). Selection mode prepends the 44 px check column.
pub fn desktop_order_header(compact: bool, selection_mode: bool, theme: &AppTheme) -> AnyElement {
    let mut cells = Vec::new();
    if selection_mode {
        cells.push(record_header_blank(CHECK_COLUMN));
    }
    cells.push(record_header_fixed(
        "Datum",
        order_date_width(compact),
        theme,
    ));
    cells.push(record_header_flex("Bestellung", theme));
    if !compact {
        cells.push(record_header_fixed("Status", ORDER_STATUS_EXPANDED, theme));
    }
    record_header_cells(cells, theme)
}

/// One order table row: the body previews, the checkbox column toggles the
/// batch ID (propagation stopped there), so a row activation can never
/// approve or cancel anything.
/// @req REQ-SCR-02 @req REQ-UI-12
#[allow(clippy::too_many_arguments)]
pub fn desktop_order_row(
    order: &OrderedMenu,
    description: &str,
    state: DesktopOrderRowState,
    compact: bool,
    on_preview: ClickHandler,
    on_check: Option<ClickHandler>,
    focus: Option<&RowFocus>,
    theme: &AppTheme,
) -> AnyElement {
    let selection_mode = on_check.is_some();
    let mut cells = Vec::new();
    if let Some(on_check) = on_check.clone() {
        cells.push(record_check_cell(
            SharedString::from(format!("order-check-{}", order.position_id)),
            state.checked,
            on_check,
            theme,
        ));
    }
    cells.push(record_cell_fixed(
        table_date(order.date_epoch_ms),
        order_date_width(compact),
        theme,
    ));
    cells.push(order_menu_cell(order, description, state, compact, theme));
    if !compact {
        cells.push(order_status_cell(order, state, theme));
    }
    record_row_focusable(
        SharedString::from(format!("desktop-order-{}", order.position_id)),
        cells,
        state.selected,
        Some(on_preview),
        selection_mode,
        on_check,
        focus,
        theme,
    )
}

/// `Bestellung`: category over status (compact) or over a one-line excerpt
/// (expanded), never squeezed into a third tiny column.
fn order_menu_cell(
    order: &OrderedMenu,
    description: &str,
    state: DesktopOrderRowState,
    compact: bool,
    theme: &AppTheme,
) -> AnyElement {
    let second = if compact {
        div()
            .text_size(px(12.))
            .line_height(px(16.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(order_status_ink(order.approved, state.cancelling, theme))
            .child(order_status_label(order.approved, state.cancelling))
    } else {
        div()
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(theme.text_secondary)
            .text_ellipsis()
            .whitespace_nowrap()
            .child(description.to_string())
    };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .justify_center()
        .child(
            div()
                .text_size(px(14.))
                .line_height(px(20.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .text_ellipsis()
                .whitespace_nowrap()
                .child(order.title.clone()),
        )
        .child(second)
        .into_any_element()
}

fn order_status_cell(
    order: &OrderedMenu,
    state: DesktopOrderRowState,
    theme: &AppTheme,
) -> AnyElement {
    div()
        .w(px(ORDER_STATUS_EXPANDED))
        .flex_shrink_0()
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(order_status_ink(order.approved, state.cancelling, theme))
                .text_ellipsis()
                .whitespace_nowrap()
                .child(order_status_label(order.approved, state.cancelling)),
        )
        .into_any_element()
}

/// The approval badge of the detail pane (brief `desktop.statusBadge`).
pub fn desktop_approval_badge(approved: bool, theme: &AppTheme) -> AnyElement {
    let (label, ink, fill) = if approved {
        ("Bestätigt", theme.success_text, theme.success_surface)
    } else {
        ("Ausstehend", theme.warning_text, theme.warning_surface)
    };
    div()
        .flex()
        .child(
            div()
                .px(px(8.))
                .py(px(4.))
                .rounded(px(8.))
                .bg(fill)
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .child(label),
        )
        .into_any_element()
}

/// One order detail (brief "Bestellungen"): long date, title, the complete
/// description fallback, approval state and the explicit `Stornieren`
/// action. Nothing about preview selects or cancels; only the action does.
/// @req REQ-SCR-02 @req REQ-UI-08
pub(crate) fn desktop_order_detail(
    order: &OrderedMenu,
    description: &str,
    cancel: OrderCancelState,
    on_cancel: ClickHandler,
    theme: &AppTheme,
    cx: &App,
) -> AnyElement {
    let mut detail = div()
        .p(px(16.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(theme.text_secondary)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(20.))
                .line_height(px(26.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.desktop.ink)
                .child(order.title.clone()),
        )
        .child(
            div()
                .text_size(px(16.))
                .line_height(px(24.))
                .text_color(theme.desktop.ink)
                .child(description.to_string()),
        )
        .child(desktop_approval_badge(order.approved, theme));

    match cancel {
        OrderCancelState::Hidden => {}
        OrderCancelState::InFlight => {
            detail = detail.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(spinner(theme.warning, 16.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.warning_text)
                            .child(order_status_label(order.approved, true)),
                    ),
            );
        }
        state => {
            let control = if state == OrderCancelState::Ready {
                ControlState::Rest
            } else {
                ControlState::Disabled
            };
            let action = (state == OrderCancelState::Ready).then_some(on_cancel);
            detail = detail.child(desktop_action_state(
                format!("desktop-order-cancel-{}", order.position_id),
                "Stornieren",
                ButtonKind::Danger,
                control,
                theme,
                cx,
                move |_, window, cx| {
                    if let Some(action) = &action {
                        action(window, cx);
                    }
                },
            ));
            if state == OrderCancelState::Cutoff {
                detail = detail.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(theme.warning_text)
                        .child("Stornierung nicht mehr möglich."),
                );
            }
        }
    }
    detail.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_enabled, mobile_order_actions_below, mobile_order_card_tappable,
        mobile_order_trailing, order_date_width, order_description, order_status_label,
        MobileOrderTrailing, ORDER_DATE_COMPACT, ORDER_DATE_EXPANDED, ORDER_STATUS_EXPANDED,
    };
    use crate::components::record_table::CHECK_COLUMN;
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_core::domain::OrderedMenu;
    use snackpilot_model::desktop::contrast_ratio;
    use snackpilot_model::AccentColor;

    fn order(title: &str, subtitle: &str) -> OrderedMenu {
        OrderedMenu {
            position_id: "1".into(),
            eating_cycle_id: "1".into(),
            date_epoch_ms: 1_794_308_400_000,
            title: title.into(),
            subtitle: subtitle.into(),
            approved: false,
        }
    }

    /// @req REQ-SCR-02
    #[test]
    fn cancel_button_disabled_past_the_cutoff() {
        assert!(cancel_enabled(true, false));
        assert!(!cancel_enabled(true, true));
        assert!(!cancel_enabled(false, false));
    }

    /// The two table densities use the brief's exact order columns, and the
    /// batch-check hit column is the 44 px one.
    /// @req REQ-SCR-02
    #[test]
    fn order_columns_match_the_brief() {
        assert_eq!(order_date_width(true), 80.);
        assert_eq!(order_date_width(false), 104.);
        assert_eq!(ORDER_DATE_COMPACT, 80.);
        assert_eq!(ORDER_DATE_EXPANDED, 104.);
        assert_eq!(ORDER_STATUS_EXPANDED, 104.);
        assert_eq!(CHECK_COLUMN, 44.);
    }

    /// The description fallback is menu lookup, then the order's own
    /// non-empty subtitle, then its title — never an invented dish.
    /// @req REQ-SCR-02
    #[test]
    fn order_description_falls_back_to_subtitle_then_title() {
        let with_subtitle = order("MENÜ I", "Rindsgulasch");
        let without = order("MENÜ II", "");
        assert_eq!(
            order_description(Some("Menü-Beschreibung".into()), &with_subtitle),
            "Menü-Beschreibung"
        );
        assert_eq!(
            order_description(Some(String::new()), &with_subtitle),
            "Rindsgulasch",
            "an empty lookup falls back to the order subtitle"
        );
        assert_eq!(order_description(None, &without), "MENÜ II");
    }

    /// The compact card's trailing control (REQ-UI-15/16): a past or read-only
    /// row has no implied action, the cutoff keeps the target visible but
    /// without a handler, an in-flight cancellation owns the same slot, and in
    /// selection mode the whole card toggles the reserved check area instead —
    /// cancel is never reachable through that tap. The painted danger glyph and
    /// boundary read on the neutral rest fill for every accent, appearance and
    /// dialect, and the disabled pair keeps readable context.
    /// @req REQ-UI-15 @req REQ-UI-16
    #[test]
    fn mobile_order_cancel_target_and_readable_states() {
        assert_eq!(
            mobile_order_trailing(false, false, false, false, false),
            MobileOrderTrailing::None,
            "a past row carries no cancel affordance at all"
        );
        assert!(
            !mobile_order_card_tappable(false),
            "a read-only row is not a button"
        );

        assert_eq!(
            mobile_order_trailing(false, false, false, true, false),
            MobileOrderTrailing::Cancel
        );
        assert!(MobileOrderTrailing::Cancel.is_action());

        assert_eq!(
            mobile_order_trailing(false, false, false, true, true),
            MobileOrderTrailing::CancelDisabled,
            "past the cutoff the target stays visible"
        );
        assert!(
            !MobileOrderTrailing::CancelDisabled.is_action(),
            "and installs no handler"
        );

        assert_eq!(
            mobile_order_trailing(false, false, true, true, false),
            MobileOrderTrailing::Cancelling,
            "an in-flight cancellation takes the same slot"
        );
        assert!(!MobileOrderTrailing::Cancelling.is_action());

        assert_eq!(
            mobile_order_trailing(true, true, false, true, false),
            MobileOrderTrailing::Check { selected: true }
        );
        assert!(!MobileOrderTrailing::Check { selected: true }.is_action());
        assert!(mobile_order_card_tappable(true));

        for dark in [false, true] {
            for flat in [false, true] {
                for accent in [
                    AccentColor::Orange,
                    AccentColor::Emerald,
                    AccentColor::Berry,
                    AccentColor::Golden,
                    AccentColor::Ocean,
                ] {
                    let theme = AppTheme::build(dark, accent, flat);
                    let card = theme.mobile_card_fill();
                    let fill = theme.mobile.secondary_fill;
                    let glyph = theme.mobile_danger_on(fill);
                    assert!(
                        contrast_ratio(to_rgb(glyph, fill), to_rgb(fill, card)) >= 4.5,
                        "{accent:?} dark={dark} flat={flat}: the X glyph reads on its rest fill"
                    );
                    assert!(
                        contrast_ratio(
                            to_rgb(theme.mobile.secondary_boundary, fill),
                            to_rgb(fill, card)
                        ) >= 3.0,
                        "{accent:?} dark={dark} flat={flat}: the target boundary reads"
                    );
                    let disabled = theme.mobile.disabled_fill;
                    assert!(
                        contrast_ratio(
                            to_rgb(theme.mobile.disabled_ink, disabled),
                            to_rgb(disabled, card)
                        ) >= 4.5,
                        "{accent:?} dark={dark} flat={flat}: disabled stays readable"
                    );
                }
            }
        }
    }

    /// The compact card's two shapes (REQ-UI-15): status and cancel sit beside
    /// the dish at the 360/390/768 content widths and move below it at large
    /// type instead of squeezing the description, while the description is
    /// always the incumbent fallback lookup.
    /// @req REQ-UI-15
    #[test]
    fn mobile_order_actions_move_below_the_dish_at_large_type() {
        // Card inner width: the shell column at 360/390/768 minus the card's
        // own 16 px padding on both sides.
        for (viewport, body) in [(360_f32, 16_f32), (390., 16.), (768., 16.)] {
            let inner = (viewport - 32.).min(560.) - 32.;
            assert!(
                !mobile_order_actions_below(inner, body, 44.),
                "{viewport} px keeps the trailing column beside the dish"
            );
        }
        let inner = 360. - 32. - 32.;
        assert!(
            mobile_order_actions_below(inner, 24., 44.),
            "at 150 % type the same card moves the actions under the dish"
        );

        let with_subtitle = order("MENÜ I", "Rindsgulasch");
        assert_eq!(
            order_description(None, &with_subtitle),
            "Rindsgulasch",
            "the compact card keeps the order's own dish fallback"
        );
    }

    /// Both status words stay available without hover, and a running
    /// cancellation wins over the approval state.
    /// @req REQ-SCR-02
    #[test]
    fn order_status_words_remain_visible() {
        assert_eq!(order_status_label(true, false), "Bestätigt");
        assert_eq!(order_status_label(false, false), "Ausstehend");
        assert_eq!(order_status_label(true, true), "Wird storniert");
        assert_eq!(order_status_label(false, true), "Wird storniert");
    }
}
