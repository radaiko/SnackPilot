//! Location geofence "no order" notification + Enter/Exit decision logic
//! (03-features/notifications-location §4-§5). The shell's CoreLocation/GeofencingClient
//! delivers Enter/Exit events into the core; the core returns the command sequence to run
//! and the shell updates the persisted `is_at_company` flag (Enter→true, Exit→false).
use crate::datetime::{vienna_date_key, Clock};
use crate::domain::{GeofenceEvent, OrderedMenu};
use crate::notify::cancel_reminder::check_cancel_reminder;
use crate::notify::{NotificationCommand, GEOFENCE_REMINDER_ID, ORDER_REMINDERS_CHANNEL};
use chrono::{TimeZone, Timelike};

const TITLE: &str = "SnackPilot";
const BODY: &str = "Du bist im Büro, hast aber noch nicht bestellt!";
/// 08:45 target, 14:00 "too late" cutoff (Vienna minutes-since-midnight).
const TARGET_MIN: i64 = 8 * 60 + 45;
const TOO_LATE_MIN: i64 = 14 * 60;

/// §5 timing — the "no order" notification. ≥14:00 → None; <08:45 → schedule at target
/// (order-reminders channel); 08:45–13:59 → fire now (no channel).
pub fn no_order_notification(clock: &dyn Clock) -> Option<NotificationCommand> {
    let now = clock.now_epoch_ms();
    let vienna_now = chrono_tz::Europe::Vienna
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch");
    let current_min = vienna_now.hour() as i64 * 60 + vienna_now.minute() as i64;

    if current_min >= TOO_LATE_MIN {
        return None; // too late in the day
    }
    if current_min < TARGET_MIN {
        Some(NotificationCommand::ScheduleAt {
            id: GEOFENCE_REMINDER_ID.to_string(),
            title: TITLE.to_string(),
            body: BODY.to_string(),
            channel_id: Some(ORDER_REMINDERS_CHANNEL.to_string()),
            fire_at_epoch_ms: now + (TARGET_MIN - current_min) * 60_000,
            screen: None, // no deep link (§5 content)
        })
    } else {
        Some(NotificationCommand::FireNow {
            id: GEOFENCE_REMINDER_ID.to_string(),
            title: TITLE.to_string(),
            body: BODY.to_string(),
            channel_id: None, // immediate fire → no channel
            screen: None,
        })
    }
}

/// §4 — dispatch a geofence event to the command sequence the shell must execute.
/// (The shell has already loaded cached orders and updates `is_at_company`.)
pub fn handle_geofence_event(
    event: GeofenceEvent,
    clock: &dyn Clock,
    orders: &[OrderedMenu],
) -> Vec<NotificationCommand> {
    match event {
        GeofenceEvent::Enter => handle_enter(clock, orders),
        GeofenceEvent::Exit => handle_exit(clock, orders),
    }
}

/// ENTER (§4): cancel any pending cancel-reminder (is_at_company is now true), then — if no
/// order for today — schedule/fire the "no order" notification.
fn handle_enter(clock: &dyn Clock, orders: &[OrderedMenu]) -> Vec<NotificationCommand> {
    let mut cmds = Vec::new();
    // step 4: checkCancelReminder with is_at_company=true always cancels.
    if let Some(c) = check_cancel_reminder(clock, true, orders) {
        cmds.push(c);
    }
    // steps 5-6: only when there is no order for today.
    let today = vienna_date_key(clock.now_epoch_ms());
    let has_order_today = orders
        .iter()
        .any(|o| vienna_date_key(o.date_epoch_ms) == today);
    if !has_order_today {
        if let Some(c) = no_order_notification(clock) {
            cmds.push(c);
        }
    }
    cmds
}

/// EXIT (§4): cancel the pending "no order" notification, then run the cancel-reminder check
/// with is_at_company=false (schedules/fires the cancel reminder if an order exists today).
fn handle_exit(clock: &dyn Clock, orders: &[OrderedMenu]) -> Vec<NotificationCommand> {
    let mut cmds = vec![NotificationCommand::CancelPending {
        id: GEOFENCE_REMINDER_ID.to_string(),
    }];
    if let Some(c) = check_cancel_reminder(clock, false, orders) {
        cmds.push(c);
    }
    cmds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;
    use crate::notify::CANCEL_REMINDER_ID;

    fn vienna_ms(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }
    fn clock_at(h: u32, m: u32) -> FixedClock {
        FixedClock {
            epoch_ms: vienna_ms(2026, 2, 10, h, m),
        }
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

    #[test]
    fn no_order_notification_windows() {
        // 14:01 → skip; 10:00 → immediate; 08:00 → scheduled (§5 boundary tests).
        assert!(no_order_notification(&clock_at(14, 1)).is_none());
        assert!(matches!(
            no_order_notification(&clock_at(10, 0)),
            Some(NotificationCommand::FireNow { .. })
        ));
        let now = vienna_ms(2026, 2, 10, 8, 0);
        match no_order_notification(&FixedClock { epoch_ms: now }).unwrap() {
            NotificationCommand::ScheduleAt {
                fire_at_epoch_ms,
                channel_id,
                id,
                ..
            } => {
                assert_eq!(id, GEOFENCE_REMINDER_ID);
                assert_eq!(fire_at_epoch_ms, now + 45 * 60_000);
                assert_eq!(channel_id.as_deref(), Some(ORDER_REMINDERS_CHANNEL));
            }
            _ => panic!("expected ScheduleAt"),
        }
    }

    #[test]
    fn exactly_1400_skips() {
        assert!(no_order_notification(&clock_at(14, 0)).is_none());
    }

    #[test]
    fn enter_no_order_cancels_cancel_reminder_and_schedules_no_order() {
        let cmds = handle_geofence_event(GeofenceEvent::Enter, &clock_at(8, 0), &[]);
        // cancel cancel-reminder + schedule geofence no-order.
        assert!(cmds.iter().any(|c| matches!(
            c,
            NotificationCommand::CancelPending { id } if id == CANCEL_REMINDER_ID
        )));
        assert!(cmds.iter().any(|c| matches!(
            c,
            NotificationCommand::ScheduleAt { id, .. } if id == GEOFENCE_REMINDER_ID
        )));
    }

    #[test]
    fn enter_with_order_only_cancels_cancel_reminder() {
        let cmds = handle_geofence_event(GeofenceEvent::Enter, &clock_at(8, 0), &[order_today()]);
        assert_eq!(cmds.len(), 1);
        assert!(matches!(
            &cmds[0],
            NotificationCommand::CancelPending { id } if id == CANCEL_REMINDER_ID
        ));
    }

    #[test]
    fn exit_cancels_geofence_then_schedules_cancel_reminder_when_order_today() {
        let cmds = handle_geofence_event(GeofenceEvent::Exit, &clock_at(8, 0), &[order_today()]);
        assert!(matches!(
            &cmds[0],
            NotificationCommand::CancelPending { id } if id == GEOFENCE_REMINDER_ID
        ));
        assert!(cmds.iter().any(|c| matches!(
            c,
            NotificationCommand::ScheduleAt { id, .. } if id == CANCEL_REMINDER_ID
        )));
    }
}
