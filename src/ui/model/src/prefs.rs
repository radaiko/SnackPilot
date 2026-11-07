//! Appearance and reminder preferences: loading, persisting and applying them
//! (themes §1.2/§6, notifications-daily-reminder §9).

use snackpilot_core::notify::{DailyReminderSettings, NotificationCommand};
use snackpilot_core::storage::Kv;

use crate::state::{AccentColor, Alert, ReminderTime, ThemePreference};
use crate::AppModel;

/// KV keys, exactly as the iOS `AppModel` persists them (AppModel.swift:647,659;
/// the reminder time/enabled additionally mirror into core's own keys so its
/// headless background callers read the same settings, §9).
pub(crate) const THEME_KEY: &str = "theme_preference";
pub(crate) const ACCENT_KEY: &str = "accent_color";
pub(crate) const REMINDER_ENABLED_KEY: &str = "daily_reminder_enabled";
pub(crate) const REMINDER_HOUR_KEY: &str = "daily_reminder_hour";
pub(crate) const REMINDER_MINUTE_KEY: &str = "daily_reminder_minute";

/// The settings picker's initial time before anything is stored (v1 parity:
/// no stored/default time until first enable,
/// notifications-daily-reminder.md:213-217; AppModel.swift:63).
pub const REMINDER_PICKER_DEFAULT_HOUR: u8 = 11;
/// Minutes counterpart of [`REMINDER_PICKER_DEFAULT_HOUR`].
pub const REMINDER_PICKER_DEFAULT_MINUTE: u8 = 0;

impl AppModel {
    /// Reads the appearance keys at startup; defaults are System and Orange
    /// (REQ-UI-05). Never calls the platform, so startup never switches the icon.
    pub(crate) fn load_preferences(&self) {
        let theme = self
            .kv
            .get(THEME_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();
        let accent = self
            .kv
            .get(ACCENT_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or_default();
        self.hub.update("preferences", |s| {
            s.theme = theme;
            s.accent = accent;
        });
    }

    /// Persists the theme (AppModel.swift:646-651); independent of the accent,
    /// no icon side effect (REQ-UI-05).
    pub fn set_theme_preference(&self, theme: ThemePreference) {
        let _ = self.kv.set(THEME_KEY, theme.raw_value());
        self.hub.update("theme", |s| s.theme = theme);
    }

    /// Persists the accent and switches the app icon — the only icon call site,
    /// mirroring v1 (AppModel.swift:655-659, themes §6; REQ-UI-05, REQ-PLT-05).
    pub fn set_accent_color(&self, accent: AccentColor) {
        if self.hub.get().accent == accent {
            return;
        }
        let _ = self.kv.set(ACCENT_KEY, accent.raw_value());
        self.hub.update("accent", |s| s.accent = accent);
        let _ = self.platform.set_app_icon(accent.raw_value());
    }

    /// Reads the reminder keys at startup; the reminder is off and there is no
    /// time until the feature is first enabled (v1 parity,
    /// notifications-daily-reminder.md:213-217; REQ-PLT-06).
    pub(crate) fn load_reminder_state(&self) {
        let enabled = self.kv.get(REMINDER_ENABLED_KEY).ok().flatten().as_deref() == Some("true");
        // Out-of-range stored values are garbage, not a time: fall back to
        // "not configured" (same spirit as core's loader, cache.rs:64-85).
        let time = self
            .kv
            .get(REMINDER_HOUR_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|h| *h < 24)
            .zip(
                self.kv
                    .get(REMINDER_MINUTE_KEY)
                    .ok()
                    .flatten()
                    .and_then(|v| v.parse::<u8>().ok())
                    .filter(|m| *m < 60),
            )
            .map(|(hour, minute)| ReminderTime { hour, minute });
        self.hub.update("reminder_state", |s| {
            s.daily_reminder_enabled = enabled;
            s.reminder_time = time;
        });
    }

    /// Toggle entry point. Off goes straight through (Swift `setDailyReminder`);
    /// on goes through the permission-gated path (Swift `requestEnableDailyReminder`,
    /// AppModel.swift:680-695): notification permission first, on denial the §7.1
    /// alert and nothing is written, so the switch stays off (REQ-PLT-06).
    pub fn set_daily_reminder_enabled(&self, enabled: bool) {
        if !enabled {
            self.disable_daily_reminder();
            return;
        }
        let granted = self
            .platform
            .request_notification_permission()
            .unwrap_or(false);
        if !granted {
            self.hub.update("reminder_denied", |s| {
                s.location_alert = Some(Alert::denied());
            });
            return;
        }
        let time = self.hub.get().reminder_time.unwrap_or(ReminderTime {
            hour: REMINDER_PICKER_DEFAULT_HOUR,
            minute: REMINDER_PICKER_DEFAULT_MINUTE,
        });
        // §9 step 2: time before the enabled flag, mirrored to core so headless
        // background callers read the same settings.
        self.write_reminder_time_keys(Some(time));
        self.core.set_daily_reminder_time(time.hour, time.minute);
        self.core.set_daily_reminder_enabled(true);
        self.hub.update("reminder_on", |s| {
            s.daily_reminder_enabled = true;
            s.reminder_time = Some(time);
        });
        self.apply_daily_reminder();
    }

    /// Persists a new reminder time and re-applies the schedule while enabled
    /// (Swift picker → `setDailyReminder`, AppModel.swift:668-676; REQ-PLT-06).
    /// v1's picker only produces valid chips (04-ui-ux §3.8), so an
    /// out-of-range call is garbage and is rejected without any write.
    pub fn set_daily_reminder_time(&self, hour: u8, minute: u8) {
        if hour > 23 || minute > 59 {
            return;
        }
        self.write_reminder_time_keys(Some(ReminderTime { hour, minute }));
        self.core.set_daily_reminder_time(hour, minute);
        self.hub.update("reminder_time", |s| {
            s.reminder_time = Some(ReminderTime { hour, minute });
        });
        if self.hub.get().daily_reminder_enabled {
            self.apply_daily_reminder();
        }
    }

    /// Writes the v1-only hour/minute keys (AppModel.swift:54-57). The enabled
    /// flag is NOT written here: core's `set_daily_reminder_enabled` persists
    /// the same `daily_reminder_enabled` key with the same "true"/"false"
    /// values (cache.rs:94-99), and core stores the time under its own
    /// `daily_reminder_time` JSON key (cache.rs:100-103), which the model
    /// never reads.
    fn write_reminder_time_keys(&self, time: Option<ReminderTime>) {
        if let Some(t) = time {
            let _ = self.kv.set(REMINDER_HOUR_KEY, &t.hour.to_string());
            let _ = self.kv.set(REMINDER_MINUTE_KEY, &t.minute.to_string());
        }
    }

    fn disable_daily_reminder(&self) {
        // Disabling keeps any stored time; before a first enable there is none
        // to keep (notifications-daily-reminder.md:213-217). The enabled key is
        // persisted by core below.
        self.core.set_daily_reminder_enabled(false);
        self.hub.update("reminder_off", |s| {
            s.daily_reminder_enabled = false;
        });
        self.apply_daily_reminder();
    }

    /// Ask core for the command given the persisted settings and deliver it;
    /// when disabled, cancel the pending reminder by id — Swift `applyDailyReminder`
    /// (AppModel.swift:697-708; REQ-PLT-02, REQ-PLT-06).
    pub fn apply_daily_reminder(&self) {
        if !self.hub.get().daily_reminder_enabled {
            let _ = self.platform.notify(NotificationCommand::CancelPending {
                id: "daily-order-reminder".into(),
            });
            return;
        }
        // One source of truth with the headless callers: read the settings back
        // from core (AppModel.swift:700).
        let settings = self.core.daily_reminder_settings();
        if let Some(cmd) = self.core.daily_reminder_command(settings.clone()) {
            let _ = self.platform.notify(cmd);
        }
    }

    /// The command core currently schedules for the saved reminder settings,
    /// `None` when the reminder is off or core sees no reason to fire.
    pub fn daily_reminder_command(&self) -> Option<NotificationCommand> {
        if !self.hub.get().daily_reminder_enabled {
            return None;
        }
        let s = self.core.daily_reminder_settings();
        self.core.daily_reminder_command(DailyReminderSettings {
            enabled: true,
            hour: s.hour,
            minute: s.minute,
        })
    }
}
