use crate::datetime::Clock;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use chrono_tz::Europe::Vienna;

/// Device-local Y/M/D for an epoch, "YYYY-MM-DD" (menus.md §1 — NOT toISOString).
pub fn local_date_key(epoch_ms: i64) -> String {
    let dt = chrono::Local
        .timestamp_millis_opt(epoch_ms)
        .single()
        .expect("valid epoch");
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

/// Europe/Vienna Y/M/D for an epoch, "YYYY-MM-DD".
pub fn vienna_date_key(epoch_ms: i64) -> String {
    let dt = Vienna
        .timestamp_millis_opt(epoch_ms)
        .single()
        .expect("valid epoch");
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

/// "YYYY-MM-DD" -> "MM-dd-yyyy" (Gourmet wire, 01 §12).
pub fn format_menu_date(date_key: &str) -> String {
    match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => format!("{:02}-{:02}-{:04}", d.month(), d.day(), d.year()),
        Err(_) => String::new(),
    }
}

/// "MM-dd-yyyy" -> "YYYY-MM-DD" (None if malformed).
pub fn parse_menu_date(mmddyyyy: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(mmddyyyy, "%m-%d-%Y").ok()?;
    Some(format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()))
}

/// "dd.MM.yyyy HH:mm:ss" (missing time -> 00:00:00) -> local epoch ms (01 §12).
pub fn parse_orders_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let (date_part, time_part) = match s.split_once(' ') {
        Some((d, t)) => (d, t),
        None => (s, "00:00:00"),
    };
    let d = NaiveDate::parse_from_str(date_part, "%d.%m.%Y").ok()?;
    let t = NaiveTime::parse_from_str(time_part, "%H:%M:%S").ok()?;
    local_epoch_ms(d.and_time(t))
}

/// "YYYY-MM-DD" -> "dd.MM.yyyy" (Ventopay wire, 02 §5).
pub fn format_ventopay_date(date_key: &str) -> String {
    match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => format!("{:02}.{:02}.{:04}", d.day(), d.month(), d.year()),
        Err(_) => String::new(),
    }
}

/// ISO-like "2026-02-10T12:00:00" (no tz) parsed as local -> epoch ms (01 §12 BillDate).
pub fn parse_bill_date(s: &str) -> Option<i64> {
    let ndt = NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%S").ok()?;
    local_epoch_ms(ndt)
}

/// Ordering/cancellation cutoff (Europe/Vienna): past day blocked; today blocked iff
/// Vienna time >= 09:00; future never (orders.md §4.2 / menus.md §6.2).
pub fn is_ordering_cutoff(clock: &dyn Clock, date_key: &str) -> bool {
    let now = clock.now_epoch_ms();
    let vienna_now = Vienna
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch");
    let today = vienna_now.date_naive();
    let target = match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return false,
    };
    if target < today {
        return true;
    }
    if target > today {
        return false;
    }
    // target == today: blocked once the clock hits 09:00 Vienna.
    vienna_now.hour() >= 9
}

fn local_epoch_ms(ndt: NaiveDateTime) -> Option<i64> {
    chrono::Local
        .from_local_datetime(&ndt)
        .single()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    fn vienna(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        use chrono::TimeZone;
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    // local-midnight epoch for a YYYY-MM-DD in device-local tz + h:m offset.
    fn vienna_like_local(key: &str, h: u32, mi: u32) -> i64 {
        use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
        let d = NaiveDate::parse_from_str(key, "%Y-%m-%d").unwrap();
        let t = NaiveTime::from_hms_opt(h, mi, 0).unwrap();
        Local
            .from_local_datetime(&d.and_time(t))
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn menu_date_roundtrip() {
        // MM-dd-yyyy is the Gourmet wire format (01 §12).
        assert_eq!(format_menu_date("2026-02-10"), "02-10-2026");
        assert_eq!(parse_menu_date("02-10-2026").as_deref(), Some("2026-02-10"));
        assert_eq!(parse_menu_date("garbage"), None);
    }

    #[test]
    fn ventopay_date_format() {
        assert_eq!(format_ventopay_date("2026-02-28"), "28.02.2026");
    }

    #[test]
    fn orders_date_defaults_missing_time_to_midnight() {
        // dd.MM.yyyy HH:mm:ss; missing time → 00:00:00 (01 §12).
        let with_time = parse_orders_date("10.02.2026 09:30:00").unwrap();
        let no_time = parse_orders_date("10.02.2026").unwrap();
        assert_eq!(no_time, vienna_like_local("2026-02-10", 0, 0));
        assert!(with_time > no_time);
    }

    #[test]
    fn cutoff_before_today_is_blocked() {
        let clock = FixedClock {
            epoch_ms: vienna(2026, 2, 10, 8, 0),
        };
        assert!(is_ordering_cutoff(&clock, "2026-02-09")); // yesterday
    }

    #[test]
    fn cutoff_today_depends_on_0900_vienna() {
        let before = FixedClock {
            epoch_ms: vienna(2026, 2, 10, 8, 59),
        };
        let after = FixedClock {
            epoch_ms: vienna(2026, 2, 10, 9, 0),
        };
        assert!(!is_ordering_cutoff(&before, "2026-02-10")); // before 09:00 → open
        assert!(is_ordering_cutoff(&after, "2026-02-10")); // at/after 09:00 → blocked
    }

    #[test]
    fn cutoff_future_never_blocked() {
        let clock = FixedClock {
            epoch_ms: vienna(2026, 2, 10, 23, 0),
        };
        assert!(!is_ordering_cutoff(&clock, "2026-02-11"));
    }
}
