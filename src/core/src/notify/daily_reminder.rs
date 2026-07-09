//! Daily order reminder decision logic (03-features/notifications-daily-reminder §3-§6).
//! Pure: given the clock, settings, and current orders, returns a NotificationCommand (or None).
//! All time comparisons are Europe/Vienna wall-clock.
use crate::datetime::{vienna_date_key, Clock};
use crate::domain::OrderedMenu;
use crate::notify::{
    DailyReminderSettings, NotificationCommand, DAILY_REMINDER_ID, ORDER_REMINDERS_CHANNEL,
};
use chrono::{TimeZone, Timelike};

const TITLE: &str = "Deine Bestellung heute";

/// §3 — the ordered guards. Returns:
/// - `None` when the reminder is disabled/unconfigured, or at/after target time (no action);
/// - `CancelPending` when enabled + before target but no order for Vienna-today;
/// - `ScheduleAt` (target time, body = today's orders) otherwise.
pub fn check_daily_reminder(
    clock: &dyn Clock,
    settings: &DailyReminderSettings,
    orders: &[OrderedMenu],
) -> Option<NotificationCommand> {
    // 1-2. enabled + configured.
    if !settings.enabled {
        return None;
    }
    let (hour, minute) = match (settings.hour, settings.minute) {
        (Some(h), Some(m)) => (h as i64, m as i64),
        _ => return None,
    };

    // 3. late-task guard: at/after target → no action.
    let now = clock.now_epoch_ms();
    let vienna_now = chrono_tz::Europe::Vienna
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch");
    let current_min = vienna_now.hour() as i64 * 60 + vienna_now.minute() as i64;
    let target_min = hour * 60 + minute;
    if current_min >= target_min {
        return None;
    }

    // 4. today's orders (no approval filter). Zero → cancel any pending reminder.
    let today = vienna_date_key(now);
    let todays: Vec<&OrderedMenu> = orders
        .iter()
        .filter(|o| vienna_date_key(o.date_epoch_ms) == today)
        .collect();
    if todays.is_empty() {
        return Some(NotificationCommand::CancelPending {
            id: DAILY_REMINDER_ID.to_string(),
        });
    }

    // 5-6. build body + schedule for the target time (now + remaining minutes preserves seconds).
    let body = todays
        .iter()
        .map(|o| {
            if o.subtitle.is_empty() {
                o.title.clone()
            } else {
                format!("{} \u{2014} {}", o.title, o.subtitle) // title — subtitle (em dash)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let fire_at_epoch_ms = now + (target_min - current_min) * 60_000;
    Some(NotificationCommand::ScheduleAt {
        id: DAILY_REMINDER_ID.to_string(),
        title: TITLE.to_string(),
        body,
        channel_id: ORDER_REMINDERS_CHANNEL.to_string(),
        fire_at_epoch_ms,
        screen: Some("/(tabs)/orders".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    fn vienna_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    fn order(date_ms: i64, title: &str, subtitle: &str) -> OrderedMenu {
        OrderedMenu {
            position_id: "P".into(),
            eating_cycle_id: String::new(),
            date_epoch_ms: date_ms,
            title: title.into(),
            subtitle: subtitle.into(),
            approved: false,
        }
    }

    fn settings(enabled: bool, h: Option<u8>, m: Option<u8>) -> DailyReminderSettings {
        DailyReminderSettings {
            enabled,
            hour: h,
            minute: m,
        }
    }

    #[test]
    fn disabled_or_unconfigured_returns_none() {
        let clock = FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, 8, 0),
        };
        assert!(check_daily_reminder(&clock, &settings(false, Some(12), Some(0)), &[]).is_none());
        assert!(check_daily_reminder(&clock, &settings(true, None, None), &[]).is_none());
    }

    #[test]
    fn at_or_after_target_returns_none() {
        let clock = FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, 12, 30),
        };
        let o = order(vienna_ms(2026, 2, 10, 12, 0), "MENÜ I", "Schnitzel");
        assert!(check_daily_reminder(&clock, &settings(true, Some(12), Some(0)), &[o]).is_none());
    }

    #[test]
    fn no_order_today_cancels_pending() {
        let clock = FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, 8, 0),
        };
        // an order for a different day only.
        let o = order(vienna_ms(2026, 2, 11, 12, 0), "MENÜ I", "x");
        let cmd = check_daily_reminder(&clock, &settings(true, Some(12), Some(0)), &[o]).unwrap();
        assert_eq!(
            cmd,
            NotificationCommand::CancelPending {
                id: DAILY_REMINDER_ID.into()
            }
        );
    }

    #[test]
    fn before_target_with_order_schedules_at_target() {
        let now = vienna_ms(2026, 2, 10, 8, 0);
        let clock = FixedClock { epoch_ms: now };
        let orders = vec![
            order(
                vienna_ms(2026, 2, 10, 12, 0),
                "MENÜ I",
                "Schnitzel mit Reis",
            ),
            order(vienna_ms(2026, 2, 10, 12, 0), "SUPPE & SALAT", ""), // no subtitle
        ];
        let cmd =
            check_daily_reminder(&clock, &settings(true, Some(12), Some(0)), &orders).unwrap();
        match cmd {
            NotificationCommand::ScheduleAt {
                id,
                title,
                body,
                channel_id,
                fire_at_epoch_ms,
                ..
            } => {
                assert_eq!(id, DAILY_REMINDER_ID);
                assert_eq!(title, TITLE);
                assert_eq!(body, "MENÜ I \u{2014} Schnitzel mit Reis\nSUPPE & SALAT");
                assert_eq!(channel_id, ORDER_REMINDERS_CHANNEL);
                // 08:00 → 12:00 is 240 minutes later.
                assert_eq!(fire_at_epoch_ms, now + 240 * 60_000);
            }
            _ => panic!("expected ScheduleAt"),
        }
    }
}
