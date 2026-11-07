use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use snackpilot_core::notify::NotificationCommand;

use crate::platform::{Capabilities, LocationFix, Platform, PlatformError, SupportEmail};

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

    pub fn set_location(&self, fix: Option<LocationFix>) {
        *self.location.lock().unwrap() = fix;
    }

    fn check(&self, method: &str) -> Result<(), PlatformError> {
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
        Ok(true)
    }

    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.record("request_location_authorisation");
        self.check("request_location_authorisation")?;
        Ok(true)
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        self.record("current_location");
        self.check("current_location")?;
        Ok(*self.location.lock().unwrap())
    }

    fn start_geofence(&self) -> Result<(), PlatformError> {
        self.record("start_geofence");
        self.check("start_geofence")
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        self.record("stop_geofence");
        self.check("stop_geofence")
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        self.record("is_geofence_monitoring");
        self.check("is_geofence_monitoring")?;
        Ok(false)
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
}
