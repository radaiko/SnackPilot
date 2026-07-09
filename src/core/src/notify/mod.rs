//! Notification decision logic — the portable "which notification should fire" halves of
//! v1's notification subsystem (03-features/notifications-*). The core computes commands;
//! the native shells deliver them (docs/architecture §3.5).
pub mod cancel_reminder;
pub mod daily_reminder;
pub mod fingerprint;

/// Fixed notification identifiers (dedupe = re-issue with the same id replaces the pending one).
pub const DAILY_REMINDER_ID: &str = "daily-order-reminder";
pub const CANCEL_REMINDER_ID: &str = "cancel-order-reminder";
pub const GEOFENCE_REMINDER_ID: &str = "geofence-no-order-reminder";

/// Android channels (05-platform-services §5.3).
pub const ORDER_REMINDERS_CHANNEL: &str = "order-reminders";
pub const MENU_UPDATES_CHANNEL: &str = "menu-updates";

/// The delivery contract between core decisions and the shell's OS notification APIs
/// (docs/architecture §4.2). The shell executes these 1:1; re-issuing an id replaces the
/// pending notification with that id.
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationCommand {
    /// Schedule a local notification to fire at `fire_at_epoch_ms`. `channel_id` is `None`
    /// when v1 attaches no Android channel (falls back to the OS default).
    ScheduleAt {
        id: String,
        title: String,
        body: String,
        channel_id: Option<String>,
        fire_at_epoch_ms: i64,
        screen: Option<String>,
    },
    /// Deliver now. `channel_id` is `None` for v1's immediate-fire paths (no channel).
    FireNow {
        id: String,
        title: String,
        body: String,
        channel_id: Option<String>,
        screen: Option<String>,
    },
    /// Cancel any pending notification with `id`.
    CancelPending { id: String },
}

/// Daily-reminder settings (03-features/notifications-daily-reminder §3).
#[derive(Debug, Clone, PartialEq)]
pub struct DailyReminderSettings {
    pub enabled: bool,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
}
