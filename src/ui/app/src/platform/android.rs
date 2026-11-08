//! Android [`Platform`] over JNI (task 3.4, REQ-PLT-01..06).
//!
//! The Kotlin shims under `src/ui/android/app/src/main/java/dev/radaiko/
//! snackpilot/` own the real Android APIs; this file marshals across the JNI
//! boundary only. The marshalled types are plain data ([`WireCommand`]), never
//! `uniffi.snackpilot_core` types — the Kotlin side has no UniFFI dependency.
//!
//! Design (design.md D4): every Rust→Kotlin call goes through a static method
//! on `dev.radaiko.snackpilot.PlatformBridge`, whose class reference (plus the
//! plain-data classes it consumes) is cached from Kotlin's main thread via
//! `nativeCacheClass` — a `FindClass` from a Rust-created thread would use the
//! system classloader and fail. Async results (permission dialog, one-shot
//! fix) resolve through `nativeResolvePermission` / `nativeResolveLocation`
//! matched to request ids handed out by [`Pending`].
//!
//! The real JNI paths are `cfg(target_os = "android")` only; the generic
//! [`AndroidPlatform<B>`] over the [`Bridge`] trait is host-testable with a
//! mock adapter (REQ-PLT-06).

use std::collections::HashMap;

use snackpilot_core::notify::NotificationCommand;
use snackpilot_model::{Capabilities, LocationFix, Platform, PlatformError, SupportEmail};

/// Command marshalled to the Kotlin `CommandData` (mirrors
/// `NotificationCommand` minus the uniffi types; `channel_id` empty = none,
/// Kotlin falls back to the default channel; `fire_at_epoch_ms` unused by
/// FireNow/CancelPending).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireCommand {
    /// 0 = FireNow, 1 = ScheduleAt, 2 = CancelPending (Kotlin `CommandData.kind`).
    pub kind: u8,
    pub id: String,
    pub title: String,
    pub body: String,
    pub channel_id: String,
    pub fire_at_epoch_ms: i64,
}

/// Marshal a core [`NotificationCommand`] into the plain-data wire form
/// (REQ-PLT-02): the `screen` field is UI-only and does not cross the boundary.
pub fn wire_command(command: &NotificationCommand) -> WireCommand {
    match command {
        NotificationCommand::FireNow {
            id,
            title,
            body,
            channel_id,
            ..
        } => WireCommand {
            kind: 0,
            id: id.clone(),
            title: title.clone(),
            body: body.clone(),
            channel_id: channel_id.clone().unwrap_or_default(),
            fire_at_epoch_ms: 0,
        },
        NotificationCommand::ScheduleAt {
            id,
            title,
            body,
            channel_id,
            fire_at_epoch_ms,
            ..
        } => WireCommand {
            kind: 1,
            id: id.clone(),
            title: title.clone(),
            body: body.clone(),
            channel_id: channel_id.clone().unwrap_or_default(),
            fire_at_epoch_ms: *fire_at_epoch_ms,
        },
        NotificationCommand::CancelPending { id } => WireCommand {
            kind: 2,
            id: id.clone(),
            title: String::new(),
            body: String::new(),
            channel_id: String::new(),
            fire_at_epoch_ms: 0,
        },
    }
}

/// The Kotlin surface this bridge calls; one method per `PlatformBridge`
/// static. Errors are the Java-side message, mapped to a typed
/// [`PlatformError`] variant by [`AndroidPlatform`].
pub trait Bridge: Send + Sync {
    fn save_secret(&self, key: &str, value: &str) -> Result<(), String>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
    fn delete_secret(&self, key: &str) -> Result<(), String>;
    /// Hand the dialog to Kotlin, then block on `pending` for the resolve.
    fn request_permission(&self, permission: &str, request_id: u64) -> Result<(), String>;
    fn request_location(&self, request_id: u64) -> Result<(), String>;
    fn execute_command(&self, command: &WireCommand) -> Result<(), String>;
    fn start_geofence(
        &self,
        latitude: f64,
        longitude: f64,
        trigger_on_entry: bool,
    ) -> Result<(), String>;
    fn stop_geofence(&self) -> Result<(), String>;
    fn is_geofence_monitoring(&self) -> Result<bool, String>;
    fn track(&self, event: &str, props: &HashMap<String, String>) -> Result<(), String>;
    fn set_app_icon(&self, accent: &str) -> Result<(), String>;
    fn open_app_settings(&self) -> Result<(), String>;
    fn compose_email(&self, email: &SupportEmail) -> Result<(), String>;
}

/// The [`Platform`] implementation, generic over the bridge so host unit
/// tests can drive it with a mock adapter.
pub struct AndroidPlatform<B: Bridge> {
    bridge: B,
    pending: &'static Pending,
    storage_dir: String,
}

impl<B: Bridge> AndroidPlatform<B> {
    /// `pending` must outlive the platform (the process-wide static on
    /// Android; a test-owned static on the host). `storage_dir` is the core
    /// KV directory, read for the saved company location (REQ-PLT-03).
    pub fn new(bridge: B, pending: &'static Pending, storage_dir: String) -> Self {
        Self {
            bridge,
            pending,
            storage_dir,
        }
    }

    /// Shared permission flow: register the id first, hand the dialog to
    /// Kotlin, then wait for the resolve (granted flag or typed timeout).
    fn request_permission(&self, permission: &str) -> Result<bool, PlatformError> {
        let id = self.pending.begin();
        self.bridge
            .request_permission(permission, id)
            .map_err(PlatformError::Notification)?;
        match self.pending.await_slot(id)? {
            Slot::Permission(granted) => Ok(granted),
            other => Err(PlatformError::System(format!(
                "unexpected resolve for permission request {id}: {other:?}"
            ))),
        }
    }
}

/// Outcome a request id is waiting on.
#[derive(Debug)]
enum Slot {
    Waiting,
    Permission(bool),
    Location(Option<LocationFix>),
}

/// Request-id registry for the two async flows (permission dialog, one-shot
/// fix). Kotlin calls back into `nativeResolve*` with the id it was handed;
/// a request that never resolves fails with a typed timeout instead of
/// hanging the caller (REQ-PLT-06).
pub struct Pending {
    next_id: std::sync::atomic::AtomicU64,
    slots: std::sync::Mutex<HashMap<u64, Slot>>,
    ready: std::sync::Condvar,
    timeout: std::time::Duration,
}

impl Pending {
    /// Production timeout: long enough for a user to answer a dialog.
    pub fn new() -> Self {
        Self::with_timeout(std::time::Duration::from_secs(30))
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            next_id: std::sync::atomic::AtomicU64::new(1),
            slots: std::sync::Mutex::new(HashMap::new()),
            ready: std::sync::Condvar::new(),
            timeout,
        }
    }

    /// Register a request before handing it to Kotlin, so a resolve that
    /// lands synchronously inside the bridge call is not lost.
    fn begin(&'static self) -> u64 {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.slots.lock().unwrap().insert(id, Slot::Waiting);
        id
    }

    fn resolve(&'static self, id: u64, slot: Slot) {
        if let Some(entry) = self.slots.lock().unwrap().get_mut(&id) {
            *entry = slot;
        }
        self.ready.notify_all();
    }

    /// Entry point of `nativeResolvePermission`.
    pub fn resolve_permission(&'static self, id: u64, granted: bool) {
        self.resolve(id, Slot::Permission(granted));
    }

    /// Entry point of `nativeResolveLocation`.
    pub fn resolve_location(&'static self, id: u64, latitude: f64, longitude: f64, found: bool) {
        self.resolve(
            id,
            Slot::Location(found.then_some(LocationFix {
                latitude,
                longitude,
            })),
        );
    }

    /// Block until `id` resolves or the timeout fires; either way the slot
    /// is dropped so the registry cannot grow without bound.
    fn await_slot(&'static self, id: u64) -> Result<Slot, PlatformError> {
        let deadline = std::time::Instant::now() + self.timeout;
        let mut slots = self.slots.lock().unwrap();
        loop {
            let done = matches!(slots.get(&id), Some(Slot::Permission(_)))
                || matches!(slots.get(&id), Some(Slot::Location(_)));
            if done {
                return match slots.remove(&id) {
                    Some(Slot::Permission(granted)) => Ok(Slot::Permission(granted)),
                    Some(Slot::Location(fix)) => Ok(Slot::Location(fix)),
                    _ => Err(PlatformError::System("request vanished".into())),
                };
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                slots.remove(&id);
                return Err(PlatformError::System(format!(
                    "platform request {id} timed out after {:?}",
                    self.timeout
                )));
            }
            (slots, _) = self.ready.wait_timeout(slots, left).unwrap();
        }
    }
}

impl Default for Pending {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Bridge> Platform for AndroidPlatform<B> {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        self.bridge.get_secret(key).map_err(PlatformError::Storage)
    }

    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        self.bridge
            .save_secret(key, value)
            .map_err(PlatformError::Storage)
    }

    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        self.bridge
            .delete_secret(key)
            .map_err(PlatformError::Storage)
    }

    fn notify(&self, command: NotificationCommand) -> Result<(), PlatformError> {
        self.bridge
            .execute_command(&wire_command(&command))
            .map_err(PlatformError::Notification)
    }

    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        self.request_permission("notifications")
    }

    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.request_permission("location")
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        let id = self.pending.begin();
        self.bridge
            .request_location(id)
            .map_err(PlatformError::Location)?;
        match self.pending.await_slot(id)? {
            Slot::Location(fix) => Ok(fix),
            other => Err(PlatformError::System(format!(
                "unexpected resolve for location request {id}: {other:?}"
            ))),
        }
    }

    fn start_geofence(&self) -> Result<(), PlatformError> {
        // v1 parity: the coordinates are the captured company location in
        // the core KV store, registered with an entry trigger.
        let kv = snackpilot_core::storage::FileKv::new(std::path::PathBuf::from(&self.storage_dir));
        let loc = snackpilot_core::storage::cache::load_company_location(&kv)
            .ok_or_else(|| PlatformError::Location("no company location saved".into()))?;
        self.bridge
            .start_geofence(loc.latitude, loc.longitude, true)
            .map_err(PlatformError::Location)
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        self.bridge.stop_geofence().map_err(PlatformError::Location)
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        self.bridge
            .is_geofence_monitoring()
            .map_err(PlatformError::Location)
    }

    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError> {
        self.bridge
            .track(event, &props)
            .map_err(PlatformError::System)
    }

    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError> {
        self.bridge
            .set_app_icon(accent)
            .map_err(PlatformError::System)
    }

    fn open_app_settings(&self) -> Result<(), PlatformError> {
        self.bridge
            .open_app_settings()
            .map_err(PlatformError::System)
    }

    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError> {
        self.bridge
            .compose_email(&email)
            .map_err(PlatformError::System)
    }

    fn capabilities(&self) -> Capabilities {
        // Android declares both subsystems (design D4).
        Capabilities {
            notifications: true,
            geofence: true,
        }
    }
}

/// Message shape for a Java exception surfaced through the JNI bridge: it
/// always carries the Java-side message, plus the underlying JNI error when
/// the call itself failed (REQ-PLT-06; pure so the host tests pin it, used
/// by the android-only `checked` helper).
#[cfg(any(test, target_os = "android"))]
fn java_exception_message(java: &str, jni: Option<&str>) -> String {
    match jni {
        Some(e) => format!("Java exception: {java} (jni: {e})"),
        None => format!("Java exception: {java}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Call {
        method: String,
        args: Vec<String>,
    }

    /// JNI-free mock adapter: records every [`Bridge`] call, returns canned
    /// results, and can simulate a never-resolving async request.
    struct MockBridge {
        calls: Mutex<Vec<Call>>,
        secret: Mutex<Option<String>>,
        fail: Mutex<Option<String>>,
        /// When set, the next failing call returns the exact error shape the
        /// android `checked` helper produces for a JNI call that failed while
        /// a Java exception was pending.
        java_exception: Mutex<Option<String>>,
        location: Mutex<Option<(f64, f64)>>,
        monitoring: Mutex<Option<bool>>,
        never_resolve: Mutex<bool>,
    }

    impl MockBridge {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                secret: Mutex::new(None),
                fail: Mutex::new(None),
                java_exception: Mutex::new(None),
                location: Mutex::new(None),
                monitoring: Mutex::new(None),
                never_resolve: Mutex::new(false),
            }
        }
        fn take_calls(&self) -> Vec<Call> {
            std::mem::take(&mut *self.calls.lock().unwrap())
        }
        fn record(&self, method: &str, args: Vec<String>) {
            self.calls.lock().unwrap().push(Call {
                method: method.into(),
                args,
            });
        }
        fn err(&self) -> Result<(), String> {
            if let Some(java) = self.java_exception.lock().unwrap().take() {
                return Err(java_exception_message(&java, Some("call_static_method")));
            }
            match self.fail.lock().unwrap().take() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    impl Bridge for MockBridge {
        fn save_secret(&self, key: &str, value: &str) -> Result<(), String> {
            self.record("save", vec![key.into(), value.into()]);
            *self.secret.lock().unwrap() = Some(value.into());
            self.err()
        }
        fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
            self.record("get", vec![key.into()]);
            self.err()?;
            Ok(self.secret.lock().unwrap().clone())
        }
        fn delete_secret(&self, key: &str) -> Result<(), String> {
            self.record("delete", vec![key.into()]);
            *self.secret.lock().unwrap() = None;
            self.err()
        }
        fn request_permission(&self, permission: &str, request_id: u64) -> Result<(), String> {
            self.record(
                "permission",
                vec![permission.into(), request_id.to_string()],
            );
            self.err()?;
            if !*self.never_resolve.lock().unwrap() {
                // The Kotlin resolve callback would land here; answer immediately.
                self.pending().resolve_permission(request_id, true);
            }
            Ok(())
        }
        fn request_location(&self, request_id: u64) -> Result<(), String> {
            self.record("location", vec![request_id.to_string()]);
            self.err()?;
            if !*self.never_resolve.lock().unwrap() {
                let fix = *self.location.lock().unwrap();
                self.pending().resolve_location(
                    request_id,
                    fix.map(|f| f.0).unwrap_or(0.0),
                    fix.map(|f| f.1).unwrap_or(0.0),
                    fix.is_some(),
                );
            }
            Ok(())
        }
        fn execute_command(&self, command: &WireCommand) -> Result<(), String> {
            self.record(
                "execute",
                vec![
                    command.kind.to_string(),
                    command.id.clone(),
                    command.title.clone(),
                    command.body.clone(),
                    command.channel_id.clone(),
                    command.fire_at_epoch_ms.to_string(),
                ],
            );
            self.err()
        }
        fn start_geofence(
            &self,
            latitude: f64,
            longitude: f64,
            trigger_on_entry: bool,
        ) -> Result<(), String> {
            self.record(
                "start_geofence",
                vec![
                    latitude.to_string(),
                    longitude.to_string(),
                    trigger_on_entry.to_string(),
                ],
            );
            self.err()
        }
        fn stop_geofence(&self) -> Result<(), String> {
            self.record("stop_geofence", vec![]);
            self.err()
        }
        fn is_geofence_monitoring(&self) -> Result<bool, String> {
            self.record("is_geofence_monitoring", vec![]);
            self.err()?;
            Ok(self.monitoring.lock().unwrap().unwrap_or(false))
        }
        fn track(&self, event: &str, props: &HashMap<String, String>) -> Result<(), String> {
            let mut args = vec![event.into()];
            args.extend(props.iter().map(|(k, v)| format!("{k}={v}")));
            self.record("track", args);
            self.err()
        }
        fn set_app_icon(&self, accent: &str) -> Result<(), String> {
            self.record("set_app_icon", vec![accent.into()]);
            self.err()
        }
        fn open_app_settings(&self) -> Result<(), String> {
            self.record("open_app_settings", vec![]);
            self.err()
        }
        fn compose_email(&self, email: &SupportEmail) -> Result<(), String> {
            self.record(
                "compose_email",
                vec![email.to.clone(), email.subject.clone(), email.body.clone()],
            );
            self.err()
        }
    }

    // The mock resolves synchronously inside the request call, so it needs a
    // handle on the same Pending the platform waits on.
    impl MockBridge {
        fn pending(&self) -> &'static Pending {
            test_pending()
        }
    }
    // Process-wide, short-timeout registry shared by platform and mock; ids are
    // unique via the atomic counter, so parallel tests cannot collide.
    static TEST_PENDING: std::sync::OnceLock<Pending> = std::sync::OnceLock::new();
    fn test_pending() -> &'static Pending {
        TEST_PENDING.get_or_init(|| Pending::with_timeout(std::time::Duration::from_millis(200)))
    }

    fn platform(bridge: MockBridge) -> AndroidPlatform<MockBridge> {
        AndroidPlatform::new(bridge, test_pending(), String::new())
    }

    // ---- wire marshalling -------------------------------------------------

    /// @req REQ-PLT-02 FireNow marshals to kind 0 with channel; no channel
    /// becomes the empty string (Kotlin default-channel fallback).
    #[test]
    fn wire_command_fire_now() {
        let w = wire_command(&NotificationCommand::FireNow {
            id: "daily-order-reminder".into(),
            title: "Bestellen?".into(),
            body: "Menü wählen".into(),
            channel_id: Some("order-reminders".into()),
            screen: None,
        });
        assert_eq!(
            w,
            WireCommand {
                kind: 0,
                id: "daily-order-reminder".into(),
                title: "Bestellen?".into(),
                body: "Menü wählen".into(),
                channel_id: "order-reminders".into(),
                fire_at_epoch_ms: 0,
            }
        );
        let none = wire_command(&NotificationCommand::FireNow {
            id: "x".into(),
            title: "t".into(),
            body: "b".into(),
            channel_id: None,
            screen: None,
        });
        assert_eq!(none.channel_id, "");
    }

    /// @req REQ-PLT-02 ScheduleAt marshals to kind 1 and keeps the fire time.
    #[test]
    fn wire_command_schedule_at() {
        let w = wire_command(&NotificationCommand::ScheduleAt {
            id: "cancel-reminder".into(),
            title: "Noch bestellen".into(),
            body: "Schluss um 13:45".into(),
            channel_id: Some("menu-updates".into()),
            fire_at_epoch_ms: 1_762_500_000_000,
            screen: None,
        });
        assert_eq!(w.kind, 1);
        assert_eq!(w.fire_at_epoch_ms, 1_762_500_000_000);
        assert_eq!(w.channel_id, "menu-updates");
    }

    /// @req REQ-PLT-02 CancelPending marshals to kind 2 with only the id set.
    #[test]
    fn wire_command_cancel_pending() {
        let w = wire_command(&NotificationCommand::CancelPending {
            id: "daily-order-reminder".into(),
        });
        assert_eq!(
            w,
            WireCommand {
                kind: 2,
                id: "daily-order-reminder".into(),
                title: String::new(),
                body: String::new(),
                channel_id: String::new(),
                fire_at_epoch_ms: 0,
            }
        );
    }

    // ---- credentials (REQ-PLT-01) ------------------------------------------

    /// @req REQ-PLT-01 save → get → clear round-trips through the bridge with
    /// the exact key.
    #[test]
    fn credential_round_trip() {
        let bridge = MockBridge::new();
        let p = platform(bridge);
        p.save_credential("gourmet_username", "test-user").unwrap();
        assert_eq!(
            p.get_credential("gourmet_username").unwrap().as_deref(),
            Some("test-user")
        );
        p.clear_credential("gourmet_username").unwrap();
        assert_eq!(p.get_credential("gourmet_username").unwrap(), None);
        let calls = p.bridge.take_calls();
        assert_eq!(calls[0].method, "save");
        assert_eq!(calls[0].args, vec!["gourmet_username", "test-user"]);
        assert_eq!(calls[2].method, "delete");
    }

    // ---- notifications (REQ-PLT-02) ----------------------------------------

    /// @req REQ-PLT-02 notify hands the marshalled command to the bridge once.
    #[test]
    fn notify_executes_one_command() {
        let bridge = MockBridge::new();
        let p = platform(bridge);
        p.notify(NotificationCommand::FireNow {
            id: "debug-fire".into(),
            title: "Bridge-Test".into(),
            body: "Banner".into(),
            channel_id: Some("order-reminders".into()),
            screen: None,
        })
        .unwrap();
        let calls = p.bridge.take_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "execute");
        assert_eq!(
            calls[0].args,
            vec![
                "0",
                "debug-fire",
                "Bridge-Test",
                "Banner",
                "order-reminders",
                "0"
            ]
        );
    }

    // ---- async resolution + timeouts ---------------------------------------

    /// @req REQ-PLT-06 A permission request resolves through the request-id
    /// callback and returns the granted flag.
    #[test]
    fn permission_resolves_by_request_id() {
        let bridge = MockBridge::new();
        let p = platform(bridge);
        assert!(p.request_notification_permission().unwrap());
        let calls = p.bridge.take_calls();
        assert_eq!(calls[0].method, "permission");
        assert_eq!(calls[0].args[0], "notifications");
    }

    /// @req REQ-PLT-06 A request that never resolves times out with a typed
    /// error instead of hanging.
    #[test]
    fn permission_times_out() {
        let bridge = MockBridge::new();
        *bridge.never_resolve.lock().unwrap() = true;
        let p = platform(bridge);
        let err = p.request_notification_permission().unwrap_err();
        assert!(matches!(err, PlatformError::System(_)), "{err:?}");
        assert!(err.to_string().contains("timed out"), "{err}");
    }

    /// @req REQ-PLT-03 A found fix returns Some(LocationFix); a miss is None.
    #[test]
    fn current_location_fix_and_miss() {
        let bridge = MockBridge::new();
        *bridge.location.lock().unwrap() = Some((48.2082, 16.3738));
        let p = platform(bridge);
        assert_eq!(
            p.current_location().unwrap(),
            Some(LocationFix {
                latitude: 48.2082,
                longitude: 16.3738
            })
        );
        *p.bridge.location.lock().unwrap() = None;
        assert_eq!(p.current_location().unwrap(), None);
    }

    // ---- geofence (REQ-PLT-03) ----------------------------------------------

    /// @req REQ-PLT-03 start_geofence reads the saved company location from
    /// the core KV dir and registers it with entry trigger (v1 capture flow).
    #[test]
    fn start_geofence_uses_saved_company_location() {
        let dir = std::env::temp_dir().join("snackpilot-3.4-geofence-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let kv = snackpilot_core::storage::FileKv::new(dir.clone());
        snackpilot_core::storage::cache::save_company_location(
            &kv,
            &snackpilot_core::domain::location::CompanyLocation {
                latitude: 48.2,
                longitude: 16.3,
            },
        )
        .unwrap();

        let bridge = MockBridge::new();
        let p = AndroidPlatform::new(bridge, test_pending(), dir.to_str().unwrap().to_string());
        p.start_geofence().unwrap();
        let calls = p.bridge.take_calls();
        assert_eq!(calls[0].method, "start_geofence");
        assert_eq!(calls[0].args, ["48.2", "16.3", "true"]);

        // No saved location → typed Location error, no Kotlin call.
        snackpilot_core::storage::cache::clear_company_location(&kv);
        let err = p.start_geofence().unwrap_err();
        assert!(matches!(err, PlatformError::Location(_)), "{err:?}");
        assert!(p.bridge.take_calls().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// @req REQ-PLT-03 stop/is_monitoring forward through the bridge.
    #[test]
    fn geofence_monitoring_flag() {
        let bridge = MockBridge::new();
        *bridge.monitoring.lock().unwrap() = Some(true);
        let p = platform(bridge);
        assert!(p.is_geofence_monitoring().unwrap());
        p.stop_geofence().unwrap();
        assert_eq!(p.bridge.take_calls()[1].method, "stop_geofence");
    }

    // ---- typed failure containment (REQ-PLT-06) ------------------------------

    /// @req REQ-PLT-06 Every bridge failure surfaces as the typed variant for
    /// its subsystem, never a panic and never a generic error.
    #[test]
    fn failures_map_to_typed_variants() {
        let cases: Vec<(
            Box<dyn FnOnce(&AndroidPlatform<MockBridge>) -> Result<(), PlatformError>>,
            PlatformError,
        )> = vec![
            (
                Box::new(|p| p.save_credential("k", "v")),
                PlatformError::Storage("java said no".into()),
            ),
            (
                Box::new(|p| p.notify(NotificationCommand::CancelPending { id: "x".into() })),
                PlatformError::Notification("java said no".into()),
            ),
            (
                Box::new(|p| p.stop_geofence()),
                PlatformError::Location("java said no".into()),
            ),
            (
                Box::new(|p| {
                    p.compose_email(SupportEmail {
                        to: "a@b.c".into(),
                        subject: "s".into(),
                        body: "b".into(),
                    })
                }),
                PlatformError::System("java said no".into()),
            ),
        ];
        for (call, expected) in cases {
            let bridge = MockBridge::new();
            *bridge.fail.lock().unwrap() = Some("java said no".into());
            let p = platform(bridge);
            assert_eq!(call(&p).unwrap_err(), expected);
        }
    }

    /// @req REQ-PLT-06 A JNI call that fails while a Java exception is
    /// pending surfaces as a typed PlatformError carrying the Java message:
    /// the android `checked` helper reads+clears the pending exception first
    /// and folds its message into the error (the mock reproduces exactly
    /// that error shape).
    #[test]
    fn jni_err_folds_java_exception_message_into_typed_error() {
        let bridge = MockBridge::new();
        *bridge.java_exception.lock().unwrap() =
            Some("java.security.KeyStoreException: key not found".into());
        let p = platform(bridge);
        let err = p.save_credential("gourmet_username", "v").unwrap_err();
        assert!(matches!(err, PlatformError::Storage(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("KeyStoreException: key not found"), "{msg}");
        assert!(msg.contains("jni: call_static_method"), "{msg}");
    }

    /// @req REQ-PLT-06 The folded message always names the Java exception;
    /// the underlying JNI error is appended only when the call itself failed.
    #[test]
    fn java_exception_message_shape() {
        assert_eq!(
            java_exception_message("NPE: boom", None),
            "Java exception: NPE: boom"
        );
        assert_eq!(
            java_exception_message("NPE: boom", Some("call failed")),
            "Java exception: NPE: boom (jni: call failed)"
        );
    }

    /// @req REQ-PLT-01 A failing get is a typed Storage error, not None.
    #[test]
    fn get_credential_failure_is_typed() {
        let bridge = MockBridge::new();
        *bridge.fail.lock().unwrap() = Some("keystore broken".into());
        let p = platform(bridge);
        assert_eq!(
            p.get_credential("gourmet_password").unwrap_err(),
            PlatformError::Storage("keystore broken".into())
        );
    }

    // ---- analytics + system (REQ-PLT-04/05) ----------------------------------

    /// @req REQ-PLT-04 track forwards event and props to the bridge.
    #[test]
    fn track_forwards_props() {
        let bridge = MockBridge::new();
        let p = platform(bridge);
        let mut props = HashMap::new();
        props.insert("source".to_string(), "geofence".to_string());
        p.track("order.submitted", props).unwrap();
        let calls = p.bridge.take_calls();
        assert_eq!(calls[0].method, "track");
        assert_eq!(calls[0].args, ["order.submitted", "source=geofence"]);
    }

    /// @req REQ-PLT-05 set_app_icon forwards the accent (Kotlin no-ops while
    /// the aliases are absent).
    #[test]
    fn set_app_icon_forwards_accent() {
        let bridge = MockBridge::new();
        let p = platform(bridge);
        p.set_app_icon("emerald").unwrap();
        assert_eq!(p.bridge.take_calls()[0].args, ["emerald"]);
    }

    /// Android declares both subsystems (design D4 capabilities).
    #[test]
    fn capabilities() {
        let p = platform(MockBridge::new());
        assert_eq!(
            p.capabilities(),
            Capabilities {
                notifications: true,
                geofence: true
            }
        );
    }
}

// ---- real JNI bridge (device only) ------------------------------------------

#[cfg(target_os = "android")]
mod jni_bridge {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use jni::errors::Error as JniError;
    use jni::objects::{Global, JClass, JObject, JString, JValue, JValueOwned};
    use jni::signature::MethodSignature;
    use jni::strings::JNIString;
    use jni::{Env, EnvUnowned, JavaVM};

    use crate::platform::android::{
        java_exception_message, pending, Bridge, SupportEmail, WireCommand,
    };

    /// JNI error mapped to the bridge's `String` error channel.
    struct JniErr(String);

    impl From<JniError> for JniErr {
        fn from(e: JniError) -> Self {
            JniErr(e.to_string())
        }
    }

    impl From<String> for JniErr {
        fn from(s: String) -> Self {
            JniErr(s)
        }
    }

    impl std::fmt::Debug for JniErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("JniErr").field(&self.0).finish()
        }
    }

    impl std::fmt::Display for JniErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for JniErr {}

    /// Attach the current (Rust) thread to the JVM for one closure, mapping
    /// JNI errors into the bridge's `String` error channel.
    fn attach<T, F>(f: F) -> Result<T, String>
    where
        F: FnOnce(&mut Env) -> Result<T, JniErr>,
    {
        java_vm().attach_current_thread(f).map_err(|e| e.0)
    }

    fn java_vm() -> &'static JavaVM {
        static VM: OnceLock<JavaVM> = OnceLock::new();
        VM.get_or_init(|| unsafe {
            let ctx = ndk_context::android_context();
            JavaVM::from_raw(ctx.vm() as *mut jni::sys::JavaVM)
        })
    }

    /// Kotlin classes cached as global refs on first use.
    struct Classes {
        bridge: Global<JClass<'static>>,
        command_data: Global<JClass<'static>>,
        support_email_data: Global<JClass<'static>>,
        activity: Global<JClass<'static>>,
    }

    static CLASSES: OnceLock<Classes> = OnceLock::new();

    /// Cache the Kotlin classes. Must run on a Java thread (the activity's
    /// onCreate): a FindClass from a Rust-created thread uses the system
    /// classloader and would not see the app's classes.
    fn cache_class(env: &mut Env) -> Result<(), JniErr> {
        if CLASSES.get().is_some() {
            return Ok(());
        }
        // Each raw result is bound first: the jni methods keep the `&mut env`
        // borrow alive in their return value until it is consumed.
        let r = env.find_class(JNIString::new("dev/radaiko/snackpilot/PlatformBridge"));
        let bridge = checked(env, r)?;
        let r = env.new_global_ref(bridge);
        let bridge = checked(env, r)?;
        let r = env.find_class(JNIString::new("dev/radaiko/snackpilot/CommandData"));
        let command_data = checked(env, r)?;
        let r = env.new_global_ref(command_data);
        let command_data = checked(env, r)?;
        let r = env.find_class(JNIString::new("dev/radaiko/snackpilot/SupportEmailData"));
        let support_email_data = checked(env, r)?;
        let r = env.new_global_ref(support_email_data);
        let support_email_data = checked(env, r)?;
        let r = env.find_class(JNIString::new(
            "dev/radaiko/gourmetclient/SnackPilotActivity",
        ));
        let activity = checked(env, r)?;
        let r = env.new_global_ref(activity);
        let activity = checked(env, r)?;
        let _ = CLASSES.set(Classes {
            bridge,
            command_data,
            support_email_data,
            activity,
        });
        Ok(())
    }

    fn classes() -> &'static Classes {
        CLASSES.get().expect(
            "PlatformBridge.nativeCacheClass() must run on the Java main thread before JNI use",
        )
    }

    /// Read and clear a pending Java exception, returning its message. The
    /// exception is always cleared so nothing stays pending on the attached
    /// thread (a pending exception breaks every later JNI call there).
    fn take_exception(env: &mut Env) -> Option<String> {
        if !env.exception_check() {
            return None;
        }
        let msg = env
            .exception_occurred()
            .and_then(|t| {
                env.call_method(
                    &t,
                    JNIString::new("toString"),
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
                .ok()?
                .l()
                .ok()
            })
            .and_then(|o| {
                let s = unsafe { JString::from_raw(env, o.into_raw()) };
                s.try_to_string(env).ok()
            })
            .unwrap_or_else(|| "Java exception".to_string());
        env.exception_clear();
        Some(msg)
    }

    /// The single funnel for every raw `env.*` call (REQ-PLT-06): a pending
    /// Java exception is always read and cleared — after a successful call,
    /// and *first of all* when the call itself failed, so the exception never
    /// leaks past the early return — and its message is folded into the
    /// typed [`JniErr`].
    fn checked<T>(env: &mut Env, r: jni::errors::Result<T>) -> Result<T, JniErr> {
        match r {
            Ok(v) => match take_exception(env) {
                Some(msg) => Err(JniErr(java_exception_message(&msg, None))),
                None => Ok(v),
            },
            Err(e) => match take_exception(env) {
                Some(msg) => Err(JniErr(java_exception_message(&msg, Some(&e.to_string())))),
                None => Err(e.into()),
            },
        }
    }

    fn call_static<'env>(
        env: &mut Env<'env>,
        name: &str,
        sig: MethodSignature,
        args: &[JValue],
    ) -> Result<JValueOwned<'env>, JniErr> {
        let r = env.call_static_method(&classes().bridge, JNIString::new(name), sig, args);
        checked(env, r)
    }

    /// Call a PlatformBridge static whose non-null String return is an error
    /// message (null = success).
    fn call_check(
        env: &mut Env,
        name: &str,
        sig: MethodSignature,
        args: &[JValue],
    ) -> Result<(), JniErr> {
        let value = call_static(env, name, sig, args)?;
        match error_of(env, value)? {
            None => Ok(()),
            Some(msg) => Err(JniErr(msg)),
        }
    }

    /// Read a nullable String return value (used for error messages and for
    /// the getSecret/isGeofenceMonitoring value channels).
    fn opt_string(env: &mut Env, value: JValueOwned) -> Result<Option<String>, JniErr> {
        let obj = match value {
            JValueOwned::Object(o) => o,
            _ => return Ok(None),
        };
        if obj.is_null() {
            return Ok(None);
        }
        let s = unsafe { JString::from_raw(env, obj.into_raw()) };
        let r = s.try_to_string(env);
        checked(env, r).map(Some)
    }

    fn error_of(env: &mut Env, value: JValueOwned) -> Result<Option<String>, JniErr> {
        opt_string(env, value)
    }

    fn jstr<'env>(env: &mut Env<'env>, s: &str) -> Result<JObject<'env>, JniErr> {
        let r = JString::new(env, s);
        Ok(JObject::from(checked(env, r)?))
    }

    fn new_command_data<'env>(
        env: &mut Env<'env>,
        cmd: &WireCommand,
    ) -> Result<JObject<'env>, JniErr> {
        let args = [
            JValue::Int(i32::from(cmd.kind)),
            JValue::Object(&jstr(env, &cmd.id)?),
            JValue::Object(&jstr(env, &cmd.title)?),
            JValue::Object(&jstr(env, &cmd.body)?),
            JValue::Object(&jstr(env, &cmd.channel_id)?),
            JValue::Long(cmd.fire_at_epoch_ms),
        ];
        let r = env.new_object(
            &classes().command_data,
            jni::jni_sig!(
                "(ILjava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;J)V"
            ),
            &args,
        );
        checked(env, r)
    }

    impl Bridge for JniBridge {
        fn save_secret(&self, key: &str, value: &str) -> Result<(), String> {
            attach(|env| {
                let key = jstr(env, key)?;
                let value = jstr(env, value)?;
                let args = [JValue::Object(&key), JValue::Object(&value)];
                call_check(
                    env,
                    "saveSecret",
                    jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
            // getSecret returns the stored value itself (null = not found);
            // failures surface as Java exceptions, caught by `checked`.
            attach(|env| {
                let key = jstr(env, key)?;
                let args = [JValue::Object(&key)];
                let value = call_static(
                    env,
                    "getSecret",
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )?;
                opt_string(env, value)
            })
        }

        fn delete_secret(&self, key: &str) -> Result<(), String> {
            attach(|env| {
                let key = jstr(env, key)?;
                let args = [JValue::Object(&key)];
                call_check(
                    env,
                    "deleteSecret",
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn request_permission(&self, permission: &str, request_id: u64) -> Result<(), String> {
            attach(|env| {
                let permission = jstr(env, permission)?;
                let id = jstr(env, &request_id.to_string())?;
                let args = [JValue::Object(&permission), JValue::Object(&id)];
                call_check(
                    env,
                    "requestPermission",
                    jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn request_location(&self, request_id: u64) -> Result<(), String> {
            attach(|env| {
                let id = jstr(env, &request_id.to_string())?;
                let args = [JValue::Object(&id)];
                call_check(
                    env,
                    "requestLocation",
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn execute_command(&self, command: &WireCommand) -> Result<(), String> {
            attach(|env| {
                let data = new_command_data(env, command)?;
                let args = [JValue::Object(&data)];
                call_check(
                    env,
                    "executeCommand",
                    jni::jni_sig!("(Ldev/radaiko/snackpilot/CommandData;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn start_geofence(
            &self,
            latitude: f64,
            longitude: f64,
            trigger_on_entry: bool,
        ) -> Result<(), String> {
            attach(|env| {
                let args = [
                    JValue::Double(latitude),
                    JValue::Double(longitude),
                    JValue::Bool(trigger_on_entry),
                ];
                call_check(
                    env,
                    "startGeofence",
                    jni::jni_sig!("(DDZ)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn stop_geofence(&self) -> Result<(), String> {
            attach(|env| {
                call_check(
                    env,
                    "stopGeofence",
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
            })
        }

        fn is_geofence_monitoring(&self) -> Result<bool, String> {
            // Kotlin reports "true" while a geofence is registered, null otherwise.
            attach(|env| {
                let value = call_static(
                    env,
                    "isGeofenceMonitoring",
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?;
                Ok(opt_string(env, value)?.as_deref() == Some("true"))
            })
        }

        fn track(&self, event: &str, props: &HashMap<String, String>) -> Result<(), String> {
            attach(|env| {
                let r = env.new_object(
                    JNIString::new("java/util/HashMap"),
                    jni::jni_sig!("()V"),
                    &[],
                );
                let map = checked(env, r)?;
                for (k, v) in props {
                    let key = jstr(env, k)?;
                    let value = jstr(env, v)?;
                    let args = [JValue::Object(&key), JValue::Object(&value)];
                    let r = env.call_method(
                        &map,
                        JNIString::new("put"),
                        jni::jni_sig!("(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;"),
                        &args,
                    );
                    checked(env, r)?;
                }
                let event = jstr(env, event)?;
                let args = [JValue::Object(&event), JValue::Object(&map)];
                call_check(
                    env,
                    "track",
                    jni::jni_sig!("(Ljava/lang/String;Ljava/util/HashMap;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn set_app_icon(&self, accent: &str) -> Result<(), String> {
            attach(|env| {
                let accent = jstr(env, accent)?;
                let args = [JValue::Object(&accent)];
                call_check(
                    env,
                    "setAppIcon",
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                    &args,
                )
            })
        }

        fn open_app_settings(&self) -> Result<(), String> {
            attach(|env| {
                call_check(
                    env,
                    "openAppSettings",
                    jni::jni_sig!("()Ljava/lang/String;"),
                    &[],
                )
            })
        }

        fn compose_email(&self, email: &SupportEmail) -> Result<(), String> {
            attach(|env| {
                // The model has no attachment fields; the Kotlin data class
                // takes empty strings there (no attachment).
                let to = jstr(env, &email.to)?;
                let subject = jstr(env, &email.subject)?;
                let body = jstr(env, &email.body)?;
                let none1 = jstr(env, "")?;
                let none2 = jstr(env, "")?;
                let ctor_args = [
                    JValue::Object(&to),
                    JValue::Object(&subject),
                    JValue::Object(&body),
                    JValue::Object(&none1),
                    JValue::Object(&none2),
                ];
                let r = env.new_object(
                    &classes().support_email_data,
                    jni::jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
                    &ctor_args,
                );
                let data = checked(env, r)?;
                let args = [JValue::Object(&data)];
                call_check(
                    env,
                    "composeEmail",
                    jni::jni_sig!("(Ldev/radaiko/snackpilot/SupportEmailData;)Ljava/lang/String;"),
                    &args,
                )
            })
        }
    }

    /// JNI-backed bridge passed to [`AndroidPlatform::new`] (device only).
    pub struct JniBridge;

    /// Cache the Kotlin classes PlatformBridge calls into. Call once from the
    /// Java main thread after System.loadLibrary.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_radaiko_snackpilot_PlatformBridge_nativeCacheClass<'caller>(
        mut unowned: EnvUnowned<'caller>,
        _class: JClass<'caller>,
    ) {
        unowned
            .with_env(cache_class)
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }

    /// Kotlin entry point: permission decision for a request id.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_radaiko_snackpilot_PlatformBridge_nativeResolvePermission<
        'caller,
    >(
        mut unowned: EnvUnowned<'caller>,
        _class: JClass<'caller>,
        request_id: JString<'caller>,
        granted: bool,
    ) {
        unowned
            .with_env(|env| -> Result<(), JniErr> {
                let r = request_id.try_to_string(env);
                let id = checked(env, r)?;
                if let Ok(id) = id.parse::<u64>() {
                    pending().resolve_permission(id, granted);
                }
                Ok(())
            })
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }

    /// Kotlin entry point: one-shot location fix for a request id.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_radaiko_snackpilot_PlatformBridge_nativeResolveLocation<
        'caller,
    >(
        mut unowned: EnvUnowned<'caller>,
        _class: JClass<'caller>,
        request_id: JString<'caller>,
        latitude: f64,
        longitude: f64,
        found: bool,
    ) {
        unowned
            .with_env(|env| -> Result<(), JniErr> {
                let r = request_id.try_to_string(env);
                let id = checked(env, r)?;
                if let Ok(id) = id.parse::<u64>() {
                    pending().resolve_location(id, latitude, longitude, found);
                }
                Ok(())
            })
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    }

    // ---- debug-only bridge exercise (REQ-PLT-07) ------------------------------

    use super::{AndroidPlatform, NotificationCommand, Platform, PlatformError};
    use std::sync::Arc;

    static DEBUG_PLATFORM: OnceLock<Arc<AndroidPlatform<JniBridge>>> = OnceLock::new();

    /// Called from `android_main` right after the bridge is built, so the
    /// activity's debug intent can drive it through the real JNI path.
    pub fn install_debug_platform(platform: Arc<AndroidPlatform<JniBridge>>) {
        let _ = DEBUG_PLATFORM.set(platform);
    }

    static DEBUG_SESSION: OnceLock<Arc<snackpilot_model::AppModel>> = OnceLock::new();

    /// Called from `android_main` right after the platform is built, so the
    /// geofence receiver can hand transitions to the model (task 3.4: the
    /// Kotlin receiver contains no notification decision logic, design D4).
    pub fn install_debug_session(session: Arc<snackpilot_model::AppModel>) {
        let _ = DEBUG_SESSION.set(session);
    }

    /// Java: `SnackPilotActivity.nativeOnGeofenceEvent(inside)` from
    /// `GeofenceBroadcastReceiver`. Returns immediately; the model call runs
    /// on its own thread because `handle_geofence` delivers notifications
    /// through the JNI bridge, which must not block the broadcast thread.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_radaiko_gourmetclient_SnackPilotActivity_nativeOnGeofenceEvent<
        'caller,
    >(
        _unowned: EnvUnowned<'caller>,
        _class: JClass<'caller>,
        inside: bool,
    ) {
        // android_main installs the session on its own thread and the
        // activity caches the Kotlin classes on the Java main thread; the
        // notify path needs both, so wait for them like the exercise does.
        std::thread::spawn(move || {
            for _ in 0..30 {
                if let Some(session) = DEBUG_SESSION.get() {
                    if CLASSES.get().is_some() {
                        log::info!("[3.4] geofence event: inside={inside}, delegating to model");
                        session.on_geofence_event(inside);
                        log::info!("[3.4] geofence event handled by model");
                        return;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            log::error!("[3.4] geofence event: model session or classes not installed");
        });
    }

    /// Java: `SnackPilotActivity.nativeDebugExercise()` when the launch intent
    /// carries `snackpilot.debug=exercise`. Returns immediately; the exercise
    /// runs on its own thread so the Java main thread never blocks on Rust.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_radaiko_gourmetclient_SnackPilotActivity_nativeDebugExercise<
        'caller,
    >(
        _unowned: EnvUnowned<'caller>,
        _activity: JObject<'caller>,
    ) {
        // android_main installs the platform on its own thread; wait for it.
        std::thread::spawn(|| {
            for _ in 0..30 {
                if let Some(platform) = DEBUG_PLATFORM.get() {
                    run_debug_exercise(platform);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            log::error!("[3.4] debug exercise: platform not installed");
        });
    }

    fn log_step(what: &str, result: &Result<(), PlatformError>) {
        match result {
            Ok(()) => log::info!("[3.4] {what}: ok"),
            Err(e) => log::error!("[3.4] {what}: {e:?}"),
        }
    }

    fn run_debug_exercise(platform: &AndroidPlatform<JniBridge>) {
        use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
        // The activity caches the Kotlin classes on its main thread after
        // loadLibrary; wait for that before any bridge call.
        for _ in 0..30 {
            if CLASSES.get().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let t0 = Instant::now();
        // Credential save/read through the bridge; the value is never logged.
        log_step(
            "save_credential(gourmet_username)",
            &platform.save_credential("gourmet_username", "evidence-user"),
        );
        match platform.get_credential("gourmet_username") {
            Ok(Some(v)) => log::info!(
                "[3.4] credential round trip match={} key=gourmet_username",
                v == "evidence-user"
            ),
            Ok(None) => log::error!("[3.4] get_credential(gourmet_username): absent"),
            Err(e) => log::error!("[3.4] get_credential(gourmet_username): {e:?}"),
        }
        log_step(
            "notify fire-now",
            &platform.notify(NotificationCommand::FireNow {
                id: "evidence-fire".into(),
                title: "3.4 Debug sofort".into(),
                body: "Sofortige Banner-Benachrichtigung".into(),
                channel_id: Some("order-reminders".into()),
                screen: None,
            }),
        );
        let fire_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis() as i64
            + 3_000;
        let schedule = |id: &str, body: &str| {
            platform.notify(NotificationCommand::ScheduleAt {
                id: id.into(),
                title: "3.4 Debug geplant".into(),
                body: body.into(),
                channel_id: Some("menu-updates".into()),
                fire_at_epoch_ms: fire_at,
                screen: None,
            })
        };
        log_step(
            "notify schedule-at",
            &schedule("evidence-scheduled", "trifft in ~3s ein"),
        );
        log_step(
            "notify schedule-at",
            &schedule("evidence-cancelled", "wird storniert"),
        );
        log_step(
            "notify cancel-pending",
            &platform.notify(NotificationCommand::CancelPending {
                id: "evidence-cancelled".into(),
            }),
        );
        log_step("start_geofence", &platform.start_geofence());
        match platform.is_geofence_monitoring() {
            Ok(m) => log::info!("[3.4] is_geofence_monitoring: {m}"),
            Err(e) => log::error!("[3.4] is_geofence_monitoring: {e:?}"),
        }
        log_step("stop_geofence", &platform.stop_geofence());
        // Geofence pump round trip (HIGH-1 evidence): call the receiver's
        // static native through the JVM exactly as GeofenceBroadcastReceiver
        // does via SnackPilotActivity.getGeofencePump(); the export then
        // delegates to the model on its own thread and logs it.
        let pump = attach(|env| {
            let r = env.call_static_method(
                &classes().activity,
                JNIString::new("nativeOnGeofenceEvent"),
                jni::jni_sig!("(Z)V"),
                &[JValue::Bool(true)],
            );
            let v = checked(env, r)?;
            drop(v);
            Ok(())
        });
        match pump {
            Ok(()) => log::info!("[3.4] geofence pump: dispatched through the JVM"),
            Err(e) => log::error!("[3.4] geofence pump: {e}"),
        }
        log_step(
            "track",
            &platform.track(
                "debug.exercise",
                HashMap::from([("task".to_string(), "3.4".to_string())]),
            ),
        );
        log::info!(
            "[3.4] debug exercise finished in {} ms",
            t0.elapsed().as_millis()
        );
    }
}

#[cfg(target_os = "android")]
pub use jni_bridge::{install_debug_platform, install_debug_session, JniBridge};

/// Process-wide request registry the JNI resolvers resolve into; shared by
/// every [`AndroidPlatform`] on the device.
#[cfg(target_os = "android")]
pub fn pending() -> &'static Pending {
    static PENDING: std::sync::OnceLock<Pending> = std::sync::OnceLock::new();
    PENDING.get_or_init(Pending::new)
}
