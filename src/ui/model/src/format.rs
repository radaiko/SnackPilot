//! de-AT formatting helpers (04-ui-ux.md). Hand-rolled locale tables: the
//! spec pins `€ 12,34` symbol-first (04-ui-ux.md:324-326), which differs from
//! what ICU's de-AT currency pattern emits, and the month tables are fixed by
//! the spec examples (`Jän.`/`Feb.`), so tables are the smaller, exact fit.

use chrono::{Datelike, NaiveDate, TimeZone, Timelike};
use chrono_tz::Europe::Vienna;

const WEEKDAYS_SHORT: [&str; 7] = ["Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa.", "So."];
const MONTHS_SHORT: [&str; 12] = [
    "Jän.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sep.", "Okt.", "Nov.", "Dez.",
];

/// EUR amount in the spec's de-AT currency form, e.g. `€ 12,34`
/// (04-ui-ux.md:324-326): symbol first, comma decimal, two places,
/// `.` thousands grouping.
pub fn format_eur(value: f64) -> String {
    let cents = (value * 100.0).round();
    let negative = cents < 0.0;
    let cents = cents.abs() as i64;
    let whole = (cents / 100).to_string();
    let mut grouped = String::new();
    // `.` grouping char approved by the human 2026-11-07 (docs/requirements
    // has no >=1000 example to pin it against).
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!(
        "{}€ {grouped},{:02}",
        if negative { "-" } else { "" },
        cents % 100
    )
}

fn parse_day(day_key: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day_key, "%Y-%m-%d").ok()
}

/// DayNavigator center and OrderItem date: `Mo., 10. Feb.`
/// (04-ui-ux.md:654, 758-759); falls back to the raw key when unparseable.
pub fn format_day_short(day_key: &str) -> String {
    match parse_day(day_key) {
        Some(d) => format!(
            "{}, {}. {}",
            WEEKDAYS_SHORT[d.weekday().num_days_from_monday() as usize],
            d.day(),
            MONTHS_SHORT[d.month0() as usize]
        ),
        None => day_key.to_string(),
    }
}

fn to_vienna(epoch_ms: i64) -> Option<chrono::DateTime<chrono_tz::Tz>> {
    Vienna.timestamp_millis_opt(epoch_ms).single()
}

/// BillCard date: `Mo., 10. Feb. 2026` (04-ui-ux.md:758).
pub fn format_date_long(epoch_ms: i64) -> String {
    match to_vienna(epoch_ms) {
        Some(dt) => format!(
            "{}, {}. {} {}",
            WEEKDAYS_SHORT[dt.weekday().num_days_from_monday() as usize],
            dt.day(),
            MONTHS_SHORT[dt.month0() as usize],
            dt.year()
        ),
        None => String::new(),
    }
}

/// BillCard time: `HH:mm` (04-ui-ux.md:759).
pub fn format_time(epoch_ms: i64) -> String {
    match to_vienna(epoch_ms) {
        Some(dt) => format!("{:02}:{:02}", dt.hour(), dt.minute()),
        None => String::new(),
    }
}

/// Today as the core's `YYYY-MM-DD` day key, in Europe/Vienna (the core's
/// cutoff timezone, so "today" agrees with `is_ordering_cutoff`).
pub fn today_key() -> String {
    use chrono::Utc;
    Utc::now()
        .with_timezone(&Vienna)
        .format("%Y-%m-%d")
        .to_string()
}

/// `YYYY-MM-DD` for an epoch-millis timestamp, in Europe/Vienna.
pub fn day_key_from_epoch_ms(epoch_ms: i64) -> String {
    to_vienna(epoch_ms)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}
