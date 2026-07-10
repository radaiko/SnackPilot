//! Billing feature service — dual-source (Gourmet + Ventopay) per-month billing
//! (03-features/billing.md, caching.md §3.3). Loading/error reflect the Gourmet fetch
//! only; Ventopay failures are non-blocking (billing §4.2 step 8).
use crate::datetime::Clock;
use crate::domain::{
    Bill, GourmetMonthlyBilling, MonthOption, VentopayMonthlyBilling, VentopayTransaction,
};
use crate::error::CoreResult;
use crate::gourmet::provider::GourmetProvider;
use crate::storage::{cache, Kv};
use crate::ventopay::provider::VentopayProvider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct BillingStore {
    gourmet: Arc<GourmetProvider>,
    ventopay: Arc<VentopayProvider>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    gourmet_months: Mutex<HashMap<String, GourmetMonthlyBilling>>,
    ventopay_months: Mutex<HashMap<String, VentopayMonthlyBilling>>,
    loading: Mutex<bool>,
    error: Mutex<Option<String>>,
}

impl BillingStore {
    pub fn new(
        gourmet: Arc<GourmetProvider>,
        ventopay: Arc<VentopayProvider>,
        kv: Arc<dyn Kv>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            gourmet,
            ventopay,
            kv,
            clock,
            gourmet_months: Mutex::new(HashMap::new()),
            ventopay_months: Mutex::new(HashMap::new()),
            loading: Mutex::new(false),
            error: Mutex::new(None),
        }
    }

    /// The three month options (offsets 0,1,2), recomputed from the clock (billing §2).
    pub fn month_options(&self) -> Vec<MonthOption> {
        (0u8..=2)
            .map(|offset| {
                let key = cache::month_key_from_offset(self.clock.as_ref(), offset as u32);
                MonthOption {
                    label: cache::month_label(&key),
                    key,
                    offset,
                }
            })
            .collect()
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }

    pub fn gourmet_month(&self, month_key: &str) -> Option<GourmetMonthlyBilling> {
        self.gourmet_months.lock().unwrap().get(month_key).cloned()
    }

    pub fn ventopay_month(&self, month_key: &str) -> Option<VentopayMonthlyBilling> {
        self.ventopay_months.lock().unwrap().get(month_key).cloned()
    }

    /// Load all three cached months (Gourmet + Ventopay) into memory (billing §5, caching §4.3).
    pub fn load_cached_months(&self) {
        for offset in 0u8..=2 {
            let key = cache::month_key_from_offset(self.clock.as_ref(), offset as u32);
            let label = cache::month_label(&key);
            if let Some(bills) = cache::load_gourmet_billing(self.kv.as_ref(), &key) {
                self.gourmet_months
                    .lock()
                    .unwrap()
                    .insert(key.clone(), build_gourmet_month(&key, &label, bills, 0));
            }
            if let Some(txs) = cache::load_ventopay_billing(self.kv.as_ref(), &key) {
                self.ventopay_months
                    .lock()
                    .unwrap()
                    .insert(key.clone(), build_ventopay_month(&key, &label, txs, 0));
            }
        }
    }

    /// §4.1 Gourmet fetch. Loading/error reflect this fetch only.
    pub async fn fetch_billing(&self, offset: u8, force: bool) -> CoreResult<()> {
        if offset > 2 {
            return Ok(()); // invalid offset → silent
        }
        if *self.loading.lock().unwrap() {
            return Ok(()); // in-flight guard
        }
        let key = cache::month_key_from_offset(self.clock.as_ref(), offset as u32);
        // past-month skip: offset != 0 and a non-empty entry already exists — unless `force`
        // (pull-to-refresh), which always re-hits the server so a stale/older cache is replaced.
        if offset != 0 && !force {
            if let Some(m) = self.gourmet_months.lock().unwrap().get(&key) {
                if !m.bills.is_empty() {
                    return Ok(());
                }
            }
        }
        *self.loading.lock().unwrap() = true;
        *self.error.lock().unwrap() = None;

        let result = self.gourmet.get_billings(&offset.to_string()).await;
        match result {
            Ok(bills) => {
                let _ = cache::save_gourmet_billing(self.kv.as_ref(), &key, &bills);
                let label = cache::month_label(&key);
                let now = self.clock.now_epoch_ms();
                self.gourmet_months
                    .lock()
                    .unwrap()
                    .insert(key.clone(), build_gourmet_month(&key, &label, bills, now));
                *self.loading.lock().unwrap() = false;
                Ok(())
            }
            Err(e) => {
                let msg = match e.to_string().as_str() {
                    "" => "Abrechnung konnte nicht geladen werden".to_string(),
                    s => s.to_string(),
                };
                *self.error.lock().unwrap() = Some(msg);
                *self.loading.lock().unwrap() = false;
                Ok(()) // failure is captured in `error`, not propagated (v1 store semantics)
            }
        }
    }

    /// §4.2 Ventopay fetch. Non-blocking: never sets loading/error; silent on failure.
    pub async fn fetch_ventopay_billing(&self, offset: u8, force: bool) -> CoreResult<()> {
        if offset > 2 {
            return Ok(());
        }
        let key = cache::month_key_from_offset(self.clock.as_ref(), offset as u32);
        if offset != 0 && !force {
            if let Some(m) = self.ventopay_months.lock().unwrap().get(&key) {
                if !m.transactions.is_empty() {
                    return Ok(());
                }
            }
        }
        if !self.ventopay.is_authenticated() {
            return Ok(()); // §4.2 step 3
        }
        let (from, to) = month_range(&key);
        match self.ventopay.get_transactions(&from, &to).await {
            Ok(txs) => {
                let _ = cache::save_ventopay_billing(self.kv.as_ref(), &key, &txs);
                let label = cache::month_label(&key);
                let now = self.clock.now_epoch_ms();
                self.ventopay_months
                    .lock()
                    .unwrap()
                    .insert(key.clone(), build_ventopay_month(&key, &label, txs, now));
            }
            Err(_) => { /* non-blocking: swallow (billing §4.2 step 8) */ }
        }
        Ok(())
    }
}

/// Gourmet totals (billing §4.1 step 6).
fn build_gourmet_month(
    key: &str,
    label: &str,
    bills: Vec<Bill>,
    fetched_at: i64,
) -> GourmetMonthlyBilling {
    let mut total_gross = 0.0;
    let mut total_subsidy = 0.0;
    let mut total_discount = 0.0;
    let mut total_billing = 0.0;
    for b in &bills {
        for it in &b.items {
            total_gross += it.total;
            total_subsidy += it.subsidy;
            total_discount += it.discount_value;
        }
        total_billing += b.billing;
    }
    GourmetMonthlyBilling {
        month_key: key.to_string(),
        label: label.to_string(),
        bills,
        total_gross,
        total_subsidy,
        total_discount,
        total_billing,
        fetched_at,
    }
}

/// Ventopay total = Σ amount (billing §4.2 step 5).
fn build_ventopay_month(
    key: &str,
    label: &str,
    transactions: Vec<VentopayTransaction>,
    fetched_at: i64,
) -> VentopayMonthlyBilling {
    let total = transactions.iter().map(|t| t.amount).sum();
    VentopayMonthlyBilling {
        month_key: key.to_string(),
        label: label.to_string(),
        transactions,
        total,
        fetched_at,
    }
}

/// "YYYY-MM" → (first-day-key, last-day-key) as "YYYY-MM-DD" (billing §2.2).
fn month_range(month_key: &str) -> (String, String) {
    let (y, m) = parse_key(month_key);
    let first = format!("{y:04}-{m:02}-01");
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let last_day = chrono::NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| {
            use chrono::Datelike;
            d.day()
        })
        .unwrap_or(28);
    (first, format!("{y:04}-{m:02}-{last_day:02}"))
}

fn parse_key(key: &str) -> (i32, u32) {
    let mut it = key.split('-');
    let y = it.next().and_then(|s| s.parse().ok()).unwrap_or(1970);
    let m = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    (y, m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::{FixedClock, SystemClock};
    use crate::domain::Credentials;
    use crate::gourmet::api::GourmetApi;
    use crate::http::{CapturingTransport, HttpResponse};
    use crate::storage::MemoryKv;
    use crate::ventopay::api::VentopayApi;
    use crate::ventopay::provider::VentopayProvider;
    use chrono::TimeZone;

    /// Wrap live APIs in providers (the store now takes providers).
    fn gp(api: Arc<GourmetApi>) -> Arc<GourmetProvider> {
        Arc::new(GourmetProvider::new(api, Arc::new(SystemClock)))
    }
    fn vp(api: Arc<VentopayApi>) -> Arc<VentopayProvider> {
        Arc::new(VentopayProvider::new(api, Arc::new(SystemClock)))
    }

    const G_LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const G_LOGIN_OK: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    fn feb_2026_clock() -> Arc<FixedClock> {
        let ms = chrono::Local
            .with_ymd_and_hms(2026, 2, 10, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        Arc::new(FixedClock { epoch_ms: ms })
    }

    async fn logged_in_gourmet(t: &Arc<CapturingTransport>) -> Arc<GourmetProvider> {
        t.queue_response(ok(G_LOGIN_PAGE));
        t.queue_response(ok(G_LOGIN_OK));
        let api = Arc::new(GourmetApi::new(t.clone()));
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        gp(api)
    }

    #[tokio::test]
    async fn month_options_are_three_with_austrian_labels() {
        let t = Arc::new(CapturingTransport::new());
        let store = BillingStore::new(
            gp(Arc::new(GourmetApi::new(t.clone()))),
            vp(Arc::new(VentopayApi::new(t.clone()))),
            Arc::new(MemoryKv::new()),
            feb_2026_clock(),
        );
        let opts = store.month_options();
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[0].key, "2026-02");
        assert_eq!(opts[0].label, "Februar 2026");
        assert_eq!(opts[1].key, "2026-01");
        assert_eq!(opts[2].key, "2025-12");
    }

    #[tokio::test]
    async fn fetch_billing_computes_totals_and_caches() {
        let t = Arc::new(CapturingTransport::new());
        let gourmet = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        let store = BillingStore::new(
            gourmet,
            vp(Arc::new(VentopayApi::new(t.clone()))),
            kv.clone(),
            feb_2026_clock(),
        );
        // getBillings: probe GET /start/ (authenticated) + POST wrapper response.
        t.queue_response(ok(G_LOGIN_OK)); // probe
        t.queue_response(ok(
            r#"{"Billings":[{"BillNr":1,"BillDate":"2026-02-10T12:00:00","Location":"Wien","Billing":4.5,"BillingItemInfo":[{"Id":"i","ArticleId":"a","Count":1,"Description":"x","Total":5.5,"Subsidy":2.5,"DiscountValue":0.0,"IsCustomMenu":false}]}]}"#,
        ));
        store.fetch_billing(0, false).await.unwrap();

        let m = store.gourmet_month("2026-02").unwrap();
        assert_eq!(m.bills.len(), 1);
        assert!((m.total_gross - 5.5).abs() < 1e-9);
        assert!((m.total_subsidy - 2.5).abs() < 1e-9);
        assert!((m.total_billing - 4.5).abs() < 1e-9);
        assert!(store.error().is_none());
        // cached
        assert!(cache::load_gourmet_billing(kv.as_ref(), "2026-02").is_some());
    }

    #[tokio::test]
    async fn ventopay_fetch_skipped_when_not_authenticated() {
        let t = Arc::new(CapturingTransport::new());
        let store = BillingStore::new(
            gp(Arc::new(GourmetApi::new(t.clone()))),
            vp(Arc::new(VentopayApi::new(t.clone()))), // never logged in
            Arc::new(MemoryKv::new()),
            feb_2026_clock(),
        );
        store.fetch_ventopay_billing(0, false).await.unwrap();
        assert!(store.ventopay_month("2026-02").is_none());
        assert_eq!(t.requests().len(), 0); // nothing sent
    }

    #[tokio::test]
    async fn past_month_with_data_is_not_refetched() {
        let t = Arc::new(CapturingTransport::new());
        let gourmet = logged_in_gourmet(&t).await;
        let store = BillingStore::new(
            gourmet,
            vp(Arc::new(VentopayApi::new(t.clone()))),
            Arc::new(MemoryKv::new()),
            feb_2026_clock(),
        );
        // seed offset-1 (2026-01) with a non-empty entry
        t.queue_response(ok(G_LOGIN_OK)); // probe
        t.queue_response(ok(
            r#"{"Billings":[{"BillNr":1,"BillDate":"2026-01-10T12:00:00","Location":"W","Billing":1.0,"BillingItemInfo":[]}]}"#,
        ));
        store.fetch_billing(1, false).await.unwrap();
        let count_after_first = t.requests().len();
        // second fetch of the same past month must NOT hit the network again.
        store.fetch_billing(1, false).await.unwrap();
        assert_eq!(t.requests().len(), count_after_first);
    }
}
