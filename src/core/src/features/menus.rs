//! Menus feature service (03-features/menus.md). Menu state, composite keys, TTL fetch,
//! availability-merge refresh, pending-order/cancellation toggles, the ordering cutoff, and
//! the full `submit_orders` pipeline (§6.5: resolve cancellations, cutoff-filter, optimistic
//! update, cancel→addToCart→confirm→refresh, analytics, revert on failure).
use crate::datetime::{is_ordering_cutoff, local_date_key, Clock};
use crate::domain::{MenuItem, MenuSnapshot, OrderProgress, OrderedMenu};
use crate::error::CoreResult;
use crate::features::{AnalyticsSink, ProgressListener};
use crate::gourmet::api::GourmetApi;
use crate::storage::{cache, Kv};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 4-hour menu cache validity (03-features/caching §3.1; v1 constants.ts:9).
const MENU_CACHE_VALIDITY_MS: i64 = 4 * 60 * 60 * 1000;
/// Minimum "Aktualisiere…" banner visibility for the availability refresh (§3.3 step 5).
const DEFAULT_MIN_REFRESH_MS: u64 = 800;
/// Exact cutoff error (menus.md §6.2; v1 menuStore.ts:11).
const ORDERING_CUTOFF_MESSAGE: &str = "Bestellung für heute geschlossen (Bestellschluss 9:00)";
/// Submit-failure fallback (menus.md §6.5 step 11).
const SUBMIT_FAILED_MESSAGE: &str = "Bestellung konnte nicht aufgegeben werden";

pub struct MenuStore {
    gourmet: Arc<GourmetApi>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    inner: Mutex<State>,
    /// Configurable so tests can set 0 (§3.3 min-visibility timer lives in the core).
    min_refresh_ms: u64,
    analytics: Option<Arc<dyn AnalyticsSink>>,
}

#[derive(Default)]
struct State {
    items: Vec<MenuItem>,
    last_fetched: Option<i64>,
    loading: bool,
    refreshing: bool,
    error: Option<String>,
    selected_date: Option<String>, // "YYYY-MM-DD" key
    pending_orders: Vec<String>,   // composite keys, insertion order
    pending_cancellations: Vec<String>,
    order_progress: Option<OrderProgress>,
}

impl MenuStore {
    pub fn new(gourmet: Arc<GourmetApi>, kv: Arc<dyn Kv>, clock: Arc<dyn Clock>) -> Self {
        Self {
            gourmet,
            kv,
            clock,
            inner: Mutex::new(State::default()),
            min_refresh_ms: DEFAULT_MIN_REFRESH_MS,
            analytics: None,
        }
    }

    /// Attach the analytics sink (order.submitted is emitted from `submit_orders`).
    pub fn with_analytics(mut self, analytics: Arc<dyn AnalyticsSink>) -> Self {
        self.analytics = Some(analytics);
        self
    }

    pub fn order_progress(&self) -> Option<OrderProgress> {
        self.inner.lock().unwrap().order_progress
    }

    #[cfg(test)]
    fn with_min_refresh(mut self, ms: u64) -> Self {
        self.min_refresh_ms = ms;
        self
    }

    /// Composite item key "{id}|{day-key}" (menus.md §1). `day` is already a "YYYY-MM-DD" key.
    fn composite_key(item: &MenuItem) -> String {
        format!("{}|{}", item.id, item.day)
    }

    /// Unique available date keys, sorted ascending (§2 getAvailableDates).
    fn available_dates(items: &[MenuItem]) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        for it in items {
            seen.insert(it.day.clone());
        }
        seen.into_iter().collect()
    }

    pub fn snapshot(&self) -> MenuSnapshot {
        let s = self.inner.lock().unwrap();
        MenuSnapshot {
            items: s.items.clone(),
            available_dates: Self::available_dates(&s.items),
            pending_orders: s.pending_orders.clone(),
            pending_cancellations: s.pending_cancellations.clone(),
            loading: s.loading,
            refreshing: s.refreshing,
            error: s.error.clone(),
        }
    }

    pub fn selected_date(&self) -> Option<String> {
        self.inner.lock().unwrap().selected_date.clone()
    }

    pub fn set_selected_date(&self, date_key: String) {
        self.inner.lock().unwrap().selected_date = Some(date_key);
    }

    /// Load `menus_items` from cache; absent/corrupt → no change; does NOT set last_fetched.
    pub fn load_cached_menus(&self) {
        if let Some(items) = cache::load_menus(self.kv.as_ref()) {
            self.inner.lock().unwrap().items = items;
        }
    }

    /// §3.2 — full fetch. Re-entrancy guard + 4h TTL unless `force`.
    pub async fn fetch_menus(&self, force: bool) -> CoreResult<()> {
        {
            let s = self.inner.lock().unwrap();
            if s.loading {
                return Ok(());
            }
            if !force {
                if let Some(lf) = s.last_fetched {
                    if self.clock.now_epoch_ms() - lf < MENU_CACHE_VALIDITY_MS {
                        return Ok(());
                    }
                }
            }
        }
        self.inner.lock().unwrap().loading = true;
        self.inner.lock().unwrap().error = None;

        match self.gourmet.get_menus().await {
            Ok(items) => {
                let now = self.clock.now_epoch_ms();
                {
                    let mut s = self.inner.lock().unwrap();
                    s.items = items.clone();
                    s.last_fetched = Some(now);
                    s.loading = false;
                    // §3.2 step 6: auto-select nearest date if the current one has no menus.
                    let dates = Self::available_dates(&s.items);
                    let target = s
                        .selected_date
                        .clone()
                        .unwrap_or_else(|| local_date_key(now));
                    if !dates.contains(&target) {
                        s.selected_date = find_nearest_date(&dates, &target);
                    }
                }
                let _ = cache::save_menus(self.kv.as_ref(), &items);
                Ok(())
            }
            Err(e) => {
                let mut s = self.inner.lock().unwrap();
                s.error = Some(match e.to_string().as_str() {
                    "" => "Menüs konnten nicht geladen werden".to_string(),
                    m => m.to_string(),
                });
                s.loading = false;
                Ok(())
            }
        }
    }

    /// §3.3 — availability-only refresh; merges volatile fields, min-visibility banner.
    pub async fn refresh_availability(&self) -> CoreResult<()> {
        {
            let mut s = self.inner.lock().unwrap();
            if s.refreshing || s.items.is_empty() {
                return Ok(());
            }
            s.refreshing = true;
        }
        let start = std::time::Instant::now();
        let fetched = self.gourmet.get_menus().await;
        // enforce the minimum banner visibility.
        let elapsed = start.elapsed().as_millis() as u64;
        if elapsed < self.min_refresh_ms {
            tokio::time::sleep(Duration::from_millis(self.min_refresh_ms - elapsed)).await;
        }
        match fetched {
            Ok(fresh) => {
                let now = self.clock.now_epoch_ms();
                let merged = {
                    let s = self.inner.lock().unwrap();
                    merge_availability(&s.items, fresh)
                };
                {
                    let mut s = self.inner.lock().unwrap();
                    s.items = merged.clone();
                    s.last_fetched = Some(now);
                    s.refreshing = false;
                }
                let _ = cache::save_menus(self.kv.as_ref(), &merged);
                Ok(())
            }
            Err(_) => {
                // silent failure (§3.3 step 6): only clear the refreshing flag.
                self.inner.lock().unwrap().refreshing = false;
                Ok(())
            }
        }
    }

    /// §6.3 — toggle a menu's pending order/cancellation by (menu_id, date_key).
    /// If the item is currently ordered → toggles a cancellation; else toggles a new order.
    pub fn toggle_pending(&self, menu_id: String, date_key: String) -> MenuSnapshot {
        let key = format!("{menu_id}|{date_key}");
        {
            let mut s = self.inner.lock().unwrap();
            let is_ordered = s
                .items
                .iter()
                .any(|it| it.id == menu_id && it.day == date_key && it.ordered);
            let set = if is_ordered {
                &mut s.pending_cancellations
            } else {
                &mut s.pending_orders
            };
            if let Some(pos) = set.iter().position(|k| *k == key) {
                set.remove(pos);
            } else {
                set.push(key);
            }
        }
        self.snapshot()
    }

    pub fn clear_pending_changes(&self) -> MenuSnapshot {
        {
            let mut s = self.inner.lock().unwrap();
            s.pending_orders.clear();
            s.pending_cancellations.clear();
        }
        self.snapshot()
    }

    /// §6.2 — 09:00 Europe/Vienna ordering cutoff for a date key.
    pub fn is_ordering_cutoff(&self, date_key: &str) -> bool {
        is_ordering_cutoff(self.clock.as_ref(), date_key)
    }

    /// §6.5 — the full submit pipeline. `current_orders` comes from the order store and is
    /// used to resolve pending cancellations to position IDs. The order store is refreshed
    /// by the caller after this returns (v1 does it in step 8; net end state is identical).
    pub async fn submit_orders(
        &self,
        current_orders: &[OrderedMenu],
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> CoreResult<MenuSnapshot> {
        // 1. no-op if nothing pending.
        let (pending_orders, pending_cancellations, items) = {
            let s = self.inner.lock().unwrap();
            (
                s.pending_orders.clone(),
                s.pending_cancellations.clone(),
                s.items.clone(),
            )
        };
        if pending_orders.is_empty() && pending_cancellations.is_empty() {
            return Ok(self.snapshot());
        }

        // 2. resolve cancellations → position IDs (unresolvable are skipped).
        let mut cancel_ids = Vec::new();
        for key in &pending_cancellations {
            let (menu_id, date_str) = split_key(key);
            let Some(item) = items
                .iter()
                .find(|it| it.id == menu_id && it.day == date_str)
            else {
                continue;
            };
            let category = item.category.display();
            if let Some(order) = current_orders
                .iter()
                .find(|o| o.title == category && local_date_key(o.date_epoch_ms) == date_str)
            {
                cancel_ids.push(order.position_id.clone());
            }
        }

        // 3. resolve new orders + 4. cutoff filter.
        let mut allowed: Vec<(String, String)> = Vec::new();
        let mut has_cutoff_blocked = false;
        for key in &pending_orders {
            let (menu_id, date_str) = split_key(key);
            if self.is_ordering_cutoff(&date_str) {
                has_cutoff_blocked = true;
            } else {
                allowed.push((menu_id, date_str));
            }
        }
        if has_cutoff_blocked && allowed.is_empty() && cancel_ids.is_empty() {
            // everything blocked, nothing to do → error, pending NOT cleared.
            self.inner.lock().unwrap().error = Some(ORDERING_CUTOFF_MESSAGE.to_string());
            return Ok(self.snapshot());
        }

        // 5. optimistic update + clear pending.
        {
            let mut s = self.inner.lock().unwrap();
            for it in s.items.iter_mut() {
                let k = format!("{}|{}", it.id, it.day);
                if pending_cancellations.contains(&k) {
                    it.ordered = false;
                }
                if pending_orders.contains(&k) {
                    it.ordered = true;
                }
            }
            s.pending_orders.clear();
            s.pending_cancellations.clear();
            s.error = has_cutoff_blocked.then(|| ORDERING_CUTOFF_MESSAGE.to_string());
        }

        // 6-8. cancel → add+confirm → refresh. Only failures in 6-7 propagate (fetch_menus
        // swallows its own errors).
        if let Err(e) = self
            .run_submit_pipeline(&cancel_ids, &allowed, &progress)
            .await
        {
            // 11. revert on failure.
            self.set_progress(None, &progress);
            self.inner.lock().unwrap().error = Some(match e.to_string().as_str() {
                "" => SUBMIT_FAILED_MESSAGE.to_string(),
                m => m.to_string(),
            });
            if let Ok(fresh) = self.gourmet.get_menus().await {
                let now = self.clock.now_epoch_ms();
                {
                    let mut s = self.inner.lock().unwrap();
                    s.items = fresh.clone();
                    s.last_fetched = Some(now);
                }
                let _ = cache::save_menus(self.kv.as_ref(), &fresh);
            }
            return Ok(self.snapshot());
        }

        // 9. analytics.
        if let Some(a) = &self.analytics {
            a.track(
                "order.submitted",
                vec![
                    ("orderedCount".to_string(), allowed.len().to_string()),
                    ("cancelledCount".to_string(), cancel_ids.len().to_string()),
                ],
            );
        }

        // 10. finish: clear progress; error = cutoff msg if anything was blocked, else None.
        self.set_progress(None, &progress);
        self.inner.lock().unwrap().error =
            has_cutoff_blocked.then(|| ORDERING_CUTOFF_MESSAGE.to_string());
        Ok(self.snapshot())
    }

    async fn run_submit_pipeline(
        &self,
        cancel_ids: &[String],
        allowed: &[(String, String)],
        progress: &Option<Arc<dyn ProgressListener>>,
    ) -> CoreResult<()> {
        if !cancel_ids.is_empty() {
            self.set_progress(Some(OrderProgress::Cancelling), progress);
            self.gourmet.cancel_orders(cancel_ids.to_vec()).await?;
        }
        if !allowed.is_empty() {
            self.set_progress(Some(OrderProgress::Adding), progress);
            self.gourmet.add_to_cart(allowed.to_vec()).await?;
            self.set_progress(Some(OrderProgress::Confirming), progress);
            self.gourmet.confirm_orders().await?;
        }
        self.set_progress(Some(OrderProgress::Refreshing), progress);
        self.fetch_menus(true).await?; // swallows its own errors → never throws here
        Ok(())
    }

    fn set_progress(
        &self,
        phase: Option<OrderProgress>,
        progress: &Option<Arc<dyn ProgressListener>>,
    ) {
        self.inner.lock().unwrap().order_progress = phase;
        if let Some(p) = progress {
            p.on_progress(phase);
        }
    }
}

/// Split a composite key "menuId|YYYY-MM-DD" into its two parts.
fn split_key(key: &str) -> (String, String) {
    match key.split_once('|') {
        Some((id, date)) => (id.to_string(), date.to_string()),
        None => (key.to_string(), String::new()),
    }
}

/// §3.2 step 6 — closest date on-or-after target, else closest before, else None.
/// `dates` is sorted ascending.
fn find_nearest_date(dates: &[String], target: &str) -> Option<String> {
    if dates.is_empty() {
        return None;
    }
    if let Some(on_or_after) = dates.iter().find(|d| d.as_str() >= target) {
        return Some(on_or_after.clone());
    }
    dates.last().cloned()
}

/// §3.3 step 4 — merge volatile fields by composite key with v1's key-collision quirk.
fn merge_availability(cached: &[MenuItem], fresh: Vec<MenuItem>) -> Vec<MenuItem> {
    // fresh map: last fresh item per key wins (Map.set overwrites).
    let mut fresh_map: HashMap<String, MenuItem> = HashMap::new();
    for it in fresh {
        fresh_map.insert(MenuStore::composite_key(&it), it);
    }
    let mut out = Vec::with_capacity(cached.len());
    for item in cached {
        let key = MenuStore::composite_key(item);
        // only the FIRST cached item per key consumes the fresh entry (then it's removed).
        if let Some(fresh_it) = fresh_map.remove(&key) {
            let mut merged = item.clone();
            merged.available = fresh_it.available;
            merged.ordered = fresh_it.ordered;
            out.push(merged);
        } else {
            out.push(item.clone()); // later duplicates / no fresh counterpart: keep as-is
        }
    }
    // append remaining fresh items (at most one per key survived).
    for (_k, it) in fresh_map {
        out.push(it);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::{FixedClock, SystemClock};
    use crate::domain::{Credentials, MenuCategory};
    use crate::http::{CapturingTransport, HttpResponse};
    use crate::storage::MemoryKv;

    const G_LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const G_LOGIN_OK: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const MENUS_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
    const MENUS_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");
    const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }

    fn item(id: &str, day: &str, available: bool, ordered: bool) -> MenuItem {
        MenuItem {
            id: id.into(),
            day: day.into(),
            title: "MENÜ I".into(),
            subtitle: "s".into(),
            allergens: vec![],
            available,
            ordered,
            category: MenuCategory::Menu1,
            price: String::new(),
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
    async fn fetch_menus_populates_and_caches_and_sets_last_fetched() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let kv = Arc::new(MemoryKv::new());
        let store = MenuStore::new(g, kv.clone(), Arc::new(SystemClock));
        t.queue_response(ok(MENUS_0));
        t.queue_response(ok(MENUS_1));
        store.fetch_menus(false).await.unwrap();
        let snap = store.snapshot();
        assert!(!snap.items.is_empty());
        assert!(cache::load_menus(kv.as_ref()).is_some());
        // second fetch within 4h TTL is a no-op (no new network requests).
        let n = t.requests().len();
        store.fetch_menus(false).await.unwrap();
        assert_eq!(t.requests().len(), n);
    }

    #[tokio::test]
    async fn toggle_pending_moves_between_orders_and_cancellations() {
        let t = Arc::new(CapturingTransport::new());
        let g = Arc::new(GourmetApi::new(t.clone()));
        let store = MenuStore::new(g, Arc::new(MemoryKv::new()), Arc::new(SystemClock));
        store.inner.lock().unwrap().items = vec![
            item("menu-001", "2026-02-10", true, false), // not ordered → toggling adds a new order
            item("menu-002", "2026-02-10", true, true),  // ordered → toggling adds a cancellation
        ];
        let snap = store.toggle_pending("menu-001".into(), "2026-02-10".into());
        assert_eq!(snap.pending_orders, ["menu-001|2026-02-10"]);
        let snap = store.toggle_pending("menu-002".into(), "2026-02-10".into());
        assert_eq!(snap.pending_cancellations, ["menu-002|2026-02-10"]);
        // toggling menu-001 again removes it.
        let snap = store.toggle_pending("menu-001".into(), "2026-02-10".into());
        assert!(snap.pending_orders.is_empty());
    }

    #[test]
    fn merge_availability_updates_first_and_appends_new() {
        let cached = vec![
            item("a", "2026-02-10", false, false),
            item("a", "2026-02-11", false, false),
        ];
        let fresh = vec![
            item("a", "2026-02-10", true, true),  // updates the first
            item("b", "2026-02-10", true, false), // brand new → appended
        ];
        let merged = merge_availability(&cached, fresh);
        let first = merged
            .iter()
            .find(|i| i.id == "a" && i.day == "2026-02-10")
            .unwrap();
        assert!(first.available && first.ordered); // volatile fields updated
        assert!(merged.iter().any(|i| i.id == "b")); // new appended
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn find_nearest_prefers_on_or_after() {
        let dates = vec!["2026-02-09".to_string(), "2026-02-11".to_string()];
        assert_eq!(
            find_nearest_date(&dates, "2026-02-10").as_deref(),
            Some("2026-02-11")
        );
        assert_eq!(
            find_nearest_date(&dates, "2026-02-12").as_deref(),
            Some("2026-02-11")
        );
        assert_eq!(find_nearest_date(&[], "x"), None);
    }

    #[tokio::test]
    async fn cutoff_delegates_to_datetime() {
        let ms = chrono_tz::Europe::Vienna
            .with_ymd_and_hms(2026, 2, 10, 10, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        use chrono::TimeZone;
        let store = MenuStore::new(
            Arc::new(GourmetApi::new(Arc::new(CapturingTransport::new()))),
            Arc::new(MemoryKv::new()),
            Arc::new(FixedClock { epoch_ms: ms }),
        );
        assert!(store.is_ordering_cutoff("2026-02-10")); // today after 09:00
        assert!(!store.is_ordering_cutoff("2026-02-11")); // future
    }

    #[tokio::test]
    async fn refresh_availability_merges_with_zero_min_visibility() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let store =
            MenuStore::new(g, Arc::new(MemoryKv::new()), Arc::new(SystemClock)).with_min_refresh(0);
        // seed cached items so refresh (not full fetch) runs.
        store.inner.lock().unwrap().items = vec![item("menu-001", "2026-02-10", false, false)];
        t.queue_response(ok(MENUS_0));
        t.queue_response(ok(MENUS_1));
        store.refresh_availability().await.unwrap();
        assert!(!store.snapshot().refreshing);
        assert!(!store.snapshot().items.is_empty());
    }

    // ---- submit_orders (§6.5) ----

    use crate::features::{AnalyticsSink, ProgressListener};
    use std::sync::Mutex as StdMutex;

    type RecordedEvent = (String, Vec<(String, String)>);

    #[derive(Default)]
    struct RecordingAnalytics {
        events: StdMutex<Vec<RecordedEvent>>,
    }
    impl AnalyticsSink for RecordingAnalytics {
        fn track(&self, event: &str, props: Vec<(String, String)>) {
            self.events.lock().unwrap().push((event.to_string(), props));
        }
    }

    struct NoopProgress;
    impl ProgressListener for NoopProgress {
        fn on_progress(&self, _phase: Option<crate::domain::OrderProgress>) {}
    }

    fn vienna_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        use chrono::TimeZone;
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[tokio::test]
    async fn submit_no_op_when_nothing_pending() {
        let t = Arc::new(CapturingTransport::new());
        let store = MenuStore::new(
            Arc::new(GourmetApi::new(t.clone())),
            Arc::new(MemoryKv::new()),
            Arc::new(SystemClock),
        );
        store.submit_orders(&[], None).await.unwrap();
        assert_eq!(t.requests().len(), 0);
    }

    #[tokio::test]
    async fn submit_all_cutoff_blocked_sets_error_and_keeps_pending() {
        let t = Arc::new(CapturingTransport::new());
        let g = Arc::new(GourmetApi::new(t.clone()));
        let store = MenuStore::new(
            g,
            Arc::new(MemoryKv::new()),
            Arc::new(FixedClock {
                epoch_ms: vienna_ms(2026, 2, 10, 10, 0),
            }),
        );
        store.inner.lock().unwrap().pending_orders = vec!["menu-001|2026-02-10".into()];
        let snap = store.submit_orders(&[], None).await.unwrap();
        assert_eq!(snap.error.as_deref(), Some(ORDERING_CUTOFF_MESSAGE));
        assert_eq!(snap.pending_orders, ["menu-001|2026-02-10"]); // NOT cleared
        assert_eq!(t.requests().len(), 0);
    }

    #[tokio::test]
    async fn submit_new_order_adds_confirms_refreshes_and_emits_analytics() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let analytics = Arc::new(RecordingAnalytics::default());
        let store = MenuStore::new(
            g,
            Arc::new(MemoryKv::new()),
            Arc::new(FixedClock {
                epoch_ms: vienna_ms(2026, 2, 10, 8, 0),
            }),
        )
        .with_analytics(analytics.clone());
        store.inner.lock().unwrap().pending_orders = vec!["menu-001|2026-02-15".into()];

        t.queue_response(ok(r#"{"success":true}"#)); // addToCart
        t.queue_response(ok(ORDERS_PAGE)); // confirm GET (editMode="True" → no toggle)
        t.queue_response(ok(MENUS_0));
        t.queue_response(ok(MENUS_1));

        let snap = store
            .submit_orders(&[], Some(Arc::new(NoopProgress)))
            .await
            .unwrap();
        assert!(snap.pending_orders.is_empty());
        assert!(snap.error.is_none());
        assert_eq!(store.order_progress(), None);

        let events = analytics.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "order.submitted");
        let props: HashMap<_, _> = events[0].1.iter().cloned().collect();
        assert_eq!(props.get("orderedCount").map(|s| s.as_str()), Some("1"));
        assert_eq!(props.get("cancelledCount").map(|s| s.as_str()), Some("0"));
    }

    #[tokio::test]
    async fn submit_failure_reverts_and_sets_error() {
        let t = Arc::new(CapturingTransport::new());
        let g = logged_in_gourmet(&t).await;
        let store = MenuStore::new(
            g,
            Arc::new(MemoryKv::new()),
            Arc::new(FixedClock {
                epoch_ms: vienna_ms(2026, 2, 10, 8, 0),
            }),
        );
        store.inner.lock().unwrap().items = vec![item("menu-001", "2026-02-15", true, false)];
        store.inner.lock().unwrap().pending_orders = vec!["menu-001|2026-02-15".into()];

        t.queue_response(ok(r#"{"success":false,"message":"nope"}"#)); // addToCart fails
        t.queue_response(ok(MENUS_0)); // revert get_menus page 0
        t.queue_response(ok(MENUS_1)); // revert get_menus page 1

        let snap = store.submit_orders(&[], None).await.unwrap();
        assert_eq!(snap.error.as_deref(), Some("Add to cart failed: nope"));
        assert!(snap.pending_orders.is_empty()); // cleared during optimistic update
        assert_eq!(store.order_progress(), None);
    }
}
