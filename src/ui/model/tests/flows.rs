//! Task 3.3: reminder, company-location, geofence and diagnostics flows
//! (04-ui-ux §3.8, notifications-daily-reminder §7-§9, notification-log §2).
//! Demo mode only, no network.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use snackpilot_core::domain::{CompanyLocation, GeofenceEvent};
use snackpilot_core::notify::log::ACTIVATED_UNTIL_KEY;
use snackpilot_core::notify::NotificationCommand;
use snackpilot_core::storage::{FileKv, Kv};
use snackpilot_model::{
    AppModel, FakePlatform, LocationFix, ReminderTime, REMINDER_PICKER_DEFAULT_HOUR,
    REMINDER_PICKER_DEFAULT_MINUTE,
};

/// Storage dir removed best-effort when the test ends.
struct TestDir(PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for TestDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

fn temp_dir(tag: &str) -> TestDir {
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-flows-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    TestDir(dir)
}

fn kv(dir: &Path) -> FileKv {
    FileKv::new(dir.to_path_buf())
}

/// Demo-mode signed-in model; `login` blocks on the model runtime, which is
/// safe from a plain test thread (same pattern as `tests/session.rs`).
fn signed_in(tag: &str) -> (AppModel, Arc<FakePlatform>, TestDir) {
    let dir = temp_dir(tag);
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(dir.to_path_buf(), fake.clone()).unwrap();
    model.login("demo", "demo1234!").unwrap();
    (model, fake, dir)
}

fn alert_pair(a: Option<snackpilot_model::Alert>) -> Option<(String, String)> {
    a.map(|a| (a.title, a.message))
}

// ---- location flow (AppModel.swift:716-766, 04-ui-ux §3.8 section 2) ----

// @req REQ-PLT-03
#[test]
fn location_flow_aborts_without_always_authorisation() {
    let (model, fake, _dir) = signed_in("loc_always");
    fake.set_location_permission(false);
    model.set_company_location_from_current_position();
    let s = model.state();
    // Exact v1 alert (AppModel.swift:722-729).
    assert_eq!(
        alert_pair(s.location_alert),
        Some((
            "Standort „Immer\" erforderlich".into(),
            "Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer\" \
             gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort \
             „Immer\" aus."
                .into()
        ))
    );
    assert!(!s.location_busy, "flow must not stay busy after aborting");
    assert!(model.core().company_location().is_none());
    assert!(
        fake.calls().iter().all(|c| c != "start_geofence"),
        "aborted flow must not start the geofence"
    );
}

// @req REQ-PLT-03
#[test]
fn location_flow_aborts_when_notifications_are_denied() {
    let (model, fake, _dir) = signed_in("loc_notif");
    fake.set_notification_permission(false);
    model.set_company_location_from_current_position();
    // Exact v1 alert (AppModel.swift:733-739).
    assert_eq!(
        alert_pair(model.state().location_alert),
        Some((
            "Berechtigung fehlt".into(),
            "Benachrichtigungen werden für diese Funktion benötigt. \
             Bitte in den Einstellungen aktivieren."
                .into()
        ))
    );
    assert!(model.core().company_location().is_none());
    assert!(fake.calls().iter().all(|c| c != "start_geofence"));
}

// @req REQ-PLT-03
#[test]
fn location_flow_saves_the_fix_and_starts_the_geofence() {
    let (model, fake, _dir) = signed_in("loc_ok");
    fake.set_location(Some(LocationFix {
        latitude: 48.2082,
        longitude: 16.3738,
    }));
    model.set_company_location_from_current_position();
    let s = model.state();
    let saved = CompanyLocation {
        latitude: 48.2082,
        longitude: 16.3738,
    };
    assert_eq!(model.core().company_location(), Some(saved.clone()));
    assert_eq!(s.company_location, Some(saved));
    assert!(
        fake.calls().iter().any(|c| c == "start_geofence"),
        "{:?}",
        fake.calls()
    );
    // Exact v1 alert (AppModel.swift:751-754).
    assert_eq!(
        alert_pair(s.location_alert),
        Some((
            "Gespeichert".into(),
            "Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro \
             bist und nicht bestellt hast."
                .into()
        ))
    );
    assert!(!s.location_busy);
}

// @req REQ-PLT-03
#[test]
fn location_flow_alerts_fehler_when_the_fix_fails() {
    let (model, fake, _dir) = signed_in("loc_fix_fail");
    fake.set_location(None); // FakePlatform: no fix → current_location is None
    model.set_company_location_from_current_position();
    // Exact v1 alert (AppModel.swift:759-762).
    assert_eq!(
        alert_pair(model.state().location_alert),
        Some((
            "Fehler".into(),
            "Standort konnte nicht ermittelt werden.".into()
        ))
    );
    assert!(model.core().company_location().is_none());
}

// @req REQ-PLT-03
#[test]
fn clear_company_location_stops_geofence_and_clears_core() {
    let (model, fake, _dir) = signed_in("loc_clear");
    fake.set_location(Some(LocationFix {
        latitude: 48.2,
        longitude: 16.37,
    }));
    model.set_company_location_from_current_position();
    assert!(model.core().company_location().is_some());

    model.clear_company_location();
    assert!(
        fake.calls().iter().any(|c| c == "stop_geofence"),
        "{:?}",
        fake.calls()
    );
    assert!(model.core().company_location().is_none());
    assert_eq!(model.state().company_location, None);
}

// ---- geofence handling (AppModel.swift:771-784) ----

// @req REQ-PLT-03
#[test]
fn geofence_exit_runs_core_decisions_and_updates_the_flag() {
    let (model, fake, _dir) = signed_in("geofence_exit");
    model.handle_geofence(GeofenceEvent::Exit);
    // handle_exit always cancels the pending no-order reminder first
    // (src/core/src/notify/geofence.rs:86-94).
    let notifs = fake.notifications();
    assert!(
        notifs.iter().any(|c| matches!(
            c,
            NotificationCommand::CancelPending { id } if id == "geofence-no-order-reminder"
        )),
        "{notifs:?}"
    );
    assert!(!model.core().is_at_company());
    // The log view state is refreshed with the event (AppModel.swift:783).
    let s = model.state();
    assert_eq!(s.log_active, model.core().log_is_active());

    model.handle_geofence(GeofenceEvent::Enter);
    assert!(model.core().is_at_company());
}

// ---- daily reminder (AppModel.swift:668-709, notifications §7-§9) ----

// @req REQ-PLT-06
#[test]
fn reminder_toggle_denied_leaves_switch_off_with_berechtigung_fehlt() {
    let (model, fake, dir) = signed_in("reminder_denied");
    fake.set_notification_permission(false);
    let baseline = fake.notifications().len(); // the login's own decisions
    model.set_daily_reminder_enabled(true);
    let s = model.state();
    assert!(
        !s.daily_reminder_enabled,
        "denial must leave the switch off"
    );
    assert!(!model.core().daily_reminder_settings().enabled);
    assert_eq!(
        fake.notifications().len(),
        baseline,
        "denial must not deliver a command"
    );
    // Exact v1 alert (AppModel.swift:684-688).
    assert_eq!(
        alert_pair(s.location_alert),
        Some((
            "Berechtigung fehlt".into(),
            "Benachrichtigungen werden für diese Funktion benötigt. \
             Bitte in den Einstellungen aktivieren."
                .into()
        ))
    );
    assert_ne!(
        kv(&dir).get("daily_reminder_enabled").unwrap().as_deref(),
        Some("true")
    );
    // Denial writes nothing at all: still no stored time (v1 parity,
    // notifications-daily-reminder.md:213-217).
    assert_eq!(kv(&dir).get("daily_reminder_hour").unwrap(), None);
    assert_eq!(kv(&dir).get("daily_reminder_minute").unwrap(), None);
}

// @req REQ-PLT-02
// @req REQ-PLT-06
#[test]
fn enabling_schedules_the_reminder_and_disabling_cancels_the_same_id() {
    use chrono::{Local, Timelike};
    use chrono_tz::Europe::Vienna;

    // Deterministic ScheduleAt: the target must be after "now" in
    // Europe/Vienna wall-clock (daily_reminder.rs:36-41), so pick 23:59.
    // The one minute where no future target exists is skipped explicitly.
    let vienna_now = Local::now().with_timezone(&Vienna);
    if (vienna_now.hour(), vienna_now.minute()) == (23, 59) {
        eprintln!("skipped: Vienna is at 23:59, no future reminder minute exists");
        return;
    }

    let (model, fake, dir) = signed_in("reminder_cycle");

    // An order for Vienna-today makes core decide ScheduleAt instead of
    // CancelPending (daily_reminder.rs:44-52): seed the core cache the way
    // the order flow writes it, then let the store load it.
    let today_ms = vienna_now.timestamp_millis();
    kv(&dir)
        .set(
            "orders_list",
            &format!(
                "[{{\"position_id\":\"p1\",\"eating_cycle_id\":\"c1\",\"date_epoch_ms\":{today_ms},\
                 \"title\":\"Test\",\"subtitle\":\"\",\"approved\":false}}]"
            ),
        )
        .unwrap();
    model.core().load_cached_orders();
    assert_eq!(model.core().orders().len(), 1, "seed did not load");

    // The picker time is set before enabling, so the toggle applies it
    // instead of the 11:00 picker default (AppModel.swift:668-676).
    model.set_daily_reminder_time(23, 59);
    let baseline = fake.notifications().len(); // the login's own decisions
    model.set_daily_reminder_enabled(true);
    assert!(model.state().daily_reminder_enabled);
    assert!(model.core().daily_reminder_settings().enabled);

    // Enabling delivered exactly core's ScheduleAt for the saved settings
    // (applyDailyReminder, AppModel.swift:697-708).
    let delivered = &fake.notifications()[baseline..];
    assert_eq!(delivered.len(), 1, "{delivered:?}");
    let scheduled_id = match &delivered[0] {
        NotificationCommand::ScheduleAt { id, .. } => id.clone(),
        other => panic!("expected ScheduleAt, got {other:?}"),
    };
    assert_eq!(scheduled_id, "daily-order-reminder");

    model.set_daily_reminder_enabled(false);
    assert!(!model.state().daily_reminder_enabled);
    // The cancel carries the SAME id that was scheduled.
    let cancel_id = match fake.notifications().last().expect("a command") {
        NotificationCommand::CancelPending { id } => id.clone(),
        other => panic!("expected CancelPending, got {other:?}"),
    };
    assert_eq!(cancel_id, scheduled_id);
    assert_eq!(
        kv(&dir).get("daily_reminder_enabled").unwrap().as_deref(),
        Some("false")
    );
}

// @req REQ-PLT-02
#[test]
fn reminder_time_persists_under_its_own_keys_and_in_core() {
    let (model, _fake, dir) = signed_in("reminder_time");
    model.set_daily_reminder_enabled(true);
    // Enabling with no stored time uses the picker default 11:00
    // (AppModel.swift:63, notifications-daily-reminder.md:213-217).
    let kv = kv(&dir);
    assert_eq!(
        kv.get("daily_reminder_hour").unwrap().as_deref(),
        Some("11")
    );
    assert_eq!(
        kv.get("daily_reminder_minute").unwrap().as_deref(),
        Some("0")
    );
    assert_eq!(
        model.state().reminder_time,
        Some(ReminderTime {
            hour: REMINDER_PICKER_DEFAULT_HOUR,
            minute: REMINDER_PICKER_DEFAULT_MINUTE
        })
    );

    model.set_daily_reminder_time(13, 30);
    assert_eq!(
        kv.get("daily_reminder_hour").unwrap().as_deref(),
        Some("13")
    );
    assert_eq!(
        kv.get("daily_reminder_minute").unwrap().as_deref(),
        Some("30")
    );
    // Core's own loader (headless background callers, §9) sees the time too.
    let s = model.core().daily_reminder_settings();
    assert_eq!((s.hour, s.minute), (Some(13), Some(30)));
    assert_eq!(
        model.state().reminder_time,
        Some(ReminderTime {
            hour: 13,
            minute: 30
        })
    );
}

// @req REQ-PLT-06
#[test]
fn out_of_range_reminder_times_are_rejected() {
    // v1's picker only produces 11:00-13:45 (04-ui-ux §3.8), so an
    // out-of-range call is garbage: it must not persist or reach the state.
    let (model, _fake, dir) = signed_in("reminder_range");
    model.set_daily_reminder_time(11, 0);
    model.set_daily_reminder_time(24, 0); // hour out of range
    model.set_daily_reminder_time(23, 60); // minute out of range
    assert_eq!(
        model.state().reminder_time,
        Some(ReminderTime {
            hour: 11,
            minute: 0
        }),
        "out-of-range sets must be ignored"
    );
    let kv = kv(&dir);
    assert_eq!(
        kv.get("daily_reminder_hour").unwrap().as_deref(),
        Some("11")
    );
    assert_eq!(
        kv.get("daily_reminder_minute").unwrap().as_deref(),
        Some("0")
    );
    let s = model.core().daily_reminder_settings();
    assert_eq!((s.hour, s.minute), (Some(11), Some(0)));

    // The valid boundaries are accepted.
    model.set_daily_reminder_time(0, 0);
    assert_eq!(
        model.state().reminder_time,
        Some(ReminderTime { hour: 0, minute: 0 })
    );
    model.set_daily_reminder_time(23, 59);
    assert_eq!(
        model.state().reminder_time,
        Some(ReminderTime {
            hour: 23,
            minute: 59
        })
    );
    assert_eq!(
        kv.get("daily_reminder_hour").unwrap().as_deref(),
        Some("23")
    );
    assert_eq!(
        kv.get("daily_reminder_minute").unwrap().as_deref(),
        Some("59")
    );
}

// @req REQ-PLT-06
#[test]
fn stored_garbage_reminder_time_loads_as_none() {
    // A hand-edited/corrupt store must not surface an impossible time
    // (load falls back to "not configured", like core's loader, cache.rs:64).
    for (hour, minute) in [("99", "0"), ("11", "75")] {
        let dir = temp_dir(&format!("reminder_garbage_{hour}_{minute}"));
        let kv = kv(&dir);
        kv.set("daily_reminder_hour", hour).unwrap();
        kv.set("daily_reminder_minute", minute).unwrap();
        let model = AppModel::new(dir.to_path_buf(), Arc::new(FakePlatform::new())).unwrap();
        assert_eq!(
            model.state().reminder_time,
            None,
            "hour={hour} minute={minute} must load as None"
        );
    }
}

// ---- diagnostics (AppModel.swift:788-801, notification-log §2, 04-ui-ux §3.8) ----

// @req REQ-SCR-08
#[test]
fn diagnostic_log_state_machine() {
    let (model, _fake, dir) = signed_in("log_states");
    let s = model.state();
    assert!(!s.log_active && !s.log_expired && s.log_status.is_empty());

    model.activate_diagnostic_log(24);
    let s = model.state();
    assert!(s.log_active && !s.log_expired);
    assert!(
        s.log_status.starts_with("Aufzeichnung läuft bis "),
        "{}",
        s.log_status
    );
    assert!(!s.log_status.contains('\n'), "status is a single line");
    assert!(
        !s.log_status.contains("Einträge"),
        "no entry suffix at n = 0: {}",
        s.log_status
    );
    let until_24: i64 = kv(&dir)
        .get(ACTIVATED_UNTIL_KEY)
        .unwrap()
        .unwrap()
        .parse()
        .unwrap();

    // 12 h activates for strictly less time than 24 h (AppModel.swift:792-795).
    model.activate_diagnostic_log(12);
    assert!(model.state().log_active);
    let until_12: i64 = kv(&dir)
        .get(ACTIVATED_UNTIL_KEY)
        .unwrap()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        until_12 < until_24,
        "12 h expiry {until_12} must be earlier than 24 h expiry {until_24}"
    );

    // Active with entries: the status carries the " (n Einträge)" suffix
    // (diagnostics.rs:46). Activation wipes entries (log.rs:46), so seed
    // them the way core stores them (log.rs:8) and reload the view state.
    kv(&dir)
        .set(
            snackpilot_core::notify::log::ENTRIES_KEY,
            "[{\"ts\":\"2026-11-07T10:00:00Z\",\"subsystem\":\"geofence\",\"level\":\"info\",\"event\":\"enter\"},\
             {\"ts\":\"2026-11-07T10:00:01Z\",\"subsystem\":\"order-sync\",\"level\":\"guard\",\"event\":\"skip\"}]",
        )
        .unwrap();
    model.refresh_diagnostic_log();
    let s = model.state();
    assert!(s.log_active);
    assert!(
        s.log_status.ends_with(" (2 Einträge)"),
        "entry-count suffix missing: {}",
        s.log_status
    );

    model.clear_diagnostic_log();
    let s = model.state();
    assert!(!s.log_active && !s.log_expired && s.log_status.is_empty());

    // Expired: seed a past expiry the way core stores it (log.rs:9), then
    // refresh — the screen reloads log state on every focus.
    kv(&dir).set(ACTIVATED_UNTIL_KEY, "1").unwrap();
    model.refresh_diagnostic_log();
    let s = model.state();
    assert!(!s.log_active && s.log_expired);
    assert_eq!(s.log_status, "Aufzeichnung abgeschlossen (0 Einträge).");
}
