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

/// Parse the Gourmet `BillDate` (01 §12) as leniently as v1's `new Date(bill.BillDate)`, which
/// accepts the several shapes .NET emits. The sanitized fixtures use a clean no-tz ISO string, but
/// real data varies (fractional seconds, a `Z`/offset timezone, or the `/Date(ms)/` form) — the old
/// strict `%Y-%m-%dT%H:%M:%S`-only parser rejected those and fell back to epoch 0 (shown as
/// 01.01.1970). Order of attempts:
///   1. `/Date(1751320800000)/` (optionally with a `+0200` offset) → the embedded epoch ms.
///   2. RFC3339 with an explicit timezone (`…Z` / `…+02:00`) → absolute instant.
///   3. ISO datetime WITHOUT a timezone (optional fractional seconds) → local time, matching JS's
///      `new Date("2026-07-01T00:00:00")` which is interpreted in the local zone.
///   4. Date only → local midnight.
pub fn parse_bill_date(s: &str) -> Option<i64> {
    let s = s.trim();

    // 1. ASP.NET "/Date(ms)/" or "/Date(ms+0200)/" — take the leading signed integer (ms).
    if let Some(inner) = s.strip_prefix("/Date(").and_then(|r| r.strip_suffix(")/")) {
        let digits: String = inner
            .char_indices()
            .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && *c == '-'))
            .map(|(_, c)| c)
            .collect();
        if let Ok(ms) = digits.parse::<i64>() {
            return Some(ms);
        }
    }

    // 2. Timezone-qualified (Z or ±hh:mm) → absolute epoch.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }

    // 3. Naive datetime (no tz) → local. Try fractional seconds and both T / space separators.
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return local_epoch_ms(ndt);
        }
    }

    // 4. Date only → local midnight.
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d.and_hms_opt(0, 0, 0).and_then(local_epoch_ms);
    }

    None
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

/// Local-tz epoch ms for Y/M/D h:m (seconds 0). None on an invalid date (Ventopay §6.4).
pub fn local_epoch_from_parts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
) -> Option<i64> {
    let d = NaiveDate::from_ymd_opt(year, month, day)?;
    let t = NaiveTime::from_hms_opt(hour, minute, 0)?;
    local_epoch_ms(d.and_time(t))
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
    fn bill_date_accepts_dotnet_variants() {
        // The sanitized fixture shape (no tz) — still parses (local).
        assert!(parse_bill_date("2026-07-01T00:00:00").is_some_and(|ms| ms != 0));
        // Real .NET variants the old strict parser rejected → had fallen back to epoch 0 (1970).
        assert!(parse_bill_date("2026-07-01T00:00:00.000").is_some_and(|ms| ms != 0));
        assert!(parse_bill_date("2026-07-01T12:30:00.1234567").is_some());
        assert!(parse_bill_date("2026-07-01 12:30:00").is_some());
        assert!(parse_bill_date("2026-07-01").is_some());
        // Timezone-qualified → absolute instant (epoch 0 == 1970-01-01T00:00:00Z).
        assert_eq!(parse_bill_date("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_bill_date("1970-01-01T02:00:00+02:00"), Some(0));
        // ASP.NET "/Date(ms)/" (with optional offset).
        assert_eq!(parse_bill_date("/Date(0)/"), Some(0));
        assert_eq!(
            parse_bill_date("/Date(1751320800000)/"),
            Some(1_751_320_800_000)
        );
        assert_eq!(
            parse_bill_date("/Date(1751320800000+0200)/"),
            Some(1_751_320_800_000)
        );
        // Genuine garbage still yields None.
        assert_eq!(parse_bill_date("not a date"), None);
        assert_eq!(parse_bill_date(""), None);
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

    #[test]
    fn local_epoch_from_parts_matches_local_midnight_helper() {
        let a = local_epoch_from_parts(2026, 2, 10, 0, 0).unwrap();
        assert_eq!(a, vienna_like_local("2026-02-10", 0, 0));
        let b = local_epoch_from_parts(2026, 2, 10, 11, 49).unwrap();
        assert_eq!(b, vienna_like_local("2026-02-10", 11, 49));
        assert_eq!(local_epoch_from_parts(2026, 13, 40, 0, 0), None);
    }
}
