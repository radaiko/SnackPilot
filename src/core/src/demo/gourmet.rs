//! Demo Gourmet backend — the offline data source swapped in on magic-credential login
//! (demo-mode.md §3, §6). Mirrors the live `GourmetApi` method surface; performs ZERO
//! network. A fresh instance is created on each demo login; menus are generated once and
//! cached on the instance.
use crate::datetime::{local_date_key, local_epoch_from_parts, Clock};
use crate::demo::data::{generate_demo_billings, generate_demo_menus};
use crate::domain::{Bill, Credentials, GourmetUserInfo, MenuItem, OrderedMenu};
use crate::error::CoreResult;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// §6 — module-level order counter starting at 1, never reset for the process lifetime, so
/// IDs keep incrementing across demo sessions/instances within one app run.
static ORDER_COUNTER: AtomicU64 = AtomicU64::new(1);

struct DemoState {
    user_info: Option<GourmetUserInfo>,
    orders: Vec<OrderedMenu>,
    /// Lazily generated, instance-cached (§3 `getMenus`).
    menus: Option<Vec<MenuItem>>,
}

pub struct DemoGourmetApi {
    clock: Arc<dyn Clock>,
    state: Mutex<DemoState>,
}

impl DemoGourmetApi {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            state: Mutex::new(DemoState {
                user_info: None,
                orders: Vec::new(),
                menus: None,
            }),
        }
    }

    /// §3 — fixed fake account.
    fn demo_user() -> GourmetUserInfo {
        GourmetUserInfo {
            username: "Demo User".into(),
            shop_model_id: "demo-shop-1".into(),
            eater_id: "demo-eater-1".into(),
            staff_group_id: "demo-staff-1".into(),
        }
    }

    pub fn user_info(&self) -> Option<GourmetUserInfo> {
        self.state.lock().unwrap().user_info.clone()
    }

    pub fn is_authenticated(&self) -> bool {
        self.state.lock().unwrap().user_info.is_some()
    }

    /// Ignores arguments; never fails (§3).
    pub async fn login(&self, _creds: Credentials) -> CoreResult<GourmetUserInfo> {
        let info = Self::demo_user();
        self.state.lock().unwrap().user_info = Some(info.clone());
        Ok(info)
    }

    /// Generate+cache on first call; return copies with `ordered` recomputed by day+subtitle
    /// match against the in-memory orders (§3 — NOT by id, because `title` holds the category).
    pub async fn get_menus(&self) -> CoreResult<Vec<MenuItem>> {
        let mut st = self.state.lock().unwrap();
        if st.menus.is_none() {
            st.menus = Some(generate_demo_menus(self.clock.as_ref()));
        }
        let orders = st.orders.clone();
        let mut menus = st.menus.clone().unwrap();
        for item in &mut menus {
            item.ordered = orders.iter().any(|o| {
                local_date_key(o.date_epoch_ms) == item.day && o.subtitle == item.subtitle
            });
        }
        Ok(menus)
    }

    pub async fn get_orders(&self) -> CoreResult<Vec<OrderedMenu>> {
        Ok(self.state.lock().unwrap().orders.clone())
    }

    /// For each `(menu_id, date_key)` find the cached item by id+day and append an order
    /// (§3, §6). The menuId is deliberately not stored on the order.
    pub async fn add_to_cart(&self, items: Vec<(String, String)>) -> CoreResult<()> {
        let mut st = self.state.lock().unwrap();
        if st.menus.is_none() {
            st.menus = Some(generate_demo_menus(self.clock.as_ref()));
        }
        let menus = st.menus.clone().unwrap();
        for (menu_id, date_key) in items {
            let found = menus.iter().find(|m| m.id == menu_id && m.day == date_key);
            let (title, subtitle) = match found {
                Some(m) => (m.title.clone(), m.subtitle.clone()),
                None => ("Demo Menü".to_string(), String::new()),
            };
            let n = ORDER_COUNTER.fetch_add(1, Ordering::SeqCst);
            st.orders.push(OrderedMenu {
                position_id: format!("demo-pos-{n}"),
                eating_cycle_id: format!("demo-cycle-{n}"),
                date_epoch_ms: epoch_for_day(&date_key),
                title,
                subtitle,
                approved: false,
            });
        }
        Ok(())
    }

    /// Marks all in-memory orders approved (§3).
    pub async fn confirm_orders(&self) -> CoreResult<()> {
        for o in &mut self.state.lock().unwrap().orders {
            o.approved = true;
        }
        Ok(())
    }

    /// Removes orders whose positionId is in the set (§3).
    pub async fn cancel_orders(&self, position_ids: Vec<String>) -> CoreResult<()> {
        let set: HashSet<String> = position_ids.into_iter().collect();
        self.state
            .lock()
            .unwrap()
            .orders
            .retain(|o| !set.contains(&o.position_id));
        Ok(())
    }

    pub async fn get_billings(&self, check_last_month_number: &str) -> CoreResult<Vec<Bill>> {
        Ok(generate_demo_billings(
            self.clock.as_ref(),
            check_last_month_number,
        ))
    }

    /// Clears user info, orders, and the cached menus (§3).
    pub async fn logout(&self) -> CoreResult<()> {
        let mut st = self.state.lock().unwrap();
        st.user_info = None;
        st.orders.clear();
        st.menus = None;
        Ok(())
    }
}

/// `YYYY-MM-DD` → local-midnight epoch ms; 0 on parse failure.
fn epoch_for_day(date_key: &str) -> i64 {
    let parts: Vec<&str> = date_key.split('-').collect();
    if let [y, mo, d] = parts[..] {
        if let (Ok(y), Ok(mo), Ok(d)) = (y.parse::<i32>(), mo.parse::<u32>(), d.parse::<u32>()) {
            if let Some(ms) = local_epoch_from_parts(y, mo, d, 0, 0) {
                return ms;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    // 2026-07-09 12:00 local-ish (epoch ms). Menus generate deterministically per calendar day.
    fn clock() -> Arc<dyn Clock> {
        Arc::new(FixedClock::new(1_752_057_600_000))
    }

    #[tokio::test]
    async fn login_returns_fixed_demo_user_and_never_fails() {
        let api = DemoGourmetApi::new(clock());
        let info = api
            .login(Credentials {
                username: "whatever".into(),
                password: "ignored".into(),
            })
            .await
            .unwrap();
        assert_eq!(info.username, "Demo User");
        assert_eq!(info.eater_id, "demo-eater-1");
        assert!(api.is_authenticated());
    }

    #[tokio::test]
    async fn menus_are_40_items_stable_and_all_available() {
        let api = DemoGourmetApi::new(clock());
        let a = api.get_menus().await.unwrap();
        let b = api.get_menus().await.unwrap();
        assert_eq!(a.len(), 40);
        assert_eq!(a, b); // instance-cached, stable
        assert!(a.iter().all(|m| m.available));
    }

    #[tokio::test]
    async fn add_to_cart_creates_order_and_marks_menu_ordered() {
        let api = DemoGourmetApi::new(clock());
        let menus = api.get_menus().await.unwrap();
        let target = &menus[0];
        api.add_to_cart(vec![(target.id.clone(), target.day.clone())])
            .await
            .unwrap();

        let orders = api.get_orders().await.unwrap();
        assert_eq!(orders.len(), 1);
        assert!(orders[0].position_id.starts_with("demo-pos-"));
        assert_eq!(orders[0].subtitle, target.subtitle);
        assert!(!orders[0].approved);

        // The matching menu item is now `ordered` (matched by day+subtitle).
        let after = api.get_menus().await.unwrap();
        let same = after.iter().find(|m| m.id == target.id).unwrap();
        assert!(same.ordered);
    }

    #[tokio::test]
    async fn confirm_then_cancel_workflow() {
        let api = DemoGourmetApi::new(clock());
        let menus = api.get_menus().await.unwrap();
        api.add_to_cart(vec![(menus[0].id.clone(), menus[0].day.clone())])
            .await
            .unwrap();
        api.confirm_orders().await.unwrap();
        let orders = api.get_orders().await.unwrap();
        assert!(orders.iter().all(|o| o.approved));

        api.cancel_orders(vec![orders[0].position_id.clone()])
            .await
            .unwrap();
        assert!(api.get_orders().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn logout_clears_everything() {
        let api = DemoGourmetApi::new(clock());
        api.login(Credentials {
            username: "demo".into(),
            password: "demo1234!".into(),
        })
        .await
        .unwrap();
        let menus = api.get_menus().await.unwrap();
        api.add_to_cart(vec![(menus[0].id.clone(), menus[0].day.clone())])
            .await
            .unwrap();
        api.logout().await.unwrap();
        assert!(!api.is_authenticated());
        assert!(api.get_orders().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn billings_generate() {
        let api = DemoGourmetApi::new(clock());
        let bills = api.get_billings("0").await.unwrap();
        // Deterministic per month; non-empty for the current month up to "today".
        assert!(!bills.is_empty());
    }
}
