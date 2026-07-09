//! Menu fingerprinting + change detection for new-menu notifications
//! (03-features/notifications-new-menu.md §1-§2). Pure logic + KV persistence.
use crate::domain::MenuItem;
use crate::storage::Kv;
use std::collections::HashMap;

/// Composite-key → content-fingerprint map (§1.1).
pub type Fingerprints = HashMap<String, String>;

/// Persisted-state keys (§2). Plain (unencrypted) KV, shared by background task + UI.
pub const KNOWN_MENUS_KEY: &str = "known_menu_fingerprints";
pub const NOTIFICATION_SENT_KEY: &str = "menu_notification_sent";

/// §1.1 — key `{id}|{day}`, value `{title}|{subtitle}|{allergens joined ','}`.
/// `day` is already a "YYYY-MM-DD" key. Same key → last item wins (map overwrite).
/// `available`/`ordered`/`category`/`price` do NOT participate.
pub fn compute_fingerprints(items: &[MenuItem]) -> Fingerprints {
    let mut map = HashMap::new();
    for it in items {
        let key = format!("{}|{}", it.id, it.day);
        let value = format!("{}|{}|{}", it.title, it.subtitle, it.allergens.join(","));
        map.insert(key, value);
    }
    map
}

/// §1.2 — true iff there is genuinely new/changed content.
/// Empty current → false; empty known → true; else any current key absent from known
/// or with a different value → true. Removed keys do NOT count.
pub fn detect_new_menus(current: &Fingerprints, known: &Fingerprints) -> bool {
    if current.is_empty() {
        return false;
    }
    if known.is_empty() {
        return true;
    }
    current
        .iter()
        .any(|(k, v)| known.get(k).map(|kv| kv != v).unwrap_or(true))
}

/// §1.3 — serialize as `[["key","value"],...]`.
pub fn serialize_known(fp: &Fingerprints) -> String {
    let entries: Vec<(&String, &String)> = fp.iter().collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

/// §1.3 — null/empty/unparseable → empty map.
pub fn deserialize_known(json: &str) -> Fingerprints {
    if json.trim().is_empty() {
        return HashMap::new();
    }
    serde_json::from_str::<Vec<(String, String)>>(json)
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

/// Load the last-acknowledged fingerprint set (§2).
pub fn load_known_menus(kv: &dyn Kv) -> Fingerprints {
    let raw = kv.get(KNOWN_MENUS_KEY).ok().flatten().unwrap_or_default();
    deserialize_known(&raw)
}

pub fn save_known_menus(kv: &dyn Kv, fp: &Fingerprints) {
    let _ = kv.set(KNOWN_MENUS_KEY, &serialize_known(fp));
}

/// One-notification-per-batch latch: `"true"`/`"false"`, missing ⇒ false (§2).
pub fn is_notification_sent(kv: &dyn Kv) -> bool {
    kv.get(NOTIFICATION_SENT_KEY).ok().flatten().as_deref() == Some("true")
}

pub fn set_notification_sent(kv: &dyn Kv, sent: bool) {
    let _ = kv.set(NOTIFICATION_SENT_KEY, if sent { "true" } else { "false" });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MenuCategory;
    use crate::storage::MemoryKv;

    fn item(id: &str, day: &str, title: &str, subtitle: &str, allergens: &[&str]) -> MenuItem {
        MenuItem {
            id: id.into(),
            day: day.into(),
            title: title.into(),
            subtitle: subtitle.into(),
            allergens: allergens.iter().map(|s| s.to_string()).collect(),
            available: true,
            ordered: false,
            category: MenuCategory::Menu1,
            price: "€ 5,50".into(), // must NOT affect the fingerprint
        }
    }

    #[test]
    fn fingerprint_entry_shape() {
        let fp = compute_fingerprints(&[item(
            "menu-001",
            "2026-02-10",
            "MENU I",
            "Schnitzel mit Reis",
            &["A", "G"],
        )]);
        assert_eq!(
            fp.get("menu-001|2026-02-10").map(|s| s.as_str()),
            Some("MENU I|Schnitzel mit Reis|A,G")
        );
    }

    #[test]
    fn same_id_two_days_two_entries_same_day_last_wins() {
        let fp = compute_fingerprints(&[
            item("menu-001", "2026-02-10", "A", "", &[]),
            item("menu-001", "2026-02-11", "B", "", &[]),
            item("menu-001", "2026-02-10", "C", "", &[]), // same key → overwrites
        ]);
        assert_eq!(fp.len(), 2);
        assert_eq!(
            fp.get("menu-001|2026-02-10").map(|s| s.as_str()),
            Some("C||")
        );
    }

    #[test]
    fn detection_rules() {
        let a = compute_fingerprints(&[item("m", "2026-02-10", "A", "", &[])]);
        let changed = compute_fingerprints(&[item("m", "2026-02-10", "B", "", &[])]);
        let empty = Fingerprints::new();
        assert!(!detect_new_menus(&empty, &a)); // empty current → false
        assert!(detect_new_menus(&a, &empty)); // empty known → true
        assert!(detect_new_menus(&changed, &a)); // changed value → true
        assert!(!detect_new_menus(&a, &a)); // identical → false
                                            // removed key does not count as a change.
        let two = compute_fingerprints(&[
            item("m", "2026-02-10", "A", "", &[]),
            item("m", "2026-02-11", "X", "", &[]),
        ]);
        assert!(!detect_new_menus(&a, &two)); // current is a subset of known
    }

    #[test]
    fn serialize_round_trip_and_bad_json() {
        let fp = compute_fingerprints(&[item("m", "2026-02-10", "A", "s", &["A"])]);
        let json = serialize_known(&fp);
        assert_eq!(deserialize_known(&json), fp);
        assert!(deserialize_known("").is_empty());
        assert!(deserialize_known("not json").is_empty());
    }

    #[test]
    fn kv_persistence_and_latch() {
        let kv = MemoryKv::new();
        assert!(load_known_menus(&kv).is_empty());
        assert!(!is_notification_sent(&kv)); // missing ⇒ false
        let fp = compute_fingerprints(&[item("m", "2026-02-10", "A", "", &[])]);
        save_known_menus(&kv, &fp);
        assert_eq!(load_known_menus(&kv), fp);
        set_notification_sent(&kv, true);
        assert!(is_notification_sent(&kv));
        set_notification_sent(&kv, false);
        assert!(!is_notification_sent(&kv));
    }
}
