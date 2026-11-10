//! `AppModel`: the UI-facing model that owns the core and drives the session
//! flows. Ports `src/ios/SnackPilot/AppModel.swift` (login, ventopayLogin,
//! logouts, attemptAutoLogin, loadSession/loadOrders/loadBilling) with v1
//! parity: credentials are saved before they are validated, and logout keeps
//! the saved credentials.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use snackpilot_core::domain::Credentials;
use snackpilot_core::ffi::{CoreConfig, SnackPilotCore};
use snackpilot_core::notify::NotificationCommand;
use snackpilot_core::storage::FileKv;

use crate::errors::{human_error, is_demo_user};
use crate::platform::{Platform, PlatformAnalytics, PlatformError};
use crate::runtime::{handle, log};
use crate::state::{Alert, ErrorText, State, StateHub, Subscriber};

/// The single model instance the UI binds to (Swift `AppModel`).
pub struct AppModel {
    pub(crate) core: Arc<SnackPilotCore>,
    pub(crate) platform: Arc<dyn Platform>,
    pub(crate) hub: Arc<StateHub>,
    /// Same directory as core's own `FileKv`; holds the model's preference keys
    /// (theme/accent/reminder) and lets the diagnostics view read core's keys.
    pub(crate) kv: FileKv,
    last_notifications: Mutex<Vec<NotificationCommand>>,
}

impl AppModel {
    /// Builds the core with an analytics sink forwarding to
    /// `Platform::track` (design D4). `storage_dir` holds the core
    /// caches; the UI thread is never a tokio worker (`runtime`).
    pub fn new(storage_dir: PathBuf, platform: Arc<dyn Platform>) -> Result<Self, PlatformError> {
        let core = SnackPilotCore::new(
            CoreConfig {
                storage_dir: storage_dir.to_string_lossy().into_owned(),
            },
            Some(Arc::new(PlatformAnalytics(platform.clone()))),
        )
        .map_err(|e| PlatformError::System(human_error(&e)))?;
        let model = Self {
            core,
            platform,
            hub: Arc::new(StateHub::new()),
            kv: FileKv::new(storage_dir),
            last_notifications: Mutex::new(Vec::new()),
        };
        // Swift init parity: restore preferences, reminder settings, the saved
        // company location and the log view state at startup. None of these
        // touch the platform, so startup never switches the app icon.
        model.load_preferences();
        model.load_reminder_state();
        model.load_location_state();
        model.refresh_diagnostic_log();
        Ok(model)
    }

    /// The core, for UI bindings that read it directly (Swift parity).
    pub fn core(&self) -> &Arc<SnackPilotCore> {
        &self.core
    }

    /// The state hub, for the screen modules' `impl AppModel` blocks.
    pub(crate) fn hub(&self) -> &Arc<StateHub> {
        &self.hub
    }

    /// Current state snapshot for the UI.
    pub fn state(&self) -> State {
        self.hub.get()
    }

    /// Subscribe to state changes; the callback fires after each change.
    pub fn subscribe(&self, callback: Subscriber) {
        self.hub.subscribe(callback);
    }

    /// Inbound: a geofence transition (design D4); runs the same flow as the
    /// Swift region-monitoring handler.
    pub fn on_geofence_event(&self, inside: bool) {
        log("geofence", if inside { "enter" } else { "exit" });
        self.handle_geofence(if inside {
            snackpilot_core::domain::GeofenceEvent::Enter
        } else {
            snackpilot_core::domain::GeofenceEvent::Exit
        });
    }

    /// Inbound: the app went to the background.
    pub fn on_background(&self) {
        log("lifecycle", "background");
    }

    // ---- session flows (AppModel.swift parity) ----

    /// Swift `login` (AppModel.swift:234): save both credentials first
    /// (persist-before-validate, REQ-PLT-01), then authenticate, then load
    /// the session. A failed login still keeps the credentials (v1 parity).
    /// No path leaves `busy` set, including a failing notification bridge.
    pub fn login(&self, username: &str, password: &str) -> Result<(), PlatformError> {
        handle().block_on(self.login_async(username, password))
    }

    async fn login_async(&self, username: &str, password: &str) -> Result<(), PlatformError> {
        self.hub.update("busy", |s| {
            s.busy = true;
            s.error_text = None;
        });
        let result = self.login_inner(username, password).await;
        self.hub.update("busy_off", |s| s.busy = false);
        result
    }

    async fn login_inner(&self, username: &str, password: &str) -> Result<(), PlatformError> {
        self.platform
            .save_credential("gourmet_username", username)?;
        self.platform
            .save_credential("gourmet_password", password)?;

        let creds = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        match self.core.gourmet_login(creds).await {
            Ok(info) => {
                let demo = is_demo_user(&info);
                self.hub.update("gourmet_logged_in", |s| {
                    s.gourmet_authenticated = true;
                    s.user_info = Some(info);
                    s.demo_mode = demo;
                });
                if demo {
                    // Swift :245 — in demo mode the same credentials open the
                    // Automaten side too; a failure there must not fail the
                    // Kantine login.
                    let demo_creds = Credentials {
                        username: username.to_string(),
                        password: password.to_string(),
                    };
                    let _ = self.core.ventopay_login(demo_creds).await;
                    let auth = self.core.ventopay_is_authenticated();
                    self.hub
                        .update("demo_ventopay", |s| s.ventopay_authenticated = auth);
                }
                self.load_session().await
            }
            Err(e) => {
                let msg = human_error(&e);
                self.hub.update("gourmet_login_failed", |s| {
                    s.error_text = Some(ErrorText {
                        message: msg.clone(),
                        retry: true,
                    });
                });
                Err(PlatformError::System(msg))
            }
        }
    }

    /// Swift `ventopayLogin` (AppModel.swift:263): save both credentials,
    /// authenticate, then refresh the month options and billing — unless the
    /// caller suppresses the fetch (auto-login loads billing through
    /// `load_session` instead, so the month is fetched exactly once,
    /// REQ-SCR-06). No path leaves `ventopay_busy` set.
    pub fn ventopay_login(
        &self,
        username: &str,
        password: &str,
        fetch_billing: bool,
    ) -> Result<(), PlatformError> {
        handle().block_on(self.ventopay_login_inner(username, password, fetch_billing))
    }

    async fn ventopay_login_inner(
        &self,
        username: &str,
        password: &str,
        fetch_billing: bool,
    ) -> Result<(), PlatformError> {
        self.hub.update("ventopay_busy", |s| {
            s.ventopay_busy = true;
            s.ventopay_error = None;
        });
        let result = self
            .ventopay_login_inner2(username, password, fetch_billing)
            .await;
        self.hub
            .update("ventopay_busy_off", |s| s.ventopay_busy = false);
        result
    }

    async fn ventopay_login_inner2(
        &self,
        username: &str,
        password: &str,
        fetch_billing: bool,
    ) -> Result<(), PlatformError> {
        self.platform
            .save_credential("ventopay_username", username)?;
        self.platform
            .save_credential("ventopay_password", password)?;

        let creds = Credentials {
            username: username.to_string(),
            password: password.to_string(),
        };
        match self.core.ventopay_login(creds).await {
            Ok(()) => {
                self.hub.update("ventopay_logged_in", |s| {
                    s.ventopay_authenticated = true;
                    s.ventopay_error = None;
                });
                if fetch_billing {
                    let options = self.core.billing_month_options();
                    self.hub
                        .update("month_options", |s| s.month_options = options);
                    self.load_billing(false).await;
                }
                Ok(())
            }
            Err(e) => {
                let msg = human_error(&e);
                self.hub.update("ventopay_login_failed", |s| {
                    s.ventopay_error = Some(ErrorText {
                        message: msg.clone(),
                        retry: true,
                    });
                });
                Err(PlatformError::System(msg))
            }
        }
    }

    /// Swift `gourmetLogout` (AppModel.swift:291): clears the Gourmet session
    /// and all menu/order state; saved credentials stay (v1 parity).
    pub fn gourmet_logout(&self) {
        handle().block_on(async {
            let _ = self.core.gourmet_logout().await;
            self.hub.update("gourmet_logged_out", |s| {
                s.gourmet_authenticated = false;
                s.user_info = None;
                s.demo_mode = false;
                s.menu_snapshot = None;
                s.orders_split = None;
                s.orders_error = None;
                s.gourmet_month = None;
            });
        });
    }

    /// Swift `ventopayLogout` (AppModel.swift:281): clears the Ventopay
    /// session and the selected month; saved credentials stay (v1 parity).
    pub fn ventopay_logout(&self) {
        handle().block_on(async {
            let _ = self.core.ventopay_logout().await;
            let auth = self.core.ventopay_is_authenticated();
            self.hub.update("ventopay_logged_out", |s| {
                s.ventopay_authenticated = auth;
                s.ventopay_month = None;
            });
        });
    }

    /// Open the system app settings (§3.8 Android hint / permission alerts).
    pub fn open_app_settings(&self) -> Result<(), PlatformError> {
        self.platform.open_app_settings()
    }

    /// Take the pending location/reminder alert so the UI can present it once
    /// (§3.8 flows set it from the model).
    pub fn take_location_alert(&self) -> Option<Alert> {
        let mut taken = None;
        self.hub.update("alert_taken", |s| {
            taken = s.location_alert.take();
        });
        taken
    }

    /// Saved credentials for the login form prefill (§3.5): username and
    /// password of the Kantine (or Automaten) pair, when stored.
    pub fn saved_credentials(&self, ventopay: bool) -> (Option<String>, Option<String>) {
        let prefix = if ventopay { "ventopay" } else { "gourmet" };
        let read = |field: &str| {
            self.platform
                .get_credential(&format!("{prefix}_{field}"))
                .ok()
                .flatten()
        };
        (read("username"), read("password"))
    }

    /// Swift `attemptAutoLogin` (AppModel.swift:166): restore both sessions
    /// from saved credentials. Ventopay is authenticated first WITHOUT its
    /// own billing fetch, so the Gourmet session-load fetches the month
    /// exactly once (REQ-SCR-06). A failing core login is contained and
    /// logged; a failing bridge answers with a typed error (REQ-PLT-06).
    pub fn attempt_auto_login(&self) -> Result<(), PlatformError> {
        handle().block_on(self.attempt_auto_login_async())
    }

    async fn attempt_auto_login_async(&self) -> Result<(), PlatformError> {
        let gu = self.platform.get_credential("gourmet_username")?;
        let gp = self.platform.get_credential("gourmet_password")?;
        let vu = self.platform.get_credential("ventopay_username")?;
        let vp = self.platform.get_credential("ventopay_password")?;
        if gu.is_none() && vu.is_none() {
            return Ok(());
        }

        if let (Some(u), Some(p)) = (vu, vp) {
            // The saved values are already current, so a re-save failure must
            // not block the restore (best effort).
            let _ = self.platform.save_credential("ventopay_username", &u);
            let _ = self.platform.save_credential("ventopay_password", &p);
            if let Err(e) = self.ventopay_login_inner(&u, &p, false).await {
                log("auto_login", &format!("ventopay: {e}"));
            }
        }

        if let (Some(u), Some(p)) = (gu, gp) {
            return self.login_inner(&u, &p).await;
        }

        // Ventopay-only user (Swift :225): no Gourmet session-load runs, so
        // the billing load belongs here.
        if self.core.ventopay_is_authenticated() {
            let options = self.core.billing_month_options();
            self.hub
                .update("month_options", |s| s.month_options = options);
            self.load_billing(false).await;
        }
        Ok(())
    }

    // ---- loading (Swift loadSession/loadOrders/loadBilling) ----

    async fn load_session(&self) -> Result<(), PlatformError> {
        // force = false: cache first (Swift loadSession :301).
        match self.core.fetch_menus(false).await {
            Ok(snap) => {
                self.hub.update("menus", |s| s.menu_snapshot = Some(snap));
                self.sync_selected_day();
            }
            Err(e) => self.hub.update("menus_failed", |s| {
                s.error_text = Some(ErrorText {
                    message: human_error(&e),
                    retry: true,
                });
            }),
        }
        // A failing notification bridge must not skip the rest of the
        // session: the month options and billing still load, and the
        // orders/notify error is returned last.
        let orders = self.load_orders().await;
        let options = self.core.billing_month_options();
        self.hub
            .update("month_options", |s| s.month_options = options);
        self.load_billing(false).await;
        orders
    }

    /// Reload of the Bestellungen tab (Cmd+R): run the orders load to
    /// completion; failures surface via `orders_error` like the async flow.
    pub fn refresh_orders(&self) {
        let _ = handle().block_on(self.load_orders());
    }

    /// Swift `loadOrders` (AppModel.swift:350): fetch orders — which returns
    /// the post-fetch notification decisions — then publish the split and
    /// deliver the commands. The split is published even when the bridge
    /// fails; the failure surfaces as a typed error instead of leaving the
    /// flow stuck (REQ-PLT-06).
    pub(crate) async fn load_orders(&self) -> Result<(), PlatformError> {
        let commands = match self.core.fetch_orders().await {
            Ok(commands) => commands,
            Err(e) => {
                log("orders", &human_error(&e));
                Vec::new()
            }
        };
        let split = self.core.split_orders();
        let error = self.core.orders_error().map(|message| ErrorText {
            message,
            retry: true,
        });
        self.hub.update("orders", |s| {
            s.orders_split = Some(split);
            s.orders_error = error;
        });
        self.record_notifications(commands)
    }

    /// Swift `loadBilling` (AppModel.swift:585): one loading window around
    /// both billing sources for the selected month; fetch errors are
    /// swallowed (pull-to-refresh is the retry, v1 parity). `force` bypasses
    /// the core's past-month cache skip (Swift `reloadBilling` :601).
    pub(crate) async fn load_billing(&self, force: bool) {
        #[cfg(test)]
        crate::BILLING_FETCHES.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.hub
            .update("billing_loading", |s| s.billing_loading = true);
        let offset = self.hub.get().selected_offset;
        let _ = self.core.fetch_billing(offset, force).await;
        let _ = self.core.fetch_ventopay_billing(offset, force).await;
        let key = self
            .hub
            .get()
            .month_options
            .iter()
            .find(|m| m.offset == offset)
            .map(|m| m.key.clone());
        self.hub.update("billing_loaded", |s| {
            s.billing_loading = false;
            if let Some(key) = key {
                s.gourmet_month = self.core.gourmet_billing_month(key.clone());
                s.ventopay_month = self.core.ventopay_billing_month(key);
            }
        });
    }

    // ---- helpers ----

    /// Deliver the core's notification decisions to the host bridge and keep
    /// them for the UI (Swift `lastNotificationCommands`). Returns the first
    /// bridge error, so the caller can surface it instead of leaving the
    /// model stuck (REQ-PLT-06).
    fn record_notifications(
        &self,
        commands: Vec<NotificationCommand>,
    ) -> Result<(), PlatformError> {
        let mut failure = None;
        for command in &commands {
            if let Err(e) = self.platform.notify(command.clone()) {
                failure.get_or_insert(e);
            }
        }
        *self.last_notifications.lock().unwrap() = commands;
        match failure {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }

    /// Notification commands delivered by the last orders fetch.
    pub fn last_notifications(&self) -> Vec<NotificationCommand> {
        self.last_notifications.lock().unwrap().clone()
    }

    /// The last notification command for `id`, or `None` when it was
    /// retracted by a later fetch (Swift `pendingNotificationCommand(for:)`).
    pub fn pending_notification_command(&self, id: &str) -> Option<NotificationCommand> {
        self.last_notifications
            .lock()
            .unwrap()
            .iter()
            .find(|c| command_id(c) == id)
            .cloned()
    }
}

fn command_id(command: &NotificationCommand) -> &str {
    match command {
        NotificationCommand::ScheduleAt { id, .. }
        | NotificationCommand::FireNow { id, .. }
        | NotificationCommand::CancelPending { id } => id,
    }
}
