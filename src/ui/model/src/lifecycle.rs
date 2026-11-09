//! Launch and foreground lifecycle (REQ-SCR-09, task 5.7).
//!
//! `bootstrap` publishes cached snapshots first, tracks `app.launched`, fetches
//! the broadcast, asks for notification permission, restores the company
//! geofence idempotently and then runs the saved-credentials auto-login (which
//! is Ventopay-first with a single billing fetch). `on_foreground` refreshes
//! orders, billing and the broadcast under a re-entrancy guard; the broadcast
//! banner comes back on every foreground after being dismissed.

use std::sync::atomic::{AtomicBool, Ordering};

use snackpilot_core::storage::Kv;

use crate::runtime::handle;
use crate::session::AppModel;

static FOREGROUND_RUNNING: AtomicBool = AtomicBool::new(false);

impl AppModel {
    /// Launch order of REQ-SCR-09.
    pub fn bootstrap(&self) {
        // 1. Cached snapshots first, so the first frame shows data.
        self.core.load_cached_menus();
        self.core.load_cached_orders();
        self.core.load_cached_billing_months();
        let menus = self.core.menu_snapshot();
        let orders = self.core.split_orders();
        let months = self.core.billing_month_options();
        self.hub.update("cache_first", |s| {
            if !s
                .menu_snapshot
                .as_ref()
                .is_some_and(|m| !m.items.is_empty())
            {
                s.menu_snapshot = Some(menus);
            }
            if s.orders_split.is_none() {
                s.orders_split = Some(orders);
            }
            if s.month_options.is_empty() {
                s.month_options = months;
            }
        });

        // 2. `app.launched`.
        let _ = self.platform.track("app.launched", Default::default());

        // 3. Broadcast (best-effort), then 4. the notification permission and
        //    5. the geofence restore (idempotent across launches).
        self.refresh_broadcast();
        // Evidence builds may suppress the launch prompt so captures are not
        // blocked by the system dialog (debug only).
        #[cfg(debug_assertions)]
        let skip_prompt = std::env::args().any(|a| a == "--no-permission-prompt");
        #[cfg(not(debug_assertions))]
        let skip_prompt = false;
        if !skip_prompt {
            let _ = self.platform.request_notification_permission();
        }
        if self.state().company_location.is_some() {
            // Idempotent across launches: one start per installed app.
            let started =
                self.kv.get("geofence_restored").ok().flatten().as_deref() == Some("true");
            if !started {
                let _ = self.platform.start_geofence();
                let _ = self.kv.set("geofence_restored", "true");
            }
        }

        // 6. Saved credentials (Ventopay first, one billing fetch), then the
        //    saved-credentials session refresh.
        let _ = self.attempt_auto_login();
        self.refresh_diagnostic_log();
    }

    /// Fetch the operator broadcast and re-arm the banner.
    pub fn refresh_broadcast(&self) {
        // Tests never reach the network: the gist fetch is production-only.
        #[cfg(not(test))]
        let message = handle().block_on(self.core.fetch_broadcast());
        #[cfg(test)]
        let message: Option<String> = None;
        self.hub.update("broadcast", move |s| {
            s.broadcast = message;
            s.broadcast_dismissed = false;
        });
    }

    /// Hide the banner until the next foreground.
    pub fn dismiss_broadcast(&self) {
        self.hub
            .update("broadcast_dismissed", |s| s.broadcast_dismissed = true);
    }

    /// Foreground return (REQ-SCR-09): refresh orders, billing and the
    /// broadcast once per transition; re-entrant calls are dropped.
    pub fn on_foreground(&self) {
        if FOREGROUND_RUNNING.swap(true, Ordering::AcqRel) {
            return;
        }
        handle().block_on(async {
            let _ = self.load_orders().await;
            self.load_billing(false).await;
        });
        self.refresh_broadcast();
        FOREGROUND_RUNNING.store(false, Ordering::Release);
    }
}
