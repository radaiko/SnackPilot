//! UniFFI facade — `SnackPilotCore` and the FFI-exported surface consumed by the SwiftUI
//! and Compose shells (docs/architecture §4). Wires the portable core behind
//! `#[uniffi::export]`; the shells inject the storage path + credentials and receive plain
//! records back. Async ops use the tokio runtime.
use crate::datetime::{Clock, SystemClock};
use crate::domain::{
    CompanyLocation, Credentials, GeofenceEvent, GourmetMonthlyBilling, GourmetUserInfo,
    MenuSnapshot, MonthOption, OrderProgress, OrderedMenu, OrdersSplit, VentopayMonthlyBilling,
};
use crate::error::CoreError;
use crate::features::billing::BillingStore;
use crate::features::menus::MenuStore;
use crate::features::orders::OrderStore;
use crate::features::{AnalyticsSink, ProgressListener};
use crate::gourmet::api::GourmetApi;
use crate::gourmet::provider::GourmetProvider;
use crate::http::reqwest_transport::ReqwestTransport;
use crate::http::Transport;
use crate::notify::cancel_reminder::check_cancel_reminder;
use crate::notify::daily_reminder::check_daily_reminder;
use crate::notify::geofence::handle_geofence_event;
use crate::notify::menu_check::run_background_menu_check;
use crate::notify::{log, DailyReminderSettings, MenuCheckResult, NotificationCommand};
use crate::storage::{cache, FileKv, Kv};
use crate::ventopay::api::VentopayApi;
use crate::ventopay::provider::VentopayProvider;
use std::sync::Arc;

/// Host-injected configuration (docs/architecture §4.1).
#[derive(uniffi::Record)]
pub struct CoreConfig {
    /// Absolute path to a host-owned directory for the unencrypted KV store.
    pub storage_dir: String,
}

/// The single exported core object. Holds the two scraping sessions, the three feature
/// stores, and the shared KV + clock.
#[derive(uniffi::Object)]
pub struct SnackPilotCore {
    gourmet: Arc<GourmetProvider>,
    ventopay: Arc<VentopayProvider>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    analytics: Option<Arc<dyn AnalyticsSink>>,
    menus: Arc<MenuStore>,
    orders: Arc<OrderStore>,
    billing: Arc<BillingStore>,
}

#[uniffi::export(async_runtime = "tokio")]
impl SnackPilotCore {
    /// Construct the core. Gourmet uses reqwest's cookie store; Ventopay disables it (the
    /// app-owned jar is the only one — ban rule #8).
    #[uniffi::constructor]
    pub fn new(
        config: CoreConfig,
        analytics: Option<Arc<dyn AnalyticsSink>>,
    ) -> Result<Arc<Self>, CoreError> {
        let gourmet_tx: Arc<dyn Transport> = Arc::new(ReqwestTransport::new(true)?);
        let ventopay_tx: Arc<dyn Transport> = Arc::new(ReqwestTransport::new(false)?);
        let kv: Arc<dyn Kv> = Arc::new(FileKv::new(config.storage_dir.into()));
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        // Wrap the live scraping APIs in providers that swap to demo data on magic-credential
        // login (demo-mode §1). Stores hold the providers; the live path is untouched.
        let gourmet = Arc::new(GourmetProvider::new(
            Arc::new(GourmetApi::new(gourmet_tx)),
            clock.clone(),
        ));
        let ventopay = Arc::new(VentopayProvider::new(
            Arc::new(VentopayApi::new(ventopay_tx)),
            clock.clone(),
        ));

        let mut menu_store = MenuStore::new(gourmet.clone(), kv.clone(), clock.clone());
        if let Some(a) = &analytics {
            menu_store = menu_store.with_analytics(a.clone());
        }
        let menus = Arc::new(menu_store);
        let orders = Arc::new(OrderStore::new(gourmet.clone(), kv.clone(), clock.clone()));
        let billing = Arc::new(BillingStore::new(
            gourmet.clone(),
            ventopay.clone(),
            kv.clone(),
            clock.clone(),
        ));

        Ok(Arc::new(Self {
            gourmet,
            ventopay,
            kv,
            clock,
            analytics,
            menus,
            orders,
            billing,
        }))
    }

    // ---- Gourmet session ----
    pub async fn gourmet_login(&self, creds: Credentials) -> Result<GourmetUserInfo, CoreError> {
        // Magic credentials activate demo mode: swap in the offline backend and clear cached
        // live menus BEFORE any request, so demo creds never reach the live server (§1).
        if crate::demo::is_demo_credentials(&creds.username, &creds.password) {
            self.gourmet.enter_demo();
            self.menus.reset_for_demo();
        } else if self.gourmet.is_demo() {
            // Real credentials after a demo session: leave demo mode and clear the demo-populated
            // menus so the live fetch starts clean. Without this, demo data sticks forever (§1).
            self.gourmet.exit_demo();
            self.menus.reset_for_demo();
        }
        self.gourmet.login(creds).await
    }
    pub async fn gourmet_logout(&self) -> Result<(), CoreError> {
        self.gourmet.logout().await
    }
    pub fn gourmet_is_authenticated(&self) -> bool {
        self.gourmet.is_authenticated()
    }
    pub fn gourmet_user_info(&self) -> Option<GourmetUserInfo> {
        self.gourmet.user_info()
    }

    // ---- Ventopay session ----
    pub async fn ventopay_login(&self, creds: Credentials) -> Result<(), CoreError> {
        // Magic credentials activate demo mode on the Ventopay side too (§1, §4) — the demo
        // creds must never reach the live server.
        if crate::demo::is_demo_credentials(&creds.username, &creds.password) {
            self.ventopay.enter_demo();
        } else if self.ventopay.is_demo() {
            // Real credentials after a demo session: leave demo mode so live transactions load (§1).
            self.ventopay.exit_demo();
        }
        self.ventopay.login(creds).await
    }
    pub async fn ventopay_logout(&self) -> Result<(), CoreError> {
        self.ventopay.logout().await
    }
    pub fn ventopay_is_authenticated(&self) -> bool {
        self.ventopay.is_authenticated()
    }

    // ---- Menus ----
    pub fn load_cached_menus(&self) {
        self.menus.load_cached_menus();
    }
    pub async fn fetch_menus(&self, force: bool) -> Result<MenuSnapshot, CoreError> {
        self.menus.fetch_menus(force).await?;
        Ok(self.menus.snapshot())
    }
    pub async fn refresh_availability(&self) -> Result<MenuSnapshot, CoreError> {
        self.menus.refresh_availability().await?;
        Ok(self.menus.snapshot())
    }
    pub fn menu_snapshot(&self) -> MenuSnapshot {
        self.menus.snapshot()
    }
    /// Current submit-pipeline phase, if a submit is in flight (menus §6.6). Lets the shell
    /// render the order-progress banner (Adding/Confirming/Cancelling/Refreshing).
    pub fn order_progress(&self) -> Option<OrderProgress> {
        self.menus.order_progress()
    }
    /// Demo menus rendered as a snapshot — no network (demo-mode §5.2). Lets the shell show
    /// real menu data offline (store review / FFI preview) without touching the live server.
    pub fn demo_menu_snapshot(&self) -> MenuSnapshot {
        let items = crate::demo::data::generate_demo_menus(self.clock.as_ref());
        let mut dates: Vec<String> = items.iter().map(|i| i.day.clone()).collect();
        dates.sort();
        dates.dedup();
        MenuSnapshot {
            items,
            available_dates: dates,
            pending_orders: vec![],
            pending_cancellations: vec![],
            loading: false,
            refreshing: false,
            error: None,
        }
    }
    pub fn toggle_pending(&self, menu_id: String, date_key: String) -> MenuSnapshot {
        self.menus.toggle_pending(menu_id, date_key)
    }
    pub fn clear_pending_changes(&self) -> MenuSnapshot {
        self.menus.clear_pending_changes()
    }
    pub fn is_ordering_cutoff(&self, date_key: String) -> bool {
        self.menus.is_ordering_cutoff(&date_key)
    }
    pub fn selected_date(&self) -> Option<String> {
        self.menus.selected_date()
    }
    pub fn set_selected_date(&self, date_key: String) {
        self.menus.set_selected_date(date_key);
    }
    /// Submit pending orders/cancellations (§6.5), then refresh the order store.
    pub async fn submit_orders(
        &self,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<MenuSnapshot, CoreError> {
        let current = self.orders.orders();
        let snap = self.menus.submit_orders(&current, progress).await?;
        let _ = self.orders.fetch_orders().await; // v1 refreshes orders in the submit flow
        Ok(snap)
    }

    // ---- Orders ----
    pub fn load_cached_orders(&self) {
        self.orders.load_cached_orders();
    }
    pub async fn fetch_orders(&self) -> Result<(), CoreError> {
        self.orders.fetch_orders().await
    }
    pub fn orders(&self) -> Vec<OrderedMenu> {
        self.orders.orders()
    }
    pub fn split_orders(&self) -> OrdersSplit {
        self.orders.split()
    }
    pub fn orders_error(&self) -> Option<String> {
        self.orders.error()
    }
    pub async fn confirm_orders(&self) -> Result<(), CoreError> {
        self.orders.confirm_orders().await
    }
    pub async fn cancel_order(&self, position_id: String) -> Result<(), CoreError> {
        self.orders.cancel_order(position_id).await
    }

    // ---- Billing ----
    pub fn billing_month_options(&self) -> Vec<MonthOption> {
        self.billing.month_options()
    }
    pub fn load_cached_billing_months(&self) {
        self.billing.load_cached_months();
    }
    pub async fn fetch_billing(&self, offset: u8) -> Result<(), CoreError> {
        self.billing.fetch_billing(offset).await
    }
    pub async fn fetch_ventopay_billing(&self, offset: u8) -> Result<(), CoreError> {
        self.billing.fetch_ventopay_billing(offset).await
    }
    pub fn gourmet_billing_month(&self, month_key: String) -> Option<GourmetMonthlyBilling> {
        self.billing.gourmet_month(&month_key)
    }
    pub fn ventopay_billing_month(&self, month_key: String) -> Option<VentopayMonthlyBilling> {
        self.billing.ventopay_month(&month_key)
    }
    pub fn billing_error(&self) -> Option<String> {
        self.billing.error()
    }

    // ---- Notification decisions (the shell delivers the returned commands) ----
    pub fn daily_reminder_command(
        &self,
        settings: DailyReminderSettings,
    ) -> Option<NotificationCommand> {
        check_daily_reminder(self.clock.as_ref(), &settings, &self.orders.orders())
    }
    pub fn cancel_reminder_command(&self, is_at_company: bool) -> Option<NotificationCommand> {
        check_cancel_reminder(self.clock.as_ref(), is_at_company, &self.orders.orders())
    }
    pub fn geofence_commands(&self, event: GeofenceEvent) -> Vec<NotificationCommand> {
        handle_geofence_event(event, self.clock.as_ref(), &self.orders.orders())
    }

    // ---- Company location (notifications-location §1) ----
    pub fn company_location(&self) -> Option<CompanyLocation> {
        cache::load_company_location(self.kv.as_ref())
    }
    pub fn set_company_location(&self, latitude: f64, longitude: f64) {
        let _ = cache::save_company_location(
            self.kv.as_ref(),
            &CompanyLocation {
                latitude,
                longitude,
            },
        );
    }
    /// `clearCompanyLocation` (§1): drop the saved location and reset the geofence flag.
    pub fn clear_company_location(&self) {
        cache::clear_company_location(self.kv.as_ref());
    }
    pub fn is_at_company(&self) -> bool {
        cache::load_is_at_company(self.kv.as_ref())
    }
    pub fn set_is_at_company(&self, value: bool) {
        cache::save_is_at_company(self.kv.as_ref(), value);
    }
    /// Background new-menu check. Uses a FRESH, isolated Gourmet session (own cookie store —
    /// deliberate v2 change, notifications-new-menu §3.3).
    pub async fn run_menu_check(
        &self,
        creds: Option<Credentials>,
    ) -> Result<MenuCheckResult, CoreError> {
        let tx: Arc<dyn Transport> = Arc::new(ReqwestTransport::new(true)?);
        let fresh = GourmetApi::new(tx);
        Ok(run_background_menu_check(
            &fresh,
            self.kv.as_ref(),
            self.clock.as_ref(),
            self.analytics.as_deref(),
            creds,
        )
        .await)
    }

    // ---- Diagnostic log ----
    pub fn log_activate(&self, hours: u32) {
        log::activate_log(self.kv.as_ref(), self.clock.as_ref(), hours);
    }
    pub fn log_clear(&self) {
        log::clear_log(self.kv.as_ref());
    }
    pub fn log_is_active(&self) -> bool {
        log::is_active(self.kv.as_ref(), self.clock.as_ref())
    }
    pub fn log_entries(&self) -> Vec<log::LogEntry> {
        log::read_entries(self.kv.as_ref())
    }
}

/// Demo-mode credential check (demo-mode §1).
#[uniffi::export]
pub fn is_demo_credentials(username: String, password: String) -> bool {
    crate::demo::is_demo_credentials(&username, &password)
}

/// Crate version (smoke check).
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
