//! Orders feature service (03-features/orders.md §8). Fetch/confirm/cancel orchestration,
//! upcoming/past split at device-local midnight, cache write-through.
//!
//! NOTE: the post-fetch notification hooks (geofence-cancel, daily-reminder, cancel-reminder)
//! are `notify::*` decisions wired in Phase 1e; `fetch_orders` currently performs the
//! fetch + cache only. The hook integration point is marked below.
use crate::datetime::Clock;
use crate::domain::{OrderedMenu, OrdersSplit};
use crate::error::CoreResult;
use crate::gourmet::api::GourmetApi;
use crate::storage::{cache, Kv};
use chrono::{Local, TimeZone};
use std::sync::{Arc, Mutex};

pub struct OrderStore {
    gourmet: Arc<GourmetApi>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    orders: Mutex<Vec<OrderedMenu>>,
    loading: Mutex<bool>,
    cancelling_id: Mutex<Option<String>>,
    error: Mutex<Option<String>>,
}

impl OrderStore {
    pub fn new(gourmet: Arc<GourmetApi>, kv: Arc<dyn Kv>, clock: Arc<dyn Clock>) -> Self {
        Self {
            gourmet,
            kv,
            clock,
            orders: Mutex::new(Vec::new()),
            loading: Mutex::new(false),
            cancelling_id: Mutex::new(None),
            error: Mutex::new(None),
        }
    }

    pub fn orders(&self) -> Vec<OrderedMenu> {
        self.orders.lock().unwrap().clone()
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
    pub fn cancelling_id(&self) -> Option<String> {
        self.cancelling_id.lock().unwrap().clone()
    }

    /// Load `orders_list` from cache into memory; absent/corrupt → no change (caching §3.4).
    pub fn load_cached_orders(&self) {
        if let Some(orders) = cache::load_orders(self.kv.as_ref()) {
            *self.orders.lock().unwrap() = orders;
        }
    }

    /// §8 — always a network fetch; in-flight guard; cache write-through; on failure keep
    /// existing orders and set `error`.
    pub async fn fetch_orders(&self) -> CoreResult<()> {
        if *self.loading.lock().unwrap() {
            return Ok(()); // in-flight guard
        }
        *self.loading.lock().unwrap() = true;
        match self.gourmet.get_orders().await {
            Ok(orders) => {
                *self.orders.lock().unwrap() = orders.clone();
                *self.loading.lock().unwrap() = false;
                let _ = cache::save_orders(self.kv.as_ref(), &orders);
                // NOTE (Phase 1e): run notify hooks here (geofence-cancel / daily / cancel
                // reminder) inside one swallow-all block; they must not affect orders/error.
                Ok(())
            }
            Err(e) => {
                let msg = match e.to_string().as_str() {
                    "" => "Bestellungen konnten nicht geladen werden".to_string(),
                    s => s.to_string(),
                };
                *self.error.lock().unwrap() = Some(msg);
                *self.loading.lock().unwrap() = false;
                Ok(()) // failure captured in `error`; existing orders kept
            }
        }
    }

    /// §5.3 — confirm: clear error, call the API confirm, re-fetch.
    pub async fn confirm_orders(&self) -> CoreResult<()> {
        *self.error.lock().unwrap() = None;
        if let Err(e) = self.gourmet.confirm_orders().await {
            *self.error.lock().unwrap() = Some(match e.to_string().as_str() {
                "" => "Bestellungen konnten nicht bestätigt werden".to_string(),
                s => s.to_string(),
            });
            return Ok(());
        }
        self.fetch_orders().await
    }

    /// §6.2 — cancel one position: mark cancelling, call the API, re-fetch, clear marker.
    pub async fn cancel_order(&self, position_id: String) -> CoreResult<()> {
        *self.cancelling_id.lock().unwrap() = Some(position_id.clone());
        *self.error.lock().unwrap() = None;
        let result = self.gourmet.cancel_orders(vec![position_id]).await;
        *self.cancelling_id.lock().unwrap() = None;
        match result {
            Ok(()) => self.fetch_orders().await,
            Err(e) => {
                *self.error.lock().unwrap() = Some(match e.to_string().as_str() {
                    "" => "Bestellung konnte nicht storniert werden".to_string(),
                    s => s.to_string(),
                });
                Ok(())
            }
        }
    }

    /// §7 — upcoming (`date >= today`) / past (`date < today`) at device-local midnight.
    pub fn split(&self) -> OrdersSplit {
        let midnight = local_midnight_ms(self.clock.now_epoch_ms());
        let orders = self.orders.lock().unwrap();
        let (upcoming, past): (Vec<_>, Vec<_>) = orders
            .iter()
            .cloned()
            .partition(|o| o.date_epoch_ms >= midnight);
        OrdersSplit { upcoming, past }
    }
}

/// Device-local start-of-today epoch ms.
fn local_midnight_ms(now_epoch_ms: i64) -> i64 {
    let dt = Local
        .timestamp_millis_opt(now_epoch_ms)
        .single()
        .expect("valid epoch");
    let midnight = dt.date_naive().and_hms_opt(0, 0, 0).unwrap();
    Local
        .from_local_datetime(&midnight)
        .single()
        .expect("valid local midnight")
        .timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::{FixedClock, SystemClock};
    use crate::domain::Credentials;
    use crate::http::{CapturingTransport, HttpResponse};
    use crate::storage::MemoryKv;

    const G_LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const G_LOGIN_OK: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    async fn logged_in_gourmet(t: &Arc<CapturingTransport>) -> Arc<GourmetApi> {
        t.queue_response(ok(G_LOGIN_PAGE));
        t.queue_response(ok(G_LOGIN_OK));
        let api = Arc::new(GourmetApi::new(t.clone()));
        api.login(Credentials {
            username: "u".into(),
            password: "p".into(),
        })
        .await
        .unwrap();
        api
    }

    #[tokio::test]
    async fn fetch_orders_parses_and_caches() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        let store = OrderStore::new(g, kv.clone(), Arc::new(SystemClock));
        t.queue_response(ok(ORDERS_PAGE));
        store.fetch_orders().await.unwrap();
        assert!(!store.orders().is_empty());
        assert!(store.error().is_none());
        assert!(cache::load_orders(kv.as_ref()).is_some());
    }

    #[tokio::test]
    async fn fetch_failure_keeps_orders_and_sets_error() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        // seed cached orders
        let seeded = vec![OrderedMenu {
            position_id: "P1".into(),
            eating_cycle_id: String::new(),
            date_epoch_ms: 0,
            title: "MENÜ I".into(),
            subtitle: String::new(),
            approved: true,
        }];
        cache::save_orders(kv.as_ref(), &seeded).unwrap();
        let store = OrderStore::new(g, kv.clone(), Arc::new(SystemClock));
        store.load_cached_orders();
        assert_eq!(store.orders().len(), 1);
        // no queued response → the GET errors → error set, orders kept.
        store.fetch_orders().await.unwrap();
        assert!(store.error().is_some());
        assert_eq!(store.orders().len(), 1);
    }

    #[tokio::test]
    async fn split_upcoming_and_past_at_local_midnight() {
        let t = Arc::new(CapturingTransport::new());
        let g = Arc::new(GourmetApi::new(t.clone()));
        // fix clock to 2026-02-10 12:00 local
        let midnight = local_midnight_ms(
            chrono::Local
                .with_ymd_and_hms(2026, 2, 10, 12, 0, 0)
                .single()
                .unwrap()
                .timestamp_millis(),
        );
        let clock = Arc::new(FixedClock {
            epoch_ms: midnight + 12 * 3_600_000,
        });
        let store = OrderStore::new(g, Arc::new(MemoryKv::new()), clock);
        *store.orders.lock().unwrap() = vec![
            OrderedMenu {
                position_id: "future".into(),
                eating_cycle_id: String::new(),
                date_epoch_ms: midnight + 24 * 3_600_000, // tomorrow
                title: "x".into(),
                subtitle: String::new(),
                approved: false,
            },
            OrderedMenu {
                position_id: "yesterday".into(),
                eating_cycle_id: String::new(),
                date_epoch_ms: midnight - 3_600_000, // before today
                title: "y".into(),
                subtitle: String::new(),
                approved: true,
            },
            OrderedMenu {
                position_id: "today".into(),
                eating_cycle_id: String::new(),
                date_epoch_ms: midnight, // exactly today → upcoming
                title: "z".into(),
                subtitle: String::new(),
                approved: false,
            },
        ];
        let split = store.split();
        let up: Vec<_> = split
            .upcoming
            .iter()
            .map(|o| o.position_id.as_str())
            .collect();
        let past: Vec<_> = split.past.iter().map(|o| o.position_id.as_str()).collect();
        assert!(up.contains(&"future") && up.contains(&"today"));
        assert_eq!(past, ["yesterday"]);
    }
}
