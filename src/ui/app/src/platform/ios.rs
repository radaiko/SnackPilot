//! iOS [`Platform`] over a C shim table (task 3.5, REQ-PLT-01..04, 06).
//!
//! The Swift shims under `src/ui/ios/Shims/` own the real iOS APIs; this file
//! marshals across the C boundary only. Swift registers one [`IosShimTable`]
//! at launch via `snackpilot_ios_register_shims`; every entry mirrors one
//! `Platform` capability, exactly like the Android JNI bridge (task 3.4).
//!
//! Design (design.md D4): the Swift side is a dumb shell. All decision logic —
//! which command to run, the geofence coordinates, the typed error mapping —
//! stays here. Asynchronous operations (permission dialogs, one-shot fixes)
//! hand Swift a request id and block on [`Pending`] until Swift calls
//! `snackpilot_ios_complete`; a request that never resolves fails with a typed
//! timeout instead of hanging the caller (REQ-PLT-06).
//!
//! Everything here is plain Rust over `extern "C"` function pointers, so the
//! whole file — table types, [`IosPlatform`] and the exports — compiles and is
//! unit-tested on the host with a Rust-side stub table.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use snackpilot_core::notify::NotificationCommand;
use snackpilot_model::{Capabilities, LocationFix, Platform, PlatformError, SupportEmail};

/// [`IosShimTable::size`]: number of function fields plus the size field.
pub const TABLE_SIZE: u32 = 16;

/// Command marshalled to the Swift `SnackPilotWireCommand`. `id` is the
/// v1 `UNNotificationRequest` identifier (a string), `channel_id` is
/// Android-only and ignored on iOS, `fire_at_epoch_ms` unused by kinds 0/2.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WireCommandC {
    /// 0 = FireNow, 1 = ScheduleAt, 2 = CancelPending.
    pub kind: i32,
    pub id: *const c_char,
    pub title: *const c_char,
    pub body: *const c_char,
    pub channel_id: *const c_char,
    pub fire_at_epoch_ms: i64,
}

/// The function table Swift registers at launch. Every entry is required
/// ([`IosShimTable`] accepts nulls so a half-filled table is rejected with a
/// typed error instead of crashing on call); `size` guards against drift
/// between the two sides.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IosShimTable {
    pub size: u32,
    pub keychain_set: Option<extern "C" fn(*const c_char, *const c_char) -> u8>,
    pub keychain_get: Option<extern "C" fn(*const c_char, *mut *mut c_char) -> u8>,
    pub keychain_delete: Option<extern "C" fn(*const c_char) -> u8>,
    pub notify: Option<extern "C" fn(*const WireCommandC) -> u8>,
    pub notify_permission: Option<extern "C" fn(u64, *mut c_void) -> u8>,
    pub location_authorisation: Option<extern "C" fn(u64, *mut c_void) -> u8>,
    pub location_current: Option<extern "C" fn(u64, *mut c_void) -> u8>,
    pub geofence_start: Option<extern "C" fn(f64, f64, u64, *mut c_void) -> u8>,
    pub geofence_stop: Option<extern "C" fn() -> u8>,
    pub geofence_is_monitoring: Option<extern "C" fn() -> u8>,
    pub track: Option<extern "C" fn(*const c_char) -> u8>,
    pub open_email: Option<extern "C" fn(*const c_char, *const c_char, *const c_char) -> u8>,
    pub open_settings: Option<extern "C" fn() -> u8>,
    pub open_app_settings: Option<extern "C" fn() -> u8>,
    pub set_app_icon: Option<extern "C" fn(*const c_char) -> u8>,
}

// The C allocator the Swift side duplicates credential values with (`strdup`).
extern "C" {
    fn free(ptr: *mut c_void);
}

/// Resolve the named shim or fail typed — never panic on a missing entry
/// (REQ-PLT-06; [`IosPlatform::new`] already rejects such tables, this is the
/// belt for hand-built ones).
macro_rules! shim {
    ($self:expr, $field:ident) => {
        $self.table.$field.ok_or_else(|| {
            PlatformError::System(concat!("shim missing: ", stringify!($field)).to_string())
        })?
    };
}

/// What an asynchronous shim completes with: a payload (`"true"`/`"false"`
/// for permissions, `"lat,lon"` for a fix, empty for a miss) or a failure
/// message from the Swift side.
#[derive(Debug, Clone)]
enum Outcome {
    Ok(Option<String>),
    Failed(String),
}

enum Slot {
    Waiting,
    Done(Outcome),
}

/// Request-id registry for the async flows (permission dialogs, one-shot
/// fix, geofence start). Swift calls back into `snackpilot_ios_complete`
/// with the id it was handed; a request that never resolves times out
/// (REQ-PLT-06).
pub struct Pending {
    next_id: AtomicU64,
    slots: Mutex<HashMap<u64, Slot>>,
    ready: Condvar,
    timeout: Duration,
}

impl Pending {
    /// Production timeout: long enough for a user to answer a dialog.
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(30))
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            next_id: AtomicU64::new(1),
            slots: Mutex::new(HashMap::new()),
            ready: Condvar::new(),
            timeout,
        }
    }

    /// Register a request before handing it to Swift, so a completion that
    /// lands synchronously inside the shim call is not lost.
    fn begin(&'static self) -> u64 {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.slots.lock().unwrap().insert(id, Slot::Waiting);
        id
    }

    /// Drop a slot when the shim rejected the request synchronously, so
    /// rejected requests do not accumulate in the registry.
    fn cancel(&'static self, id: u64) {
        self.slots.lock().unwrap().remove(&id);
    }

    fn resolve(&'static self, id: u64, outcome: Outcome) {
        if let Some(entry) = self.slots.lock().unwrap().get_mut(&id) {
            *entry = Slot::Done(outcome);
        }
        self.ready.notify_all();
    }

    /// Block until `id` completes or the timeout fires; either way the slot
    /// is dropped so the registry cannot grow without bound.
    fn await_slot(&'static self, id: u64) -> Result<Outcome, PlatformError> {
        let deadline = std::time::Instant::now() + self.timeout;
        let mut slots = self.slots.lock().unwrap();
        loop {
            if matches!(slots.get(&id), Some(Slot::Done(_))) {
                return match slots.remove(&id) {
                    Some(Slot::Done(outcome)) => Ok(outcome),
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

/// Process-wide request registry `snackpilot_ios_complete` resolves into.
pub fn pending() -> &'static Pending {
    static PENDING: OnceLock<Pending> = OnceLock::new();
    PENDING.get_or_init(Pending::new)
}

/// The [`Platform`] implementation over the registered table.
pub struct IosPlatform {
    table: IosShimTable,
    pending: &'static Pending,
    storage_dir: PathBuf,
}

impl IosPlatform {
    /// Reject a table that is the wrong size or misses any entry, with a
    /// typed error instead of a null-pointer crash on first use (REQ-PLT-06).
    pub fn new(
        table: IosShimTable,
        pending: &'static Pending,
        storage_dir: PathBuf,
    ) -> Result<Self, PlatformError> {
        if table.size != TABLE_SIZE {
            return Err(PlatformError::System(format!(
                "shim table size {} != {TABLE_SIZE}",
                table.size
            )));
        }
        let required = [
            ("keychain_set", table.keychain_set.is_some()),
            ("keychain_get", table.keychain_get.is_some()),
            ("keychain_delete", table.keychain_delete.is_some()),
            ("notify", table.notify.is_some()),
            ("notify_permission", table.notify_permission.is_some()),
            (
                "location_authorisation",
                table.location_authorisation.is_some(),
            ),
            ("location_current", table.location_current.is_some()),
            ("geofence_start", table.geofence_start.is_some()),
            ("geofence_stop", table.geofence_stop.is_some()),
            (
                "geofence_is_monitoring",
                table.geofence_is_monitoring.is_some(),
            ),
            ("track", table.track.is_some()),
            ("open_email", table.open_email.is_some()),
            ("open_settings", table.open_settings.is_some()),
            ("open_app_settings", table.open_app_settings.is_some()),
            ("set_app_icon", table.set_app_icon.is_some()),
        ];
        if let Some((name, _)) = required.iter().find(|(_, present)| !present) {
            return Err(PlatformError::System(format!("shim missing: {name}")));
        }
        *ACTIVE_PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(pending);
        Ok(Self {
            table,
            pending,
            storage_dir,
        })
    }

    /// Wait for an async shim's completion, mapping a Swift-side failure to
    /// the variant owned by the calling capability.
    fn await_outcome(
        &self,
        id: u64,
        fail: fn(String) -> PlatformError,
    ) -> Result<Option<String>, PlatformError> {
        match self.pending.await_slot(id)? {
            Outcome::Ok(payload) => Ok(payload),
            Outcome::Failed(message) => Err(fail(message)),
        }
    }

    fn request_permission(
        &self,
        shim: extern "C" fn(u64, *mut c_void) -> u8,
        name: &str,
    ) -> Result<bool, PlatformError> {
        let id = self.pending.begin();
        if shim(id, std::ptr::null_mut()) == 0 {
            self.pending.cancel(id);
            return Err(PlatformError::Notification(format!(
                "{name} shim rejected the request"
            )));
        }
        let payload = self.await_outcome(id, PlatformError::Notification)?;
        Ok(payload.as_deref() == Some("true"))
    }
}

fn to_cstring(value: &str, fail: fn(String) -> PlatformError) -> Result<CString, PlatformError> {
    CString::new(value).map_err(|e| fail(e.to_string()))
}

impl Platform for IosPlatform {
    fn get_credential(&self, key: &str) -> Result<Option<String>, PlatformError> {
        let shim = shim!(self, keychain_get);
        let key = to_cstring(key, PlatformError::Storage)?;
        let mut out: *mut c_char = std::ptr::null_mut();
        if shim(key.as_ptr(), &mut out) == 0 {
            return Ok(None);
        }
        if out.is_null() {
            return Err(PlatformError::Storage(
                "keychain_get reported a value without a pointer".into(),
            ));
        }
        // The Swift side duplicated the string with strdup; hand it back on
        // every path, including the invalid-UTF-8 error below.
        let value = unsafe { CStr::from_ptr(out) }
            .to_str()
            .map(|s| s.to_string());
        unsafe { free(out as *mut c_void) };
        let value = value.map_err(|e| PlatformError::Storage(e.to_string()))?;
        Ok(Some(value))
    }

    fn save_credential(&self, key: &str, value: &str) -> Result<(), PlatformError> {
        let shim = shim!(self, keychain_set);
        let key = to_cstring(key, PlatformError::Storage)?;
        let value = to_cstring(value, PlatformError::Storage)?;
        if shim(key.as_ptr(), value.as_ptr()) == 0 {
            return Err(PlatformError::Storage("keychain set failed".into()));
        }
        Ok(())
    }

    fn clear_credential(&self, key: &str) -> Result<(), PlatformError> {
        let shim = shim!(self, keychain_delete);
        let key = to_cstring(key, PlatformError::Storage)?;
        if shim(key.as_ptr()) == 0 {
            return Err(PlatformError::Storage("keychain delete failed".into()));
        }
        Ok(())
    }

    fn notify(&self, command: NotificationCommand) -> Result<(), PlatformError> {
        let shim = shim!(self, notify);
        let wire = super::android::wire_command(&command);
        let id = to_cstring(&wire.id, PlatformError::Notification)?;
        let title = to_cstring(&wire.title, PlatformError::Notification)?;
        let body = to_cstring(&wire.body, PlatformError::Notification)?;
        let channel = to_cstring(&wire.channel_id, PlatformError::Notification)?;
        let c = WireCommandC {
            kind: i32::from(wire.kind),
            id: id.as_ptr(),
            title: title.as_ptr(),
            body: body.as_ptr(),
            channel_id: channel.as_ptr(),
            fire_at_epoch_ms: wire.fire_at_epoch_ms,
        };
        if shim(&c) == 0 {
            return Err(PlatformError::Notification("notify shim failed".into()));
        }
        Ok(())
    }

    fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        self.request_permission(shim!(self, notify_permission), "notify_permission")
    }

    fn request_location_authorisation(&self) -> Result<bool, PlatformError> {
        self.request_permission(
            shim!(self, location_authorisation),
            "location_authorisation",
        )
    }

    fn current_location(&self) -> Result<Option<LocationFix>, PlatformError> {
        let shim = shim!(self, location_current);
        let id = self.pending.begin();
        if shim(id, std::ptr::null_mut()) == 0 {
            self.pending.cancel(id);
            return Err(PlatformError::Location(
                "location_current shim rejected the request".into(),
            ));
        }
        // Empty payload = no fix available (parity with Android found=false).
        let Some(payload) = self
            .await_outcome(id, PlatformError::Location)?
            .filter(|p| !p.is_empty())
        else {
            return Ok(None);
        };
        let malformed =
            || PlatformError::Location(format!("malformed location payload: {payload}"));
        let (lat, lon) = payload.split_once(',').ok_or_else(malformed)?;
        Ok(Some(LocationFix {
            latitude: lat.parse().map_err(|_| malformed())?,
            longitude: lon.parse().map_err(|_| malformed())?,
        }))
    }

    fn start_geofence(&self) -> Result<(), PlatformError> {
        // v1 parity: the coordinates are the captured company location in
        // the core KV store; without one there is nothing to monitor and
        // the shim is never called (REQ-PLT-03).
        let kv = snackpilot_core::storage::FileKv::new(self.storage_dir.clone());
        let loc = snackpilot_core::storage::cache::load_company_location(&kv)
            .ok_or_else(|| PlatformError::Location("no company location saved".into()))?;
        let shim = shim!(self, geofence_start);
        let id = self.pending.begin();
        if shim(loc.latitude, loc.longitude, id, std::ptr::null_mut()) == 0 {
            self.pending.cancel(id);
            return Err(PlatformError::Location(
                "geofence_start shim rejected the request".into(),
            ));
        }
        self.await_outcome(id, PlatformError::Location)?;
        Ok(())
    }

    fn stop_geofence(&self) -> Result<(), PlatformError> {
        let shim = shim!(self, geofence_stop);
        if shim() == 0 {
            return Err(PlatformError::Location("geofence stop failed".into()));
        }
        Ok(())
    }

    fn is_geofence_monitoring(&self) -> Result<bool, PlatformError> {
        let shim = shim!(self, geofence_is_monitoring);
        Ok(shim() != 0)
    }

    fn track(&self, event: &str, props: HashMap<String, String>) -> Result<(), PlatformError> {
        let shim = shim!(self, track);
        let json = to_cstring(&track_json(event, &props), PlatformError::System)?;
        if shim(json.as_ptr()) == 0 {
            return Err(PlatformError::System("track shim failed".into()));
        }
        Ok(())
    }

    fn set_app_icon(&self, accent: &str) -> Result<(), PlatformError> {
        let shim = shim!(self, set_app_icon);
        let accent = to_cstring(accent, PlatformError::System)?;
        if shim(accent.as_ptr()) == 0 {
            return Err(PlatformError::System("set_app_icon shim failed".into()));
        }
        Ok(())
    }

    fn open_app_settings(&self) -> Result<(), PlatformError> {
        let shim = shim!(self, open_app_settings);
        if shim() == 0 {
            return Err(PlatformError::System(
                "open_app_settings shim failed".into(),
            ));
        }
        Ok(())
    }

    fn compose_email(&self, email: SupportEmail) -> Result<(), PlatformError> {
        let shim = shim!(self, open_email);
        let to = to_cstring(&email.to, PlatformError::System)?;
        let subject = to_cstring(&email.subject, PlatformError::System)?;
        let body = to_cstring(&email.body, PlatformError::System)?;
        if shim(to.as_ptr(), subject.as_ptr(), body.as_ptr()) == 0 {
            return Err(PlatformError::System("open_email shim failed".into()));
        }
        Ok(())
    }

    fn capabilities(&self) -> Capabilities {
        // iOS declares both subsystems (design D4).
        Capabilities {
            notifications: true,
            geofence: true,
        }
    }
}

/// Serialise a track call into the single-argument JSON Swift parses:
/// `{"event":"…","props":{"k":"v",…}}` with sorted keys. Hand-rolled to keep
/// the app crate free of a serde dependency; escapes the JSON string subset.
fn track_json(event: &str, props: &HashMap<String, String>) -> String {
    let mut out = String::from("{\"event\":\"");
    json_escape(event, &mut out);
    out.push_str("\",\"props\":{");
    let mut keys: Vec<&String> = props.keys().collect();
    keys.sort();
    for (i, key) in keys.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        json_escape(key, &mut out);
        out.push_str("\":\"");
        json_escape(&props[*key], &mut out);
        out.push('"');
    }
    out.push_str("}}");
    out
}

fn json_escape(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
}

// ---- exports called by the Swift shims ---------------------------------------

struct Registered {
    _table: IosShimTable,
    _storage_dir: PathBuf,
}

/// First registration wins; a `Mutex` rather than a `OnceLock` so the host
/// tests can reset between cases.
static TABLE: Mutex<Option<Registered>> = Mutex::new(None);

/// The registry `snackpilot_ios_complete` resolves into: the one the most
/// recently created platform uses. Production registers once with the
/// process-wide [`pending()`]; tests inject a short-timeout registry.
static ACTIVE_PENDING: Mutex<Option<&'static Pending>> = Mutex::new(None);

static DEBUG_PLATFORM: OnceLock<Arc<IosPlatform>> = OnceLock::new();
static DEBUG_SESSION: OnceLock<Arc<snackpilot_model::AppModel>> = OnceLock::new();

/// Routes Rust `log` to stderr; `xcrun simctl launch --console-pty` shows it.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        eprintln!(
            "[{}] {}: {}",
            record.level(),
            record.target(),
            record.args()
        );
    }
    fn flush(&self) {}
}

/// Installs [`StderrLogger`] once. Called from
/// [`snackpilot_ios_register_shims`] (the first Rust code `bootstrap()`
/// reaches) and from `gpui_ios_register_app`; the debug exercise runs right
/// after `bootstrap()`, before the GPUI view controller exists, so the
/// registration side must install it too.
pub fn install_stderr_logger() {
    static LOGGER: StderrLogger = StderrLogger;
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// Reads a table struct Swift keeps alive for the duration of the call.
/// # Safety
/// `table` must be null-checked by the caller and point at a valid struct.
unsafe fn read_table(table: *const IosShimTable) -> IosShimTable {
    // SAFETY: checked non-null by the caller; plain-data struct owned by Swift.
    unsafe { *table }
}

/// Reads a NUL-terminated string Swift owns for the duration of the call.
/// # Safety
/// `ptr` must be null-checked by the caller and point at a valid C string.
unsafe fn read_cstr<'a>(ptr: *const c_char) -> &'a CStr {
    // SAFETY: checked non-null by the caller; Swift owns the string.
    unsafe { CStr::from_ptr(ptr) }
}

/// Swift: registers the shim table plus the storage directory the core KV
/// store uses. Returns 1 on success, 0 on a null/invalid/second registration.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn snackpilot_ios_register_shims(
    table: *const IosShimTable,
    storage_dir: *const c_char,
) -> u8 {
    install_stderr_logger();
    if table.is_null() || storage_dir.is_null() {
        log::error!("[3.5] register_shims: null table or storage dir");
        return 0;
    }
    let table = unsafe { read_table(table) };
    if table.size != TABLE_SIZE {
        log::error!("[3.5] register_shims: bad table size {}", table.size);
        return 0;
    }
    let dir = PathBuf::from(
        unsafe { read_cstr(storage_dir) }
            .to_string_lossy()
            .into_owned(),
    );
    let mut guard = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        log::error!("[3.5] register_shims: table already registered");
        return 0;
    }
    let platform = match IosPlatform::new(table, pending(), dir.clone()) {
        Ok(platform) => Arc::new(platform),
        Err(e) => {
            log::error!("[3.5] register_shims: {e:?}");
            return 0;
        }
    };
    let _ = DEBUG_PLATFORM.set(platform.clone());
    match snackpilot_model::AppModel::new(dir.clone(), platform) {
        Ok(session) => {
            let session = Arc::new(session);
            // Task 5.7: expose the session to the GPUI shell and run the
            // launch order (cached first, broadcast, auto-login).
            crate::install_shell_model(Arc::clone(&session));
            session.bootstrap();
            let _ = DEBUG_SESSION.set(session);
        }
        Err(e) => log::error!("[3.5] register_shims: model session failed: {e:?}"),
    }
    *guard = Some(Registered {
        _table: table,
        _storage_dir: dir,
    });
    log::info!("[3.5] shim table registered");
    1
}

/// Swift: completes a pending asynchronous request. `ok = 1` carries an
/// optional payload, `ok = 0` carries the error message. The payload is read
/// before this call returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn snackpilot_ios_complete(request_id: u64, ok: u8, payload: *const c_char) {
    // SAFETY: null-checked; Swift owns the NUL-terminated string for the call.
    let payload = if payload.is_null() {
        None
    } else {
        Some(unsafe { read_cstr(payload) }.to_string_lossy().into_owned())
    };
    let outcome = if ok != 0 {
        Outcome::Ok(payload)
    } else {
        Outcome::Failed(payload.unwrap_or_else(|| "platform shim failed".into()))
    };
    let registry = *ACTIVE_PENDING.lock().unwrap_or_else(|e| e.into_inner());
    (registry.unwrap_or_else(pending)).resolve(request_id, outcome);
}

/// Swift: CoreLocation enter/exit edge. Returns immediately; the model call
/// runs on its own thread because `handle_geofence` delivers notifications
/// back through the shim table, which must not block the location callback.
#[unsafe(no_mangle)]
pub extern "C" fn snackpilot_ios_on_geofence_event(inside: u8) {
    let inside = inside != 0;
    std::thread::spawn(move || {
        // register_shims installs the session; the location callback can
        // arrive before bootstrap finished, so wait for it.
        for _ in 0..30 {
            if let Some(session) = DEBUG_SESSION.get() {
                log::info!("[3.5] geofence event: inside={inside}, delegating to model");
                session.on_geofence_event(inside);
                log::info!("[3.5] geofence event handled by model");
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        log::error!("[3.5] geofence event: model session not installed");
    });
}

/// Swift: runs the evidence platform exercise on a background thread; every
/// step logs to stderr tagged "[3.5]".
#[unsafe(no_mangle)]
pub extern "C" fn snackpilot_ios_run_debug_exercise() {
    std::thread::spawn(|| {
        for _ in 0..30 {
            if let Some(platform) = DEBUG_PLATFORM.get() {
                // The registration happens before the window exists; a banner
                // only presents while the app is active, so let it come up
                // first (this is why the exercise is not run inline).
                std::thread::sleep(Duration::from_millis(1500));
                run_exercise(platform);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        log::error!("[3.5] debug exercise: platform not installed");
    });
}

fn log_step(what: &str, result: &Result<(), PlatformError>) {
    match result {
        Ok(()) => log::info!("[3.5] {what}: ok"),
        Err(e) => log::error!("[3.5] {what}: {e:?}"),
    }
}

/// Mirrors the Android exercise (task 3.4): credential round trip, immediate
/// and scheduled notifications with a cancel, the geofence lifecycle and one
/// analytics event. Credential values are never logged, only the match flag.
fn run_exercise(platform: &IosPlatform) {
    use std::time::{Instant, SystemTime, UNIX_EPOCH};
    let t0 = Instant::now();
    // Ask up front so the simulator shows the system dialog and the banners
    // below actually display (REQ-PLT-02); the evidence run taps Allow.
    match platform.request_notification_permission() {
        Ok(g) => log::info!("[3.5] request_notification_permission: granted={g}"),
        Err(e) => log::error!("[3.5] request_notification_permission: {e:?}"),
    }
    log_step(
        "save_credential(gourmet_username)",
        &platform.save_credential("gourmet_username", "evidence-user"),
    );
    match platform.get_credential("gourmet_username") {
        Ok(Some(v)) => log::info!(
            "[3.5] credential round trip match={} key=gourmet_username",
            v == "evidence-user"
        ),
        Ok(None) => log::error!("[3.5] get_credential(gourmet_username): absent"),
        Err(e) => log::error!("[3.5] get_credential(gourmet_username): {e:?}"),
    }
    log_step(
        "notify fire-now",
        &platform.notify(NotificationCommand::FireNow {
            id: "evidence-fire".into(),
            title: "3.5 Debug sofort".into(),
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
            title: "3.5 Debug geplant".into(),
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
    // The geofence steps need a saved company location (REQ-PLT-03); the
    // evidence run may be on a fresh install that never completed the
    // location setup, so seed the Vienna company coordinates if absent.
    let kv = snackpilot_core::storage::FileKv::new(platform.storage_dir.clone());
    if snackpilot_core::storage::cache::load_company_location(&kv).is_none() {
        let seeded = snackpilot_core::storage::cache::save_company_location(
            &kv,
            &snackpilot_core::domain::location::CompanyLocation {
                latitude: 48.2082,
                longitude: 16.3738,
            },
        );
        if let Err(e) = seeded {
            log::error!("[3.5] seed company location: {e:?}");
        }
    }
    log_step("start_geofence", &platform.start_geofence());
    match platform.is_geofence_monitoring() {
        Ok(m) => log::info!("[3.5] is_geofence_monitoring: {m}"),
        Err(e) => log::error!("[3.5] is_geofence_monitoring: {e:?}"),
    }
    log_step("stop_geofence", &platform.stop_geofence());
    log_step(
        "track",
        &platform.track(
            "debug.exercise",
            HashMap::from([("task".to_string(), "3.5".to_string())]),
        ),
    );
    log::info!(
        "[3.5] debug exercise finished in {} ms",
        t0.elapsed().as_millis()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::MutexGuard;

    // The stub keychain duplicates values like Swift's `strdup` does.
    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
    }

    /// The stubs below are process-global, so tests take this lock instead of
    /// running in parallel.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[derive(Default)]
    struct StubState {
        secret: Option<String>,
        /// When set, the next shim call returns 0 (Swift-side failure).
        fail: Option<String>,
        /// When set, the async shims accept the request but never complete it.
        never_complete: bool,
        location: Option<(f64, f64)>,
        location_fail: bool,
        granted: bool,
        monitoring: bool,
        calls: Vec<(String, Vec<String>)>,
        track_json: Option<String>,
    }

    static STUB: Mutex<StubState> = Mutex::new(StubState {
        secret: None,
        fail: None,
        never_complete: false,
        location: None,
        location_fail: false,
        granted: false,
        monitoring: false,
        calls: Vec::new(),
        track_json: None,
    });

    fn stub() -> MutexGuard<'static, StubState> {
        STUB.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn s(p: *const c_char) -> String {
        if p.is_null() {
            String::new()
        } else {
            // SAFETY: stubs are only called with CStrings built by the bridge.
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    }

    /// Mimic Swift's `strdup` so the bridge's `free` is exercised too.
    extern "C" fn stub_keychain_set(key: *const c_char, value: *const c_char) -> u8 {
        let mut stub = stub();
        stub.calls
            .push(("keychain_set".into(), vec![s(key), s(value)]));
        if stub.fail.take().is_some() {
            return 0;
        }
        stub.secret = Some(s(value));
        1
    }

    extern "C" fn stub_keychain_get(key: *const c_char, out: *mut *mut c_char) -> u8 {
        let mut stub = stub();
        stub.calls.push(("keychain_get".into(), vec![s(key)]));
        let Some(value) = stub.secret.clone() else {
            return 0;
        };
        if out.is_null() {
            return 0;
        }
        let bytes = value.as_bytes();
        // SAFETY: plain malloc/memcpy pair; the bridge frees with free().
        unsafe {
            let p = malloc(bytes.len() + 1) as *mut c_char;
            assert!(!p.is_null());
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, p, bytes.len());
            *p.add(bytes.len()) = 0;
            *out = p;
        };
        1
    }

    extern "C" fn stub_keychain_delete(key: *const c_char) -> u8 {
        let mut stub = stub();
        stub.calls.push(("keychain_delete".into(), vec![s(key)]));
        if stub.fail.take().is_some() {
            return 0;
        }
        stub.secret = None;
        1
    }

    extern "C" fn stub_notify(cmd: *const WireCommandC) -> u8 {
        let mut stub = stub();
        if cmd.is_null() {
            return 0;
        }
        // SAFETY: the bridge passes a valid WireCommandC with valid strings.
        let c = unsafe { *cmd };
        stub.calls.push((
            "notify".into(),
            vec![
                c.kind.to_string(),
                s(c.id),
                s(c.title),
                s(c.body),
                s(c.channel_id),
                c.fire_at_epoch_ms.to_string(),
            ],
        ));
        u8::from(stub.fail.take().is_none())
    }

    fn permission_shim(name: &str, id: u64) -> u8 {
        let mut stub = stub();
        stub.calls.push((name.into(), vec![id.to_string()]));
        if stub.fail.take().is_some() {
            return 0;
        }
        if !stub.never_complete {
            let payload = CString::new(if stub.granted { "true" } else { "false" }).unwrap();
            drop(stub);
            unsafe { snackpilot_ios_complete(id, 1, payload.as_ptr()) };
        }
        1
    }

    extern "C" fn stub_notify_permission(id: u64, _ctx: *mut c_void) -> u8 {
        permission_shim("notify_permission", id)
    }

    extern "C" fn stub_location_authorisation(id: u64, _ctx: *mut c_void) -> u8 {
        permission_shim("location_authorisation", id)
    }

    extern "C" fn stub_location_current(id: u64, _ctx: *mut c_void) -> u8 {
        let mut stub = stub();
        stub.calls
            .push(("location_current".into(), vec![id.to_string()]));
        if stub.fail.take().is_some() {
            return 0;
        }
        if !stub.never_complete {
            let payload = if stub.location_fail {
                drop(stub);
                unsafe { snackpilot_ios_complete(id, 0, c"location unavailable".as_ptr()) };
                return 1;
            } else {
                match stub.location {
                    Some((lat, lon)) => format!("{lat},{lon}"),
                    None => String::new(),
                }
            };
            drop(stub);
            let payload = CString::new(payload).unwrap();
            unsafe { snackpilot_ios_complete(id, 1, payload.as_ptr()) };
        }
        1
    }

    extern "C" fn stub_geofence_start(lat: f64, lon: f64, id: u64, _ctx: *mut c_void) -> u8 {
        let mut stub = stub();
        stub.calls.push((
            "geofence_start".into(),
            vec![lat.to_string(), lon.to_string(), id.to_string()],
        ));
        if stub.fail.take().is_some() {
            return 0;
        }
        if !stub.never_complete {
            drop(stub);
            unsafe { snackpilot_ios_complete(id, 1, std::ptr::null()) };
        }
        1
    }

    extern "C" fn stub_geofence_stop() -> u8 {
        let mut stub = stub();
        stub.calls.push(("geofence_stop".into(), vec![]));
        u8::from(stub.fail.take().is_none())
    }

    extern "C" fn stub_geofence_is_monitoring() -> u8 {
        let mut stub = stub();
        stub.calls.push(("geofence_is_monitoring".into(), vec![]));
        u8::from(stub.monitoring)
    }

    extern "C" fn stub_track(json: *const c_char) -> u8 {
        let mut stub = stub();
        stub.calls.push(("track".into(), vec![s(json)]));
        stub.track_json = Some(s(json));
        u8::from(stub.fail.take().is_none())
    }

    extern "C" fn stub_open_email(
        to: *const c_char,
        subject: *const c_char,
        body: *const c_char,
    ) -> u8 {
        let mut stub = stub();
        stub.calls
            .push(("open_email".into(), vec![s(to), s(subject), s(body)]));
        u8::from(stub.fail.take().is_none())
    }

    extern "C" fn stub_open_settings() -> u8 {
        stub().calls.push(("open_settings".into(), vec![]));
        1
    }

    extern "C" fn stub_open_app_settings() -> u8 {
        stub().calls.push(("open_app_settings".into(), vec![]));
        1
    }

    extern "C" fn stub_set_app_icon(icon: *const c_char) -> u8 {
        let mut stub = stub();
        stub.calls.push(("set_app_icon".into(), vec![s(icon)]));
        u8::from(stub.fail.take().is_none())
    }

    fn stub_table() -> IosShimTable {
        IosShimTable {
            size: TABLE_SIZE,
            keychain_set: Some(stub_keychain_set),
            keychain_get: Some(stub_keychain_get),
            keychain_delete: Some(stub_keychain_delete),
            notify: Some(stub_notify),
            notify_permission: Some(stub_notify_permission),
            location_authorisation: Some(stub_location_authorisation),
            location_current: Some(stub_location_current),
            geofence_start: Some(stub_geofence_start),
            geofence_stop: Some(stub_geofence_stop),
            geofence_is_monitoring: Some(stub_geofence_is_monitoring),
            track: Some(stub_track),
            open_email: Some(stub_open_email),
            open_settings: Some(stub_open_settings),
            open_app_settings: Some(stub_open_app_settings),
            set_app_icon: Some(stub_set_app_icon),
        }
    }

    /// Short-timeout registry shared by every test platform; ids are unique
    /// via the atomic counter, so slots never collide.
    fn test_pending() -> &'static Pending {
        static P: OnceLock<Pending> = OnceLock::new();
        P.get_or_init(|| Pending::with_timeout(Duration::from_millis(250)))
    }

    /// Serialise the tests and clear the stub state.
    fn begin() -> MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        *stub() = StubState::default();
        guard
    }

    fn platform_in(dir: PathBuf) -> IosPlatform {
        IosPlatform::new(stub_table(), test_pending(), dir).unwrap()
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "snackpilot-ios-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH_STUB)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const UNIX_EPOCH_STUB: std::time::SystemTime = std::time::SystemTime::UNIX_EPOCH;

    fn take_calls() -> Vec<(String, Vec<String>)> {
        std::mem::take(&mut stub().calls)
    }

    fn methods(calls: &[(String, Vec<String>)]) -> Vec<&str> {
        calls.iter().map(|(m, _)| m.as_str()).collect()
    }

    // ---- credentials ---------------------------------------------------------

    /// @req REQ-PLT-01 save/get/delete round trip through the stub table; the
    /// strdup'ed value is read back and freed.
    #[test]
    fn credential_round_trip() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        p.save_credential("gourmet_username", "evidence-user")
            .unwrap();
        assert_eq!(
            p.get_credential("gourmet_username").unwrap().as_deref(),
            Some("evidence-user")
        );
        p.clear_credential("gourmet_username").unwrap();
        assert_eq!(p.get_credential("gourmet_username").unwrap(), None);
        let calls = take_calls();
        assert_eq!(
            methods(&calls),
            vec![
                "keychain_set",
                "keychain_get",
                "keychain_delete",
                "keychain_get"
            ]
        );
        assert_eq!(calls[0].1, vec!["gourmet_username", "evidence-user"]);
    }

    /// @req REQ-PLT-06 a failing keychain shim surfaces as a typed Storage
    /// error, never a panic.
    #[test]
    fn credential_failure_is_typed() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        stub().fail = Some("keychain down".into());
        let err = p.save_credential("gourmet_password", "x").unwrap_err();
        assert!(matches!(err, PlatformError::Storage(_)), "{err:?}");
    }

    // ---- notifications -------------------------------------------------------

    /// @req REQ-PLT-02 all three command kinds marshal one-to-one: kind, id,
    /// title, body, channel and fire time cross the boundary unchanged.
    #[test]
    fn notify_marshals_all_kinds() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        p.notify(NotificationCommand::FireNow {
            id: "daily-order-reminder".into(),
            title: "Bestellen?".into(),
            body: "Menü wählen".into(),
            channel_id: Some("order-reminders".into()),
            screen: None,
        })
        .unwrap();
        p.notify(NotificationCommand::ScheduleAt {
            id: "menu-new".into(),
            title: "Neues Menü".into(),
            body: "Heute".into(),
            channel_id: None,
            fire_at_epoch_ms: 1_700_000_000_000,
            screen: None,
        })
        .unwrap();
        p.notify(NotificationCommand::CancelPending {
            id: "menu-new".into(),
        })
        .unwrap();
        let calls = take_calls();
        assert_eq!(methods(&calls), vec!["notify"; 3]);
        assert_eq!(
            calls[0].1,
            vec![
                "0",
                "daily-order-reminder",
                "Bestellen?",
                "Menü wählen",
                "order-reminders",
                "0"
            ]
        );
        assert_eq!(
            calls[1].1,
            vec!["1", "menu-new", "Neues Menü", "Heute", "", "1700000000000"]
        );
        assert_eq!(calls[2].1, vec!["2", "menu-new", "", "", "", "0"]);
    }

    // ---- permissions ---------------------------------------------------------

    /// @req REQ-PLT-06 a granted and a denied dialog both resolve through the
    /// request-id completion without panicking.
    #[test]
    fn permission_resolves_granted_and_denied() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        stub().granted = true;
        assert!(p.request_notification_permission().unwrap());
        stub().granted = false;
        assert!(!p.request_notification_permission().unwrap());
        stub().granted = true;
        assert!(p.request_location_authorisation().unwrap());
        assert_eq!(
            methods(&take_calls()),
            vec![
                "notify_permission",
                "notify_permission",
                "location_authorisation"
            ]
        );
    }

    /// @req REQ-PLT-06 a shim that never completes times out with a typed
    /// System error instead of hanging.
    #[test]
    fn permission_times_out() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        stub().never_complete = true;
        let err = p.request_notification_permission().unwrap_err();
        match err {
            PlatformError::System(message) => assert!(
                message.contains("timed out"),
                "unexpected message: {message}"
            ),
            other => panic!("expected System timeout, got {other:?}"),
        }
    }

    // ---- location and geofence -------------------------------------------------

    /// @req REQ-PLT-03 a fix arrives as `"lat,lon"`, an empty payload is a
    /// miss (Ok(None), parity with Android found=false), and a Swift-side
    /// failure is a typed Location error.
    #[test]
    fn current_location_fix_and_miss() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        stub().location = Some((48.2082, 16.3738));
        assert_eq!(
            p.current_location().unwrap(),
            Some(LocationFix {
                latitude: 48.2082,
                longitude: 16.3738
            })
        );
        stub().location = None;
        assert_eq!(p.current_location().unwrap(), None);
        stub().location_fail = true;
        assert!(matches!(
            p.current_location().unwrap_err(),
            PlatformError::Location(_)
        ));
        assert_eq!(methods(&take_calls()), vec!["location_current"; 3]);
    }

    /// @req REQ-PLT-03 start_geofence reads the saved company location from
    /// the core KV store and hands its coordinates to the shim; without a
    /// saved location it fails typed and never calls the shim.
    #[test]
    fn start_geofence_uses_saved_company_location() {
        let _guard = begin();
        let dir = temp_dir("geofence");
        snackpilot_core::storage::cache::save_company_location(
            &snackpilot_core::storage::FileKv::new(dir.clone()),
            &snackpilot_core::domain::location::CompanyLocation {
                latitude: 48.2082,
                longitude: 16.3738,
            },
        )
        .unwrap();
        let p = platform_in(dir.clone());
        p.start_geofence().unwrap();
        let calls = take_calls();
        assert_eq!(methods(&calls), vec!["geofence_start"]);
        assert_eq!(calls[0].1[0], "48.2082");
        assert_eq!(calls[0].1[1], "16.3738");

        let empty = platform_in(temp_dir("geofence-empty"));
        let err = empty.start_geofence().unwrap_err();
        assert!(matches!(err, PlatformError::Location(_)), "{err:?}");
        assert!(take_calls().is_empty(), "shim must not be called");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// @req REQ-PLT-03 the monitoring flag and stop cross the boundary as
    /// plain synchronous calls.
    #[test]
    fn geofence_monitoring_flag() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        assert!(!p.is_geofence_monitoring().unwrap());
        stub().monitoring = true;
        assert!(p.is_geofence_monitoring().unwrap());
        p.stop_geofence().unwrap();
        assert_eq!(
            methods(&take_calls()),
            vec![
                "geofence_is_monitoring",
                "geofence_is_monitoring",
                "geofence_stop"
            ]
        );
    }

    // ---- analytics -------------------------------------------------------------

    /// @req REQ-PLT-04 the event and its properties cross as one sorted,
    /// escaped JSON object the Swift sink parses.
    #[test]
    fn track_forwards_json() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        p.track(
            "order.submitted",
            HashMap::from([
                ("b".to_string(), "2".to_string()),
                ("a".to_string(), "1".to_string()),
                ("quote".to_string(), "x\"y\\z\n".to_string()),
            ]),
        )
        .unwrap();
        assert_eq!(
            stub().track_json.as_deref(),
            Some(r#"{"event":"order.submitted","props":{"a":"1","b":"2","quote":"x\"y\\z\n"}}"#)
        );
        take_calls();
    }

    // ---- failures are typed per capability --------------------------------------

    /// @req REQ-PLT-06 a failing shim maps to the variant the capability owns:
    /// notify → Notification, location → Location, track → System.
    #[test]
    fn failures_map_to_typed_variants() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        stub().fail = Some("nope".into());
        assert!(matches!(
            p.notify(NotificationCommand::CancelPending { id: "x".into() })
                .unwrap_err(),
            PlatformError::Notification(_)
        ));
        stub().fail = Some("nope".into());
        assert!(matches!(
            p.current_location().unwrap_err(),
            PlatformError::Location(_)
        ));
        stub().fail = Some("nope".into());
        assert!(matches!(
            p.track("e", HashMap::new()).unwrap_err(),
            PlatformError::System(_)
        ));
        stub().fail = Some("nope".into());
        assert!(matches!(
            p.set_app_icon("emerald").unwrap_err(),
            PlatformError::System(_)
        ));
    }

    /// @req REQ-PLT-06 a shim that rejects the request synchronously (returns
    /// 0 after begin) must not leave its slot behind in the pending registry,
    /// on any of the three async paths.
    #[test]
    fn rejected_shim_leaves_no_pending_slot() {
        let _guard = begin();
        let dir = temp_dir("reject");
        snackpilot_core::storage::cache::save_company_location(
            &snackpilot_core::storage::FileKv::new(dir.clone()),
            &snackpilot_core::domain::location::CompanyLocation {
                latitude: 48.2082,
                longitude: 16.3738,
            },
        )
        .unwrap();
        let p = platform_in(dir.clone());
        let slots = || test_pending().slots.lock().unwrap().len();
        assert_eq!(slots(), 0, "registry must start empty");

        for call in [
            Box::new(|| p.request_notification_permission().map(|_| ()))
                as Box<dyn Fn() -> Result<(), PlatformError>>,
            Box::new(|| p.current_location().map(|_| ())),
            Box::new(|| p.start_geofence()),
        ] {
            stub().fail = Some("nope".into());
            assert!(call().is_err(), "rejected shim must surface an error");
            assert_eq!(slots(), 0, "rejected request leaked a pending slot");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- system actions -----------------------------------------------------------

    #[test]
    fn system_actions_cross_the_table() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        p.compose_email(SupportEmail {
            to: "support@example.com".into(),
            subject: "Hallo".into(),
            body: "Text".into(),
        })
        .unwrap();
        p.open_app_settings().unwrap();
        p.set_app_icon("emerald").unwrap();
        let calls = take_calls();
        assert_eq!(
            methods(&calls),
            vec!["open_email", "open_app_settings", "set_app_icon"]
        );
        assert_eq!(calls[0].1, vec!["support@example.com", "Hallo", "Text"]);
        assert_eq!(calls[2].1, vec!["emerald"]);
    }

    #[test]
    fn capabilities_declare_both_subsystems() {
        let _guard = begin();
        let p = platform_in(PathBuf::new());
        let caps = p.capabilities();
        assert!(caps.notifications && caps.geofence);
    }

    // ---- registration -------------------------------------------------------------

    #[test]
    fn register_shims_installs_and_rejects() {
        let _guard = begin();
        *TABLE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let dir = temp_dir("register");
        let dir_c = CString::new(dir.to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { snackpilot_ios_register_shims(std::ptr::null(), dir_c.as_ptr()) },
            0
        );
        let mut table = stub_table();
        table.size = 3;
        assert_eq!(
            unsafe { snackpilot_ios_register_shims(&table, dir_c.as_ptr()) },
            0,
            "wrong size must be rejected"
        );
        table.size = TABLE_SIZE;
        let mut missing = stub_table();
        missing.notify = None;
        assert_eq!(
            unsafe { snackpilot_ios_register_shims(&missing, dir_c.as_ptr()) },
            0,
            "missing entry must be rejected"
        );
        assert_eq!(
            unsafe { snackpilot_ios_register_shims(&table, dir_c.as_ptr()) },
            1
        );
        assert_eq!(
            unsafe { snackpilot_ios_register_shims(&stub_table(), dir_c.as_ptr()) },
            0,
            "second registration must be refused"
        );
        assert!(DEBUG_PLATFORM.get().is_some());
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- debug exercise -------------------------------------------------------------

    #[test]
    fn exercise_exercises_every_capability() {
        let _guard = begin();
        stub().granted = true;
        stub().monitoring = true;
        let dir = temp_dir("exercise");
        let p = platform_in(dir.clone());
        run_exercise(&p);
        let calls = take_calls();
        assert_eq!(
            methods(&calls),
            vec![
                "notify_permission",
                "keychain_set",
                "keychain_get",
                "notify",
                "notify",
                "notify",
                "notify",
                "geofence_start",
                "geofence_is_monitoring",
                "geofence_stop",
                "track",
            ]
        );
        assert_eq!(calls[3].1[0], "0", "fire-now first");
        assert_eq!(calls[6].1[0], "2", "cancel last");
        assert_eq!(calls[6].1[1], "evidence-cancelled");
        assert!(stub()
            .track_json
            .as_deref()
            .is_some_and(|j| j.contains("\"task\":\"3.5\"")));
        std::fs::remove_dir_all(&dir).ok();
    }
}
