//! Typed cache helpers over the `Kv` store (03-features/caching §1-§3).
//!
//! This is v2's own cache format (v1's AsyncStorage files are NOT migrated), so the domain
//! models are serialized directly with serde and only need to round-trip within v2. Cache
//! keys and the month-key/label rules match v1 so behavior transfers (caching §2.5).
//!
//! Absent/corrupt semantics (caching §3.4): a loader returns `None` when the key is absent
//! OR the value is corrupt — the caller must then leave existing in-memory state untouched
//! (never clear it). Menus/orders additionally REMOVE a corrupt key; billing does not.
use crate::datetime::Clock;
use crate::domain::{Bill, MenuItem, OrderedMenu, VentopayTransaction};
use crate::error::CoreResult;
use crate::storage::Kv;
use chrono::{Datelike, TimeZone};

pub const MENUS_KEY: &str = "menus_items";
pub const ORDERS_KEY: &str = "orders_list";
pub const GOURMET_BILLING_PREFIX: &str = "billing_";
pub const VENTOPAY_BILLING_PREFIX: &str = "ventopay_billing_";

/// Austrian German month names (caching §2.5), 1-based index via `[m-1]`.
const MONTH_NAMES: [&str; 12] = [
    "Jänner",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// Load `menus_items`; `None` if absent or corrupt (corrupt → key removed, §3.4).
pub fn load_menus(kv: &dyn Kv) -> Option<Vec<MenuItem>> {
    load_removing_corrupt(kv, MENUS_KEY)
}

pub fn save_menus(kv: &dyn Kv, items: &[MenuItem]) -> CoreResult<()> {
    save(kv, MENUS_KEY, items)
}

/// Load `orders_list`; `None` if absent or corrupt (corrupt → key removed, §3.4).
pub fn load_orders(kv: &dyn Kv) -> Option<Vec<OrderedMenu>> {
    load_removing_corrupt(kv, ORDERS_KEY)
}

pub fn save_orders(kv: &dyn Kv, orders: &[OrderedMenu]) -> CoreResult<()> {
    save(kv, ORDERS_KEY, orders)
}

/// Load Gourmet bills for a month; `None` if absent or corrupt (corrupt NOT removed, §3.4).
pub fn load_gourmet_billing(kv: &dyn Kv, month_key: &str) -> Option<Vec<Bill>> {
    load_keep_corrupt(kv, &format!("{GOURMET_BILLING_PREFIX}{month_key}"))
}

pub fn save_gourmet_billing(kv: &dyn Kv, month_key: &str, bills: &[Bill]) -> CoreResult<()> {
    save(kv, &format!("{GOURMET_BILLING_PREFIX}{month_key}"), bills)
}

/// Load Ventopay transactions for a month; `None` if absent or corrupt (corrupt NOT removed).
pub fn load_ventopay_billing(kv: &dyn Kv, month_key: &str) -> Option<Vec<VentopayTransaction>> {
    load_keep_corrupt(kv, &format!("{VENTOPAY_BILLING_PREFIX}{month_key}"))
}

pub fn save_ventopay_billing(
    kv: &dyn Kv,
    month_key: &str,
    txs: &[VentopayTransaction],
) -> CoreResult<()> {
    save(kv, &format!("{VENTOPAY_BILLING_PREFIX}{month_key}"), txs)
}

/// "YYYY-MM" for today-minus-`offset` months in device-local time (caching §2.5).
pub fn month_key_from_offset(clock: &dyn Clock, offset: u32) -> String {
    let now = chrono::Local
        .timestamp_millis_opt(clock.now_epoch_ms())
        .single()
        .expect("valid epoch");
    // subtract `offset` whole months from the first of the current month.
    let total = now.year() * 12 + (now.month0() as i32) - offset as i32;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1; // 1-based
    format!("{year:04}-{month:02}")
}

/// Display label "{Monat} {yyyy}" from a "YYYY-MM" key (caching §2.5).
pub fn month_label(month_key: &str) -> String {
    let (y, m) = match month_key.split_once('-') {
        Some((y, m)) => (y, m),
        None => return month_key.to_string(),
    };
    let idx = m.parse::<usize>().unwrap_or(0);
    let name = MONTH_NAMES.get(idx.wrapping_sub(1)).copied().unwrap_or("");
    format!("{name} {y}")
}

fn save<T: serde::Serialize + ?Sized>(kv: &dyn Kv, key: &str, value: &T) -> CoreResult<()> {
    let json = serde_json::to_string(value).map_err(|e| crate::error::CoreError::Storage {
        message: e.to_string(),
    })?;
    kv.set(key, &json)
}

/// Load + parse; on absent → None; on corrupt → remove the key, None.
fn load_removing_corrupt<T: serde::de::DeserializeOwned>(kv: &dyn Kv, key: &str) -> Option<Vec<T>> {
    match kv.get(key).ok().flatten() {
        None => None,
        Some(raw) => match serde_json::from_str::<Vec<T>>(&raw) {
            Ok(v) => Some(v),
            Err(_) => {
                let _ = kv.remove(key);
                None
            }
        },
    }
}

/// Load + parse; on absent or corrupt → None (key NOT removed).
fn load_keep_corrupt<T: serde::de::DeserializeOwned>(kv: &dyn Kv, key: &str) -> Option<Vec<T>> {
    let raw = kv.get(key).ok().flatten()?;
    serde_json::from_str::<Vec<T>>(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;
    use crate::domain::{MenuCategory, MenuItem};
    use crate::storage::MemoryKv;

    fn item(id: &str) -> MenuItem {
        MenuItem {
            id: id.into(),
            day: "2026-02-10".into(),
            title: "MENÜ I".into(),
            subtitle: String::new(),
            allergens: vec![],
            available: true,
            ordered: false,
            category: MenuCategory::Menu1,
            price: String::new(),
        }
    }

    #[test]
    fn menus_round_trip() {
        let kv = MemoryKv::new();
        assert_eq!(load_menus(&kv), None); // absent
        save_menus(&kv, &[item("menu-001")]).unwrap();
        let loaded = load_menus(&kv).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "menu-001");
    }

    #[test]
    fn corrupt_menus_returns_none_and_removes_key() {
        let kv = MemoryKv::new();
        kv.set(MENUS_KEY, "not json").unwrap();
        assert_eq!(load_menus(&kv), None);
        assert_eq!(kv.get(MENUS_KEY).unwrap(), None); // removed
    }

    #[test]
    fn corrupt_billing_returns_none_but_keeps_key() {
        let kv = MemoryKv::new();
        let key = format!("{GOURMET_BILLING_PREFIX}2026-02");
        kv.set(&key, "not json").unwrap();
        assert_eq!(load_gourmet_billing(&kv, "2026-02"), None);
        assert!(kv.get(&key).unwrap().is_some()); // NOT removed
    }

    #[test]
    fn month_key_from_offset_wraps_year() {
        // 2026-02-10 12:00 local
        use chrono::TimeZone;
        let ms = chrono::Local
            .with_ymd_and_hms(2026, 2, 10, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        let clock = FixedClock { epoch_ms: ms };
        assert_eq!(month_key_from_offset(&clock, 0), "2026-02");
        assert_eq!(month_key_from_offset(&clock, 1), "2026-01");
        assert_eq!(month_key_from_offset(&clock, 2), "2025-12"); // year wrap
    }

    #[test]
    fn month_label_uses_austrian_names() {
        assert_eq!(month_label("2026-01"), "Jänner 2026");
        assert_eq!(month_label("2026-03"), "März 2026");
        assert_eq!(month_label("2026-12"), "Dezember 2026");
    }
}
