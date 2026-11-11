//! Order item card — docs/requirements/04-ui-ux.md §4.6.
//!
//! Row card: date / category / dish on the left, approval badge and the cancel
//! affordance on the right. The caller passes `can_cancel`, whether this order
//! is currently being cancelled, and whether the cutoff has passed.

use gpui::*;

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

/// One §4.6 card.
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
    let description = dish
        .filter(|d| !d.is_empty())
        .or_else(|| (!order.subtitle.is_empty()).then(|| order.subtitle.clone()))
        .unwrap_or_else(|| order.title.clone());

    let mut card = theme::apply(
        div()
            .flex()
            .items_center()
            .justify_between()
            .p(px(16.))
            .mb(px(8.)),
        &theme.card_surface(),
    )
    .id(SharedString::from(format!("order-{}", order.position_id)));
    if cancelling {
        card = card.opacity(0.6);
    }
    if let (true, Some(on_select)) = (select_mode, on_select) {
        card = card
            .tab_index(0)
            .focus(crate::theme::desktop_focus(theme.desktop.focus))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_select(window, cx));
    }

    let left = div()
        .flex_1()
        .mr(px(12.))
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_tertiary)
                .child(format_date_long(order.date_epoch_ms)),
        )
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.primary)
                .child(order.title.clone()),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(theme.text_primary)
                .child(description),
        );

    let (badge_bg, badge_border, badge_fg, badge_label) = if order.approved {
        (
            if theme.flat {
                theme.success_surface
            } else {
                theme.glass_success
            },
            theme.success,
            theme.success_text,
            "Bestätigt",
        )
    } else {
        (
            if theme.flat {
                theme.warning_surface
            } else {
                theme.glass_warning
            },
            theme.warning,
            theme.warning_text,
            "Ausstehend",
        )
    };
    let mut right = div().flex().flex_col().items_end().gap(px(8.)).child(
        div()
            .px(px(10.))
            .py(px(4.))
            .rounded(px(12.))
            .border_1()
            .border_color(badge_border)
            .bg(badge_bg)
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(badge_fg)
            .child(badge_label),
    );

    if select_mode {
        right = right.child(
            div()
                .w(px(24.))
                .h(px(24.))
                .rounded(px(12.))
                .border_1()
                .border_color(if selected {
                    theme.primary
                } else {
                    theme.border
                })
                .bg(if selected {
                    theme.primary
                } else {
                    hsla(0., 0., 0., 0.)
                })
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(14.))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.on_primary)
                .child(if selected { "✓" } else { "" }),
        );
    } else if cancelling {
        right = right.child(
            div()
                .w(px(32.))
                .h(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(theme.error, 16.)),
        );
    } else if can_cancel {
        let bg = if theme.flat {
            theme.error_surface
        } else {
            theme.glass_error
        };
        let (fill, border, glyph) = if cutoff {
            (
                if theme.flat {
                    theme.surface_variant
                } else {
                    theme.glass_surface_variant
                },
                theme.text_tertiary,
                theme.text_tertiary,
            )
        } else {
            (bg, theme.error, theme.error)
        };
        let button = div()
            .id(SharedString::from(format!(
                "order-cancel-{}",
                order.position_id
            )))
            .w(px(32.))
            .h(px(32.))
            .rounded(px(16.))
            .bg(fill)
            .border_1()
            .border_color(border)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(glyph)
                    .child("✕"),
            );
        // §4.6: past the cutoff the button is disabled, not merely greyed.
        let button = if cancel_enabled(can_cancel, cutoff) {
            button
                .tab_index(0)
                .focus(crate::theme::desktop_focus(theme.desktop.focus))
                .cursor_pointer()
                .on_click(move |_, window, cx| on_cancel(window, cx))
        } else {
            button.opacity(0.4)
        };
        right = right.child(button);
    }

    card.child(left).child(right)
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
        cancel_enabled, order_date_width, order_description, order_status_label,
        ORDER_DATE_COMPACT, ORDER_DATE_EXPANDED, ORDER_STATUS_EXPANDED,
    };
    use crate::components::record_table::CHECK_COLUMN;
    use snackpilot_core::domain::OrderedMenu;

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
