//! Notification diagnostic log (03-features/notification-log §1-§2). A user-activatable,
//! time-boxed, KV-backed log. Never throws — callers are fire-and-forget background tasks.
use crate::datetime::Clock;
use crate::storage::Kv;
use chrono::TimeZone;
use serde::{Deserialize, Serialize};

pub const ENTRIES_KEY: &str = "notification_debug_log_entries";
pub const ACTIVATED_UNTIL_KEY: &str = "notification_debug_log_activated_until";
const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "kebab-case")]
pub enum LogSubsystem {
    Geofence,
    OrderSync,
    DailyReminder,
    MenuCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Guard,
    Error,
    Notification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct LogEntry {
    /// ISO 8601 UTC, produced at append time.
    pub ts: String,
    pub subsystem: LogSubsystem,
    pub level: LogLevel,
    pub event: String,
    /// Omitted from the stored object when absent (§1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// §2.1 — activate for `hours` (start a fresh empty log).
pub fn activate_log(kv: &dyn Kv, clock: &dyn Clock, hours: u32) {
    let until = clock.now_epoch_ms() + hours as i64 * 60 * 60 * 1000;
    let _ = kv.set(ACTIVATED_UNTIL_KEY, &until.to_string());
    let _ = kv.remove(ENTRIES_KEY); // always starts fresh
}

/// §2.1 — delete both keys.
pub fn clear_log(kv: &dyn Kv) {
    let _ = kv.remove(ENTRIES_KEY);
    let _ = kv.remove(ACTIVATED_UNTIL_KEY);
}

/// §2.1 — active iff `now < until` (strict); missing/corrupt → inactive.
pub fn is_active(kv: &dyn Kv, clock: &dyn Clock) -> bool {
    match read_until(kv) {
        Some(until) => clock.now_epoch_ms() < until,
        None => false,
    }
}

fn read_until(kv: &dyn Kv) -> Option<i64> {
    kv.get(ACTIVATED_UNTIL_KEY)
        .ok()
        .flatten()
        .and_then(|s| s.trim().parse::<i64>().ok())
}

/// Current entries (oldest first); empty on absent/corrupt.
pub fn read_entries(kv: &dyn Kv) -> Vec<LogEntry> {
    kv.get(ENTRIES_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<LogEntry>>(&raw).ok())
        .unwrap_or_default()
}

/// §2.2 — append an entry (activation-guarded, capped at 200, never throws).
pub fn append_log_entry(
    kv: &dyn Kv,
    clock: &dyn Clock,
    subsystem: LogSubsystem,
    level: LogLevel,
    event: &str,
    detail: Option<&str>,
) {
    // 2. activation guard.
    if !is_active(kv, clock) {
        return;
    }
    // 3. parse existing (corrupt → empty).
    let mut entries = read_entries(kv);
    // 4. build + append + keep last 200.
    entries.push(LogEntry {
        ts: iso8601_utc(clock.now_epoch_ms()),
        subsystem,
        level,
        event: event.to_string(),
        detail: detail.map(|d| d.to_string()),
    });
    if entries.len() > MAX_ENTRIES {
        let drop = entries.len() - MAX_ENTRIES;
        entries.drain(0..drop);
    }
    // 5. persist (swallow errors).
    if let Ok(json) = serde_json::to_string(&entries) {
        let _ = kv.set(ENTRIES_KEY, &json);
    }
}

fn iso8601_utc(epoch_ms: i64) -> String {
    chrono::Utc
        .timestamp_millis_opt(epoch_ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;
    use crate::storage::MemoryKv;

    fn clock(ms: i64) -> FixedClock {
        FixedClock { epoch_ms: ms }
    }

    #[test]
    fn inactive_by_default_and_append_is_noop() {
        let kv = MemoryKv::new();
        let c = clock(1_000_000_000_000);
        assert!(!is_active(&kv, &c));
        append_log_entry(&kv, &c, LogSubsystem::MenuCheck, LogLevel::Info, "x", None);
        assert!(read_entries(&kv).is_empty());
    }

    #[test]
    fn activate_starts_fresh_and_window_is_strict() {
        let kv = MemoryKv::new();
        // seed a stale entry, then activate → entries wiped.
        kv.set(
            ENTRIES_KEY,
            r#"[{"ts":"t","subsystem":"geofence","level":"info","event":"old"}]"#,
        )
        .unwrap();
        let now = 1_000_000_000_000;
        activate_log(&kv, &clock(now), 24);
        assert!(read_entries(&kv).is_empty());
        assert!(is_active(&kv, &clock(now)));
        assert!(is_active(&kv, &clock(now + 24 * 3_600_000 - 1)));
        assert!(!is_active(&kv, &clock(now + 24 * 3_600_000))); // strict: at `until` inactive
    }

    #[test]
    fn append_records_entry_with_omitted_detail_and_iso_ts() {
        let kv = MemoryKv::new();
        let now = 1_770_000_000_000; // some 2026 instant
        activate_log(&kv, &clock(now), 12);
        append_log_entry(
            &kv,
            &clock(now),
            LogSubsystem::OrderSync,
            LogLevel::Guard,
            "g",
            None,
        );
        append_log_entry(
            &kv,
            &clock(now),
            LogSubsystem::DailyReminder,
            LogLevel::Notification,
            "n",
            Some("k=1"),
        );
        let entries = read_entries(&kv);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].subsystem, LogSubsystem::OrderSync);
        assert_eq!(entries[0].detail, None);
        assert_eq!(entries[1].detail.as_deref(), Some("k=1"));
        assert!(entries[0].ts.ends_with('Z') && entries[0].ts.contains('T'));

        // detail key is truly absent from the serialized object (not null).
        let raw = kv.get(ENTRIES_KEY).unwrap().unwrap();
        assert!(!raw.contains("\"detail\":null"));
        assert!(raw.contains("order-sync")); // kebab subsystem
        assert!(raw.contains("\"level\":\"guard\""));
    }

    #[test]
    fn caps_at_200_dropping_oldest() {
        let kv = MemoryKv::new();
        let now = 1_770_000_000_000;
        activate_log(&kv, &clock(now), 12);
        for i in 0..205 {
            append_log_entry(
                &kv,
                &clock(now),
                LogSubsystem::MenuCheck,
                LogLevel::Info,
                &format!("e{i}"),
                None,
            );
        }
        let entries = read_entries(&kv);
        assert_eq!(entries.len(), 200);
        assert_eq!(entries[0].event, "e5"); // oldest 5 dropped
        assert_eq!(entries[199].event, "e204");
    }
}
