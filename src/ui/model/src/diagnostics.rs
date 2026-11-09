//! Notification diagnostic-log view state (notification-log §2, 04-ui-ux §3.8),
//! Swift `refreshLog`/`activateLog`/`clearLog` (AppModel.swift:788-801).

use chrono::DateTime;
use chrono_tz::Europe::Vienna;

use snackpilot_core::notify::log::ACTIVATED_UNTIL_KEY;
use snackpilot_core::storage::Kv;

use crate::AppModel;

impl AppModel {
    /// Activate the log for `hours` (12 or 24, the two choices the settings
    /// screen offers) and refresh the view state (AppModel.swift:792-795).
    pub fn activate_diagnostic_log(&self, hours: u32) {
        self.core.log_activate(hours);
        self.refresh_diagnostic_log();
    }

    /// Delete the log and refresh the view state (AppModel.swift:797-800).
    pub fn clear_diagnostic_log(&self) {
        self.core.log_clear();
        self.refresh_diagnostic_log();
    }

    /// Recompute `log_active`/`log_expired`/`log_status` from core and the KV
    /// (AppModel.swift:788-791; the screen reloads this on every focus).
    ///
    /// States (04-ui-ux §3.8): active → `Aufzeichnung läuft bis {dd.MM., HH:mm}`
    /// plus ` ({n} Einträge)` only when n > 0; expired (a past expiry is still
    /// stored) → `Aufzeichnung abgeschlossen ({n} Einträge).`; never activated →
    /// inactive with an empty status.
    pub fn refresh_diagnostic_log(&self) {
        let active = self.core.log_is_active();
        let entries = self.core.log_entries().len();
        let until = self
            .kv
            .get(ACTIVATED_UNTIL_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok());
        let expired = !active && until.is_some();
        let status = match (active, until, expired) {
            (true, Some(until), _) => {
                let base = format!("Aufzeichnung läuft bis {}", format_expiry(until));
                if entries > 0 {
                    format!("{base} ({entries} Einträge)")
                } else {
                    base
                }
            }
            (false, _, true) => format!("Aufzeichnung abgeschlossen ({entries} Einträge)."),
            _ => String::new(),
        };
        self.hub.update("diagnostic_log", |s| {
            s.log_active = active;
            s.log_expired = expired;
            s.log_status = status;
        });
    }

    /// Reads the persisted location and at-company flag at startup, so a relaunch
    /// shows the saved company location (notifications-location §1).
    pub(crate) fn load_location_state(&self) {
        let location = self.core.company_location();
        let at = self.core.is_at_company();
        self.hub.update("location_state", |s| {
            s.company_location = location;
            s.is_at_company = at;
        });
    }
}

/// Expiry epoch-ms as Vienna local `dd.MM., HH:mm` (04-ui-ux §3.8).
fn format_expiry(epoch_ms: i64) -> String {
    let Some(utc) = DateTime::<chrono::Utc>::from_timestamp_millis(epoch_ms) else {
        return epoch_ms.to_string();
    };
    utc.with_timezone(&Vienna)
        .format("%d.%m., %H:%M")
        .to_string()
}

/// Plain-text export of the log (§4 of notification-log): one block per entry,
/// `[ts] [subsystem] [LEVEL] event`, plus `\n  detail` when present.
fn format_log_entries(entries: &[snackpilot_core::notify::log::LogEntry]) -> String {
    use snackpilot_core::notify::log::{LogLevel, LogSubsystem};
    if entries.is_empty() {
        return "(keine Einträge aufgezeichnet)".to_string();
    }
    entries
        .iter()
        .map(|e| {
            let subsystem = match e.subsystem {
                LogSubsystem::Geofence => "geofence",
                LogSubsystem::OrderSync => "order-sync",
                LogSubsystem::DailyReminder => "daily-reminder",
                LogSubsystem::MenuCheck => "menu-check",
            };
            let level = match e.level {
                LogLevel::Info => "INFO",
                LogLevel::Guard => "GUARD",
                LogLevel::Error => "ERROR",
                LogLevel::Notification => "NOTIFICATION",
            };
            let mut block = format!("[{}] [{}] [{}] {}", e.ts, subsystem, level, e.event);
            if let Some(detail) = &e.detail {
                block.push_str(&format!("\n  {detail}"));
            }
            block
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl AppModel {
    /// The §3.8 "Log per E-Mail senden" composer payload: recipient, subject
    /// with the full de-AT expiry, body = the plain-text export.
    pub fn diagnostic_log_email(&self) -> crate::platform::SupportEmail {
        let entries = self.core.log_entries();
        let until = self
            .kv
            .get(ACTIVATED_UNTIL_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok());
        let expiry = until
            .and_then(DateTime::<chrono::Utc>::from_timestamp_millis)
            .map(|utc| {
                utc.with_timezone(&Vienna)
                    .format("%d.%m.%Y, %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        crate::platform::SupportEmail {
            to: "aiko@spitzbub.app".to_string(),
            subject: format!("SnackPilot Notification Log (bis {expiry})"),
            body: format_log_entries(&entries),
        }
    }

    /// Open the system mail composer with the log; failures surface as the
    /// §3.8 `Fehler` alert at the call site.
    pub fn send_diagnostic_log(&self) -> Result<(), crate::PlatformError> {
        self.platform.compose_email(self.diagnostic_log_email())
    }
}

#[cfg(test)]
mod email_tests {
    use super::format_log_entries;
    use snackpilot_core::notify::log::{LogEntry, LogLevel, LogSubsystem};

    /// @req REQ-SCR-08
    #[test]
    fn email_export_matches_the_v1_format() {
        assert_eq!(format_log_entries(&[]), "(keine Einträge aufgezeichnet)");
        let entries = vec![
            LogEntry {
                ts: "2026-01-01T08:00:00Z".into(),
                subsystem: LogSubsystem::Geofence,
                level: LogLevel::Info,
                event: "enter".into(),
                detail: None,
            },
            LogEntry {
                ts: "2026-01-01T08:05:00Z".into(),
                subsystem: LogSubsystem::OrderSync,
                level: LogLevel::Error,
                event: "fail".into(),
                detail: Some("status=500".into()),
            },
        ];
        assert_eq!(
            format_log_entries(&entries),
            "[2026-01-01T08:00:00Z] [geofence] [INFO] enter\n\
             [2026-01-01T08:05:00Z] [order-sync] [ERROR] fail\n  status=500"
        );
    }
}
