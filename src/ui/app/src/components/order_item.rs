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
    record_header_fixed, record_header_flex, record_row_focusable, record_status_cell, table_date,
    RowFocus, CHECK_COLUMN,
};
use crate::components::spinner::spinner;
use crate::navigation::ClickHandler;
use crate::theme::AppTheme;
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

/// The compact row's trailing control (task 1.4), in the incumbent precedence:
/// a running cancellation, the cancel target before the cutoff, the
/// visible-but-inert target past it, and nothing on a read-only past row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileOrderTrailing {
    /// This order's cancellation runs: the spinner in the same 44 px slot.
    Cancelling,
    /// Before the cutoff and with no other cancellation in flight.
    Cancel,
    /// Past the cutoff: the target stays visible and installs no handler.
    CancelDisabled,
    /// A past/read-only row, or selection mode: no trailing action.
    None,
}

impl MobileOrderTrailing {
    /// Whether this control really dispatches a cancellation.
    pub fn is_action(self) -> bool {
        matches!(self, Self::Cancel)
    }
}

/// The one trailing control for a row, in the incumbent order: the in-flight
/// cancellation, then the cancel target. Selection mode hides the cancel
/// target (the row toggles its leading check); past rows carry none at all.
pub fn mobile_order_trailing(
    select_mode: bool,
    _selected: bool,
    cancelling: bool,
    can_cancel: bool,
    cutoff: bool,
) -> MobileOrderTrailing {
    if select_mode {
        MobileOrderTrailing::None
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

/// A read-only row is never a button: only the incumbent selection mode makes
/// the whole row the check target.
pub fn mobile_order_card_tappable(select_mode: bool) -> bool {
    select_mode
}

/// The selection checkbox at the start of a row, in px.
pub const MOBILE_CHECKBOX: f32 = 24.;

/// The row's leading slot: the 24 px checkbox in selection mode, nothing
/// otherwise.
pub fn mobile_order_leading(select_mode: bool) -> Option<f32> {
    select_mode.then_some(MOBILE_CHECKBOX)
}

/// The date/category line: the metadata role, 8 px apart and allowed to wrap,
/// so the formatter's output is never abbreviated.
fn order_meta_row(order: &OrderedMenu, theme: &AppTheme) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(order.title.clone()),
        )
}

/// The status chip: one visible word on its toned surface in the metadata role.
fn order_status_chip(order: &OrderedMenu, cancelling: bool, theme: &AppTheme) -> AnyElement {
    let warning = cancelling || !order.approved;
    let (bg, ink) = if warning {
        (theme.warning_surface, theme.warning)
    } else {
        (theme.success_surface, theme.success)
    };
    div()
        .h(px(24.))
        .flex_shrink_0()
        .px(px(theme.scale.space_2))
        .rounded(px(theme.scale.radius_chip))
        .border(px(1.))
        .border_color(ink)
        .bg(bg)
        .flex()
        .items_center()
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(ink)
                .child(order_status_label(order.approved, cancelling)),
        )
        .into_any_element()
}

/// The 24 px selection checkbox: a check glyph once selected, never colour
/// alone.
fn selection_check(selected: bool, theme: &AppTheme) -> AnyElement {
    let (fill, ink, border) = if selected {
        (theme.accent, theme.on_accent, theme.accent)
    } else {
        (theme.surface, theme.text_secondary, theme.text_tertiary)
    };
    div()
        .w(px(MOBILE_CHECKBOX))
        .h(px(MOBILE_CHECKBOX))
        .flex_shrink_0()
        .rounded(px(theme.scale.radius_chip))
        .border(px(1.))
        .border_color(border)
        .bg(fill)
        .flex()
        .items_center()
        .justify_center()
        .child(if selected {
            Icon::new(IconName::Check)
                .with_size(px(theme.scale.icon))
                .text_color(ink)
                .into_any_element()
        } else {
            div().into_any_element()
        })
        .into_any_element()
}

/// The trailing control as painted: a separate 44×44 target on the neutral
/// fill with the readable danger `X`, the visible-but-inert cutoff state, or
/// the 16 px spinner in the same slot while this order's cancellation runs.
/// Only the enabled cancellation installs a callback.
fn order_trailing_control(
    trailing: MobileOrderTrailing,
    order: &OrderedMenu,
    on_cancel: ClickHandler,
    theme: &AppTheme,
) -> Option<AnyElement> {
    let size = theme.scale.target;
    let slot = || {
        div()
            .w(px(size))
            .h(px(size))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(theme.scale.radius_control))
    };
    match trailing {
        MobileOrderTrailing::None => None,
        MobileOrderTrailing::Cancelling => Some(
            slot()
                .child(spinner(theme.error, theme.scale.icon))
                .into_any_element(),
        ),
        MobileOrderTrailing::Cancel | MobileOrderTrailing::CancelDisabled => {
            let enabled = trailing.is_action();
            let glyph = if enabled {
                theme.error
            } else {
                theme.text_secondary
            };
            let mut target = slot()
                .id(SharedString::from(format!(
                    "order-cancel-{}",
                    order.position_id
                )))
                .bg(theme.surface_alt)
                .child(
                    Icon::new(IconName::X)
                        .with_size(px(theme.scale.icon))
                        .text_color(glyph),
                );
            if enabled {
                target = target
                    .tab_index(0)
                    .cursor_pointer()
                    .hover(move |s| s.bg(theme.pressed(theme.surface_alt)))
                    .focus(crate::theme::desktop_focus(theme.accent))
                    .on_click(move |_, window, cx| on_cancel(window, cx));
            }
            Some(target.into_any_element())
        }
    }
}

/// One compact §4.6 row: surface, 1 px hairline, 12 radius, 16 padding and
/// 8 gaps. The selection checkbox leads in selection mode, the dish is strong
/// with the date/category line quiet beneath it, the status chip carries its
/// word, and the cancellation is a separate 44×44 neutral target with the
/// danger `X` (its spinner takes the same slot while it runs). A read-only row
/// has no action at all.
/// @req REQ-SCR-02 @req REQ-UI-15 @req REQ-UI-16
#[allow(clippy::too_many_arguments)]
pub fn order_item(
    order: &OrderedMenu,
    dish: Option<String>,
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
    let m = &theme.scale;

    let mut row = div()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(m.space_2))
        .p(px(m.space_4))
        .mb(px(m.space_2))
        .rounded(px(theme.scale.radius_card))
        .bg(theme.surface)
        .border(px(1.))
        .border_color(theme.border)
        .id(SharedString::from(format!("order-{}", order.position_id)));
    if let (true, Some(on_select)) = (mobile_order_card_tappable(select_mode), on_select) {
        row = row
            .tab_index(0)
            .cursor_pointer()
            .hover(move |s| s.bg(theme.pressed(theme.surface)))
            .focus(crate::theme::desktop_focus(theme.accent))
            .on_click(move |_, window, cx| on_select(window, cx));
    }

    if let Some(slot) = mobile_order_leading(select_mode) {
        row = row.child(
            div()
                .h(px(m.body_line))
                .flex_shrink_0()
                .flex()
                .items_center()
                .w(px(slot))
                .child(selection_check(selected, theme)),
        );
    }

    let trailing = mobile_order_trailing(select_mode, selected, cancelling, can_cancel, cutoff);
    let target = order_trailing_control(trailing, order, on_cancel, theme);

    let mut head = div().flex().items_start().gap(px(m.space_2)).child(
        div()
            .flex_1()
            .min_w(px(0.))
            .text_size(px(m.body))
            .line_height(px(m.body_line))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.text_primary)
            .child(description),
    );
    if let Some(target) = target {
        head = head.child(target);
    }

    let mut foot = div().flex().items_center().gap(px(m.space_2)).child(
        div()
            .flex_1()
            .min_w(px(0.))
            .child(order_meta_row(order, theme)),
    );
    foot = foot.child(order_status_chip(order, cancelling, theme));

    row.child(
        div()
            .flex_1()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(m.space_2))
            .child(head)
            .child(foot),
    )
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
        theme.warning
    } else {
        theme.success
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
    cells.push(order_menu_cell(order, theme));
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

/// `Bestellung`: the order title in `BODY`; the detail carries the dish and
/// status, so a compact master row needs no second line.
fn order_menu_cell(order: &OrderedMenu, theme: &AppTheme) -> AnyElement {
    div()
        .flex_1()
        .min_w_0()
        .text_size(px(theme.scale.body))
        .line_height(px(theme.scale.body_line))
        .font_weight(FontWeight::SEMIBOLD)
        .text_ellipsis()
        .whitespace_nowrap()
        .child(order.title.clone())
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
        .child(record_status_cell(
            order_status_label(order.approved, state.cancelling),
            order_status_ink(order.approved, state.cancelling, theme),
            theme,
        ))
        .into_any_element()
}

/// The approval badge of the detail pane (brief `desktop.statusBadge`).
pub fn desktop_approval_badge(approved: bool, theme: &AppTheme) -> AnyElement {
    let (label, ink, fill) = if approved {
        ("Bestätigt", theme.success, theme.success_surface)
    } else {
        ("Ausstehend", theme.warning, theme.warning_surface)
    };
    div()
        .flex()
        .child(
            div()
                .px(px(theme.scale.space_2))
                .py(px(theme.scale.space_1))
                .rounded(px(theme.scale.radius_chip))
                .bg(fill)
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
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
        .p(px(theme.scale.space_4))
        .flex()
        .flex_col()
        .gap(px(theme.scale.space_2))
        .child(
            div()
                .text_size(px(theme.scale.meta))
                .line_height(px(theme.scale.meta_line))
                .text_color(theme.text_secondary)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(theme.scale.headline))
                .line_height(px(theme.scale.headline_line))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_primary)
                .child(order.title.clone()),
        )
        .child(
            div()
                .text_size(px(theme.scale.body))
                .line_height(px(theme.scale.body_line))
                .text_color(theme.text_primary)
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
                    .gap(px(theme.scale.space_2))
                    .child(spinner(theme.warning, 16.))
                    .child(
                        div()
                            .text_size(px(theme.scale.meta))
                            .line_height(px(theme.scale.meta_line))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.warning)
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
                        .text_size(px(theme.scale.meta))
                        .line_height(px(theme.scale.meta_line))
                        .text_color(theme.warning)
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
        cancel_enabled, mobile_order_card_tappable, mobile_order_leading, mobile_order_trailing,
        order_date_width, order_description, order_status_label, MobileOrderTrailing,
        MOBILE_CHECKBOX, ORDER_DATE_COMPACT, ORDER_DATE_EXPANDED, ORDER_STATUS_EXPANDED,
    };
    use crate::components::record_table::CHECK_COLUMN;
    use crate::theme::{to_rgb, AppTheme};
    use snackpilot_core::domain::OrderedMenu;
    use snackpilot_model::desktop::contrast_ratio;
    use snackpilot_model::tokens;

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
            MobileOrderTrailing::None,
            "selection mode hides the cancel target; the row toggles its check"
        );
        assert!(mobile_order_card_tappable(true));

        for dark in [false, true] {
            let theme = AppTheme::build(dark);
            let card = theme.surface;
            let fill = theme.surface_alt;
            assert!(
                contrast_ratio(to_rgb(theme.error, fill), to_rgb(fill, card)) >= 4.5,
                "dark={dark}: the X glyph reads on its neutral rest fill"
            );
            assert!(
                contrast_ratio(to_rgb(theme.text_secondary, fill), to_rgb(fill, card)) >= 4.5,
                "dark={dark}: the inert cutoff target keeps readable context"
            );
        }
    }

    /// The row's fixed slots (REQ-SCR-02/REQ-UI-15): selection mode leads with
    /// the 24 px checkbox and hides the cancel target, the cancellation is a
    /// 44×44 target that keeps its slot for the in-flight spinner, and the row
    /// keeps the shared 12 radius / 16 padding / 1 px surface recipe.
    /// @req REQ-SCR-02 @req REQ-UI-15
    #[test]
    fn mobile_order_row_checkbox_and_cancel_slot() {
        assert_eq!(mobile_order_leading(true), Some(MOBILE_CHECKBOX));
        assert_eq!(mobile_order_leading(false), None);
        assert_eq!(MOBILE_CHECKBOX, 24.);

        assert_eq!(tokens::TARGET, 44.);
        assert_eq!(
            mobile_order_trailing(false, false, true, true, false),
            MobileOrderTrailing::Cancelling,
            "the spinner takes the cancellation's own 44 px slot"
        );

        let theme = AppTheme::build(false);
        let row = theme.card_surface();
        assert_eq!(row.radius, gpui_kit::px(tokens::RADIUS_CARD));
        assert_eq!(row.border.map(|(w, _)| w), Some(gpui_kit::px(1.)));
        assert_eq!(theme.scale.space_4, 16.);

        let with_subtitle = order("MENÜ I", "Rindsgulasch");
        assert_eq!(
            order_description(None, &with_subtitle),
            "Rindsgulasch",
            "the compact row keeps the order's own dish fallback"
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
