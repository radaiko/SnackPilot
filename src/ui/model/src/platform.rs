use std::collections::HashMap;
use std::path::{Path, PathBuf};

use snackpilot_core::notify::NotificationCommand;

use crate::update::VerifiedManifest;

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

/// The running desktop host as the updater sees it (design D6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateHostInfo {
    /// `macos` or `windows`.
    pub platform: &'static str,
    /// `arm64` or `x64`.
    pub architecture: &'static str,
    /// The OS version when known; `None` never blocks an update.
    pub os_version: Option<String>,
    /// The installed version, e.g. `2.2.0`.
    pub installed_version: String,
    /// The installed build, e.g. `41`.
    pub installed_build: String,
}

/// A staged update the host has verified and may hand off (REQ-UPD-05/06).
/// The model only ever names it; the files belong to the host's staging directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedUpdate {
    /// The version the update installs, e.g. `2.3.0`.
    pub version: String,
    /// The staged, integrity-checked artifact.
    pub artifact: PathBuf,
    /// What the host prepared from the artifact (the extracted bundle on macOS).
    pub prepared: Option<PathBuf>,
}

/// What a successful handoff did: the model's wording follows (REQ-UPD-07/08/09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffOutcome {
    /// The installer runs and the app must quit.
    QuitRequested,
    /// The bundle is revealed in Finder; the app keeps running.
    Revealed,
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
    /// Arm the single company region. `trigger_on_entry` asks the OS for the
    /// initial Enter when the device is already inside — only the setup path
    /// wants that (notifications-location §3.3); a launch restore must not
    /// re-fire it (§3.2).
    fn start_geofence(&self, trigger_on_entry: bool) -> Result<(), PlatformError>;
    fn stop_geofence(&self) -> Result<(), PlatformError>;
    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError>;

    /// Register (or re-register) the OS background task that re-evaluates the
    /// reminders while the app is closed. Idempotent: the task has one id, so a
    /// second call replaces the first. Hosts without background work accept it
    /// as a no-op.
    fn register_background_sync(&self) -> Result<(), PlatformError> {
        Ok(())
    }

    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError>;
    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError>;
    fn open_app_settings(&self) -> Result<(), PlatformError>;
    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError>;

    fn capabilities(&self) -> Capabilities;

    // ---- desktop update hooks (design D6) ----
    // The default bodies keep every mobile bridge and a host without update
    // support compiling and silent: no update traffic ever leaves them.
    /// `Some` only on macOS arm64 and Windows x64. Never reads the network.
    fn update_target(&self) -> Option<UpdateHostInfo> {
        None
    }

    /// A fresh, empty directory for one update's files.
    fn update_staging_dir(&self, _version: &str) -> Result<PathBuf, PlatformError> {
        Err(PlatformError::System("unsupported".into()))
    }

    /// The platform step between hash success and `Ready` (macOS bundle trust);
    /// Windows needs none. The host deletes its staging directory on failure.
    fn verify_update_artifact(
        &self,
        _artifact: &Path,
        _manifest: &VerifiedManifest,
    ) -> Result<VerifiedUpdate, PlatformError> {
        Err(PlatformError::System("unsupported".into()))
    }

    /// Starts the installer or reveals the bundle (REQ-UPD-08/09).
    fn hand_off_update(&self, _update: &VerifiedUpdate) -> Result<HandoffOutcome, PlatformError> {
        Err(PlatformError::System("unsupported".into()))
    }
}

/// Forwards core analytics to [`Platform::track`]; a failing platform never
/// breaks the core (the sink contract is infallible).
pub struct PlatformAnalytics(pub std::sync::Arc<dyn Platform>);

impl snackpilot_core::features::AnalyticsSink for PlatformAnalytics {
    fn track(&self, event: String, props: HashMap<String, String>) {
        let _ = self.0.track(&event, props);
    }
}
