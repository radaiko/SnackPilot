//! Desktop host bridge (openspec: gpui-kit-ui-rewrite, design D4).
//!
//! Desktop runs the same UI as the phones but has no notification, location or
//! geofence services, so those calls answer with a typed `PlatformError`
//! (REQ-PLT-06) and [`DesktopPlatform::capabilities`] reports both
//! capabilities off (REQ-PLT-07). Credentials use the OS keychain through the
//! `keyring` crate under the service `app:no-auth` (REQ-PLT-01), the same
//! slots as the mobile bridges. Analytics are a no-op: desktop sends nothing.

use std::collections::HashMap;

use snackpilot_core::notify::NotificationCommand;
use snackpilot_model::{Capabilities, LocationFix, Platform, PlatformError, SupportEmail};

/// Keyring service for the four credential slots (REQ-PLT-01).
pub const SERVICE: &str = "app:no-auth";

/// Desktop implementation: OS keychain credentials, everything else
/// unavailable or a no-op.
#[derive(Debug, Default, Clone, Copy)]
pub struct DesktopPlatform;

impl DesktopPlatform {
    pub fn new() -> Self {
        Self
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, PlatformError> {
        keyring::Entry::new(SERVICE, key).map_err(|e| PlatformError::Storage(e.to_string()))
    }
}

impl Platform for DesktopPlatform {
    /// The four credential slots (`gourmet_username`, `gourmet_password`,
    /// `ventopay_username`, `ventopay_password`) read from the OS keychain
    /// under service `app:no-auth` (REQ-PLT-01). A missing entry is `Ok(None)`.
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(PlatformError::Storage(e.to_string())),
        }
    }

    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| PlatformError::Storage(e.to_string()))
    }

    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(PlatformError::Storage(e.to_string())),
        }
    }

    /// Desktop has no notification service (REQ-PLT-06).
    fn notify(&self, _command: NotificationCommand) -> Result<(), PlatformError> {
        Err(PlatformError::Notification(
            "desktop has no notification service".to_string(),
        ))
    }

    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        Err(PlatformError::Notification(
            "desktop has no notification service".to_string(),
        ))
    }

    /// Desktop has no location or geofence service (REQ-PLT-06).
    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        Err(PlatformError::Location(
            "desktop has no location service".to_string(),
        ))
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        Err(PlatformError::Location(
            "desktop has no location service".to_string(),
        ))
    }

    fn start_geofence(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Location(
            "desktop has no geofence service".to_string(),
        ))
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        Err(PlatformError::Location(
            "desktop has no geofence service".to_string(),
        ))
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        Err(PlatformError::Location(
            "desktop has no geofence service".to_string(),
        ))
    }

    /// Desktop sends no analytics (REQ-PLT-06: infallible no-op).
    fn track(&self, _event: &str, _props: HashMap<String, String>) -> Result<(), PlatformError> {
        Ok(())
    }

    fn set_app_icon(&self, _accent: &str) -> Result<(), PlatformError> {
        Ok(())
    }

    fn open_app_settings(&self) -> Result<(), PlatformError> {
        Ok(())
    }

    fn compose_email(&self, _email: SupportEmail) -> Result<(), PlatformError> {
        Ok(())
    }

    /// No notifications, no geofence: the Benachrichtigungen row and the
    /// location section stay hidden on desktop (REQ-PLT-07).
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            notifications: false,
            geofence: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};
    use keyring::mock::MockCredential;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    /// The four credential slots (REQ-PLT-01).
    const KEYS: [&str; 4] = [
        "gourmet_username",
        "gourmet_password",
        "ventopay_username",
        "ventopay_password",
    ];

    /// keyring's own in-memory mock credential, shared between the `Entry`
    /// instances a test builds for the same (service, user) slot, so values
    /// persist across entries like a real keystore.
    struct SharedMock(Arc<MockCredential>);

    impl CredentialApi for SharedMock {
        fn set_secret(&self, secret: &[u8]) -> keyring::Result<()> {
            self.0.set_secret(secret)
        }

        fn get_secret(&self) -> keyring::Result<Vec<u8>> {
            self.0.get_secret()
        }

        fn delete_credential(&self) -> keyring::Result<()> {
            self.0.delete_credential()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Mock credential builder over `keyring::mock`: caches one
    /// `MockCredential` per (service, user) so the whole test process shares
    /// one in-memory keychain.
    #[derive(Default)]
    struct MockStoreBuilder(Mutex<HashMap<String, Arc<MockCredential>>>);

    impl CredentialBuilderApi for MockStoreBuilder {
        fn build(
            &self,
            _target: Option<&str>,
            service: &str,
            user: &str,
        ) -> keyring::Result<Box<Credential>> {
            let mut store = self.0.lock().unwrap();
            let credential = store
                .entry(format!("{service}\u{0}{user}"))
                .or_insert_with(|| Arc::new(MockCredential::default()))
                .clone();
            Ok(Box::new(SharedMock(credential)))
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// Install the keyring mock store as the default builder once per test
    /// process. When `SNACKPILOT_REAL_KEYCHAIN=1` it is never installed, so
    /// the real-keychain test below reaches the actual OS keychain instead.
    fn fake_keychain() {
        assert!(
            std::env::var("SNACKPILOT_REAL_KEYCHAIN").is_err(),
            "SNACKPILOT_REAL_KEYCHAIN is set: run only the real-keychain test with --exact"
        );
        static INIT: OnceLock<()> = OnceLock::new();
        INIT.get_or_init(|| {
            keyring::set_default_credential_builder(Box::new(MockStoreBuilder::default()));
        });
    }

    /// The credential tests share one in-memory keychain and use the same
    /// exact slot names, so they run one at a time.
    fn credential_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// @req REQ-PLT-01 The four credential slots round-trip through the
    /// keyring service `app:no-auth`: missing reads as `Ok(None)`, save then
    /// read returns the value, clear removes it, and clearing a missing slot
    /// is not an error.
    #[test]
    fn credential_slots_round_trip() {
        let _lock = credential_lock();
        fake_keychain();
        let platform = DesktopPlatform::new();
        for (i, key) in KEYS.iter().enumerate() {
            assert_eq!(
                platform.get_credential(key).unwrap(),
                None,
                "{key} starts empty"
            );
            let value = format!("value-{i}");
            platform.save_credential(key, &value).unwrap();
            assert_eq!(
                platform.get_credential(key).unwrap().as_deref(),
                Some(value.as_str()),
                "{key} reads back what was saved"
            );
            platform.clear_credential(key).unwrap();
            assert_eq!(
                platform.get_credential(key).unwrap(),
                None,
                "{key} is gone after clear"
            );
            platform.clear_credential(key).unwrap();
        }
    }

    /// @req REQ-PLT-01 Credentials live under the service `app:no-auth`, not
    /// under any other service name. The service string is written out in
    /// full here so the `SERVICE` constant itself is under test.
    #[test]
    fn credentials_use_the_app_no_auth_service() {
        let _lock = credential_lock();
        fake_keychain();
        let platform = DesktopPlatform::new();
        platform
            .save_credential("gourmet_username", "user")
            .unwrap();
        let in_service = keyring::Entry::new("app:no-auth", "gourmet_username")
            .unwrap()
            .get_password()
            .unwrap();
        assert_eq!(in_service, "user");
        let other = keyring::Entry::new("some-other-service", "gourmet_username")
            .unwrap()
            .get_password();
        assert!(matches!(other, Err(keyring::Error::NoEntry)));
        platform.clear_credential("gourmet_username").unwrap();
    }

    /// @req REQ-PLT-06 Notification calls answer with a typed
    /// `PlatformError::Notification`, never a panic or a silent success.
    #[test]
    fn notifications_are_unavailable() {
        let platform = DesktopPlatform::new();
        assert!(matches!(
            platform.request_notification_permission(),
            Err(PlatformError::Notification(_))
        ));
        let command = NotificationCommand::FireNow {
            id: "test".to_string(),
            title: "t".to_string(),
            body: "b".to_string(),
            channel_id: None,
            screen: None,
        };
        assert!(matches!(
            platform.notify(command),
            Err(PlatformError::Notification(_))
        ));
    }

    /// @req REQ-PLT-06 Location and geofence calls answer with a typed
    /// `PlatformError::Location`.
    #[test]
    fn location_and_geofence_are_unavailable() {
        let platform = DesktopPlatform::new();
        assert!(matches!(
            platform.request_location_authorisation(),
            Err(PlatformError::Location(_))
        ));
        assert!(matches!(
            platform.current_location(),
            Err(PlatformError::Location(_))
        ));
        assert!(matches!(
            platform.start_geofence(),
            Err(PlatformError::Location(_))
        ));
        assert!(matches!(
            platform.stop_geofence(),
            Err(PlatformError::Location(_))
        ));
        assert!(matches!(
            platform.is_geofence_monitoring(),
            Err(PlatformError::Location(_))
        ));
    }

    /// @req REQ-PLT-06 Analytics and system actions are infallible no-ops on
    /// desktop.
    #[test]
    fn no_op_services_succeed() {
        let platform = DesktopPlatform::new();
        platform.track("menu_viewed", HashMap::new()).unwrap();
        platform.set_app_icon("green").unwrap();
        platform.open_app_settings().unwrap();
        platform
            .compose_email(SupportEmail {
                to: "support@example.com".to_string(),
                subject: "s".to_string(),
                body: "b".to_string(),
            })
            .unwrap();
    }

    /// @req REQ-PLT-07 Desktop reports no notifications and no geofence, so
    /// the UI hides the Benachrichtigungen row and the location section.
    #[test]
    fn capabilities_report_no_notifications_and_no_geofence() {
        assert_eq!(
            DesktopPlatform::new().capabilities(),
            Capabilities {
                notifications: false,
                geofence: false,
            }
        );
    }

    /// Round trip against the real OS keychain, run manually on a desktop
    /// with `SNACKPILOT_REAL_KEYCHAIN=1 cargo test -p snackpilot-app
    /// -- platform::desktop::tests::real_keychain -- --exact`. Uses a
    /// dedicated evidence account; never touches the four real slots.
    #[test]
    fn real_keychain_round_trip() {
        if std::env::var("SNACKPILOT_REAL_KEYCHAIN").is_err() {
            return;
        }
        const ACCOUNT: &str = "dev.radaiko.gourmetclient.desktop-evidence";
        let platform = DesktopPlatform::new();
        platform.save_credential(ACCOUNT, "test-user").unwrap();
        assert_eq!(
            platform.get_credential(ACCOUNT).unwrap().as_deref(),
            Some("test-user")
        );
        platform.save_credential(ACCOUNT, "test-pass").unwrap();
        assert_eq!(
            platform.get_credential(ACCOUNT).unwrap().as_deref(),
            Some("test-pass")
        );
        platform.clear_credential(ACCOUNT).unwrap();
        assert_eq!(platform.get_credential(ACCOUNT).unwrap(), None);
    }
}
