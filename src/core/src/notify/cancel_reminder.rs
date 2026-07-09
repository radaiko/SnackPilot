//! Cancel-order reminder decision logic (03-features/notifications-cancel-reminder §2, §4).
//! Pure: given the clock, `is_at_company`, and current orders → a NotificationCommand or None.
//! Reads NO settings/permissions; gating is structural + the 08:45/09:00 Vienna windows.
use crate::datetime::{vienna_date_key, Clock};
use crate::domain::OrderedMenu;
use crate::notify::{NotificationCommand, CANCEL_REMINDER_ID, ORDER_REMINDERS_CHANNEL};
use chrono::{TimeZone, Timelike};

const TITLE: &str = "SnackPilot";
const BODY: &str = "Du hast heute bestellt, bist aber nicht im Büro. Stornieren?";
const SCREEN: &str = "/(tabs)/orders";
/// 08:45 target, 09:00 deadline (Vienna minutes-since-midnight).
const TARGET_MIN: i64 = 8 * 60 + 45;
const DEADLINE_MIN: i64 = 9 * 60;

/// §2 — decision. `is_at_company` or no-order-today → cancel any pending reminder; otherwise
/// schedule/fire per the §4 windows (before 08:45 = schedule at target; 08:45–08:59 = fire now;
/// at/after 09:00 = nothing).
pub fn check_cancel_reminder(
    clock: &dyn Clock,
    is_at_company: bool,
    orders: &[OrderedMenu],
) -> Option<NotificationCommand> {
    let now = clock.now_epoch_ms();
    let today = vienna_date_key(now);
    let has_order_today = orders
        .iter()
        .any(|o| vienna_date_key(o.date_epoch_ms) == today);

    // §2 step 3.
    if is_at_company || !has_order_today {
        return Some(NotificationCommand::CancelPending {
            id: CANCEL_REMINDER_ID.to_string(),
        });
    }

    // §4 scheduling windows.
    let vienna_now = chrono_tz::Europe::Vienna
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch");
    let current_min = vienna_now.hour() as i64 * 60 + vienna_now.minute() as i64;

    if current_min >= DEADLINE_MIN {
        return None; // at/past 09:00 → too late, schedule nothing
    }
    if current_min < TARGET_MIN {
        // before 08:45 → date-triggered at target (order-reminders channel).
        Some(NotificationCommand::ScheduleAt {
            id: CANCEL_REMINDER_ID.to_string(),
            title: TITLE.to_string(),
            body: BODY.to_string(),
            channel_id: Some(ORDER_REMINDERS_CHANNEL.to_string()),
            fire_at_epoch_ms: now + (TARGET_MIN - current_min) * 60_000,
            screen: Some(SCREEN.to_string()),
        })
    } else {
        // 08:45 ≤ now < 09:00 → immediate fire, NO channel (v1 passes no channelId).
        Some(NotificationCommand::FireNow {
            id: CANCEL_REMINDER_ID.to_string(),
            title: TITLE.to_string(),
            body: BODY.to_string(),
            channel_id: None,
            screen: Some(SCREEN.to_string()),
        })
    }
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

    fn order_today() -> OrderedMenu {
        OrderedMenu {
            position_id: "P".into(),
            eating_cycle_id: String::new(),
            date_epoch_ms: vienna_ms(2026, 2, 10, 12, 0),
            title: "MENÜ I".into(),
            subtitle: String::new(),
            approved: false,
        }
    }

    fn clock_at(h: u32, m: u32) -> FixedClock {
        FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, h, m),
        }
    }

    #[test]
    fn at_company_cancels() {
        let cmd = check_cancel_reminder(&clock_at(8, 0), true, &[order_today()]).unwrap();
        assert_eq!(
            cmd,
            NotificationCommand::CancelPending {
                id: CANCEL_REMINDER_ID.into()
            }
        );
    }

    #[test]
    fn no_order_today_cancels() {
        let cmd = check_cancel_reminder(&clock_at(8, 0), false, &[]).unwrap();
        assert_eq!(
            cmd,
            NotificationCommand::CancelPending {
                id: CANCEL_REMINDER_ID.into()
            }
        );
    }

    #[test]
    fn before_target_schedules_with_channel() {
        let now = vienna_ms(2026, 2, 10, 8, 0);
        let cmd =
            check_cancel_reminder(&FixedClock { epoch_ms: now }, false, &[order_today()]).unwrap();
        match cmd {
            NotificationCommand::ScheduleAt {
                id,
                title,
                body,
                channel_id,
                fire_at_epoch_ms,
                screen,
            } => {
                assert_eq!(id, CANCEL_REMINDER_ID);
                assert_eq!(title, TITLE);
                assert_eq!(body, BODY);
                assert_eq!(channel_id.as_deref(), Some(ORDER_REMINDERS_CHANNEL));
                assert_eq!(fire_at_epoch_ms, now + 45 * 60_000); // 08:00 → 08:45
                assert_eq!(screen.as_deref(), Some(SCREEN));
            }
            _ => panic!("expected ScheduleAt"),
        }
    }

    #[test]
    fn in_window_fires_now_without_channel() {
        let cmd = check_cancel_reminder(&clock_at(8, 50), false, &[order_today()]).unwrap();
        match cmd {
            NotificationCommand::FireNow { id, channel_id, .. } => {
                assert_eq!(id, CANCEL_REMINDER_ID);
                assert_eq!(channel_id, None); // immediate fire → no channel
            }
            _ => panic!("expected FireNow"),
        }
    }

    #[test]
    fn at_or_after_deadline_does_nothing() {
        assert!(check_cancel_reminder(&clock_at(9, 0), false, &[order_today()]).is_none());
        assert!(check_cancel_reminder(&clock_at(9, 30), false, &[order_today()]).is_none());
    }
}
