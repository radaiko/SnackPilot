//! UI-facing model layer for the GPUI rewrite (design D1): the UI talks only
//! to this crate, never to `snackpilot_core::ffi` directly. Owns the core
//! session flows, the observable state hub and the `Platform` host abstraction.

mod billing;
pub mod desktop;
mod diagnostics;
mod errors;
mod fake;
mod flows;
mod format;
mod lifecycle;
mod menus;
pub mod navigation;
mod orders;
mod platform;
mod prefs;
mod runtime;
mod session;
mod state;
pub mod tokens;

pub use billing::{BillingEntry, BillingLine, BillingTotals, EntrySource};
pub use errors::{human_error, is_demo_user};
pub use fake::FakePlatform;
pub use format::{
    day_key_from_epoch_ms, format_date_long, format_day_short, format_eur, format_time, today_key,
};
pub use menus::{category_heading, category_rank, menu_key, nearest_day, progress_label};
pub use orders::unconfirmed_banner_text;
pub use platform::{
    Capabilities, LocationFix, Platform, PlatformAnalytics, PlatformError, SupportEmail,
};
pub use prefs::{REMINDER_PICKER_DEFAULT_HOUR, REMINDER_PICKER_DEFAULT_MINUTE};
pub use runtime::{block_on_model, log, metrics};
#[cfg(test)]
pub(crate) static BILLING_FETCHES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
pub use session::AppModel;
pub use state::{
    Alert, BillingSource, ErrorText, LaunchStage, ReminderTime, State, StateHub, Subscriber,
    ThemePreference,
};
