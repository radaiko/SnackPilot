use std::collections::HashMap;

use snackpilot_core::notify::NotificationCommand;

/// Why a [`Platform`] call failed.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum PlatformError {
    #[error("notification failed: {0}")]
    Notification(String),
    #[error("credential storage failed: {0}")]
    Storage(String),
    #[error("location failed: {0}")]
    Location(String),
    #[error("system action failed: {0}")]
    System(String),
}

/// What the host device supports, per design D4.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub notifications: bool,
    pub geofence: bool,
}

/// A one-shot GPS fix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocationFix {
    pub latitude: f64,
    pub longitude: f64,
}

/// A pre-filled email for the "contact support" flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// The host bridge, per design D4: the model never touches iOS or Android
/// APIs directly, and every call answers with a typed `Result` (REQ-PLT-06).
pub trait Platform: Send + Sync {
    /// The four credential slots are `gourmet_username`, `gourmet_password`,
    /// `ventopay_username` and `ventopay_password` (REQ-PLT-01). Each slot
    /// holds a single value: the username goes under the `*_username` key,
    /// the password under the `*_password` key.
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError>;
    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError>;
    fn clear_credential(&self, key: &str) -> Result<(), PlatformError>;

    fn notify(&self, command: NotificationCommand) -> Result<(), PlatformError>;
    fn request_notification_permission(&self) -> Result<bool, PlatformError>;

    fn request_location_authorisation(&self) -> Result<bool, PlatformError>;
    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError>;
    fn start_geofence(&self) -> Result<(), PlatformError>;
    fn stop_geofence(&self) -> Result<(), PlatformError>;
    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError>;

    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError>;
    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError>;
    fn open_app_settings(&self) -> Result<(), PlatformError>;
    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError>;

    fn capabilities(&self) -> Capabilities;
}

/// Forwards core analytics to [`Platform::track`]; a failing platform never
/// breaks the core (the sink contract is infallible).
pub struct PlatformAnalytics(pub std::sync::Arc<dyn Platform>);

impl snackpilot_core::features::AnalyticsSink for PlatformAnalytics {
    fn track(&self, event: String, props: HashMap<String, String>) {
        let _ = self.0.track(&event, props);
    }
}
