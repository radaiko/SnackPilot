# Tasks

Conventions for every task below:

- **Task 0 is the dispatcher's, not a coder task**: before task 1.1, merge `origin/fix/notification-pipeline` unmodified (history kept, no squash) into the change branch. Every task starts from that code. A task with `status: verify` must already be satisfied by the merged code; the coder adds the missing tests and `@req` tags and changes production code only if the done bar fails (the handoff says which). A task with `status: gap` or `partial` names the new behaviour.
- One task at a time, in order. A task is done only when its done bar is met and `make check` is green.
- `check:` always starts with `make check`. Platform commands are listed in addition, never instead. `make check` only runs the core and spec validation; it does not build a shell.
- `host:` says where the work runs. **Linux** is this VM. **Mac** means `ssh mac`: start every remote command with `export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH`; the worktree is copied to the Mac as the `macbook-build` skill describes (this change adds no sync tooling). **Emulator** is the Android AVD `headless` (`~/android-sdk/emulator/emulator -avd headless -no-window -no-audio -gpu swiftshader_indirect`). **Device** is the connected Dev iPhone (XR), reached through the Mac. iOS builds and tests never run in the VM (`xcodegen`, `xcodebuild` are Mac-only); `make doctor` failing in the VM for them or for `ANDROID_NDK_HOME` is an environment gap and `tools/devops/doctor.sh` is not edited.
- Shorthand used in `check:` lines:
  - `IOS_TEST` = on the Mac, in the worktree copy: `cd src/ios && ./bootstrap.sh && xcodebuild test -scheme SnackPilotTests -destination 'platform=iOS Simulator,name=iPhone 17 Pro' CODE_SIGNING_ALLOWED=NO`. If xcodegen's `SnackPilot` scheme already contains the test target, `-scheme SnackPilot` is equivalent; the handoff records the command that ran.
  - `ANDROID_TEST` = on Linux: `cd src/android && ./bootstrap.sh && ./gradlew :app:testDebugUnitTest :app:assembleDebug --no-daemon`. It needs `ANDROID_NDK_HOME`; if that is unset the task stops and reports, it does not work around it. As with `IOS_TEST`, the command is a best guess: the handoff records the command that actually ran.
- Android and iOS install the same application id, so every install step first uninstalls the other build (`adb uninstall dev.radaiko.gourmetclient`, `xcrun simctl uninstall booted dev.radaiko.gourmetclient`).
- Tests tag requirements with a `// @req REQ-NTF-…` comment on the line above the test function (Rust, Swift and Kotlin alike). A tag must name a requirement in `specs/notification-pipeline/spec.md`.
- Tests use fake clocks, temp storage directories and the demo credentials (`demo` / `demo1234!`, which the core never sends to a host). No test reaches a real host. No fixture under `docs/fixtures/` or `src/core/tests/fixtures/` changes. No Gourmet or Ventopay request shape changes.
- `deps: none` means `Cargo.toml`, `Cargo.lock`, Gradle files and `project.yml` gain no new dependency. Dependencies not named in a task's `deps:` are not approved.
- Evidence screenshots show demo data only, are saved under `docs/evidence/notification-pipeline/<task-id>/` and named in the handoff. No real credential, cookie or live response is captured.
- `proof:` names the workflow or test that proves the task, as `openspec/config.yaml` requires.

## 1. Core

- [x] 1.1 Daily-reminder settings in the core KV
  - status: verify (implemented in `f836f12`)
  - tier: trivial
  - host: Linux
  - files: `src/core/src/storage/cache.rs`, `src/core/src/ffi.rs`
  - deps: none
  - reqs: REQ-NTF-01
  - acceptance: `cache::load_daily_reminder_settings`, `set_daily_reminder_enabled` and `set_daily_reminder_time` store `daily_reminder_enabled` as `"true"`/`"false"` and `daily_reminder_time` as `{"hour":H,"minute":M}`; defaults are disabled with no hour or minute; a time that is not an object with numeric `hour` and `minute` reads as unconfigured; `SnackPilotCore` exposes `daily_reminder_settings()`, `set_daily_reminder_enabled(bool)` and `set_daily_reminder_time(u8, u8)`; there is no `daily_reminder_sent_date` key.
  - tests: in `src/core/src/storage/cache.rs`: `daily_reminder_settings_round_trip` (`@req REQ-NTF-01`), `daily_reminder_time_ignores_garbage` (`@req REQ-NTF-01`), new `daily_reminder_enabled_without_time_reads_unconfigured` (`@req REQ-NTF-01`: enabled true, no time stored, hour and minute are `None`).
  - check: `make check`
  - proof: core workflow (`make test`)

- [x] 1.2 Post-fetch checks, geofence-notice cancel and one headless check
  - status: partial (post-fetch commands and the headless check are implemented; the geofence cancel and the single implementation are new)
  - tier: normal
  - host: Linux
  - files: `src/core/src/features/orders.rs`, `src/core/src/ffi.rs`
  - deps: none
  - reqs: REQ-NTF-02, REQ-NTF-03
  - acceptance: `OrderStore::fetch_orders` returns `Vec<NotificationCommand>`; after a successful fetch the commands are `CancelPending(GEOFENCE_REMINDER_ID)` when an order exists for Vienna-today, then the daily-reminder command, then the cancel-reminder command, in that order; a failed fetch returns an empty vec and sets the orders error; the checks never change `orders` or `error`. A private `order_reminder_commands(orders)` returns only the daily then cancel commands from the KV-backed settings and the persisted at-company flag; `post_fetch_notifications` prepends the geofence cancel to it; `OrderStore::run_order_sync_check` is `load_cached_orders()` then `order_reminder_commands(orders())` with no geofence cancel and no network; `SnackPilotCore::fetch_orders` returns the commands and `SnackPilotCore::run_order_sync_check` is a one-line delegate to the store (no copy of the check logic remains in `ffi.rs`); `confirm_orders`, `cancel_order` and `cancel_orders` keep discarding the commands as in the branch.
  - tests: in `src/core/src/features/orders.rs`: `fetch_emits_post_fetch_notification_commands` (`@req REQ-NTF-02`, `@req REQ-NTF-03` for its headless half), `fetch_on_failure_emits_no_commands` (`@req REQ-NTF-02`), new `fetch_cancels_geofence_notice_when_order_today` (`@req REQ-NTF-02`), new `fetch_without_order_today_emits_no_geofence_cancel` (`@req REQ-NTF-02`; use a clock date on which `ORDERS_PAGE` holds no order, read from the fixture), new `run_order_sync_check_empty_cache_retracts_cancel_reminder` (`@req REQ-NTF-03`: exactly one command, `CancelPending` for `CANCEL_REMINDER_ID`), new `run_order_sync_check_excludes_geofence_cancel` (`@req REQ-NTF-03`: cache holds an order for today, no `GEOFENCE_REMINDER_ID` command).
  - check: `make check`; additionally `grep -c check_daily_reminder src/core/src/ffi.rs` shows the function only in `daily_reminder_command`
  - proof: core workflow (`make test`)

## 2. iOS

- [x] 2.1 Packaged Info.plist from the xcodegen `info:` block
  - status: verify (implemented in `1725a19`, `ef4b9d3`, `292e94d`)
  - tier: normal
  - host: Mac
  - files: `src/ios/project.yml`, `src/ios/.gitignore`, `src/ios/README.md`, `src/ios/SnackPilotTests/NotificationPipelineTests.swift`
  - deps: none (the `SnackPilotTests` target adds no package)
  - reqs: REQ-NTF-07, REQ-NTF-06
  - acceptance: the app target has `GENERATE_INFOPLIST_FILE: "NO"` and an `info:` block writing `Info.plist` (git-ignored via `/Info.plist`); the built app's plist has `UIBackgroundModes` equal to `["fetch"]`, `BGTaskSchedulerPermittedIdentifiers` equal to `["dev.radaiko.gourmetclient.menu-refresh"]`, `CFBundleDisplayName` `SnackPilot`, both `NSLocation…UsageDescription` strings exactly as in the previous build, `ITSAppUsesNonExemptEncryption` false, a scene manifest, a launch screen, `CFBundleIdentifier` `dev.radaiko.gourmetclient`, and `CFBundleShortVersionString` / `CFBundleVersion` equal to `MARKETING_VERSION` / `CURRENT_PROJECT_VERSION` in `project.yml`; `tools/devops/ship.sh` is read (not edited) and its version-bump patterns still match those two lines, recorded in the handoff; `.forgejo/workflows/ios.yml`'s PlistBuddy step still reads `CFBundleIdentifier` from the built app (checked by running the same PlistBuddy command locally on the Mac).
  - tests: in `src/ios/SnackPilotTests/NotificationPipelineTests.swift`: `testPackagedInfoPlistPermitsBackgroundRefresh` (`@req REQ-NTF-07`), `testBackgroundIdentifierMatchesProjectConfig` (`@req REQ-NTF-06`, `@req REQ-NTF-07`), new `testPackagedInfoPlistKeepsAppIdentity` (`@req REQ-NTF-07`: bundle id, display name, both location strings non-empty, export-compliance flag false, scene manifest and launch screen present, version keys contain no `$(`).
  - check: `make check`; additionally `IOS_TEST`, then on the Mac `/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString'` and `-c 'Print :CFBundleVersion'` on the built `.app/Info.plist` compared with `grep -E 'MARKETING_VERSION|CURRENT_PROJECT_VERSION' src/ios/project.yml`
  - proof: ios workflow (`bootstrap.sh` then simulator build) plus the XCTest run above

- [x] 2.2 iOS background refresh task with a headless body
  - status: partial (registration, requeue, non-blocking completion, manual button implemented in `f836f12`, `01b0db9`, `53b0f4a`; the headless runner and the shared single-flight are new)
  - tier: hard
  - host: Mac
  - files: `src/ios/SnackPilot/BackgroundTasks.swift`, `src/ios/SnackPilot/SnackPilotApp.swift`, `src/ios/SnackPilot/AppModel.swift`, `src/ios/SnackPilot/MainTabView.swift`, `src/ios/SnackPilotTests/NotificationPipelineTests.swift`
  - deps: none
  - reqs: REQ-NTF-06, REQ-NTF-13, REQ-NTF-14, REQ-NTF-08
  - acceptance: `BackgroundTasks.registerLaunchHandler()` runs from `SnackPilotApp.init`; `scheduleRefresh()` runs on every `.active` scene phase, submits `BGAppRefreshTaskRequest` for `dev.radaiko.gourmetclient.menu-refresh` with `earliestBeginDate` 15 minutes ahead and logs a submit failure with `NSLog`; `handle(_:)` requeues first, never blocks the main queue, sets `expirationHandler` to cancel the work and completes the task exactly once with `success = finished && !cancelled`. New: `BackgroundTasks.runSync(core:credentials:deliver:) async -> Bool` per design D5 builds nothing but its arguments, runs `core.runMenuCheck(creds:)` (delivering `result.notification`) then `core.runOrderSyncCheck()` (delivering each command), is guarded by one `@MainActor static var isRunning` and returns `false` without any call when already running; `handle(_:)` builds a `SnackPilotCore` over the Application Support `snackpilot` directory with `analytics: nil`, reads `KeychainStore.savedGourmet()` and calls `runSync`; `BackgroundTasks.swift` no longer references `AppModel`; `AppModel.performBackgroundSync()` calls `runSync` with `self.core` and the per-instance `menuCheckRunning` flag is replaced by the shared one (`AppModel.runMenuCheck` uses `BackgroundTasks.isRunning`); the Diagnose button reads `Hintergrund-Sync ausführen`; a `#if DEBUG` launch argument `-uiTestBackgroundSync` runs `performBackgroundSync()` once, placed next to the `-uiTestLog` hook in the post-login DEBUG block of `AppModel.swift` so the demo credentials are already saved.
  - tests: in `src/ios/SnackPilotTests/NotificationPipelineTests.swift`: new `testHeadlessRunDeliversOrderSyncCommandsWithoutNetwork` (`@req REQ-NTF-13`, `@req REQ-NTF-14`: temp-dir core, demo credentials, recording `deliver`, expects `true` and exactly one delivered `.cancelPending(id: "cancel-order-reminder")`), new `testBackgroundRunIsSingleFlight` (`@req REQ-NTF-13`: sets `isRunning = true`, expects `false` and zero deliveries), `testCoreExposesHeadlessSyncCheckAndSettings` (`@req REQ-NTF-03`, `@req REQ-NTF-01`).
  - check: `make check`; additionally `IOS_TEST`; additionally `grep -n 'AppModel' src/ios/SnackPilot/BackgroundTasks.swift` prints nothing
  - proof: XCTest run above; Device registration evidence in 4.1

- [x] 2.3 iOS FireNow shows immediately
  - status: gap (the `after <= 0` branch from `f836f12` is unreachable; consuming `FireNow` and post-fetch commands is implemented)
  - tier: normal
  - host: Mac
  - files: `src/ios/SnackPilot/NotificationService.swift`, `src/ios/SnackPilot/AppModel.swift`, `src/ios/SnackPilotTests/NotificationPipelineTests.swift`
  - deps: none
  - reqs: REQ-NTF-09, REQ-NTF-08
  - acceptance: `NotificationService` has `static func request(id:title:body:after:) -> UNNotificationRequest` (nil trigger when `after <= 0`, otherwise a non-repeating `UNTimeIntervalNotificationTrigger`) and `static func delay(for:nowMs:) -> TimeInterval` (0 for `.fireNow`, `max(1, (fireAtEpochMs - nowMs) / 1000)` for `.scheduleAt`); `execute(.fireNow)` uses delay 0 so the request has no trigger; `.scheduleAt` and `.cancelPending` behave as before (cancel removes pending and delivered); the foreground delegate still presents banner, sound and list; `AppModel.runMenuCheck` and `loadOrders` deliver the commands they receive (verify by reading; no change expected).
  - tests: in `src/ios/SnackPilotTests/NotificationPipelineTests.swift`: new `testFireNowRequestHasNoTrigger` (`@req REQ-NTF-09`), new `testScheduleAtRequestUsesTimeIntervalTrigger` (`@req REQ-NTF-09`: 600 s ahead gives a non-repeating trigger of about 600 s), new `testDelayForCommands` (`@req REQ-NTF-09`: fireNow 0; scheduleAt in the past 1; scheduleAt 600 s ahead about 600), `testFireNowCommandCarriesPayload` (`@req REQ-NTF-08`), `testScheduleAtCommandCarriesFireTime` (`@req REQ-NTF-08`).
  - check: `make check`; additionally `IOS_TEST`
  - proof: XCTest run above; simulator banner screenshot in 4.1

- [ ] 2.4 iOS reminder settings through the core, picker window and permission gate
  - status: partial (core-backed settings, 11:00 default and window clamp implemented; the permission gate and the nearest-bound rule are new)
  - tier: normal
  - host: Mac
  - files: `src/ios/SnackPilot/AppModel.swift`, `src/ios/SnackPilot/MainTabView.swift`, `src/ios/SnackPilotTests/NotificationPipelineTests.swift`
  - deps: none
  - reqs: REQ-NTF-04, REQ-NTF-05
  - acceptance: an `enum ReminderWindow` in `AppModel.swift` provides `clamp(hour:minute:)` (design D10), `hydrate(_:) -> (enabled: Bool, hour: Int, minute: Int)` from `DailyReminderSettings` (no stored time gives 11:00) and `enable(core:hour:minute:requestPermission:) async -> Bool`, which returns `false` and writes nothing when the permission closure returns false, otherwise writes the time then the enabled flag through the core and returns `true`; `AppModel` hydrates its mirrors from `core.dailyReminderSettings()` at init, defaults are off and 11:00, `setDailyReminder` writes time then enabled through the core and no longer writes `UserDefaults`; the Einstellungen toggle calls a new `AppModel.requestEnableDailyReminder()` when turning on, which uses `NotificationService.shared.requestPermissionGranted()` and, on denial, sets the existing alert state to title `Berechtigung fehlt` and message `Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.` and leaves the toggle off; turning off and the picker call `setDailyReminder` directly; the `DatePicker` uses `in: reminderWindow` (11:00-13:45) and its setter uses `ReminderWindow.clamp`; the `-uiTestReminder` hook uses 11:30.
  - tests: in `src/ios/SnackPilotTests/NotificationPipelineTests.swift`: new `testReminderClampUsesNearestBound` (`@req REQ-NTF-04`: 08:30 gives 11:00, 13:50 gives 13:45, 14:30 gives 13:45, 12:20 stays), new `testReminderHydrationDefaultsAndStoredValues` (`@req REQ-NTF-04`), new `testEnableDeniedPersistsNothing` (`@req REQ-NTF-05`: temp-dir core, closure returns false, expects `false` and `dailyReminderSettings()` still disabled with no hour), new `testEnableGrantedWritesTimeThenFlag` (`@req REQ-NTF-05`: closure returns true, expects `true` and settings enabled at the given time).
  - check: `make check`; additionally `IOS_TEST`
  - proof: XCTest run above

## 3. Android

- [ ] 3.1 WorkManager worker, boot receiver and single-flight
  - status: partial (worker, boot receiver, schedule at app start and the manual button implemented in `f836f12`; `SyncGate`, constants and the debug extra are new)
  - tier: normal
  - host: Linux + Emulator
  - files: `src/android/app/src/main/java/dev/radaiko/snackpilot/BackgroundSyncWorker.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/BootBroadcastReceiver.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/SnackPilotApp.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/AppViewModel.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/MainActivity.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/ui/AppUi.kt`, `src/android/app/src/main/AndroidManifest.xml`, `src/android/app/build.gradle.kts`, `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`
  - deps: `androidx.work:work-runtime-ktx:2.9.1` (implementation), `junit:junit:4.13.2` (testImplementation); no other dependency (coroutine tests use `runBlocking` from the existing coroutines dependency)
  - reqs: REQ-NTF-10, REQ-NTF-13, REQ-NTF-14, REQ-NTF-08
  - acceptance: `BackgroundSync.schedule` enqueues unique periodic work `background-sync` (`INTERVAL_MINUTES = 15`, `INITIAL_DELAY_MINUTES = 5`, `ExistingPeriodicWorkPolicy.KEEP`) and is called from `SnackPilotApp.onCreate` and `BootBroadcastReceiver` (action `BOOT_COMPLETED`, `exported="false"`, permission `RECEIVE_BOOT_COMPLETED` declared); `BackgroundSyncWorker` builds its own core over `filesDir/snackpilot`, returns `Result.retry()` when the core cannot be built, runs `core.runMenuCheck` with the saved Gourmet credentials inside `SyncGate.runExclusive` and delivers its `notification`, then runs `core.runOrderSyncCheck()` and delivers each command; new `object SyncGate` (`AtomicBoolean`, `suspend fun <T> runExclusive(block: suspend () -> T): T?`, null when busy) also wraps `AppViewModel.runMenuCheck`; `AppViewModel.runBackgroundSyncAsync` runs the same two steps and the Diagnose control reads `Hintergrund-Sync`; a debug-only intent extra `uiTestBackgroundSync` (guarded by `BuildConfig.DEBUG`, read in `MainActivity` and passed through `AppUi.kt` to a `debugAutoBackgroundSync` flag, exactly like `uiTestLog` and `debugAutoLog`) calls it after login; `AppViewModel.runMenuCheck` and `loadOrders` deliver the commands they receive (verify by reading).
  - tests: in `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`: new `backgroundSyncScheduleIsPinned` (`@req REQ-NTF-10`: unique name `background-sync`, interval at least 15), new `syncGateRejectsOverlappingRuns` (`@req REQ-NTF-13`: nested `runExclusive` returns null), new `syncGateReleasesAfterRun` (`@req REQ-NTF-13`: a second sequential run executes), `commandVariantsCarryChannelRouting` (`@req REQ-NTF-08`).
  - check: `make check`; additionally `ANDROID_TEST`; on the emulator, after `adb uninstall dev.radaiko.gourmetclient` then `adb install -r` of the debug APK and one launch, `adb shell dumpsys jobscheduler | grep -c dev.radaiko.gourmetclient` is at least 1, `adb shell cmd jobscheduler run -f dev.radaiko.gourmetclient <jobId>` followed by `adb logcat -d` shows no `FATAL EXCEPTION`, and `adb shell am broadcast -a android.intent.action.BOOT_COMPLETED -n dev.radaiko.gourmetclient/dev.radaiko.snackpilot.BootBroadcastReceiver` leaves exactly one job (if the shell may not target a non-exported receiver, `adb reboot` the emulator and repeat the `dumpsys` count instead; record which)
  - proof: android workflow (`assembleDebug`) plus the JVM unit tests above

- [ ] 3.2 Android delivery: permission gate, channels, Doze-safe alarms
  - status: verify (implemented in `f836f12`; constants and pure helpers are extracted so they can be tested)
  - tier: normal
  - host: Linux + Emulator
  - files: `src/android/app/src/main/java/dev/radaiko/snackpilot/NotificationService.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/MainActivity.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/AppViewModel.kt`, `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`
  - deps: none
  - reqs: REQ-NTF-11, REQ-NTF-12
  - acceptance: `NotificationService` creates `order-reminders` named `Bestellungs-Erinnerungen`, importance HIGH, vibration `[0, 250, 250, 250]`, and `menu-updates` named `Neue Menüs`, importance DEFAULT, from named constants; `deliver` posts nothing when `notificationsAllowed(sdkInt, granted)` is false (internal pure function, true below API 33); `schedule` uses an internal pure `alarmMode(sdkInt, canScheduleExact)` returning `EXACT_IDLE` below API 31 or when exact is permitted and `INEXACT_IDLE` otherwise, mapped to `setExactAndAllowWhileIdle` / `setAndAllowWhileIdle`; the same id replaces the pending alarm and `CancelPending` cancels it (unchanged); `MainActivity` requests `POST_NOTIFICATIONS` only when not granted and routes a denial to `AppViewModel.onNotificationPermissionDenied()`, resolving the same `AppViewModel` instance the UI holds; the manifest is not edited and `SCHEDULE_EXACT_ALARM` stays undeclared (design D8), and the handoff states that on API 31+ alarms use `setAndAllowWhileIdle` and may fire minutes late under Doze.
  - tests: in `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`: `channelIdsMatchSpecNames` (`@req REQ-NTF-12`), new `channelDefinitionsMatchSpec` (`@req REQ-NTF-12`: names, importance values, vibration array), new `alarmModeIsExactBelowApi31` (`@req REQ-NTF-12`), new `alarmModeFallsBackWhenExactNotPermitted` (`@req REQ-NTF-12`), new `alarmModeIsExactWhenPermitted` (`@req REQ-NTF-12`), new `notificationsAllowedBelowApi33Always` (`@req REQ-NTF-11`), new `notificationsDeniedBlocksOnApi33` (`@req REQ-NTF-11`).
  - check: `make check`; additionally `ANDROID_TEST`; on the emulator after a fresh install, `adb shell dumpsys notification --noredact | grep -A6 'order-reminders'` shows the name `Bestellungs-Erinnerungen` and importance 4, and the same for `menu-updates` with `Neue Menüs` and importance 3
  - proof: android workflow (`assembleDebug`) plus the JVM unit tests above

- [ ] 3.3 Android reminder settings through the core, picker window and permission gate
  - status: partial (core-backed settings, 11:00 default and window clamp implemented; the toggle permission gate, the spec wording and the nearest-bound rule are new)
  - tier: normal
  - host: Linux
  - files: `src/android/app/src/main/java/dev/radaiko/snackpilot/AppViewModel.kt`, `src/android/app/src/main/java/dev/radaiko/snackpilot/ui/AppUi.kt`, `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`
  - deps: none
  - reqs: REQ-NTF-04, REQ-NTF-05
  - acceptance: `AppViewModel` companion has `clampReminder(hour, minute): Pair<Int, Int>` (design D10), `hydrateReminder(enabled: Boolean?, hour: Int?, minute: Int?)` returning the UI values with defaults off and 11:00, and the constants `PERMISSION_DENIED_TITLE = "Berechtigung fehlt"` and `PERMISSION_DENIED_MESSAGE = "Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren."`; `onNotificationPermissionDenied()` uses them; `init` hydrates from `core.dailyReminderSettings()`; `setDailyReminder` writes the time then the enabled flag through the core and no longer writes `SharedPreferences`; in `SettingsRootList` turning the toggle on calls `vm.setDailyReminder(true, …)` only when `notificationsAllowed(Build.VERSION.SDK_INT, granted)` is true, otherwise it launches `rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission())` and, in the callback, enables on grant or calls `vm.onNotificationPermissionDenied()` and leaves the switch off; turning off requests nothing; the time picker callback uses `clampReminder`; the debug auto-reminder uses 11:30.
  - tests: in `src/android/app/src/test/java/dev/radaiko/snackpilot/NotificationPipelineTest.kt`: new `clampReminderUsesNearestBound` (`@req REQ-NTF-04`: 08:30 gives 11:00, 13:50 and 14:30 give 13:45, 12:20 stays), new `hydrateReminderFallsBackToSpecDefaults` (`@req REQ-NTF-04`), new `permissionDeniedTextMatchesSpec` (`@req REQ-NTF-05`: title and message strings equal the spec text).
  - check: `make check`; additionally `ANDROID_TEST`
  - proof: android workflow (`assembleDebug`) plus the JVM unit tests above

## 4. End-to-end evidence

- [ ] 4.1 Device and emulator proof of the pipeline
  - status: gap (no end-to-end proof exists)
  - tier: normal
  - host: Mac (simulator and Device), Emulator
  - files: `docs/evidence/notification-pipeline/4.1/**`
  - deps: none
  - reqs: REQ-NTF-06, REQ-NTF-07, REQ-NTF-09, REQ-NTF-10, REQ-NTF-12, REQ-NTF-14
  - acceptance: all earlier tasks are merged. iOS simulator (iPhone 17 Pro, uninstall first): launch with `-uiTestNotify` and save a screenshot that shows the banner for "Neues Menü verfügbar" (immediate path); launch with `-uiTestDemo -uiTestTab settings -uiTestLog -uiTestBackgroundSync` and save the Diagnose log screenshot showing `demo_credentials_skip`. iOS Device (Dev iPhone via the Mac, per the `macbook-build` skill): the installed app's `Info.plist` contains `UIBackgroundModes` `fetch` and the permitted identifier (`plutil -p` output saved as text), and a device log capture of the first 60 s after launch contains no `SnackPilot BGTask: submit failed` line; if signing over non-interactive SSH fails with `errSecInternalComponent`, the task records the exact error, completes everything else and hands the Device part to the human, with no workaround. Android emulator (uninstall first): `adb shell cmd jobscheduler run -f dev.radaiko.gourmetclient <jobId>` produces logcat output from `BackgroundSyncWorker` and no crash; launch with `--ez uiTestNotify true` and save a screenshot of the notification shade; `--ez uiTestSchedule true` runs without `FATAL EXCEPTION`; `--ez uiTestDemo true --ez uiTestLog true --ez uiTestBackgroundSync true` shows `demo_credentials_skip` in the Diagnose log (screenshot); `dumpsys notification` shows both channels as in 3.2. The handoff lists every file under `docs/evidence/notification-pipeline/4.1/` and states plainly that real iOS background firing was not proven (OS-scheduled, not asserted).
  - tests: none added; the evidence above is the test. Every earlier task's unit tests are re-run: `make check`, `IOS_TEST`, `ANDROID_TEST`.
  - check: `make check`; additionally `IOS_TEST` and `ANDROID_TEST`
  - proof: the evidence files named in the handoff plus the ios and android workflows
