//! Menus state and actions: ports the menu/day/submit half of
//! `AppModel.swift` (toggle, day navigation, submit with the progress
//! listener, order ↔ menu cross-reference) with v1 parity.

use std::sync::Arc;

use snackpilot_core::domain::{MenuCategory, MenuItem, MenuSnapshot, OrderProgress, OrderedMenu};
use snackpilot_core::features::ProgressListener;

use crate::errors::human_error;
use crate::format::day_key_from_epoch_ms;
use crate::runtime::handle;
use crate::session::AppModel;
use crate::state::{ErrorText, StateHub};

/// Ordering of category sections: MENÜ I, II, III, SUPPE & SALAT, UNKNOWN
/// (MenusView `categoryOrder`, menus §5).
pub fn category_rank(category: MenuCategory) -> u8 {
    match category {
        MenuCategory::Menu1 => 0,
        MenuCategory::Menu2 => 1,
        MenuCategory::Menu3 => 2,
        MenuCategory::SoupAndSalad => 3,
        MenuCategory::Unknown => 4,
    }
}

/// Section heading; `None` for SUPPE & SALAT, whose heading is suppressed
/// (MenusView `categoryHeading`, menus §5).
pub fn category_heading(category: MenuCategory) -> Option<&'static str> {
    match category {
        MenuCategory::SoupAndSalad => None,
        other => Some(other.display()),
    }
}

/// Live submit-pipeline phase labels (MenusView `progressLabel`, menus §6.6).
pub fn progress_label(phase: OrderProgress) -> &'static str {
    match phase {
        OrderProgress::Adding => "Wird in den Warenkorb gelegt...",
        OrderProgress::Confirming => "Bestellung wird bestätigt...",
        OrderProgress::Cancelling => "Bestellung wird storniert...",
        OrderProgress::Refreshing => "Menüs werden aktualisiert...",
    }
}

/// Nearest available day to today: first date on-or-after today, else the last
/// (Swift `nearestDay`, mirrors the core's `find_nearest_date`, menus §3.2/§4.1).
/// `dates` are sorted ascending "YYYY-MM-DD".
pub fn nearest_day(dates: &[String]) -> String {
    let today = crate::format::today_key();
    dates
        .iter()
        .find(|d| **d >= today)
        .cloned()
        .unwrap_or_else(|| dates[dates.len() - 1].clone())
}

/// Forwards the core's submit pipeline phases into `State::order_progress`
/// (Swift `ProgressBridge`).
struct HubProgress(Arc<StateHub>);

impl ProgressListener for HubProgress {
    fn on_progress(&self, phase: Option<OrderProgress>) {
        self.0.update("order_progress", |s| {
            s.order_progress = phase.map(progress_label).map(str::to_string);
        });
    }
}

impl AppModel {
    /// Swift `syncSelectedDay` (:370): after every fetch that can change
    /// `availableDates`, prefer the core's tracked day when valid; otherwise
    /// fall back to the NEAREST day — and do NOT write the fallback back, so a
    /// later fetch can still run its own nearest-selection.
    pub(crate) fn sync_selected_day(&self) {
        let dates = self
            .state()
            .menu_snapshot
            .map(|s| s.available_dates)
            .unwrap_or_default();
        let day = if dates.is_empty() {
            None
        } else {
            self.core()
                .selected_date()
                .filter(|sel| dates.contains(sel))
                .or_else(|| Some(nearest_day(&dates)))
        };
        self.hub().update("selected_day", |s| s.selected_day = day);
    }

    /// Swift `refreshMenus` (:336): force a fresh fetch and republish
    /// ("Erneut versuchen"); failures surface via `error_text`.
    pub fn refresh_menus(&self) {
        handle().block_on(async {
            self.hub().update("busy", |s| {
                s.busy = true;
                s.error_text = None;
            });
            match self.core().fetch_menus(true).await {
                Ok(snap) => {
                    self.hub().update("menus", |s| s.menu_snapshot = Some(snap));
                    self.sync_selected_day();
                }
                Err(e) => self.hub().update("menus_failed", |s| {
                    s.error_text = Some(ErrorText {
                        message: human_error(&e),
                        retry: true,
                    });
                }),
            }
            self.hub().update("busy_off", |s| s.busy = false);
        });
    }

    /// Swift `toggle` (:361): pending order/cancellation for an item
    /// (composite key handled in the core).
    pub fn toggle(&self, item: &MenuItem) {
        let snap = self
            .core()
            .toggle_pending(item.id.clone(), item.day.clone());
        self.hub()
            .update("menus_pending", |s| s.menu_snapshot = Some(snap));
    }

    /// Swift `clearPending` (:493): discard all pending changes.
    pub fn clear_pending(&self) {
        let snap = self.core().clear_pending_changes();
        self.hub()
            .update("menus_pending_cleared", |s| s.menu_snapshot = Some(snap));
    }

    /// Swift `hasPendingChanges` (:469).
    pub fn has_pending_changes(&self) -> bool {
        self.state()
            .menu_snapshot
            .is_some_and(|s| !s.pending_orders.is_empty() || !s.pending_cancellations.is_empty())
    }

    /// Swift `selectDay` (:394): writes through to the core so the pick
    /// survives refreshes.
    pub fn select_day(&self, date_key: &str) {
        self.core().set_selected_date(date_key.to_string());
        let day = date_key.to_string();
        self.hub()
            .update("selected_day", move |s| s.selected_day = Some(day));
    }

    /// Swift `prevDay` (:401): no-op at the first day or when the current day
    /// is not in the list (menus §4).
    pub fn prev_day(&self) {
        let state = self.state();
        let Some(dates) = state.menu_snapshot.as_ref().map(|s| &s.available_dates) else {
            return;
        };
        let Some(cur) = state.selected_day.as_ref() else {
            return;
        };
        let Some(idx) = dates.iter().position(|d| d == cur) else {
            return;
        };
        if idx > 0 {
            let day = dates[idx - 1].clone();
            self.select_day(&day);
        }
    }

    /// Swift `nextDay` (:409): when the current day is absent from the list,
    /// selects the first date (menus §4).
    pub fn next_day(&self) {
        let state = self.state();
        let Some(dates) = state.menu_snapshot.as_ref().map(|s| &s.available_dates) else {
            return;
        };
        if dates.is_empty() {
            return;
        }
        match dates
            .iter()
            .position(|d| Some(d) == state.selected_day.as_ref())
        {
            Some(idx) if idx < dates.len() - 1 => {
                let day = dates[idx + 1].clone();
                self.select_day(&day);
            }
            Some(_) => {}
            None => {
                let day = dates[0].clone();
                self.select_day(&day);
            }
        }
    }

    /// Swift `goToToday` (:420): jump to the nearest menu day (menus §4.1).
    pub fn go_to_today(&self) {
        let dates = self
            .state()
            .menu_snapshot
            .map(|s| s.available_dates)
            .unwrap_or_default();
        if dates.is_empty() {
            return;
        }
        let day = nearest_day(&dates);
        self.select_day(&day);
    }

    /// Swift `isCutoff` (:427): ordering closed for a day (menus §6.2 —
    /// 09:00 Europe/Vienna, computed in the core).
    pub fn is_cutoff(&self, date_key: &str) -> bool {
        self.core().is_ordering_cutoff(date_key.to_string())
    }

    /// Swift `submitOrders` (:474): persist the selected day first so the
    /// post-submit refetch keeps it, run the core pipeline with a progress
    /// listener, then republish menus and reload orders (the core refreshes
    /// orders internally; Swift reloads for the notification commands).
    pub fn submit_orders(&self) {
        handle().block_on(async {
            self.hub().update("busy", |s| {
                s.busy = true;
                s.error_text = None;
            });
            if let Some(day) = self.state().selected_day {
                self.core().set_selected_date(day);
            }
            let listener = Arc::new(HubProgress(self.hub().clone()));
            match self.core().submit_orders(Some(listener)).await {
                Ok(snap) => {
                    self.hub().update("menus", |s| s.menu_snapshot = Some(snap));
                    self.sync_selected_day();
                    let _ = self.load_orders().await;
                }
                Err(e) => self.hub().update("submit_failed", |s| {
                    s.error_text = Some(ErrorText {
                        message: human_error(&e),
                        retry: true,
                    });
                }),
            }
            self.hub().update("done", |s| {
                s.order_progress = None;
                s.busy = false;
            });
        });
    }

    /// Swift `dish(for:)` (:453): the matching menu item's subtitle, `None`
    /// when that day's menu is not loaded (caller falls back to the order's
    /// own subtitle).
    pub fn dish(&self, order: &OrderedMenu) -> Option<String> {
        let key = day_key_from_epoch_ms(order.date_epoch_ms);
        let sub = self
            .state()
            .menu_snapshot?
            .items
            .into_iter()
            .find(|i| i.title == order.title && i.day == key)?;
        (!sub.subtitle.is_empty()).then_some(sub.subtitle)
    }

    /// Swift `isOrdered` (:461): the parsed flag OR a matching entry in the
    /// orders list (the menu HTML doesn't always mark confirmed orders).
    pub fn is_ordered(&self, item: &MenuItem) -> bool {
        if item.ordered {
            return true;
        }
        let Some(split) = self.state().orders_split else {
            return false;
        };
        split
            .upcoming
            .iter()
            .chain(split.past.iter())
            .any(|o| o.title == item.title && day_key_from_epoch_ms(o.date_epoch_ms) == item.day)
    }

    /// Items of a day grouped into sections in `category_rank` order,
    /// empty sections skipped, original order within a section.
    pub fn categories_for_day(&self, day: &str) -> Vec<(MenuCategory, Vec<MenuItem>)> {
        let Some(snapshot) = self.state().menu_snapshot else {
            return Vec::new();
        };
        group_categories(&snapshot, day)
    }

    /// FAB label (04-ui-ux.md §3.1, v1 index.tsx:146-154) from the pending
    /// counts; `newOrderCount = pendingCount − cancellationCount`.
    pub fn fab_label(&self) -> String {
        let Some(s) = self.state().menu_snapshot else {
            return String::new();
        };
        let cancellations = s.pending_cancellations.len();
        let new_orders = s.pending_orders.len();
        let pending = cancellations + new_orders;
        if cancellations > 0 && new_orders > 0 {
            format!("Änderungen bestätigen ({pending})")
        } else if cancellations > 0 {
            format!("Stornieren ({cancellations})")
        } else {
            format!("Bestellen ({new_orders})")
        }
    }
}

fn group_categories(snapshot: &MenuSnapshot, day: &str) -> Vec<(MenuCategory, Vec<MenuItem>)> {
    let mut groups: Vec<(MenuCategory, Vec<MenuItem>)> = Vec::new();
    for item in snapshot.items.iter().filter(|i| i.day == day) {
        match groups.iter_mut().find(|(c, _)| c == &item.category) {
            Some((_, items)) => items.push(item.clone()),
            None => groups.push((item.category, vec![item.clone()])),
        }
    }
    groups.sort_by_key(|(c, _)| category_rank(*c));
    groups
}
