use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;

use snackpilot_core::notify::NotificationCommand;

use crate::platform::{
    Capabilities, HandoffOutcome, LocationFix, Platform, PlatformError, SupportEmail,
    UpdateHostInfo, VerifiedUpdate,
};

/// Test double that records every call and keeps credentials in memory, one
/// single value per slot (REQ-PLT-01). Call names look like
/// `save_credential(gourmet_username)`; `fail("notify")` makes every
/// matching call return a typed error, persistently until the fake is
/// dropped (REQ-PLT-06).
#[derive(Default)]
pub struct FakePlatform {
    calls: Mutex<Vec<String>>,
    store: Mutex<HashMap<String, String>>,
    notifications: Mutex<Vec<NotificationCommand>>,
    tracks: Mutex<Vec<(String, HashMap<String, String>)>>,
    failing: Mutex<HashSet<String>>,
    capabilities: Capabilities,
    location: Mutex<Option<LocationFix>>,
    /// The `trigger_on_entry` flag of every `start_geofence` call, in order.
    geofence_triggers: Mutex<Vec<bool>>,
    /// Whether a region is currently registered, as the OS would report it.
    monitoring: Mutex<bool>,
    // Permission grants default to true; tests deny them to drive the
    // §7.1/§7.2 alert branches.
    deny_location: Mutex<bool>,
    deny_notification: Mutex<bool>,
    /// Remaining calls a method fails through [`FakePlatform::fail_next`].
    fail_counts: Mutex<HashMap<String, usize>>,
    // Update support is off unless a test names a host (REQ-UPD-13).
    update_host: Mutex<Option<UpdateHostInfo>>,
    update_staging: Mutex<Option<PathBuf>>,
    /// What `verify_update_artifact` answers, and what the verified update says.
    update_verified: Mutex<Option<VerifiedUpdate>>,
    update_handoff: Mutex<Option<HandoffOutcome>>,
    /// The `version` of every `update_staging_dir` call, in order.
    staged_versions: Mutex<Vec<String>>,
    handed_off: Mutex<Vec<VerifiedUpdate>>,
}

impl FakePlatform {
    pub fn new() -> Self {
        Self {
            capabilities: Capabilities {
                notifications: true,
                geofence: true,
            },
            ..Default::default()
        }
    }

    fn record(&self, call: &str) {
        self.calls.lock().unwrap().push(call.to_string());
    }

    /// Make calls whose recorded name starts with `method` return an error.
    pub fn fail(&self, method: &str) {
        self.failing.lock().unwrap().insert(method.to_string());
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    pub fn credential(&self, key: &str) -> Option<String> {
        self.store.lock().unwrap().get(key).cloned()
    }

    pub fn notifications(&self) -> Vec<NotificationCommand> {
        self.notifications.lock().unwrap().clone()
    }

    pub fn tracks(&self) -> Vec<(String, HashMap<String, String>)> {
        self.tracks.lock().unwrap().clone()
    }

    /// Whether each `start_geofence` call asked for the initial Enter: setup
    /// passes `true`, a launch restore `false` (§3.2/§3.3).
    pub fn geofence_triggers(&self) -> Vec<bool> {
        self.geofence_triggers.lock().unwrap().clone()
    }

    pub fn set_location(&self, fix: Option<LocationFix>) {
        *self.location.lock().unwrap() = fix;
    }

    /// Deny (or re-grant) the location permission check (REQ-PLT-03).
    pub fn set_location_permission(&self, granted: bool) {
        *self.deny_location.lock().unwrap() = !granted;
    }

    /// Deny (or re-grant) the notification permission check (REQ-PLT-06).
    pub fn set_notification_permission(&self, granted: bool) {
        *self.deny_notification.lock().unwrap() = !granted;
    }

    /// Make the next `count` calls of `method` fail, then succeed again.
    pub fn fail_next(&self, method: &str, count: usize) {
        self.fail_counts
            .lock()
            .unwrap()
            .insert(method.to_string(), count);
    }

    /// Report `info` as a desktop host the updater supports (REQ-UPD-13).
    pub fn set_update_host(&self, info: Option<UpdateHostInfo>) {
        *self.update_host.lock().unwrap() = info;
    }

    /// The directory `update_staging_dir` answers with.
    pub fn set_update_staging(&self, dir: Option<PathBuf>) {
        *self.update_staging.lock().unwrap() = dir;
    }

    /// A verification still in flight answers with this; it is used once, so a
    /// later download really verifies again. `None` fails like an untrusted bundle.
    pub fn set_update_verified(&self, verified: Option<VerifiedUpdate>) {
        *self.update_verified.lock().unwrap() = verified;
    }

    /// What a successful handoff reports (REQ-UPD-07).
    pub fn set_update_handoff(&self, outcome: HandoffOutcome) {
        *self.update_handoff.lock().unwrap() = Some(outcome);
    }

    /// The versions whose staging directory was asked for, in order.
    pub fn staged_versions(&self) -> Vec<String> {
        self.staged_versions.lock().unwrap().clone()
    }

    /// Every update handed off to this host, in order.
    pub fn handed_off(&self) -> Vec<VerifiedUpdate> {
        self.handed_off.lock().unwrap().clone()
    }

    fn check(&self, method: &str) -> Result<(), PlatformError> {
        let mut counts = self.fail_counts.lock().unwrap();
        for (name, remaining) in counts.iter_mut() {
            if method.starts_with(name.as_str()) && *remaining > 0 {
                *remaining -= 1;
                return Err(PlatformError::System(format!("{method} failed")));
            }
        }
        drop(counts);
        if self
            .failing
            .lock()
            .unwrap()
            .iter()
            .any(|f| method.starts_with(f))
        {
            return Err(PlatformError::System(format!("{method} failed")));
        }
        Ok(())
    }
}

impl Platform for FakePlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        self.record(&format!("get_credential({key})"));
        self.check("get_credential")?;
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        self.record(&format!("save_credential({key})"));
        self.check("save_credential")?;
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        self.record(&format!("clear_credential({key})"));
        self.check("clear_credential")?;
        self.store.lock().unwrap().remove(key);
        Ok(())
    }

    fn notify(&self, command: NotificationCommand) -> Result<(), PlatformError> {
        self.record("notify");
        self.check("notify")?;
        self.notifications.lock().unwrap().push(command);
        Ok(())
    }

    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        self.record("request_notification_permission");
        self.check("request_notification_permission")?;
        Ok(!*self.deny_notification.lock().unwrap())
    }

    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.record("request_location_authorisation");
        self.check("request_location_authorisation")?;
        Ok(!*self.deny_location.lock().unwrap())
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        self.record("current_location");
        self.check("current_location")?;
        Ok(*self.location.lock().unwrap())
    }

    fn start_geofence(&self, trigger_on_entry: bool) -> Result<(), PlatformError> {
        self.record("start_geofence");
        self.geofence_triggers
            .lock()
            .unwrap()
            .push(trigger_on_entry);
        self.check("start_geofence")?;
        // The real hosts report what is actually registered, so the fake does
        // too: a successful arm is what `is_geofence_monitoring` then sees.
        *self.monitoring.lock().unwrap() = true;
        Ok(())
    }

    fn register_background_sync(&self) -> Result<(), PlatformError> {
        self.record("register_background_sync");
        self.check("register_background_sync")
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        self.record("stop_geofence");
        self.check("stop_geofence")?;
        *self.monitoring.lock().unwrap() = false;
        Ok(())
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        self.record("is_geofence_monitoring");
        self.check("is_geofence_monitoring")?;
        Ok(*self.monitoring.lock().unwrap())
    }

    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError> {
        self.record(&format!("track({event})"));
        self.check("track")?;
        self.tracks.lock().unwrap().push((event.to_string(), props));
        Ok(())
    }

    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError> {
        self.record(&format!("set_app_icon({accent})"));
        self.check("set_app_icon")
    }

    fn open_app_settings(&self) -> Result<(), PlatformError> {
        self.record("open_app_settings");
        self.check("open_app_settings")
    }

    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError> {
        self.record(&format!("compose_email({})", email.to));
        self.check("compose_email")
    }

    fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    fn update_target(&self) -> Option<UpdateHostInfo> {
        self.record("update_target");
        self.update_host.lock().unwrap().clone()
    }

    fn update_staging_dir(&self, version: &str) -> Result<PathBuf, PlatformError> {
        self.record(&format!("update_staging_dir({version})"));
        self.check("update_staging_dir")?;
        self.staged_versions
            .lock()
            .unwrap()
            .push(version.to_string());
        self.update_staging
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| PlatformError::System("unsupported".into()))
    }

    fn verify_update_artifact(
        &self,
        artifact: &std::path::Path,
        manifest: &crate::update::VerifiedManifest,
    ) -> Result<VerifiedUpdate, PlatformError> {
        self.record(&format!("verify_update_artifact({})", artifact.display()));
        self.check("verify_update_artifact")?;
        let _ = manifest;
        self.update_verified
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| PlatformError::System("untrusted bundle".into()))
    }

    fn hand_off_update(&self, update: &VerifiedUpdate) -> Result<HandoffOutcome, PlatformError> {
        self.record(&format!("hand_off_update({})", update.version));
        self.check("hand_off_update")?;
        let outcome = *self
            .update_handoff
            .lock()
            .unwrap()
            .as_ref()
            .ok_or_else(|| PlatformError::System("handoff failed".into()))?;
        self.handed_off.lock().unwrap().push(update.clone());
        Ok(outcome)
    }
}
