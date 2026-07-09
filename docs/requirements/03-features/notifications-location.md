# Location-based notifications

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

The user saves their company (office) coordinates once; the app then monitors a single 500 m
geofence around them. Arriving at the office without having ordered today's lunch produces a local
notification at 08:45 Vienna time: `Du bist im Büro, hast aber noch nicht bestellt!`. Leaving the
office retracts it. The saved location itself is the on/off switch — there is no separate enable
toggle (v1: docs/plans/2026-02-24-location-notifications-design.md:69). Purely client-side: no HTTP
requests of its own. Mobile-only in v1 (iOS + Android); web/desktop had no-op stubs, dropped in v2.

Related docs (do not duplicate their content):

- `03-features/orders.md` — order model (`date` field) and `fetchOrders()`.
- `03-features/caching.md` — the AsyncStorage order cache that `loadCachedOrders()` reads.
- `03-features/notifications-daily-reminder.md` — shares the background-sync task, the Android
  channel, the Vienna time helpers (`viennaMinutes`, `viennaToday`, `isSameDay`; see its §7), and
  the pre-schedule-then-replace notification pattern.
- `03-features/notifications-cancel-reminder.md` — the inverse case ("ordered but not at office"),
  which the design doc originally specified as part of this feature but which shipped as its own
  module; the geofence Enter/Exit handlers invoke its check (see §6).
- `03-features/notification-log.md` — the diagnostic log written by the geofence task.
- `03-features/analytics.md` — the `notification.locationSet` signal.
- `05-platform-services.md` — notification permission mechanics, foreground presentation handler,
  Android channel creation, background task registration.

---

## 1. Persisted state — the location store

State (v1: src/app/src-rn/store/locationStore.ts:5-18):

| Field | Type | Default | Semantics |
|---|---|---|---|
| `companyLocation` | `{ latitude: number, longitude: number }` or null | `null` | Saved office coordinates (decimal degrees). `null` = feature off. |
| `isAtCompany` | boolean | `false` | Last known geofence state, set by Enter/Exit events (§4); also reset to `false` by `clearCompanyLocation()` (§8, "Standort entfernen", v1: src/app/src-rn/store/locationStore.ts:29-30). Read by the cancel-reminder feature. |

Operations (v1: src/app/src-rn/store/locationStore.ts:26-36, locked by
src/app/src-rn/__tests__/store/locationStore.test.ts):

- `setCompanyLocation(lat, lng)` — sets `companyLocation` only (does not touch `isAtCompany`).
- `clearCompanyLocation()` — sets `companyLocation = null` **and** `isAtCompany = false`.
- `setIsAtCompany(value)` — sets the flag.
- `hasCompanyLocation()` — `companyLocation !== null`.

Persistence (v1 mechanism): Zustand `persist` middleware over AsyncStorage (unencrypted app-local
key-value store) under storage key `company-location`
(v1: src/app/src-rn/store/locationStore.ts:38-41). The value is the standard Zustand persist
envelope, JSON `{"state":{"companyLocation":…,"isAtCompany":…},"version":0}`. Both fields survive
app restarts — `isAtCompany` deliberately persists so background tasks launched after a process
kill still see the last known geofence state
(v1: docs/plans/2026-02-24-location-notifications-design.md:22). v2 needs equivalent durable
storage; exact envelope only matters if migrating v1 user data.

## 2. Geofence region definition

Exactly **one** region is ever registered (v1: src/app/src-rn/utils/notificationService.ts:58-67):

| Property | Exact value |
|---|---|
| Monitoring task name | `COMPANY_GEOFENCE_TASK` (`GEOFENCE_TASK_NAME`, v1: src/app/src-rn/utils/constants.ts:31) |
| Region identifier | `company` |
| Center | `companyLocation.latitude` / `companyLocation.longitude` (the saved coordinates, unmodified) |
| Radius | **500 m** (`COMPANY_GEOFENCE_RADIUS_M = 500`, v1: src/app/src-rn/utils/constants.ts:33) |
| notifyOnEnter | `true` |
| notifyOnExit | `true` |

Radius and center are fixed — no user-configurable radius, no map UI; the center is whatever
one-shot GPS fix the user captured (§7)
(v1: docs/plans/2026-02-24-location-notifications-design.md:65-66).

v1 mechanism: `expo-location` geofencing + `expo-task-manager` background task, i.e. CoreLocation
region monitoring on iOS and Play-services geofencing on Android. v2 equivalents:
`CLLocationManager`/`CLMonitor` region monitoring (iOS), `GeofencingClient` (Android).

## 3. Geofence lifecycle

`startGeofencing()` (v1: src/app/src-rn/utils/notificationService.ts:49-68):

1. No-op if `companyLocation` is null.
2. **No-op if monitoring is already active for `COMPANY_GEOFENCE_TASK`.** It does not
   stop-and-restart. Code comment gives the reason verbatim: "Skip if already running — restarting
   causes iOS to re-fire Enter events when the device is already inside the zone, which triggers
   spurious notifications" (v1: src/app/src-rn/utils/notificationService.ts:53-54). Locked by test
   "skips starting when geofencing is already running"
   (v1: src/app/src-rn/__tests__/utils/notificationService.test.ts:150-158).
3. Otherwise start monitoring the single region of §2. Implication the comment relies on: starting
   monitoring while the device is *inside* the region fires an initial Enter event — so saving the
   location while standing at the office immediately sets `isAtCompany = true` and runs the Enter
   logic of §4.

`stopGeofencing()` — stops monitoring only if currently active
(v1: src/app/src-rn/utils/notificationService.ts:70-75).

Aggregates (v1: src/app/src-rn/utils/notificationService.ts:293-302):

- `enableNotifications()` = `startGeofencing()` then `registerBackgroundSync()` (shared
  background-sync task, min interval 15 min, skipped when the OS reports background work
  Restricted — details in `03-features/notifications-daily-reminder.md` §2 /
  `05-platform-services.md`).
- `disableNotifications()` = `stopGeofencing()`, unregister background sync, then
  `cancelDailyNotification()` which calls **cancel-ALL-scheduled-notifications**
  (v1: src/app/src-rn/utils/notificationService.ts:258-260). Cross-feature hazard: this also wipes
  any pending daily-reminder or cancel-reminder notification (see
  `03-features/notifications-daily-reminder.md` §10).

Called from:

- Settings flow after saving a location (§7).
- **App start restore**: on every launch, if a company location is saved, `enableNotifications()`
  runs (native only) — this re-establishes monitoring after device reboot or app reinstall; the
  skip-if-running guard makes it idempotent (v1: src/app/app/_layout.tsx:67-78).
- `disableNotifications()` from the "Standort entfernen" action (§8).

Task definitions must exist at process start: v1 imports the task-definition module at the very top
of the root layout so the OS can invoke the handlers when it launches the app in the background
(v1: src/app/app/_layout.tsx:9, src/app/src-rn/utils/notificationTasks.ts:19-20). v2 equivalent:
geofence callbacks must be registered app-delegate/Application-level, not tied to any UI screen.

## 4. What fires on Enter / Exit

Handler for `COMPANY_GEOFENCE_TASK` events (v1: src/app/src-rn/utils/notificationTasks.ts:27-77).
If the OS delivers an error, log it (`geofence`/`error`/`task_error` with the error message) and
return (v1: src/app/src-rn/utils/notificationTasks.ts:30-33).

**ENTER** (v1: src/app/src-rn/utils/notificationTasks.ts:35-61), in this exact order:

1. `setIsAtCompany(true)`; log `region_enter` (detail `isAtCompany=true`).
2. Load cached orders from storage into the in-memory store — required because on a cold background
   launch the store is empty (v1: src/app/src-rn/utils/notificationTasks.ts:39-40). No network.
3. Compute `hasOrderToday` = any order whose `date` is the same calendar day as Vienna-today
   (`viennaToday()` + `isSameDay`; v1: src/app/src-rn/utils/dateUtils.ts:80-86, 55-61).
4. Run `checkCancelReminder()` — since `isAtCompany` is now true this cancels any pending
   cancel-reminder notification (v1: src/app/src-rn/utils/cancelReminderCheck.ts:22-33). Note this
   runs **before** the has-order guard.
5. If `hasOrderToday` → log guard `has_order_today` (detail `ordersLoaded=${orders.length}`) and
   stop. No notification.
6. Else call `scheduleGeofenceNotification()` (§5) and log `notification_scheduled` (detail
   `scheduled=${scheduled} ordersLoaded=${orders.length}`).
7. Any throw in steps 2-6 is caught and logged as `enter_notify_error` with the message.

**EXIT** (v1: src/app/src-rn/utils/notificationTasks.ts:62-74), in this exact order:

1. `setIsAtCompany(false)`; log `region_exit` (detail `isAtCompany=false`).
2. `cancelGeofenceNotification()` — retract a pending (not yet fired) "no order" notification; log
   `notification_cancelled` (detail `region_exit`).
3. Load cached orders, then `checkCancelReminder()` — if an order exists for today, this schedules
   the cancel-reminder notification (owned by `03-features/notifications-cancel-reminder.md`).
4. Any throw in steps 2-3 is caught and logged as `exit_cancel_error`.

The shared `BACKGROUND_ORDER_SYNC_TASK` never schedules the geofence notification — only Enter
events do (v1: src/app/src-rn/utils/notificationTasks.ts:82-112 contains no geofence-notification
call).

## 5. The "no order" notification

### Content

(v1: src/app/src-rn/utils/notificationService.ts:112-136)

| Field | Exact value |
|---|---|
| Identifier | `geofence-no-order-reminder` (`GEOFENCE_NOTIFICATION_ID`, v1: src/app/src-rn/utils/constants.ts:37) |
| Title | `SnackPilot` |
| Body | `Du bist im Büro, hast aber noch nicht bestellt!` |
| Sound | `default` |
| Data payload | none — tapping opens the app via OS default, no deep link (matches v1: docs/plans/2026-02-24-location-notifications-design.md:41) |
| Android channel | `order-reminders` on the scheduled path; **no channel specified** on the immediate-fire path (v1: src/app/src-rn/utils/notificationService.ts:119-124 vs 127-135) |

### Timing rules

`scheduleGeofenceNotification()` (v1: src/app/src-rn/utils/notificationService.ts:101-138). All
times are **Europe/Vienna wall-clock** via `viennaMinutes()` (minute granularity,
v1: src/app/src-rn/utils/dateUtils.ts:66-77), independent of device timezone. Target =
08:45 (`NOTIFICATION_HOUR = 8`, `NOTIFICATION_MINUTE = 45`,
v1: src/app/src-rn/utils/constants.ts:34-35). Fixed — no user-configurable time
(v1: docs/plans/2026-02-24-location-notifications-design.md:66).

| Vienna time at Enter | Behavior | Returns |
|---|---|---|
| ≥ 14:00 | Skip entirely — "too late in the day" | `false` |
| < 08:45 | Schedule a date-triggered local notification at `now + (targetMin − currentMin)` minutes (fires at 08:45:ss, seconds inherited from the scheduling instant) | `true` |
| ≥ 08:45 and < 14:00 | Fire immediately (`trigger: null`) | `true` |

Boundary values locked by tests: 10:00 → immediate; 14:01 → skip; 8:00 → scheduled
(v1: src/app/src-rn/__tests__/utils/notificationService.test.ts:322-364). Exactly 14:00
(`currentMin >= 14 * 60`) skips.

### Dedupe and retraction

- The fixed identifier means at most one pending geofence notification exists; a re-schedule
  replaces the previous one (same pattern as the daily reminder).
- `cancelGeofenceNotification()` cancels by that identifier
  (v1: src/app/src-rn/utils/notificationService.ts:143-145). Retraction paths:
  1. Geofence **Exit** (§4).
  2. **Successful foreground order fetch that finds an order for today**: `fetchOrders()` cancels
     the pending geofence notification whenever `hasOrderToday` becomes true — so ordering via the
     app before 08:45 retracts the reminder (v1: src/app/src-rn/store/orderStore.ts:69-78; errors
     swallowed with comment "notification service may not be available").
  3. Removing the company location → `disableNotifications()` cancels **all** notifications (§3).
- **Not retracted** when the user orders outside the app (other device / website) and never
  refreshes: the Enter-time snapshot of the cached orders decides, and only paths 1-3 can cancel
  afterwards. Also, cancelling by identifier only removes a *pending* notification — once fired
  (immediate path, or after 08:45) it stays in the notification tray.
- Nothing is date-scoped: an Enter event always targets *today's* 08:45 (the ≥14:00 guard prevents
  evening arrivals from scheduling anything, and an overnight stay produces no new Enter event).

## 6. Cross-feature interactions

- **Cancel-reminder** (`03-features/notifications-cancel-reminder.md`): geofence Enter cancels it;
  geofence Exit (with an order today) schedules it; both via `checkCancelReminder()`, which reads
  this feature's `isAtCompany` flag (v1: src/app/src-rn/utils/cancelReminderCheck.ts:22-36). The
  design doc's 2×2 decision table (v1: docs/plans/2026-02-24-location-notifications-design.md:34-39)
  is realized as these two independent notifications, not as a single 08:45 check (see §13).
- **Background sync task** (`BACKGROUND_ORDER_SYNC_TASK`): registered by `enableNotifications()`;
  its body serves the daily-reminder and cancel-reminder checks only (cached orders, no network) —
  see `03-features/notifications-daily-reminder.md` §2.
- **Order store**: `fetchOrders()` cancellation hook, §5.

## 7. Permission flow

Two permission domains are requested, in this order, when the user saves a location
(v1: src/app/app/notifications.tsx:145-177):

**1. Location** — `requestLocationPermissions()`
(v1: src/app/src-rn/utils/notificationService.ts:22-29):

- Request **foreground** ("when in use") permission first; if not granted → return false without
  requesting background.
- Then request **background** ("always") permission; return true only if granted. Code comment: on
  iOS the "Always Allow" prompt may not be shown (iOS often defers the always-upgrade), hence the
  settings fallback below.
- `hasBackgroundLocationPermission()` checks background permission **without prompting**
  (v1: src/app/src-rn/utils/notificationService.ts:35-38).

Failure handling in the UI (v1: src/app/app/notifications.tsx:148-161): if
`requestLocationPermissions()` returned false **and** `hasBackgroundLocationPermission()` is also
false, show an alert and then open the OS app-settings page (`Linking.openSettings()`):

- Alert title: `Standort „Immer" erforderlich` (opening quote U+201E `„`, closing quote is a plain
  ASCII `"` — copy byte-exact)
- Alert message: `Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer" gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort „Immer" aus.`

Then abort — nothing is saved. Quirk (factual, code wins): if foreground was denied but background
happens to already be granted, the `hasBg` check passes and the flow aborts **silently with no
alert** (v1: src/app/app/notifications.tsx:150-160).

**2. Notifications** — `requestNotificationPermissions()`
(v1: src/app/src-rn/utils/notificationService.ts:40-47): if already granted return true; otherwise
request with iOS options `allowAlert: true, allowBadge: true, allowSound: true`. If denied, alert
title `Berechtigung fehlt`, message
`Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.`,
and abort (v1: src/app/app/notifications.tsx:162-167).

**3. Capture position** — one-shot GPS fix at **high accuracy** (`Location.Accuracy.High`),
returning `{ latitude, longitude }`
(v1: src/app/src-rn/utils/notificationService.ts:283-291). Then:
`setCompanyLocation(lat, lng)` → `await enableNotifications()` → success alert (title
`Gespeichert`, message
`Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro bist und nicht bestellt hast.`)
→ analytics signal `notification.locationSet` (no properties)
(v1: src/app/app/notifications.tsx:168-172).

Any throw in the whole flow (typically GPS failure) → alert title `Fehler`, message
`Standort konnte nicht ermittelt werden.` (v1: src/app/app/notifications.tsx:173-175).

OS-level permission declarations are in §10. Note the app also requests notification permission
from other features (new-menu check on auth, daily-reminder toggle) — first-granted wins globally.

## 8. Settings UI

Lives on the "Benachrichtigungen" screen (pushed from the Settings tab, header back-label
`Einstellungen`), section **`Standort-Benachrichtigungen`** with hint text
`Erinnerung um 8:45 basierend auf deinem Standort`
(v1: src/app/app/notifications.tsx:276-301). UI chrome is covered by `04-ui-ux.md`; behaviorally
required:

- **No location saved**: one primary button, label
  `Aktuellen Standort als Firmenstandort setzen`; while the flow of §7 runs the button is disabled
  and its label switches to `Standort wird ermittelt...` (three ASCII dots)
  (v1: src/app/app/notifications.tsx:291-300, 72).
- **Location saved**: status text `Firmenstandort gesetzt` and a danger-styled button
  `Standort entfernen`. Tapping it (no confirmation dialog) calls `clearCompanyLocation()` then
  `await disableNotifications()` (v1: src/app/app/notifications.tsx:179-182, 282-290).
- The saved coordinates are never displayed; there is no map, no radius setting, no time setting
  (v1: docs/plans/2026-02-24-location-notifications-design.md:64-69).

## 9. App-start integration

On every native app start (v1: src/app/app/_layout.tsx:9, 67-78):

1. Task definitions module is imported at module scope (before any UI) so geofence/background
   callbacks are available on OS-initiated background launches.
2. The notification foreground handler and the Android channel `order-reminders`
   (name `Bestellungs-Erinnerungen`, importance HIGH, vibration `[0, 250, 250, 250]`) are set up
   (v1: src/app/src-rn/utils/notificationService.ts:262-281; shared, see `05-platform-services.md`).
3. If `companyLocation` is non-null → `enableNotifications()` (restore geofence + background sync).
   No permissions are (re-)requested here; if the user has meanwhile revoked "Always" location, the
   start call fails silently inside the effect.

## 10. Platform configuration (v1: src/app/app.json)

iOS (v1: src/app/app.json:22-28):

- `UIBackgroundModes`: `["location", "processing"]`
- `BGTaskSchedulerPermittedIdentifiers`: `["com.expo.modules.backgroundtask.processing"]`
  (v1-mechanism identifier of expo-background-task; v2 will define its own BGTask identifier)

Android permissions (v1: src/app/app.json:39-47): `ACCESS_COARSE_LOCATION`,
`ACCESS_FINE_LOCATION`, `ACCESS_BACKGROUND_LOCATION`, `FOREGROUND_SERVICE`,
`FOREGROUND_SERVICE_LOCATION`, `RECEIVE_BOOT_COMPLETED`, `WAKE_LOCK`.

Permission-prompt texts (expo-location plugin config, v1: src/app/app.json:98-107 — these are the
OS purpose strings, i.e. `NSLocation*UsageDescription` on iOS):

- Always + when-in-use: `SnackPilot nutzt deinen Standort, um dich an Bestellungen zu erinnern, wenn du im Büro bist.`
- When-in-use: `SnackPilot nutzt deinen Standort, um deinen Firmenstandort zu speichern.`
- `isIosBackgroundLocationEnabled: true`, `isAndroidBackgroundLocationEnabled: true`,
  `isAndroidForegroundServiceEnabled: true`.

Notification icon/color (expo-notifications plugin, v1: src/app/app.json:58-64): icon
`./assets/icons/icon-orange.png`, color `#FF6B35` (Android small-icon tint).

## 11. Diagnostic log events

Subsystem `geofence` (mechanism and persistence gating in `03-features/notification-log.md`).
Exact events (v1: src/app/src-rn/utils/notificationTasks.ts:29-74):

| Level | Event | Detail (exact template) |
|---|---|---|
| `error` | `task_error` | OS geofencing error message |
| `info` | `region_enter` | `isAtCompany=true` |
| `guard` | `has_order_today` | `ordersLoaded=${orders.length}` |
| `info` | `notification_scheduled` | `scheduled=${scheduled} ordersLoaded=${orders.length}` |
| `error` | `enter_notify_error` | error message |
| `info` | `region_exit` | `isAtCompany=false` |
| `info` | `notification_cancelled` | `region_exit` |
| `error` | `exit_cancel_error` | error message |

## 12. Battery considerations

The design docs contain **no explicit battery/power section**
(v1: docs/plans/2026-02-24-location-notifications-design.md,
docs/plans/2026-02-24-location-notifications.md — no battery-related statements; verified by
search). The battery-relevant behavior is implicit in the shipped design and must be preserved:

- OS **region monitoring** (geofencing) instead of continuous location tracking; exactly one
  region (v1: docs/plans/2026-02-24-location-notifications-design.md:20-21).
- High-accuracy GPS is used only for the **one-shot** capture when saving the location (§7), never
  continuously.
- Background work is a single OS-controlled task with `minimumInterval: 15` minutes, best-effort
  (v1: src/app/src-rn/utils/notificationService.ts:83-85,
  docs/plans/2026-02-24-location-notifications-design.md:26).
- The Android hint banner about allowing background activity exists only in the daily-reminder
  section, not here (v1: src/app/app/notifications.tsx:262-270).

## 13. Discrepancies — design/plan docs vs shipped code (code wins)

1. **No 08:45 scheduled check exists.** The design doc specifies a daily "scheduled check at
   8:45am Europe/Vienna" evaluating a 2×2 (at company × ordered) table
   (v1: docs/plans/2026-02-24-location-notifications-design.md:30-39). Shipped code instead
   **pre-schedules on geofence Enter** (with immediate-fire and 14:00 cutoff rules, §5), and the
   "ordered but not at office" row became the separate cancel-reminder feature — with different
   text (`Du hast heute bestellt, bist aber nicht im Büro. Stornieren?` vs the design's
   `Du hast heute bestellt, bist aber nicht im Büro!`), a `data.screen` payload, and a 09:00
   deadline (v1: src/app/src-rn/utils/notificationService.ts:153-187; see
   `03-features/notifications-cancel-reminder.md`).
2. **Background task does not scrape.** Design/plan say the background task auto-logs-in and calls
   `fetchOrders()` (v1: docs/plans/2026-02-24-location-notifications-design.md:24-28,
   docs/plans/2026-02-24-location-notifications.md:416-441). Shipped code only loads **cached**
   orders — comment: "no network calls to avoid concurrent scraping"
   (v1: src/app/src-rn/utils/notificationTasks.ts:86-87). v2 must keep background runs
   network-silent toward Gourmet (scraping-safety, see `01-gourmet-scraping.md`).
3. **startGeofencing skips instead of restarting.** The plan's test expected stop-then-restart
   when already running (v1: docs/plans/2026-02-24-location-notifications.md:643-653); shipped
   code returns without touching the running geofence, to avoid iOS Enter re-fire (§3).
4. **`scheduleDailyNotification()` is vestigial.** The plan had `enableNotifications()` schedule a
   daily 08:45 placeholder notification with body `Bestellungs-Check läuft...`
   (v1: docs/plans/2026-02-24-location-notifications.md:808-823, 864-868); the shipped function
   still exists and is unit-tested, but has **no production caller**
   (v1: src/app/src-rn/utils/notificationService.ts:241-256; `enableNotifications` at 293-296 does
   not call it). Its companion `cancelDailyNotification()` survives only as the cancel-all step of
   `disableNotifications()`. v2 must not implement the placeholder notification.
5. **Settings placement.** Design says a section in the Settings tab
   (v1: docs/plans/2026-02-24-location-notifications-design.md:11-16); shipped UI lives on the
   dedicated "Benachrichtigungen" screen pushed from Settings (§8).
6. **Package drift.** Design names `expo-background-fetch`
   (v1: docs/plans/2026-02-24-location-notifications-design.md:47); shipped code uses
   `expo-background-task` (v1 mechanism either way).
7. The geofence Enter/Exit handlers do far more than the design's "updates persisted `isAtCompany`
   flag" (v1: docs/plans/2026-02-24-location-notifications-design.md:22) — see §4.

CLAUDE.md does not describe this feature; no conflict.

## 14. Dropped in v2

- Web/desktop: v1 shipped no-op stubs — the permission functions resolve `false`, everything else
  resolves without effect, and `getCurrentPosition` throws `Location not available on web`
  (v1: src/app/src-rn/utils/notificationService.web.ts:1-33,
  src/app/src-rn/utils/notificationTasks.web.ts); the settings section and task definitions were
  gated to native platforms (v1: src/app/src-rn/utils/notificationTasks.ts:20). v2 is mobile-only:
  implement natively (iOS: CLLocationManager region monitoring + UNUserNotificationCenter;
  Android: GeofencingClient + NotificationManager) preserving the semantics above.
- The Expo plugin/config machinery of §10 is a v1 mechanism; the OS-level facts to carry over are
  the permission set, the background modes (location + background processing), and the purpose
  strings.
