# Design: notification pipeline contract

See `proposal.md` for motivation, the implementation-status table, the gaps and the assumptions. Requirement ids `REQ-NTF-*` are defined in `specs/notification-pipeline/spec.md`.

## Context

- The core owns every notification decision (`src/core/src/notify/{daily_reminder,cancel_reminder,geofence,fingerprint}.rs`) and returns `NotificationCommand` values (`FireNow`, `ScheduleAt`, `CancelPending`). Shells only execute them. This split is correct and unchanged.
- Before the branch, nothing called the decision functions without a UI: the settings lived in `UserDefaults` / `SharedPreferences`, `fetch_orders` returned `()`, and no background entry existed. iOS shipped background keys as plain build settings that `GENERATE_INFOPLIST_FILE` drops.
- Verification hosts: Linux runs `make check` (fmt, clippy, core tests, spec validation), which never builds a shell. iOS builds and tests run on the Mac over `ssh mac` (xcodegen and Xcode 26.2 are installed there, not in the VM; `make doctor` failing in the VM for `xcodegen`, `xcodebuild` and `ANDROID_NDK_HOME` is an environment gap, not a product defect, and no task touches `doctor.sh`).
- `docs/requirements/` is authoritative and is not edited. Where it contradicts the shipped design (`05-platform-services.md` §4.5 background modes) the decision is recorded here.

## Goals / Non-Goals

Goals: make the existing fix branch verifiable by the loop, close gaps A-F and the Android single-flight hole, keep one source of truth for reminder settings, and produce device evidence.

Non-goals: new notification features; changes to decision logic, timings, texts or channel ids; notification tap deep-linking (v1 had none, `notifications-daily-reminder.md` §6); the location/geofence feature beyond cancelling its pending notice; the 15-minute picker step; `notification.reminderToggled` analytics; any scraping change; CI or dependency changes outside the approvals in the proposal.

## Decisions

### D1. Retrofit on top of the merged branch

The dispatcher merges `origin/fix/notification-pipeline` unmodified first. Tasks marked *verify* prove the merged code against the done bar and add the missing `@req` tags and tests; they change production code only if the bar fails. Tasks containing a gap fix say exactly which behaviour is new. Why: rewriting reviewed, device-tested code (two commits record validation on a simulator) wastes the work and loses its history; the loop needs contracts, not new code.

### D2. Reminder settings live in the core KV with v1 keys

`cache::{load_daily_reminder_settings, set_daily_reminder_enabled, set_daily_reminder_time}` store `daily_reminder_enabled` (`"true"`/`"false"`) and `daily_reminder_time` (`{"hour":H,"minute":M}`), the exact v1 formats (`notifications-daily-reminder.md` §9). The shells keep only UI mirrors, hydrated from `daily_reminder_settings()` at launch. Callers write the time before the enabled flag (§9 step 2). `daily_reminder_sent_date` is dropped, as the doc allows. No migration of v2.1.0 shell preferences (proposal assumption 3).

### D3. Two entry points, one reminder evaluation

`OrderStore` gets a private `order_reminder_commands(orders)` returning the daily then cancel commands from the KV-backed settings and at-company flag. Then:

- `post_fetch_notifications(orders)` = `[CancelPending(geofence-no-order-reminder) if an order exists for Vienna-today] + order_reminder_commands(orders)`. Used only inside `fetch_orders` after a successful fetch (REQ-NTF-02).
- `run_order_sync_check()` = `load_cached_orders()` then `order_reminder_commands(orders())`. No geofence cancel: the background order-sync task of the docs runs the daily and cancel checks only (REQ-NTF-03). This keeps the iOS XCTest expectation (empty cache gives exactly one `CancelPending(cancel-order-reminder)`) true.
- `SnackPilotCore::run_order_sync_check` in `ffi.rs` becomes a one-line delegate to the store. The branch duplicated the body; two copies would drift (gap F).

The geofence id constant `GEOFENCE_REMINDER_ID` already exists in `notify/mod.rs`. `fetch_orders` keeps swallowing everything in these checks, as `orders.md` §8 requires; the docs' "one shared try/catch" has no analogue because the Rust checks are infallible.

### D4. iOS packaged Info.plist from xcodegen

`GENERATE_INFOPLIST_FILE` is `NO` for the app target and `info:` in `project.yml` produces `src/ios/Info.plist` (git-ignored), carrying everything the generated plist carried plus `UIBackgroundModes=[fetch]` and `BGTaskSchedulerPermittedIdentifiers`. `CFBundleShortVersionString` and `CFBundleVersion` are pinned to `$(MARKETING_VERSION)` / `$(CURRENT_PROJECT_VERSION)` because xcodegen otherwise writes its own constants and `tools/devops/ship.sh` bumps those two settings. `fetch` is the mode `BGAppRefreshTask` needs; `processing` belongs to `BGProcessingTask`. `05-platform-services.md` §4.5 lists `location` and `processing` for v1's expo mechanism; the same section allows v2 its own identifier, and region monitoring needs no background mode, so only `fetch` is declared (proposal assumption 6). A host-app XCTest reads `Bundle.main.infoDictionary` because that is the only place the packaged result can be pinned. The CI `ios.yml` smoke step reads `$app/Info.plist` with PlistBuddy, which still works with a real plist.

### D5. iOS background body is headless (gap B)

`BackgroundTasks.handle(_:)` currently runs `AppModel().performBackgroundSync()`. `AppModel.init` is the full launch sequence (analytics start and `app.launched`, `loadBroadcast`, permission prompt, geofence restore, `attemptAutoLogin` with a Gourmet login and billing fetch). That is wrong for a background task. Replace it with a static function in `BackgroundTasks.swift`:

`BackgroundTasks.runSync(core:, credentials:, deliver:) async -> Bool`

- `core`: a `SnackPilotCore` the caller builds over the same Application Support `snackpilot` directory with `analytics: nil` (the launch event and the sink belong to the UI process).
- `credentials`: `KeychainStore.savedGourmet()` mapped to `Credentials`, or nil.
- `deliver`: closure, `NotificationService.shared.execute` in production, a recorder in tests.
- Single-flight: a `@MainActor` static flag; a second concurrent call returns `false` immediately without any request. The existing per-instance `menuCheckRunning` in `AppModel` is replaced by this flag so the manual button, the foreground menu check and the background task share one gate (REQ-NTF-13).
- Body: `core.runMenuCheck(creds:)` (delivering `result.notification`), then `core.runOrderSyncCheck()` (delivering each command). Cancellation (`Task.isCancelled`) is checked between the two steps.
- `AppModel.performBackgroundSync()` (the manual button) calls the same function with `self.core`, so there is one body (REQ-NTF-14).
- `handle(_:)` keeps the branch's structure: requeue first, `expirationHandler` cancels the work task, completion from a continuation with `success = finished && !cancelled`.

Risk: two `SnackPilotCore` instances over one storage directory (background run while the UI process is alive is not expected, because iOS runs refresh tasks for a suspended or terminated app). `FileKv` writes whole keys, so the worst case is last-writer-wins on `known_menu_fingerprints`, the same exposure `GeofenceBroadcastReceiver` already has on Android.

### D6. iOS immediate delivery is testable (gap A)

`NotificationService` exposes two pure helpers so the contract is unit-testable without a notification centre:

- `static func request(id:title:body:after:) -> UNNotificationRequest` — `trigger == nil` when `after <= 0`, a non-repeating `UNTimeIntervalNotificationTrigger` otherwise.
- `static func delay(for command: NotificationCommand, nowMs: Double) -> TimeInterval` — `0` for `fireNow`, `max(1, (fireAtEpochMs - nowMs) / 1000)` for `scheduleAt`.

`execute(.fireNow)` uses `delay` (0), so the branch's `after <= 0` path becomes reachable. `scheduleAt` for a time already past keeps the one-second floor, which preserves the cancel reminder's "pre-scheduled branch only before 08:45" semantics (the core decides immediate versus scheduled).

### D7. Android worker, boot receiver, single-flight

`BackgroundSync.schedule(context)` owns the unique periodic request (`background-sync`, 15 minutes, 5-minute initial delay, `KEEP`). Its interval and delay become named constants (`INTERVAL_MINUTES`, `INITIAL_DELAY_MINUTES`) so a JVM test can pin them. `SyncGate` (an `AtomicBoolean`, `suspend fun <T> runExclusive(block: suspend () -> T): T?`, returns null when busy) wraps the menu scrape in `BackgroundSyncWorker`, `AppViewModel.runMenuCheck` and `runBackgroundSyncAsync`, giving Android the same single-flight rule iOS has (REQ-NTF-13). The worker stays self-contained: it builds its own core (`AptabaseSink` as before), like `GeofenceBroadcastReceiver`. `Result.retry()` on core init failure.

The boot receiver is not exported and filters on `ACTION_BOOT_COMPLETED`. WorkManager persists its own schedule; the receiver is the v1-parity belt and braces the docs ask for (`RECEIVE_BOOT_COMPLETED`).

### D8. Android channels and alarms

Channel names, importance and vibration become constants on `NotificationService` (`ORDER_REMINDERS_NAME = "Bestellungs-Erinnerungen"`, `MENU_UPDATES_NAME = "Neue Menüs"`, `ORDER_REMINDERS_VIBRATION = longArrayOf(0, 250, 250, 250)`) and an internal pure `alarmMode(sdkInt, canScheduleExact): AlarmMode` (`EXACT_IDLE` or `INEXACT_IDLE`) decides the call; the service maps it to `setExactAndAllowWhileIdle` or `setAndAllowWhileIdle`. Exact-ness is true below API 31 and otherwise follows `canScheduleExactAlarms()`.

**Decision (human ruling, gap C): do not declare `SCHEDULE_EXACT_ALARM`; keep the inexact fallback.** SnackPilot is a reminder app, not an alarm clock. The permission appears in system Settings as "can schedule exact alarms", which is disproportionate for a lunch reminder, and the branch already has the `setAndAllowWhileIdle` path. Consequence, accepted as a known tradeoff and not an oversight: on API 31+ `canScheduleExactAlarms()` is false, the exact branch is effectively dead there, and under Doze the daily reminder (11:00-13:45) and the cancel reminder (08:45 target, 09:00 deadline) may fire minutes late. The cancel reminder's window is the tightest; a late delivery after 09:00 is still harmless because the order can no longer be cancelled anyway. The `EXACT_IDLE` branch stays for API below 31 and for a future opt-in. The manifest is not edited for alarms.

**Deliberate deviation (human ruling): pickers accept any minute.** `settings.md` §7.1 specifies 15-minute steps (0/15/30/45). The native time pickers on both platforms (SwiftUI `DatePicker`, Android `TimePickerDialog`) offer free minutes, and snapping would add custom picker UI for no behavioural gain, since the core accepts any minute inside the window. The window itself (11:00-13:45) is enforced. No task.

### D9. Permission gate on the reminder toggle (gap D)

iOS: `AppModel.setDailyReminder` stays the persistence call. A new `AppModel.requestEnableDailyReminder() async` is what the toggle calls when turning on: `NotificationService.shared.requestPermissionGranted()`, then on false the existing `LocationAlert` state shows `Berechtigung fehlt` with the §7.1 message and the toggle stays off (the toggle binding reads `dailyReminderEnabled`, which is not touched); on true, `setDailyReminder(enabled: true, ...)`. Turning off and the picker keep calling `setDailyReminder` directly.

Android: `SettingsRootList` uses `rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission())`. Toggle on: if `Build.VERSION.SDK_INT < 33` or the permission is granted, call `vm.setDailyReminder(true, ...)`; otherwise launch the request and, in the callback, call `setDailyReminder` when granted or `vm.onNotificationPermissionDenied()` when not. `onNotificationPermissionDenied()` is changed to the exact §7.1 message (the launch-time dialog used a different sentence).

### D10. Clamp rule (shared by both shells)

Human ruling: nearest bound. Clamping hour and minute independently (the branch's behaviour, 08:30 becomes 11:30) is a bug. Tests on both shells pin both directions: 08:30 gives 11:00 and 13:50 gives 13:45.

`clamp(hour, minute)`: if `hour*60+minute < 660` return `(11, 0)`; if `> 825` return `(13, 45)`; else unchanged. 660 = 11:00, 825 = 13:45. Implemented as a pure function on each shell (`ReminderWindow.clamp` in Swift, `AppViewModel.clampReminder` in Kotlin) and used by the picker callbacks. The SwiftUI `DatePicker` additionally receives `in: reminderWindow`.

### D11. What proof means here

`make check` exercises only the core and spec validation. Shell tests run on their own hosts and are listed in each task's `check:`. iOS BGTask firing and Android job timing are OS-scheduled, so device proof (task 4.1) asserts registration, the manual trigger, and the delivery path, not a wall-clock firing. Evidence uses demo data only; a menu check with demo credentials is skipped by the core (`demo_credentials_skip`), so no host is contacted and no real credential is captured.

## Risks / Trade-offs

- Two cores over one directory on iOS (D5): accepted, documented.
- Tasks 2.2 and 2.3 both edit `AppModel.swift`; they touch different functions and run in order.
- `xcodebuild test` needs a simulator and the scheme name that xcodegen generates for `SnackPilotTests`; the coder records the exact command that worked in the handoff (the `-scheme SnackPilot` form is used if the app scheme already includes the test target).
- Device signing over non-interactive SSH can fail with `errSecInternalComponent` (see the `macbook-build` skill). Task 4.1 then stops and reports; no workaround is applied.
- The Android emulator proof of `BOOT_COMPLETED` may be refused for a non-exported receiver; the fallback in task 3.1 is an emulator reboot.
