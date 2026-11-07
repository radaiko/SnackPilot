//! UI-facing model layer for the GPUI rewrite (design D1): the UI talks only
//! to this crate, never to `snackpilot_core::ffi` directly. Owns the core
//! session flows, the observable state hub and the `Platform` host abstraction.

mod errors;
mod fake;
mod platform;
mod runtime;
mod session;
mod state;

pub use errors::{human_error, is_demo_user};
pub use fake::FakePlatform;
pub use platform::{
    Capabilities, LocationFix, Platform, PlatformAnalytics, PlatformError, SupportEmail,
};
pub use runtime::{block_on_model, log, metrics};
pub use session::AppModel;
pub use state::{ErrorText, State, StateHub, Subscriber};
