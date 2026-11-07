//! Orders state and actions: ports the orders half of `AppModel.swift`
//! (confirm, single-flight cancel, multi-select batch cancel) with v1 parity.

use snackpilot_core::domain::OrderedMenu;

use crate::errors::human_error;
use crate::format::day_key_from_epoch_ms;
use crate::runtime::handle;
use crate::session::AppModel;
use crate::state::{ErrorText, StateHub};

/// OrdersView banner text (OrdersView.swift:87): "N unbestätigte
/// Bestellung(en)" over the upcoming orders, None when all are confirmed
/// (04-ui-ux.md §3 "Bestätigen-Banner").
pub fn unconfirmed_banner_text(orders: &[OrderedMenu]) -> Option<String> {
    let count = orders.iter().filter(|o| !o.approved).count();
    if count == 0 {
        return None;
    }
    Some(format!(
        "{count} unbestätigte {}",
        if count == 1 {
            "Bestellung"
        } else {
            "Bestellungen"
        }
    ))
}

/// Clears the single-flight marker on scope exit, including during a panic
/// unwind out of the cancel flow.
struct ClearCancelling<'a>(&'a StateHub);

impl Drop for ClearCancelling<'_> {
    fn drop(&mut self) {
        self.0.update("cancelling_off", |s| s.cancelling_id = None);
    }
}

impl AppModel {
    /// Banner over the upcoming list: [`unconfirmed_banner_text`] over the
    /// current split (Swift `unconfirmedCount`, OrdersView.swift:9-10).
    pub fn unconfirmed_banner_text(&self) -> Option<String> {
        unconfirmed_banner_text(&self.state().orders_split?.upcoming)
    }

    /// Swift `confirmOrders` (:497): `busy` guards the banner button against
    /// double-taps → duplicate confirm POSTs; failures land in `orders_error`.
    pub fn confirm_orders(&self) {
        handle().block_on(async {
            self.hub().update("busy", |s| s.busy = true);
            if let Err(e) = self.core().confirm_orders().await {
                let message = human_error(&e);
                self.hub().update("orders_error", |s| {
                    s.orders_error = Some(ErrorText {
                        message,
                        retry: true,
                    });
                });
            }
            let _ = self.load_orders().await;
            self.hub().update("busy_off", |s| s.busy = false);
        });
    }

    /// Swift `cancelOrder` (:508): one cancel at a time (orders §6.2) — the
    /// check and the claim happen in ONE hub update, so two taps racing on the
    /// same instant cannot both pass. The guard clears the marker even when the
    /// flow panics, so a crashed cancel never wedges the guard.
    pub fn cancel_order(&self, position_id: &str) {
        let id = position_id.to_string();
        let claimed = self.hub().try_update(
            "cancelling",
            |s| s.cancelling_id.is_none(),
            |s| {
                s.cancelling_id = Some(id);
            },
        );
        if !claimed {
            return;
        }
        let _guard = ClearCancelling(self.hub());
        handle().block_on(async {
            let _ = self.core().cancel_order(position_id.to_string()).await;
            let _ = self.load_orders().await;
        });
    }

    /// Swift `isCancellationCutoff` (:518): past the 09:00 Europe/Vienna
    /// cutoff of the order's day (orders §6.4, same rule as ordering).
    pub fn is_cancellation_cutoff(&self, order: &OrderedMenu) -> bool {
        self.is_cutoff(&day_key_from_epoch_ms(order.date_epoch_ms))
    }

    /// Enter OrdersView multi-select mode (orders §6.1).
    pub fn begin_order_selection(&self) {
        self.hub().update("selection_mode", |s| {
            s.order_selection_mode = true;
            s.selected_order_ids.clear();
        });
    }

    /// Swift `toggleOrderSelection` (:527).
    pub fn toggle_order_selection(&self, position_id: &str) {
        let id = position_id.to_string();
        self.hub().update("selection", move |s| {
            if let Some(pos) = s.selected_order_ids.iter().position(|x| *x == id) {
                s.selected_order_ids.remove(pos);
            } else {
                s.selected_order_ids.push(id);
            }
        });
    }

    /// Swift `endOrderSelection` (:531).
    pub fn end_order_selection(&self) {
        self.hub().update("selection_end", |s| {
            s.order_selection_mode = false;
            s.selected_order_ids.clear();
        });
    }

    /// Swift `cancelSelectedOrders` (:536): batch-cancel the selection in ONE
    /// edit-mode session; guarded on a non-empty selection and `!busy`.
    pub fn cancel_selected_orders(&self) {
        let state = self.state();
        let ids = state.selected_order_ids.clone();
        if ids.is_empty() || state.busy {
            return;
        }
        self.hub().update("busy", |s| s.busy = true);
        handle().block_on(async {
            let _ = self.core().cancel_orders(ids).await;
            let _ = self.load_orders().await;
        });
        self.end_order_selection();
        self.hub().update("busy_off", |s| s.busy = false);
    }
}
