//! Launch and foreground lifecycle (REQ-SCR-09, task 5.7).
//!
//! `bootstrap` publishes cached snapshots first, tracks `app.launched`, fetches
//! the broadcast, asks for notification permission, restores the company
//! geofence idempotently and then runs the saved-credentials auto-login (which
//! is Ventopay-first with a single billing fetch). `on_foreground` refreshes
//! orders, billing and the broadcast under a re-entrancy guard; the broadcast
//! banner comes back on every foreground after being dismissed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use snackpilot_core::notify::NotificationCommand;
use snackpilot_core::storage::Kv;

use crate::platform::{Capabilities, LocationFix, Platform, PlatformError, SupportEmail};
use crate::runtime::handle;
use crate::session::AppModel;

static FOREGROUND_RUNNING: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FOREGROUND_RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

impl AppModel {
    /// Whether this process asked for the isolated evidence harness:
    /// `--evidence` on a desktop debug run, `SNACKPILOT_EVIDENCE=1` on a
    /// simulator or emulator. Never true in a release build.
    pub fn evidence_requested() -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        std::env::args().any(|arg| arg == "--evidence")
            || std::env::var("SNACKPILOT_EVIDENCE").ok().as_deref() == Some("1")
    }

    /// Switch the harness on for this store. Called by the host entry point
    /// before `bootstrap`, so the launch order below already runs isolated. The
    /// flag lives in the store, so one isolated run cannot leak into another
    /// and the default store never turns it on. A release build ignores it, so
    /// no production path can be talked into offline dummy data.
    pub fn set_evidence_mode(&self, enabled: bool) {
        #[cfg(debug_assertions)]
        let _ = self
            .kv
            .set("evidence_mode", if enabled { "true" } else { "false" });
        #[cfg(not(debug_assertions))]
        let _ = enabled;
    }

    /// Whether this store's launch and foreground paths run the isolated
    /// evidence harness: isolated data and dummy credentials, and never a live
    /// broadcast, system permission or geofence restore.
    pub fn evidence_mode(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.kv.get("evidence_mode").ok().flatten().as_deref() == Some("true")
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }

    /// The harness credential store for `inner`: everything the app does keeps
    /// talking to the real host, but every credential read and write stays in
    /// memory with the dummy pair. An evidence run therefore never reads or
    /// writes the normal credential slots, and the dummy pair opens the core's
    /// offline demo data. The host installs it only for an evidence run.
    pub fn evidence_platform(inner: Arc<dyn Platform>) -> Arc<dyn Platform> {
        let credentials = [
            "gourmet_username",
            "gourmet_password",
            "ventopay_username",
            "ventopay_password",
        ]
        .into_iter()
        .map(|slot| {
            let value = if slot.ends_with("username") {
                EvidencePlatform::DUMMY_USERNAME
            } else {
                EvidencePlatform::DUMMY_PASSWORD
            };
            (slot.to_string(), value.to_string())
        })
        .collect();
        Arc::new(EvidencePlatform {
            inner,
            credentials: Mutex::new(credentials),
        })
    }
}

/// In-memory credentials over the real host platform for the evidence harness.
struct EvidencePlatform {
    inner: Arc<dyn Platform>,
    credentials: Mutex<HashMap<String, String>>,
}

impl EvidencePlatform {
    /// The core's offline demo credentials: the fixture set, never a live host.
    const DUMMY_USERNAME: &'static str = snackpilot_core::demo::DEMO_USERNAME;
    const DUMMY_PASSWORD: &'static str = snackpilot_core::demo::DEMO_PASSWORD;

    fn store(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.credentials.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Platform for EvidencePlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        Ok(self.store().get(key).cloned())
    }

    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        self.store().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        self.store().remove(key);
        Ok(())
    }

    fn notify(&self, _command: NotificationCommand) -> Result<(), PlatformError> {
        // An evidence capture never delivers a system notification, even when
        // a foreground refresh asks for one (REQ-PLT-07 isolation).
        Ok(())
    }

    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        self.inner.request_notification_permission()
    }

    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.inner.request_location_authorisation()
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        self.inner.current_location()
    }

    fn start_geofence(&self) -> Result<(), PlatformError> {
        self.inner.start_geofence()
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        self.inner.stop_geofence()
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        self.inner.is_geofence_monitoring()
    }

    fn track(&self, _event: &str, _props: HashMap<String, String>) -> Result<(), PlatformError> {
        // Evidence mode is offline end to end: no analytics leaves the host,
        // not even the bootstrap `app.launched` (REQ-PLT-07).
        Ok(())
    }

    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError> {
        self.inner.set_app_icon(accent)
    }

    fn open_app_settings(&self) -> Result<(), PlatformError> {
        self.inner.open_app_settings()
    }

    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError> {
        self.inner.compose_email(email)
    }

    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
}

/// Clears the re-entrancy flag on every exit path, including a panic.
struct ForegroundGuard;

impl Drop for ForegroundGuard {
    fn drop(&mut self) {
        FOREGROUND_RUNNING.store(false, Ordering::Release);
    }
}

impl AppModel {
    /// Launch order of REQ-SCR-09.
    ///
    /// Split in two because most of it waits on the network: the cached
    /// snapshots are local and must land before the first frame, while the
    /// broadcast fetch, the auto-login and the tracking call can take seconds
    /// on a cold network. A host that cares about launch time calls
    /// [`Self::bootstrap_local`] and then [`Self::bootstrap_deferred`] off the
    /// UI thread, so the shell paints immediately and fills in.
    pub fn bootstrap(&self) {
        self.bootstrap_local();
        self.bootstrap_deferred();
    }

    /// The local half of the launch order: cached menus, orders and months, so
    /// the first frame shows the last state without touching the network.
    pub fn bootstrap_local(&self) {
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
    }

    /// The deferred half of the launch order: everything that can wait on the
    /// network or a system prompt.
    pub fn bootstrap_deferred(&self) {
        // The host capabilities gate the OS-service steps below. Desktop
        // declares notifications and geofence unavailable, so bootstrap must
        // not call either service there; iOS/Android declare both and keep
        // their existing order (REQ-PLT-07, REQ-DST-04).
        let capabilities = self.platform.capabilities();

        // 2. `app.launched`.
        let _ = self.platform.track("app.launched", Default::default());

        // 3. Broadcast (best-effort), then 4. the notification permission and
        //    5. the geofence restore (idempotent across launches). The evidence
        //    harness suppresses all three: it must not reach a live host, ask
        //    for a system permission or arm a region it cannot own.
        self.refresh_broadcast();
        // Evidence builds may suppress the launch prompt so captures are not
        // blocked by the system dialog (debug only).
        #[cfg(debug_assertions)]
        let skip_prompt =
            self.evidence_mode() || std::env::args().any(|a| a == "--no-permission-prompt");
        #[cfg(not(debug_assertions))]
        let skip_prompt = false;
        if !skip_prompt && capabilities.notifications {
            let _ = self.platform.request_notification_permission();
        }
        if capabilities.geofence && !self.evidence_mode() && self.state().company_location.is_some()
        {
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

    /// Fetch the operator broadcast and re-arm the banner. The evidence
    /// harness suppresses the fetch entirely and keeps the banner hidden, so an
    /// offline capture never waits on or shows a live message.
    pub fn refresh_broadcast(&self) {
        if self.evidence_mode() {
            self.hub.update("broadcast_suppressed", |s| {
                s.broadcast = None;
                s.broadcast_dismissed = true;
            });
            return;
        }
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
        #[cfg(test)]
        FOREGROUND_RUNS.fetch_add(1, Ordering::AcqRel);
        let _guard = ForegroundGuard;
        handle().block_on(async {
            let _ = self.load_orders().await;
            self.load_billing(false).await;
        });
        self.refresh_broadcast();
    }
}

/// Debug-only synthetic evidence scenarios (mobile-ui-rework task 1.1).
///
/// A capture run selects one *before* bootstrap and the host applies it after
/// the launch order, so the isolated store shows a chosen state without any
/// production state override: every variant below is either existing demo
/// data or a fixture in this evidence store, and the entry is absent from a
/// release build. No scenario sends credentials, reaches a host or arms a
/// system service.
#[cfg(debug_assertions)]
impl AppModel {
    /// Applies one named synthetic evidence scenario. Inert unless this store
    /// is already isolated with [`AppModel::set_evidence_mode`], so a normal
    /// launch can never be talked into fixture state. Unknown names are
    /// ignored.
    pub fn apply_evidence_scenario(&self, scenario: &str) {
        if !self.evidence_mode() {
            return;
        }
        match scenario
            .trim()
            .to_ascii_lowercase()
            .replace('_', "-")
            .as_str()
        {
            // The dummy evidence pair already logged into the offline demo
            // core; republish its generated snapshot so a scenario sequence
            // can return to the populated baseline.
            "populated" => {
                let menus = self.core().menu_snapshot();
                let orders = self.core().split_orders();
                let months = self.core().billing_month_options();
                self.hub().update("evidence.populated", |s| {
                    s.menu_snapshot = Some(menus);
                    s.orders_split = Some(orders);
                    if s.month_options.is_empty() {
                        s.month_options = months;
                    }
                    s.error_text = None;
                    s.busy = false;
                });
            }
            // The permission *result* is a fake answer, not a provider call.
            "permission-granted" => self.hub().update("evidence.permission", |s| {
                s.location_alert = None;
            }),
            "empty" => self.hub().update("evidence.empty", |s| {
                s.menu_snapshot = Some(snackpilot_core::domain::MenuSnapshot {
                    items: Vec::new(),
                    available_dates: Vec::new(),
                    pending_orders: Vec::new(),
                    pending_cancellations: Vec::new(),
                    loading: false,
                    refreshing: false,
                    error: None,
                });
                s.orders_split = Some(snackpilot_core::domain::OrdersSplit {
                    upcoming: Vec::new(),
                    past: Vec::new(),
                });
                s.error_text = None;
            }),
            "loading" => self.hub().update("evidence.loading", |s| {
                s.menu_snapshot = None;
                s.orders_split = None;
                s.error_text = None;
                s.busy = true;
            }),
            "refresh" => self.hub().update("evidence.refresh", |s| {
                if let Some(snapshot) = s.menu_snapshot.as_mut() {
                    snapshot.loading = false;
                    snapshot.refreshing = true;
                    snapshot.error = None;
                }
                s.error_text = None;
            }),
            "error" => self.hub().update("evidence.error", |s| {
                s.error_text = Some(crate::ErrorText {
                    message: "Synthetischer Fehler (Evidence)".into(),
                    retry: true,
                });
            }),
            "pending" => {
                let state = self.state();
                if let (Some(day), Some(snapshot)) = (state.selected_day, state.menu_snapshot) {
                    if let Some(item) = snapshot.items.iter().find(|i| i.day == day) {
                        self.toggle(item);
                    }
                }
            }
            "progress" => self.hub().update("evidence.progress", |s| {
                s.order_progress = Some(
                    crate::progress_label(snackpilot_core::domain::OrderProgress::Confirming)
                        .to_string(),
                );
            }),
            // The fake permission *result* renders the existing exact denial
            // alert; nothing asks the OS or a provider.
            "permission-denied" => self.hub().update("evidence.permission", |s| {
                s.location_alert = Some(crate::Alert::denied());
            }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FOREGROUND_RUNNING, FOREGROUND_RUNS};
    use crate::{AppModel, FakePlatform};
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    fn model(tag: &str) -> AppModel {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-lifecycle-unit-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        AppModel::new(dir, Arc::new(FakePlatform::new())).unwrap()
    }

    /// @req REQ-SCR-09
    #[test]
    fn cache_first_publishes_the_core_snapshot() {
        let model = model("cache");
        let expected = model.core().menu_snapshot();
        model.bootstrap();
        assert_eq!(model.state().menu_snapshot, Some(expected));
    }

    /// @req REQ-SCR-09
    #[test]
    fn auto_login_fetches_billing_once() {
        use crate::platform::Platform;
        let platform = Arc::new(FakePlatform::new());
        platform
            .save_credential("gourmet_username", "demo")
            .unwrap();
        platform
            .save_credential("gourmet_password", "demo1234!")
            .unwrap();
        platform
            .save_credential("ventopay_username", "demo")
            .unwrap();
        platform
            .save_credential("ventopay_password", "demo1234!")
            .unwrap();
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-lifecycle-unit-billing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let model = AppModel::new(dir, platform).unwrap();
        crate::BILLING_FETCHES.store(0, Ordering::Release);
        model.bootstrap();
        assert_eq!(crate::BILLING_FETCHES.load(Ordering::Acquire), 1);
    }

    /// @req REQ-SCR-09
    #[test]
    fn foreground_guard_drops_reentrant_calls() {
        let model = model("guard");
        FOREGROUND_RUNNING.store(true, Ordering::Release);
        let before = FOREGROUND_RUNS.load(Ordering::Acquire);
        model.on_foreground();
        assert_eq!(FOREGROUND_RUNS.load(Ordering::Acquire), before);
        FOREGROUND_RUNNING.store(false, Ordering::Release);
    }
}
