//! Menus feature service (03-features/menus.md). Menu state, composite keys, TTL fetch,
//! availability-merge refresh, pending-order/cancellation toggles, and the ordering cutoff.
//!
//! NOTE: `submit_orders` (§6.5 — the pipeline that calls addToCart/cancelOrders/confirm,
//! optimistic update + revert, analytics) is intentionally NOT in this file. It orchestrates
//! the ban-critical writes and is implemented in its own focused pass; see the store fields
//! (`pending_orders`, `pending_cancellations`, `order_progress`) it will use.
use crate::datetime::{is_ordering_cutoff, local_date_key, Clock};
use crate::domain::{MenuItem, MenuSnapshot};
use crate::error::CoreResult;
use crate::gourmet::api::GourmetApi;
use crate::storage::{cache, Kv};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 4-hour menu cache validity (03-features/caching §3.1; v1 constants.ts:9).
const MENU_CACHE_VALIDITY_MS: i64 = 4 * 60 * 60 * 1000;
/// Minimum "Aktualisiere…" banner visibility for the availability refresh (§3.3 step 5).
const DEFAULT_MIN_REFRESH_MS: u64 = 800;

pub struct MenuStore {
    gourmet: Arc<GourmetApi>,
    kv: Arc<dyn Kv>,
    clock: Arc<dyn Clock>,
    inner: Mutex<State>,
    /// Configurable so tests can set 0 (§3.3 min-visibility timer lives in the core).
    min_refresh_ms: u64,
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
}

impl MenuStore {
    pub fn new(gourmet: Arc<GourmetApi>, kv: Arc<dyn Kv>, clock: Arc<dyn Clock>) -> Self {
        Self {
            gourmet,
            kv,
            clock,
            inner: Mutex::new(State::default()),
            min_refresh_ms: DEFAULT_MIN_REFRESH_MS,
        }
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
}
