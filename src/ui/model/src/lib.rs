//! UI-facing model layer for the GPUI rewrite (design D1): the UI talks only
//! to this crate, never to `snackpilot_core::ffi` directly. Owns the core
//! session flows, the observable state hub and the `Platform` host abstraction.

mod billing;
mod errors;
mod fake;
mod format;
mod menus;
mod orders;
mod platform;
mod runtime;
mod session;
mod state;

pub use billing::{BillingEntry, BillingLine, BillingTotals, EntrySource};
pub use errors::{human_error, is_demo_user};
pub use fake::FakePlatform;
pub use format::{
    day_key_from_epoch_ms, format_date_long, format_day_short, format_eur, format_time, today_key,
};
pub use menus::{category_heading, category_rank, nearest_day, progress_label};
pub use orders::unconfirmed_banner_text;
pub use platform::{
    Capabilities, LocationFix, Platform, PlatformAnalytics, PlatformError, SupportEmail,
};
pub use runtime::{block_on_model, log, metrics};
pub use session::AppModel;
pub use state::{BillingSource, ErrorText, State, StateHub, Subscriber};
