# SnackPilot v2 — Architecture

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This document expands §4 ("Architecture — Rust Core, Native Shells") of the approved
design spec (docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md) into the
working architecture for v2: repository layout, the thick Rust core and its module map,
the UniFFI facade surface, host-injection points, the threading/async model, the
per-platform service table, the credential-takeover flow, and the testing architecture.

Behavioral truth lives in `docs/requirements/`; this document assigns each behavior an
owner (core module or native shell) and defines the boundary between them. Where this
doc restates a v1 fact, the provenance annotation and the owning requirements doc are
given — the requirements doc always wins on behavioral detail.

Platform scope: **iOS 17+ (Swift/SwiftUI) and Android 10+/API 29 (Kotlin/Jetpack
Compose) only.** The v1 Tauri desktop and web targets are dropped (design spec §1).

---

## 1. Repository layout (v2 branch)

The `v2` orphan branch starts with only `docs/` + scaffolding (design spec §3) and grows
into:

```
docs/
├── requirements/                  # Phase-0 extraction (00–07, 03-features/*, appendix)
├── architecture/
│   └── v2-architecture.md         # this document
├── fixtures/                      # 13 sanitized fixtures carried from v1 (canonical copy;
│   ├── gourmet/                   #   mirrored into core test assets, see §9.1)
│   └── ventopay/
└── privacy.html                   # store privacy policy, hosted via GitHub Pages
                                   #   (07-release §7 — URL must stay stable)
src/core/                          # snackpilot-core: Rust crate + UniFFI bindings
├── Cargo.toml
├── src/
│   ├── lib.rs                     # UniFFI proc-macro exports (facade, §4)
│   ├── http/                      # transport trait + reqwest impl, cookie handling
│   ├── gourmet/                   # client.rs, parser.rs, api.rs   (01-gourmet-scraping)
│   ├── ventopay/                  # client.rs, parser.rs, api.rs   (02-ventopay-scraping)
│   ├── demo/                      # demo_data.rs, demo_gourmet.rs, demo_ventopay.rs
│   ├── domain/                    # plain records: menu, order, billing, ventopay, user
│   ├── features/                  # menus.rs, orders.rs, billing.rs (store-equivalents)
│   ├── storage/                   # kv.rs (plain persistent KV under injected dir), cache.rs
│   ├── notify/                    # fingerprint.rs, menu_check.rs, daily_reminder.rs,
│   │                              #   cancel_reminder.rs, geofence.rs, log.rs, commands.rs
│   ├── datetime/                  # Vienna time, date formats, cutoffs (dateUtils port)
│   └── ffi/                       # facade types & glue (records, enums, callbacks)
├── tests/                         # fixture-driven contract tests (§9)
│   └── fixtures/                  # working copy of docs/fixtures (kept byte-identical)
└── tools/
    └── recorder/                  # dev-only fixture recorder binary (§9.4)
src/ios/                           # SwiftUI app (Xcode project; core via XCFramework + SPM)
src/android/                       # Kotlin/Compose app (Gradle; core as AAR via cargo-ndk)
tools/icon-tools/                  # icon SVG/PNG pipeline, carried from v1 (07-release §6)
.github/workflows/                 # core-test, ios-build, android-build, release, security-audit
.env.example                       # KANTINE_USERNAME/PASSWORD, AUTOMATEN_USERNAME/PASSWORD
                                   #   (same four variables as v1; 06-testing §4)
```

Build artifacts flow: `src/core` compiles to an XCFramework (aarch64-apple-ios +
simulator slices) consumed by `src/ios` via SPM, and to `.so`s packaged as an AAR (via
`cargo-ndk`, arm64-v8a at minimum) consumed by `src/android`. UniFFI generates the
Swift and Kotlin binding sources as part of those packaging steps.

---

## 2. Ownership map — requirements doc → implementer

Rule of thumb (design spec §4): the core owns everything portable — HTTP, parsing,
domain models, caching, fingerprinting, notification *decisions*, demo mode. The shells
own UI, secure storage, OS scheduling, notification *delivery*, location, permissions,
and analytics transport.

| Requirements doc | Rust core | Native shells |
|---|---|---|
| `01-gourmet-scraping.md` | **Entire doc**: `gourmet::{client,parser,api}` — request sequences, multipart encoding, `ufprt`+`__ncforminfo`, selectors, session expiry/re-login, JSON APIs, logout | — |
| `02-ventopay-scraping.md` | **Entire doc**: `ventopay::{client,parser,api}` — ASP.NET state flow, manual cookie jar (§3.2 below), transactions parsing, Gourmet filter | — |
| `03-features/menus.md` | `features::menus` — items/cache/TTL state, pending-order/cancellation sets, composite keys, cutoff rules (`datetime`), submit pipeline, availability merge, nearest-date selection, error strings | Menus screen: day navigator, swipe gesture, category grouping/rendering, card visuals, FAB, banners (04-ui-ux §3.1) |
| `03-features/orders.md` | `features::orders` — fetch/confirm/cancel orchestration, upcoming/past split, post-fetch notification hooks, error catalog | Orders screen: tabs, confirm banner, cancel dialog, row rendering, description lookup display |
| `03-features/billing.md` | `features::billing` — month options/keys (Austrian labels), date ranges, dual-source fetch + skip rules, totals, per-month caches | Billing screen: month selector, source-filter chips, summary bar, cards, `de-AT` currency/date formatting |
| `03-features/caching.md` | **Entire doc**: `storage::{kv,cache}` — cache keys, serialization, TTL/invalidation, corrupt-entry handling, merge semantics | — (shells never touch cache files directly) |
| `03-features/notifications-new-menu.md` | `notify::{fingerprint,menu_check}` — fingerprint algorithm + persisted state, full background-task algorithm/state machine, foreground detect-and-acknowledge, notification content | BGTask/WorkManager registration, delivering the local notification, `menu-updates` channel, the in-app toast component |
| `03-features/notifications-daily-reminder.md` | `notify::daily_reminder` — guards, Vienna-time math, body construction, schedule/cancel decisions; reminder settings persistence | Executing schedule/cancel commands via UNUserNotificationCenter / NotificationManager; settings UI; background-task registration |
| `03-features/notifications-cancel-reminder.md` | `notify::cancel_reminder` — decision logic, 08:45 target / 08:45–09:00 immediate window / 09:00 deadline | Command execution; geofence event delivery into the core |
| `03-features/notifications-location.md` | `notify::geofence` — Enter/Exit decision logic, `is_at_company` + company-location persistence, 14:00 cutoff, notification content | CoreLocation region monitoring / GeofencingClient (1 region, id `company`, 500 m), permission flows, one-shot GPS capture, settings UI |
| `03-features/notification-log.md` | **Storage & format**: `notify::log` — keys, 200-entry cap, activation window, e-mail formatting, all core-side call sites | Log section UI, mail composer, shell-side `append_log_entry` calls (e.g. OS geofence errors) |
| `03-features/themes.md` | — | **Entire doc**, twice: palette/accent constants, light/dark resolution, persistence, alternate app icons. The color tables in that doc are the single source both shells transcribe. |
| `03-features/demo-mode.md` | **Entire doc**: `demo::*` — magic-credential detection, seeded PRNG generators, demo API implementations swapped in behind the same facade | No demo-specific UI exists (v1 shows only the `Demo User` name) |
| `03-features/settings.md` | Auth state machines' *logic* (login/loginWithSaved/logout semantics, demo branch, error strings) inside `gourmet::api`/`ventopay::api` | All four settings screens, credential input UX, save-before-validate ordering, secure-storage reads/writes, startup auto-login trigger |
| `03-features/analytics.md` | Emits core-originated events (`order.submitted`, `order.confirmed`, `order.cancelled`, `auth.*`, `menu.newDetected`) through the injected `AnalyticsSink` (§5) | TelemetryDeck transport (native Swift/Kotlin SDKs), app ID `BA25F62D-0154-4A92-BF85-29FC5FDDA3EC`, constant `clientUser = "anonymous"`, default payload, lifecycle + `screen.viewed` events, release-build gating |
| `04-ui-ux.md` | — | **Entire doc**: navigation graph, tab bars (iOS glass pill / Android Material), all screens, dialogs, German strings |
| `05-platform-services.md` | Plain KV persistence lives in the core under the injected storage dir (§5) | Secure storage + credential takeover (§8), background scheduling, notification channels/permissions, location permissions, app identity config |
| `06-testing.md` | Fixture-driven contract test suite (§9) | Thin shell tests (§9.5) |
| `07-release.md` | Crate version participates in the single-version scheme | Store identities, CI, ship script, icon pipeline |

Every v1 source file in `appendix-source-map.md` therefore lands in exactly one of:
a core module above, a shell feature above, or the "Dropped in v2" list (desktop/web).

---

## 3. Rust core — key internal decisions

### 3.1 HTTP transport

- One internal trait, `http::Transport`, with a production implementation on `reqwest`
  and a capturing fake for tests (§9.2). All request-shape guarantees are enforced in
  `gourmet::client` / `ventopay::client` *above* this trait, so tests pin the exact
  bytes independent of reqwest.
- **Gourmet client**: dedicated `reqwest` cookie store per client instance; redirects
  followed (limit 5), status 200–399 = success, `multipart/form-data` bodies with parts
  emitted in insertion order, `Origin`/`Referer` header rules and `lastPageUrl` tracking
  exactly per 01-gourmet-scraping §2. reqwest must be explicitly configured to send
  `Accept: application/json, text/plain, */*` on every GET and POST — this was an axios
  app-level default in v1 (not a platform default), and omitting it changes request bytes
  on a ban-sensitive site (01 §2). This replaces v1's NSURLSession/OkHttp native cookie
  handling — a deliberate, documented behavior change (design spec §4): same request
  sequence, one implementation on both platforms, validated against fixtures.
- **Ventopay client**: reqwest's cookie store is **disabled**; the core implements the
  app-owned jar with v1's exact semantics — take each `Set-Cookie` up to the first `;`,
  split on the first `=`, ignore all attributes, never expire, overwrite preserving
  insertion position, emit one `Cookie: name1=value1; name2=value2` header in insertion
  order, no header when empty (02-ventopay-scraping §2.2; v1:
  src/app/src-rn/api/ventopayClient.ts:31-58). A generic cookie store must not be used
  here because it would not reproduce insertion-order emission and attribute-ignoring.
- **No custom User-Agent** is set by the core (01 §2 / 02 §2.1 ban rules). Note that
  reqwest sends *no* User-Agent header by default, whereas v1 sent the platform HTTP
  stack's default UA — see Open Questions (§10).
- **No throttling, no artificial delays** anywhere in the request path
  (01-gourmet-scraping §13.8).
- Exactly one session per service at a time; operations on a service are serialized
  (§6.3).

### 3.2 HTML parsing

`scraper` crate (CSS selectors). v1's Cheerio selectors transfer 1:1; the handful of
non-CSS constructs in v1 (`:contains(...)` in the logout-form lookup, "direct text
nodes only" title extraction, prefix-matched input names) are implemented as small
explicit helpers in `gourmet::parser` with the exact semantics documented in
01-gourmet-scraping §5, §8.2, §9, §11. All regexes (category detection
`/MEN(?:Ü|U)\s+([I]{1,3})/i`, Ventopay login check `/href="Ausloggen\.aspx"/i`,
timestamp `/(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})/u`, title
`/€\s*([\d,]+)\s*\((.+)\)/`) are copied byte-for-byte from the requirements docs.

### 3.3 Persistence (`storage::kv`)

A single durable, unencrypted key-value store rooted at the host-injected storage
directory (one JSON file per key, or SQLite — implementation detail; semantics are the
contract). It carries everything v1 kept in AsyncStorage, with the same key names and
value formats so behavior transfers verbatim (03-features/caching §1–2;
05-platform-services §2):

| Key | Owner module |
|---|---|
| `menus_items`, `orders_list`, `billing_{YYYY-MM}`, `ventopay_billing_{YYYY-MM}` | `features::*` via `storage::cache` |
| `known_menu_fingerprints`, `menu_notification_sent` | `notify::menu_check` |
| `daily_reminder_enabled`, `daily_reminder_time` | `notify::daily_reminder` |
| `notification_debug_log_entries`, `notification_debug_log_activated_until` | `notify::log` |
| `company-location` (holds `companyLocation` + `isAtCompany`) | `notify::geofence` |

Credentials are **never** written here — secure storage is a shell concern (§5, §8).
`daily_reminder_sent_date` is written-but-never-read in v1
(notifications-daily-reminder §5) and is dropped in v2 (no migration impact: v2 does
not read v1's AsyncStorage files).

The KV store must be safely readable/writable from headless background entry points
(BGTask handlers, WorkManager workers, geofence callbacks) concurrently with the UI
process — an in-process lock (plus atomic file writes) suffices since both run in the
same app process on iOS/Android.

### 3.4 Feature services (the v1 Zustand stores' new home)

`features::menus`, `features::orders`, `features::billing` hold the in-memory state and
orchestration that v1 kept in `menuStore`/`orderStore`/`billingStore`: TTL guards,
in-flight guards, pending sets, optimistic updates and revert, dual-source billing
asymmetry (loading/error reflect Gourmet only), cache write-through points. The shells
render **snapshots** of this state and call operations; they contain no business logic.
The 800 ms minimum-visibility timer of the availability refresh
(03-features/menus §3.3) stays in the core so both shells behave identically.

### 3.5 Notification decisions vs. delivery

The core never calls a notification API. Decision functions return
`NotificationCommand` values (§4.4); the shell executes them 1:1 against
UNUserNotificationCenter / NotificationManager. Identifier-replacement dedupe, the
Vienna-time windows (daily reminder target guard; cancel-reminder 08:45/09:00; geofence
08:45/14:00), body construction, and channel routing are all computed in the core;
the shell contributes only the OS call. This keeps every account-of-record behavior in
tested Rust and makes the shell mapping mechanical.

---

## 4. UniFFI facade surface (proposed)

One exported object, `SnackPilotCore`, plus plain-data records/enums and two
foreign-implemented callback interfaces. All exported functions that touch I/O are
`async` (UniFFI async support; design spec §4 "FFI boundary"). Field names below are
Rust snake_case; UniFFI renders them camelCase in Swift/Kotlin.

Conventions:
- Calendar days cross the FFI as local-date keys `"YYYY-MM-DD"` (the composite-key date
  format of v1, v1: src/app/src-rn/utils/dateUtils.ts:45-50); timestamps as epoch
  milliseconds `i64`. The core performs all date math (Vienna time, Gourmet/Ventopay
  wire formats) internally.
- All user-visible German strings (badges, errors, notification texts) originate in
  the core or are fixed in 04-ui-ux; the facade returns them ready to display.

### 4.1 Lifecycle & injection

```rust
#[derive(uniffi::Record)]
pub struct CoreConfig {
    pub storage_dir: String,          // absolute path, host-owned directory (§5)
}

#[uniffi::export(with_foreign)]
pub trait AnalyticsSink: Send + Sync {  // implemented in Swift/Kotlin
    fn track(&self, event: String, props: HashMap<String, String>); // fire-and-forget
}

#[uniffi::export(with_foreign)]
pub trait ProgressListener: Send + Sync {
    fn on_progress(&self, phase: OrderProgress);   // drives the submit banner
}

impl SnackPilotCore {
    #[uniffi::constructor]
    pub fn new(config: CoreConfig, analytics: Option<Arc<dyn AnalyticsSink>>) -> Arc<Self>;
}
```

### 4.2 Records & enums (derived from the v1 domain models)

```rust
// 01-gourmet-scraping §5 (v1: src-rn/types/menu.ts:26-31)
pub struct GourmetUserInfo { username: String, shop_model_id: String,
                             eater_id: String, staff_group_id: String }

pub struct Credentials { username: String, password: String }

// 03-features/menus §1 (v1: src-rn/types/menu.ts:1-24)
pub enum MenuCategory { Menu1, Menu2, Menu3, SoupAndSalad, Unknown }
// display strings owned by core: "MENÜ I" | "MENÜ II" | "MENÜ III" | "SUPPE & SALAT" | "UNKNOWN"

pub struct MenuItem {
    id: String, day: String /* YYYY-MM-DD */, title: String, subtitle: String,
    allergens: Vec<String>, available: bool, ordered: bool,
    category: MenuCategory, price: String,
}

pub enum OrderProgress { Adding, Confirming, Cancelling, Refreshing }

pub struct MenuSnapshot {
    items: Vec<MenuItem>,
    available_dates: Vec<String>,            // sorted ascending, deduped
    pending_orders: Vec<String>,             // composite keys "{menuId}|{YYYY-MM-DD}"
    pending_cancellations: Vec<String>,
    loading: bool, refreshing: bool,
    error: Option<String>,                   // German store error strings, verbatim
}

// 03-features/orders §1 (v1: src-rn/types/order.ts:1-8)
pub struct OrderedMenu {
    position_id: String, eating_cycle_id: String,
    date_epoch_ms: i64, title: String, subtitle: String, approved: bool,
}

// 03-features/billing §1 (v1: src-rn/types/billing.ts, types/ventopay.ts)
pub struct BillingItem { id: String, article_id: String, count: i64, description: String,
                         total: f64, subsidy: f64, discount_value: f64, is_custom_menu: bool }
pub struct Bill { bill_nr: i64, bill_date_epoch_ms: i64, location: String,
                  items: Vec<BillingItem>, billing: f64 }
pub struct GourmetMonthlyBilling { month_key: String, label: String, bills: Vec<Bill>,
                                   total_gross: f64, total_subsidy: f64,
                                   total_discount: f64, total_billing: f64,
                                   fetched_at: i64 /* 0 = restored from cache */ }
pub struct VentopayTransaction { id: String, date_epoch_ms: i64, amount: f64,
                                 restaurant: String, location: String }
pub struct VentopayMonthlyBilling { month_key: String, label: String,
                                    transactions: Vec<VentopayTransaction>,
                                    total: f64, fetched_at: i64 }
pub struct MonthOption { key: String /* YYYY-MM */, label: String /* "Jänner 2026" */,
                         offset: u8 /* 0..=2 */ }

// §3.5 — the delivery contract between core decisions and shell notification APIs
pub enum NotificationCommand {
    ScheduleAt { id: String, title: String, body: String,
                 channel_id: String, fire_at_epoch_ms: i64, screen: Option<String> },
    FireNow    { id: String, title: String, body: String,
                 channel_id: String, screen: Option<String> },
    CancelPending { id: String },
}
// id ∈ { "daily-order-reminder", "cancel-order-reminder", "geofence-no-order-reminder",
//        or a fresh id for the new-menu notification }; channel_id ∈ { "order-reminders",
//        "menu-updates" } (05-platform-services §5.3/§5.4)

pub enum GeofenceEvent { Enter, Exit }
pub struct CompanyLocation { latitude: f64, longitude: f64 }
pub struct DailyReminderSettings { enabled: bool, hour: Option<u8>, minute: Option<u8> }

pub enum MenuCheckOutcome { NoCredentials, DemoSkipped, Notified, NoNotification }
pub struct MenuCheckResult { outcome: MenuCheckOutcome,
                             notification: Option<NotificationCommand> }

pub struct NewMenuForegroundResult { show_toast: bool }   // "Neue Menüs verfügbar!" toast

// 03-features/notification-log §1–2
pub struct LogState { activated_until_epoch_ms: Option<i64>, entry_count: u32 }
```

Error type (UniFFI error enum) — variants preserve v1's exact user-facing messages
where the shell displays them (01 §14, 02 §3–4, orders §8):

```rust
pub enum CoreError {
    LoginFailed { message: String },   // "Login failed: invalid credentials or account blocked"
                                       // "Ventopay login failed: invalid credentials or account blocked"
    SessionExpired,                    // v1 SessionExpiredError, message "Session expired"
    NotLoggedIn,                       // JSON APIs without cached user info
    AddToCartFailed { message: String },
    EditModeFailed,                    // "Failed to enter edit mode"
    Parse { message: String },         // parser errors incl. missing-token messages, verbatim
    Http { message: String },          // transport / status >= 400
    Storage { message: String },
}
```

### 4.3 Operations — sessions, menus, orders, billing

```rust
// ---- Gourmet session (01 §6–7, settings §3; demo branch per demo-mode §1 is internal)
async fn gourmet_login(&self, creds: Credentials) -> Result<GourmetUserInfo, CoreError>;
async fn gourmet_logout(&self) -> Result<(), CoreError>;      // always resets local session
fn gourmet_is_authenticated(&self) -> bool;
fn gourmet_user_info(&self) -> Option<GourmetUserInfo>;

// ---- Ventopay session (02 §3–4)
async fn ventopay_login(&self, creds: Credentials) -> Result<(), CoreError>;
async fn ventopay_logout(&self) -> Result<(), CoreError>;
fn ventopay_is_authenticated(&self) -> bool;

// ---- Menus (03-features/menus)
async fn load_cached_menus(&self) -> MenuSnapshot;            // corrupt entry → deleted, ignored
async fn fetch_menus(&self, force: bool) -> MenuSnapshot;     // 4 h TTL unless force; in-flight guard
async fn refresh_availability(&self) -> MenuSnapshot;         // merge-only refresh, ≥800 ms, silent failure
fn menu_snapshot(&self) -> MenuSnapshot;
fn toggle_pending_order(&self, menu_id: String, date_key: String) -> MenuSnapshot;
fn clear_pending_changes(&self) -> MenuSnapshot;
fn is_ordering_cutoff(&self, date_key: String) -> bool;       // 09:00 Europe/Vienna rule
async fn submit_orders(&self, progress: Option<Arc<dyn ProgressListener>>)
    -> Result<MenuSnapshot, CoreError>;                       // full §6.5 pipeline of menus.md,
                                                              // incl. optimistic update + revert,
                                                              // emits analytics order.submitted

// ---- Orders (03-features/orders)
async fn load_cached_orders(&self) -> Vec<OrderedMenu>;
async fn fetch_orders(&self) -> Result<OrdersFetchResult, CoreError>;
//   OrdersFetchResult { orders: Vec<OrderedMenu>,
//                       notification_commands: Vec<NotificationCommand> }
//   — the post-fetch hooks of orderStore.fetchOrders (geofence-cancel when an order
//   for Vienna-today exists, daily-reminder check, cancel-reminder check) run inside
//   and surface as commands; hook failures are swallowed (orders.md §8).
async fn confirm_orders(&self) -> Result<Vec<OrderedMenu>, CoreError>;   // emits order.confirmed
async fn cancel_order(&self, position_id: String) -> Result<Vec<OrderedMenu>, CoreError>;
fn split_orders(&self, orders: Vec<OrderedMenu>) -> OrdersSplit;  // upcoming/past at
                                                                  // device-local midnight

// ---- Billing (03-features/billing)
fn month_options(&self) -> Vec<MonthOption>;                  // exactly 3, recomputed per call
async fn load_cached_months(&self) -> BillingSnapshot;
async fn fetch_gourmet_billing(&self, offset: u8) -> Result<Option<GourmetMonthlyBilling>, CoreError>;
async fn fetch_ventopay_billing(&self, offset: u8) -> Option<VentopayMonthlyBilling>;
//   Ventopay failures are swallowed (warn-only, error stays null — billing.md §4.2);
//   skip rules (past-month non-empty, unauthenticated) are internal.
```

### 4.4 Operations — notifications, log, settings-adjacent

```rust
// ---- Background menu check (notifications-new-menu §3; credentials injected by host)
async fn run_background_menu_check(&self, creds: Option<Credentials>) -> MenuCheckResult;
//   Fresh client + cookie store per run, ISOLATED from the foreground session (a
//   deliberate change from v1's shared app-wide cookie store — see §6.3 / §10);
//   demo-credential guard; fingerprint compare; latch state machine; log entries;
//   emits analytics menu.newDetected on fire. Errors → outcome for the host to map
//   to the OS task result (Success/Failed).

// ---- Foreground new-menu detection + acknowledgment (notifications-new-menu §4)
async fn acknowledge_menus(&self) -> NewMenuForegroundResult;
//   Called by the Menus screen after its refresh resolves; performs detection,
//   returns whether to show the toast, then unconditionally writes
//   known=current, sent=false. All errors swallowed internally.

// ---- Order-sync background job (daily + cancel reminder; cached orders only, NO network)
async fn run_order_sync_check(&self) -> Vec<NotificationCommand>;

// ---- Geofence events (notifications-location §4; host delivers OS events)
async fn on_geofence_event(&self, event: GeofenceEvent) -> Vec<NotificationCommand>;
fn set_company_location(&self, loc: Option<CompanyLocation>) -> Vec<NotificationCommand>;
//   None clears the location, resets is_at_company=false, and returns the commands the
//   host must pair with stopping geofencing (the v1 clear path cancels ALL pending
//   notifications — see notifications-daily-reminder §10 hazard, preserved).
fn company_location(&self) -> Option<CompanyLocation>;

// ---- Daily-reminder settings (persisted in core KV; shell renders the UI)
fn daily_reminder_settings(&self) -> DailyReminderSettings;
fn set_daily_reminder_settings(&self, s: DailyReminderSettings);

// ---- Notification debug log (notification-log §2, §4–5)
async fn log_activate(&self, hours: u32);      // 12 or 24; wipes previous entries
async fn log_clear(&self);
async fn log_state(&self) -> LogState;
async fn log_format_for_email(&self) -> String;  // "(keine Einträge aufgezeichnet)" when empty
fn append_log_entry(&self, subsystem: String, level: String,
                    event: String, detail: Option<String>);  // shell-side call sites
                                                             // (e.g. geofence task_error)
```

What is deliberately **not** on the facade:

- Secure-storage access, notification/location OS APIs, permissions — host-only (§5).
- Theme state, screen state, dialog flows — shell-only (03-features/themes, 04-ui-ux).
- v1 dead surface: `OrderDateGroup`, `getDayMenus()` production-unused getter,
  `isLogActive()`, `scheduleDailyNotification()` placeholder, `getReminderSentDate()` —
  all documented as vestigial in the requirements docs; not ported.

---

## 5. Host-injection points

The core is a guest in the app process. The host injects exactly:

1. **Storage directory** (`CoreConfig.storage_dir`), once at construction.
   - iOS: a subdirectory of Application Support
     (`FileManager.urls(for: .applicationSupportDirectory)` + `/snackpilot-core`),
     excluded from iCloud backup is *not* required (v1's AsyncStorage was backed up;
     preserving that is harmless — cached menus only).
   - Android: `context.filesDir/snackpilot-core`.
   - The directory must be readable in background execution contexts. Note the
     credential analogue: v1 chose keychain accessibility AFTER_FIRST_UNLOCK
     specifically so background tasks can read credentials while locked
     (05-platform-services §1.2); on iOS the storage dir must likewise not use
     `NSFileProtectionComplete` (use `.completeUntilFirstUserAuthentication`).
2. **Credentials**, per call: `gourmet_login`/`ventopay_login` and
   `run_background_menu_check` take `Credentials` read by the shell from secure
   storage. After a successful login the core retains them **in memory only** for
   automatic re-login on session expiry (01-gourmet-scraping §6–7); it never persists
   them. There is no credentials callback interface — background handlers read the
   Keychain/Keystore themselves (AFTER_FIRST_UNLOCK / equivalent) and pass values in.
3. **Settings**:
   - Decision-relevant settings (daily-reminder enabled/time, company location,
     `isAtCompany`) are persisted **by the core** in its KV (§3.3) so headless
     background entry points need no shell-side state. The shell settings screens
     read/write them through the facade.
   - Pure-UI settings (theme preference, accent color) are persisted natively per
     shell (UserDefaults / DataStore) — the core never needs them.
4. **AnalyticsSink** callback (optional), at construction. Fire-and-forget; the core
   never awaits or retries a send, mirroring v1's swallow-everything contract
   (03-features/analytics §1.2). Passing `None` (dev builds, tests) silently disables
   core-originated events.
5. **Clock and RNG are *not* injected across the FFI** — the core reads the system
   clock. Internally both are traits so tests can fake them (§9.3).

---

## 6. Threading & async model

### 6.1 Runtime

The core owns a small multi-threaded tokio runtime, created lazily at construction.
Exported `async fn`s are bridged by UniFFI's async support: Swift callers use
`async/await`, Kotlin callers use `suspend` functions. No facade call ever blocks the
calling thread; shells may call from the main thread and hop back for UI updates when
the future resolves (Swift `@MainActor` continuation / Kotlin `Dispatchers.Main`).

Callback interfaces (`AnalyticsSink`, `ProgressListener`) are invoked from core runtime
threads — shell implementations must dispatch to the main thread themselves before
touching UI.

### 6.2 Background execution contexts

`run_background_menu_check`, `run_order_sync_check`, and `on_geofence_event` are
designed to be called from headless OS contexts (BGTask handler, WorkManager
`CoroutineWorker`, geofence callback) with no UI attached. They:

- never prompt, never touch OS notification/location APIs (they return commands),
- never throw across the FFI for expected conditions (guards return outcomes),
- are idempotent and tolerant of irregular invocation (design spec §9 — BGTaskScheduler
  gives no timing guarantees; the decision logic already assumes this, e.g. the daily
  reminder's pre-scheduling model, notifications-daily-reminder §1).

The order-sync path performs **no network I/O** — it reads only cached orders,
preserving v1's "no concurrent scraping" rule (v1:
src/app/src-rn/utils/notificationTasks.ts:86-87; caching.md §4.4).

### 6.3 Concurrency rules inside the core

- **Per-service session serialization.** One async mutex per service (Gourmet,
  Ventopay) serializes all scraping operations on that session. This is mandatory for
  Gourmet: CSRF/anti-bot tokens are single-use per page load and the edit-mode cancel
  loop is a strict GET→POST→GET sequence (01-gourmet-scraping §3, §9.4) — interleaving
  two operations would ban accounts.
- **The background menu check bypasses the session mutex** by constructing its own
  client + cookie store per run (notifications-new-menu §3.3 step 4). Note this is a
  **deliberate behavior change** (§10): v1's headless task shared the app-wide native
  cookie store with the foreground session (no isolated background session), so a
  background login could overwrite/invalidate the foreground Gourmet session server-side;
  v2's per-run isolated cookie store avoids that cross-contamination. The task may still
  overlap a foreground session in wall-clock time.
- **In-flight guards** replicate v1 store semantics: a second `fetch_menus` while one
  runs is a no-op; `refresh_availability` skips when already refreshing or the list is
  empty; billing's Gourmet fetch skips while loading (menus.md §3, billing.md §4).
- **Cancellation:** facade futures for *mutating* scraping flows (`submit_orders`,
  `confirm_orders`, `cancel_order`, logins) run to completion even if the foreign
  future is dropped (spawn-and-join internally). Aborting mid-cancel-loop would leave
  the Gourmet orders page in edit mode with stale tokens — a state v1 can never
  produce. Read-only fetches may be cancel-safe.
- **KV access** is guarded by an in-process lock with atomic writes (§3.3).

---

## 7. Per-platform service table

Concretization of the design-spec table with the exact values from
05-platform-services (and the notification feature docs):

| Concern | Core contract | iOS (Swift) | Android (Kotlin) |
|---|---|---|---|
| UI | renders `*Snapshot` records; executes ops | SwiftUI, 4-tab navigation per 04-ui-ux (glass pill tab bar §2.1) | Jetpack Compose, Material 3 bottom bar with labels (04-ui-ux §2.2) |
| Secure storage | receives `Credentials` per call; never persists them | Keychain, generic-password items; v2's own entries written `kSecAttrAccessibleAfterFirstUnlock`; takeover read per §8 | Keystore-backed encryption (EncryptedSharedPreferences or equivalent); takeover read per §8 |
| Credential keys | — | `gourmet_username`, `gourmet_password`, `ventopay_username`, `ventopay_password` (same logical names as v1, 05 §1.1) | same |
| Background execution | `run_background_menu_check` (network), `run_order_sync_check` (no network); min interval semantics: 15 min, OS-controlled best-effort | `BGTaskScheduler` — v2-owned identifiers declared in `Info.plist` `BGTaskSchedulerPermittedIdentifiers` (v1 used expo's `com.expo.modules.backgroundtask.processing`; v2 defines its own — Open Questions) | WorkManager periodic work, 15-minute minimum; `RECEIVE_BOOT_COMPLETED` + `WAKE_LOCK` permissions |
| Notification delivery | emits `NotificationCommand`s; identifiers `daily-order-reminder`, `cancel-order-reminder`, `geofence-no-order-reminder`; foreground presentation banner+list+sound, no badge (05 §5.2) | `UNUserNotificationCenter`; identifier reuse replaces pending requests; permission request with alert+badge+sound options | `NotificationManager`; channels `order-reminders` (`Bestellungs-Erinnerungen`, HIGH, vibration `[0, 250, 250, 250]`) and `menu-updates` (`Neue Menüs`, DEFAULT) — exact ids/names/importance preserved for upgraded installs (05 §5.3) |
| Location | `on_geofence_event`, `set_company_location`; decision logic + persistence | CoreLocation region monitoring: 1 region, identifier `company`, radius 500 m, enter+exit; "Always" permission flow with settings fallback; one-shot high-accuracy fix on save | `GeofencingClient`; `ACCESS_FINE/COARSE/BACKGROUND_LOCATION`, `FOREGROUND_SERVICE(_LOCATION)`; FusedLocationProvider one-shot fix |
| Geofence idempotency | — | never stop-and-restart monitoring if already active (avoids iOS Enter re-fire; notifications-location §3) | same skip-if-running rule |
| Alternate app icons | — | `UIApplication.setAlternateIconName`; alternates `emerald`, `berry`, `golden`, `ocean`; orange = primary icon (themes §6) | one `activity-alias` per variant + `PackageManager.setComponentEnabledSetting` (silent switch) |
| Analytics transport | core events via `AnalyticsSink` | TelemetryDeck Swift SDK | TelemetryDeck Kotlin SDK |
| Analytics identity | — | app ID `BA25F62D-0154-4A92-BF85-29FC5FDDA3EC`, `clientUser` = literal `anonymous`, no per-device ID — anonymity guarantees of analytics.md §5 are load-bearing for the published privacy policy | same |
| Mail (log export) | `log_format_for_email` string | `MFMailComposeViewController` | `ACTION_SENDTO`/`ACTION_SEND` intent |
| App identity | — | bundle id `dev.radaiko.gourmetclient`, scheme `snackpilot`, portrait only, `ITSAppUsesNonExemptEncryption=false`, `UIBackgroundModes` = location + the BG-task mode | package `dev.radaiko.gourmetclient` (same signing key), edge-to-edge, portrait |
| Localization | — | none — all strings hard-coded German (05 §8); system locale used only for analytics payload + `de-AT` date/currency formatting | same |

---

## 8. Credential takeover from v1 (best-effort)

v2 ships under v1's exact app identity (07-release §1), so v1's secure-store items
survive the store update in place. On **first launch**, before showing any login UI and
before startup auto-login, each shell runs a one-shot import (guarded by a
`v1_takeover_done` flag in its native preferences):

**Common flow (both platforms):**

1. If the flag is set → skip to normal startup.
2. Attempt to read the four v1 values: `gourmet_username`, `gourmet_password`,
   `ventopay_username`, `ventopay_password`.
3. For each service where **both** values are non-empty (v1's own rule — empty string
   counts as absent, 05 §1.2): write the pair into v2's own secure-storage entries.
4. Delete the legacy v1 items.
5. Set the flag. Then run normal startup auto-login: read v2 storage, call
   `gourmet_login` / `ventopay_login` concurrently, fire-and-forget (settings §3.7).
6. **Best-effort:** every step is wrapped; any error (missing items, Keychain error,
   decryption failure, absent keystore alias) aborts the import silently, sets the
   flag, and lands the user on the normal `no_credentials` path. The import must never
   crash or block startup (design spec §5); implement with a hard time budget and no
   user-visible error.

**iOS specifics** (05-platform-services §1.4):

- v1 items are `kSecClassGenericPassword` with service **`app:no-auth`**,
  `kSecAttrAccount` = UTF-8 bytes of the key name, value = plaintext UTF-8,
  no access group (default group `$(AppIdentifierPrefix)dev.radaiko.gourmetclient`).
- Read with the library's fallback chain: service `app:no-auth`, then `app:auth`, then
  legacy `app` (`kSecMatchLimitOne`, `kSecReturnData`). Accessibility may be either
  `WhenUnlocked` (pre-migration) or `AfterFirstUnlock` — accessibility is not part of
  the lookup query, so reads are unaffected.
- Delete legacy items across all three service names (mirrors v1's own delete
  behavior).
- Precondition: same bundle id **and** same Apple team prefix (Open Questions).

**Android specifics** (05-platform-services §1.5):

- v1 values live in `shared_prefs/SecureStore.xml` (file name `SecureStore`,
  `MODE_PRIVATE`) under entry keys `key_v1-gourmet_username`, `key_v1-gourmet_password`,
  `key_v1-ventopay_username`, `key_v1-ventopay_password`.
- Each entry is a JSON string:
  `{"ct":<b64>,"iv":<b64>,"tlen":128,"scheme":"aes","usesKeystoreSuffix":true,"keystoreAlias":"key_v1","requireAuthentication":false}`.
- Decrypt: load `AndroidKeyStore`, get the SecretKey under alias
  **`AES/GCM/NoPadding:key_v1:keystoreUnauthenticated`**, Base64-decode `ct`/`iv`
  (NO_WRAP), decrypt `AES/GCM/NoPadding` with `GCMParameterSpec(tlen, iv)`, UTF-8
  decode. Reject `tlen < 96` (library behavior). Ignore the legacy `"hybrid"` scheme
  (never produced by v1's API level).
- Delete the four prefs entries afterwards; optionally delete the keystore alias once
  all four are gone.
- Precondition: in-place update of the same package signed with the same key — a
  reinstall wipes both the prefs file and the keystore, in which case step 6 applies.

Note: if the user was in demo mode, the imported credentials are `demo` /
`demo1234!` — that is correct; demo detection happens inside `gourmet_login` /
`ventopay_login` (demo-mode §1), so demo mode survives the migration exactly like a
real login would.

---

## 9. Testing architecture

Direct port of v1's record & replay strategy (06-testing) into the Rust core, plus thin
native tests. v1's safety guarantee — "the fixture tests are the proof the fragile
scraping behavior matches the spec" — transfers to `cargo test`.

### 9.1 Fixtures

- The 13 sanitized fixtures live canonically in `docs/fixtures/{gourmet,ventopay}/`
  (carried verbatim from v1; 9 Gourmet + 4 Ventopay files, inventory in 06-testing §3)
  and are mirrored byte-identically into `src/core/tests/fixtures/` (a CI check or
  build-script copy keeps them in sync).
- The fixtures are **hand-authored synthetic replicas with sentinel values**
  (`ufprt="CSRF-TOKEN-LOGIN-ABC123"`, `data-id="menu-001"`, …) that tests assert
  verbatim — never regenerate them from the recorder (06-testing §2).

### 9.2 HTTP boundary — request-shape tests

The `http::Transport` trait gets a capturing fake that (a) returns queued fixture
bodies and (b) records every outgoing request: method, full URL, query params, headers
(incl. `Origin`/`Referer`/`Content-Type`, and the app-level `Accept: application/json,
text/plain, */*` that v1 sent on every request via axios and reqwest must be configured
to reproduce — 01 §2, 02 §2.1), body encoding, and **form-field order**.
Tests then assert the exact shapes listed in 06-testing §6.1 — login POST field sets,
stale-session pre-logout, pagination call sequence (`/menus/` bare, then `page=1`…),
session-expiry re-login + single retry, `addToCart` JSON with lowercase `staffgroupId`,
the confirm/cancel edit-mode POST sequences with fresh tokens per step, `getBillings`
payload and response-shape tolerance, Ventopay's 11-field login POST, `dd.MM.yyyy`
transaction params, cookie-jar capture/injection, and both logout flows. Multipart
bodies are decoded by the fake so field order and values are asserted, not byte
boundaries.

### 9.3 Test layers in the core

| Layer | v1 equivalent | Technique |
|---|---|---|
| Parser tests | `gourmetParser.test.ts`, `ventopayParser.test.ts` | Pure functions over fixture strings; assert every fixture-exact value in 06-testing §6.2 (7-item desktop-only parse, category detection incl. `UNKNOWN`, both approval markers, cancel-form fallback, ASP.NET state errors, Gourmet filter 6→5, `Mrz`/`Jän` months, German amounts, all error messages verbatim) |
| Client/orchestration tests | `gourmetClient/Api.test.ts`, `ventopayClient/Api.test.ts` | Capturing transport (§9.2) |
| Feature-service tests | `menuStore/orderStore/billingStore.test.ts` | Fake API layer (trait), in-memory KV, **fake clock** (cutoff tests; v1 dodges cutoffs with dates +14 days), assert state transitions, cache write-through, TTL, merge semantics, corrupt-entry deletion, error strings |
| Decision/notification tests | `dailyReminderCheck/cancelReminderCheck/notificationService.test.ts` | Fake clock pinned to Vienna wall-clock scenarios; assert returned `NotificationCommand`s (schedule/fire-now/cancel, boundary minutes 08:45/09:00/14:00, identifier reuse, no-window re-scheduling) |
| Fingerprint / log / storage tests | `menuFingerprint/menuChangeStorage/notificationLogStorage.test.ts` | Pure + in-memory KV; empty/corrupt-JSON handling, 200-entry cap, e-mail format string |
| Demo determinism tests | `demoGourmetApi/demoVentopayApi.test.ts` | Fixed clock; 40 items = 10 weekdays × 4 categories, LCG draw-order stability, per-month billing/transaction seeds |

Internal traits injected for testability: `Transport` (HTTP), `Kv` (storage), `Clock`.
None of these cross the FFI. The `AnalyticsSink` is simply `None` or a recording fake.

Rust-specific care: the demo PRNG must replicate JavaScript number semantics — the
multiply happens in f64 and only the sum is truncated to 31 bits
(demo-mode §5.1; implement as `s = ((s as f64 * 1103515245.0 + 12345.0) as i64 & 0x7fffffff)`
with the same f64 rounding, verified against the v1 test vectors).

### 9.4 Recorder (dev-only)

`src/core/tools/recorder`: a Rust binary (workspace member, never shipped) that
reproduces v1's `record-fixtures` behavior (06-testing §4): reads
`KANTINE_USERNAME/PASSWORD`, `AUTOMATEN_USERNAME/PASSWORD` from the repo-root `.env`;
performs exactly the documented safe request sequences (never POSTs orders or
cancellations); writes the same output file names; applies the same sanitization table
(`TestUser`, `SM-TEST-123`, `EATER-TEST-456`, `SG-TEST-789`, `Test User`). Its output
is an inspection aid for diffing live markup against the synthetic fixtures — it does
not overwrite `docs/fixtures/`.

### 9.5 Native shell tests (thin)

Business logic is not duplicated natively, so it is not tested natively (design spec
§6). Shell tests cover only platform glue:

- **iOS (XCTest):** Keychain adapter round-trip; takeover import against seeded
  legacy-format items (service `app:no-auth`); `NotificationCommand` →
  `UNNotificationRequest` mapper (identifier, trigger date, channel ignored on iOS);
  BGTask registration mapper; snapshot/UI smoke tests for the 4 tabs + 4 sub-screens.
- **Android (JUnit/Robolectric + Compose testing):** SecureStore-decryption takeover
  path against a seeded prefs file + keystore key; command → `NotificationCompat`
  mapper incl. both channels; WorkManager scheduling mapper; navigation/render smoke
  tests.

### 9.6 CI (07-release §5)

- **core-test** (PRs + main): `cargo test` on the host, `cargo build` for
  `aarch64-apple-ios` and `aarch64-linux-android` (compile proof for both targets);
  `cargo clippy`/`fmt` gates; fixture-sync check (§9.1).
- **ios-build / android-build** (PRs): compile checks of the shells against a fresh
  core binary.
- **Release**: tag-triggered `ios/vX.Y.Z` / `android/vX.Y.Z` per 07-release §3 (EAS
  replaced by xcodebuild + App Store Connect API, Gradle + Play Developer API,
  `track: alpha`). Single version shared by shells and core crate, bumped in one
  release commit (`Release vX.Y.Z (ios,android)`).
- **security-audit**: weekly + manifest-triggered `cargo audit` on the core lockfile;
  npm audit for `tools/icon-tools`; Dependabot for cargo, gradle, swift, github-actions.

This is a strict upgrade over v1, which ran no tests in CI (07-release §4).

---

## 10. Deliberate behavior changes vs v1 & open questions

Documented per the design spec's requirement that deviations be explicit:

1. **Cookie handling (Gourmet).** reqwest's cookie store replaces NSURLSession/OkHttp
   native handling. Same documented request sequence; one implementation on both
   platforms; first live validation with a low-value account (design spec §4, §9).
   Consequence: cookies no longer persist across app restarts (v1's native stores did,
   which is why the stale-session pre-logout exists, 01 §6.1). The pre-logout step is
   **kept anyway** — it is part of the documented sequence and harmless when the GET
   returns the login page.
2. **User-Agent.** v1 sent the platform HTTP stack's default UA (it only forbade a
   *custom* one); reqwest sends **no** UA header unless configured. Whether an absent
   UA is safe for the ban-sensitive Gourmet backend is not determinable from source —
   **open question**; to be resolved during the careful first live test. If needed, the
   shells can pass their platform-default UA string into `CoreConfig` (would be a
   facade addition).
3. **`daily_reminder_sent_date`** is not persisted in v2 (write-only in v1,
   notifications-daily-reminder §5). No behavioral difference.
4. **Sticky demo API quirk** (demo-mode §1/§8): in v1, a live login after a demo login
   in the same process still hits the demo API until restart. Whether v2 replicates
   this or fixes it (fresh live client on non-demo login) is an **open question**
   carried from the demo-mode doc; the core makes either trivial.
5. **BGTask identifiers** are necessarily new (v1 used expo-background-task's fixed
   `com.expo.modules.backgroundtask.processing`); the exact v2 identifier strings are a
   v2 naming decision with no v1 precedent.
6. **Analytics wire fidelity**: native TelemetryDeck SDKs replace the JS SDK; event
   names, payload keys, app ID, and the constant-`anonymous` anonymity are preserved,
   but SDK-internal fields (`telemetryClientVersion`, `TelemetryDeck.SDK.name`) will
   differ — accepted, flagged in analytics.md as an open question.
7. **Status-bar quirk** (themes §1.1): v1's status bar follows the *system* scheme even
   when the app preference overrides it. v2 shells should fix this (drive the status
   bar from the resolved app scheme) — a conscious deviation, note kept here.
8. **Accept header made explicit.** v1 sent `Accept: application/json, text/plain, */*`
   on every request as an axios app-level default (not a platform default). reqwest sends
   no such default, so v2 must configure it explicitly on both clients (§3.1; 01 §2,
   02 §2.1). Not a behavior change in intent — it *preserves* v1's bytes — but called out
   because a naive reqwest port would silently drop it.
9. **Background menu-check cookie isolation.** v1's headless task shared the app-wide
   native cookie store with the foreground session, so a background login could
   overwrite/invalidate the foreground Gourmet session server-side. v2 gives the
   background run its own isolated reqwest cookie store (§6.3), removing that
   cross-contamination — a deliberate change flagged in notifications-new-menu §3.3.
