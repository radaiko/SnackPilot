//! Orders feature service (03-features/orders.md §8). Fetch/confirm/cancel orchestration,
//! upcoming/past split at device-local midnight, cache write-through.
//!
//! The post-fetch notification hooks (daily reminder / cancel reminder) run inside
//! `fetch_orders` after a successful fetch (Phase 1e; notifications-daily-reminder §2 trigger 2,
//! notifications-location §5 retraction path 2) and are returned as commands for the shell to
//! deliver — errors there never affect orders state.
use crate::datetime::Clock;
use crate::domain::{OrderedMenu, OrdersSplit};
use crate::error::CoreResult;
use crate::features::AnalyticsSink;
use crate::gourmet::provider::GourmetProvider;
use crate::notify::cancel_reminder::check_cancel_reminder;
use crate::notify::daily_reminder::check_daily_reminder;
use crate::notify::NotificationCommand;
use crate::storage::{cache, Kv};
use chrono::{Local, TimeZone};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct OrderStore {
    gourmet: Arc<GourmetProvider>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    orders: Mutex<Vec<OrderedMenu>>,
    loading: Mutex<bool>,
    cancelling_id: Mutex<Option<String>>,
    error: Mutex<Option<String>>,
    analytics: Option<Arc<dyn AnalyticsSink>>,
}

impl OrderStore {
    pub fn new(gourmet: Arc<GourmetProvider>, kv: Arc<dyn Kv>, clock: Arc<dyn Clock>) -> Self {
        Self {
            gourmet,
            kv,
            clock,
            orders: Mutex::new(Vec::new()),
            loading: Mutex::new(false),
            cancelling_id: Mutex::new(None),
            error: Mutex::new(None),
            analytics: None,
        }
    }

    /// Attach the analytics sink (`order.cancelled` is emitted from the cancel paths).
    pub fn with_analytics(mut self, analytics: Arc<dyn AnalyticsSink>) -> Self {
        self.analytics = Some(analytics);
        self
    }

    /// Fire-and-forget `order.cancelled` with the number of positions cancelled (§6.1/§6.2).
    fn emit_cancelled(&self, count: usize) {
        if let Some(a) = &self.analytics {
            a.track(
                "order.cancelled".to_string(),
                HashMap::from([("count".to_string(), count.to_string())]),
            );
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
    /// existing orders and set `error`. After a **successful** fetch, re-run the notification
    /// checks (notifications-daily-reminder §2 trigger 2: `fetchOrders` → `checkDailyReminder`;
    /// notifications-location §5 retraction path 2) and return the commands for the shell to
    /// deliver. All errors are swallowed — a notification failure never affects orders/error.
    pub async fn fetch_orders(&self) -> CoreResult<Vec<NotificationCommand>> {
        if *self.loading.lock().unwrap() {
            return Ok(vec![]); // in-flight guard
        }
        *self.loading.lock().unwrap() = true;
        match self.gourmet.get_orders().await {
            Ok(orders) => {
                *self.orders.lock().unwrap() = orders.clone();
                *self.loading.lock().unwrap() = false;
                let _ = cache::save_orders(self.kv.as_ref(), &orders);
                // Phase 1e hooks, in one swallow-all block: they must not affect orders/error.
                Ok(self.post_fetch_notifications(&orders))
            }
            Err(e) => {
                let msg = match e.to_string().as_str() {
                    "" => "Bestellungen konnten nicht geladen werden".to_string(),
                    s => s.to_string(),
                };
                *self.error.lock().unwrap() = Some(msg);
                *self.loading.lock().unwrap() = false;
                Ok(vec![]) // failure captured in `error`; existing orders kept
            }
        }
    }

    /// The post-fetch notification decisions (daily reminder reschedule + geofence/cancel-reminder
    /// retraction). Settings come from the core KV so background callers need no shell state.
    fn post_fetch_notifications(&self, orders: &[OrderedMenu]) -> Vec<NotificationCommand> {
        let settings = cache::load_daily_reminder_settings(self.kv.as_ref());
        let at_company = cache::load_is_at_company(self.kv.as_ref());
        let mut commands = Vec::new();
        if let Some(c) = check_daily_reminder(self.clock.as_ref(), &settings, orders) {
            commands.push(c);
        }
        if let Some(c) = check_cancel_reminder(self.clock.as_ref(), at_company, orders) {
            commands.push(c);
        }
        commands
    }

    /// Headless order-sync check (§4.4): evaluate both order-driven reminders against the
    /// CURRENT cached/in-memory orders without any network call. This is what the iOS
    /// BGTaskScheduler handler and the Android WorkManager worker run in the background —
    /// it needs no shell state because the daily-reminder settings live in the core KV.
    /// Loads the cache first so a freshly-started background process sees real orders.
    pub fn run_order_sync_check(&self) -> Vec<NotificationCommand> {
        self.load_cached_orders();
        self.post_fetch_notifications(&self.orders())
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
        self.fetch_orders().await?;
        Ok(())
    }

    /// §6.2 — cancel one position: mark cancelling, call the API, re-fetch, clear marker.
    pub async fn cancel_order(&self, position_id: String) -> CoreResult<()> {
        *self.cancelling_id.lock().unwrap() = Some(position_id.clone());
        *self.error.lock().unwrap() = None;
        let result = self.gourmet.cancel_orders(vec![position_id]).await;
        *self.cancelling_id.lock().unwrap() = None;
        match result {
            Ok(()) => {
                self.emit_cancelled(1);
                self.fetch_orders().await?;
                Ok(())
            }
            Err(e) => {
                *self.error.lock().unwrap() = Some(match e.to_string().as_str() {
                    "" => "Bestellung konnte nicht storniert werden".to_string(),
                    s => s.to_string(),
                });
                Ok(())
            }
        }
    }

    /// §6.1 batch — cancel several positions in ONE edit-mode session (the "week of holiday" case).
    /// Far cheaper + safer than N single cancels (one enter/exit, not N). No-op on an empty list.
    pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()> {
        if position_ids.is_empty() {
            return Ok(());
        }
        *self.error.lock().unwrap() = None;
        let count = position_ids.len();
        let result = self.gourmet.cancel_orders(position_ids).await;
        match result {
            Ok(()) => {
                self.emit_cancelled(count);
                self.fetch_orders().await?;
                Ok(())
            }
            Err(e) => {
                *self.error.lock().unwrap() = Some(match e.to_string().as_str() {
                    "" => "Bestellungen konnten nicht storniert werden".to_string(),
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
    use crate::gourmet::api::GourmetApi;
    use crate::http::{CapturingTransport, HttpResponse};
    use crate::notify::{CANCEL_REMINDER_ID, DAILY_REMINDER_ID};
    use crate::storage::MemoryKv;

    fn vienna_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    /// Wrap a live GourmetApi in a provider (stores now take `Arc<GourmetProvider>`).
    fn gp(api: Arc<GourmetApi>) -> Arc<GourmetProvider> {
        Arc::new(GourmetProvider::new(api, Arc::new(SystemClock)))
    }

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

    /// Phase 1e wiring: a successful fetch runs the post-fetch hooks (spec
    /// notifications-daily-reminder §2 step 2 / cancel-reminder §2 step 1) and hands the
    /// commands back to the caller. Clock fixed at 08:00 Vienna on 2026-02-10, reminder
    /// at 11:00 → before target with an order today → ScheduleAt for the daily id; the
    /// cancel hook is not at company with an order today before its 08:45 target →
    /// ScheduleAt for the cancel id too. The headless run_order_sync_check reads the same
    /// cache without any network call.
    #[tokio::test]
    async fn fetch_emits_post_fetch_notification_commands() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        let clock = Arc::new(FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, 8, 0),
        });
        cache::set_daily_reminder_enabled(kv.as_ref(), true);
        cache::set_daily_reminder_time(kv.as_ref(), 11, 0);
        let store = OrderStore::new(g, kv.clone(), clock);
        t.queue_response(ok(ORDERS_PAGE));
        let cmds = store.fetch_orders().await.unwrap();
        assert!(store.error().is_none());
        assert!(cmds.iter().any(|c| matches!(
            c,
            NotificationCommand::ScheduleAt { id, .. } if id == DAILY_REMINDER_ID
        )));
        assert!(cmds.iter().any(|c| matches!(
            c,
            NotificationCommand::ScheduleAt { id, .. } if id == CANCEL_REMINDER_ID
        )));

        // Headless check: same commands from the cache alone, no further network calls
        // (the transport has nothing queued — a request would error, and `error` stays clear).
        let again = store.run_order_sync_check();
        assert!(again.iter().any(|c| matches!(
            c,
            NotificationCommand::ScheduleAt { id, .. } if id == DAILY_REMINDER_ID
        )));
        assert!(store.error().is_none());
    }

    #[tokio::test]
    async fn fetch_on_failure_emits_no_commands() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        let clock = Arc::new(FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, 11, 30),
        });
        cache::set_daily_reminder_enabled(kv.as_ref(), true);
        cache::set_daily_reminder_time(kv.as_ref(), 11, 0);
        let store = OrderStore::new(g, kv.clone(), clock);
        // no queued response → fetch fails → hooks must NOT run (spec: only after success)
        let cmds = store.fetch_orders().await.unwrap();
        assert!(cmds.is_empty());
        assert!(store.error().is_some());
    }

    #[tokio::test]
    async fn split_upcoming_and_past_at_local_midnight() {
        let t = Arc::new(CapturingTransport::new());
        let g = gp(Arc::new(GourmetApi::new(t.clone())));
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
