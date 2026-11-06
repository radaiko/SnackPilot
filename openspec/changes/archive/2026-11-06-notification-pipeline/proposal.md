# Proposal: Notification pipeline contract (retrofit)

domain: architectural

## Why

The review of 2026-10-28 (main at `744785c`, release v2.1.0) found five independent breaks that together meant **zero notifications reached a device** on either platform: the headless background entry points did not exist, the daily-reminder settings lived in shell storage the background could not read, the reminder picker defaulted to a time the core refuses to fire at, the iOS background configuration never reached the packaged `Info.plist`, and neither shell delivered the commands the core returned.

Branch `origin/fix/notification-pipeline` (6 commits off `744785c`, 22 files, +764/-82) already fixes most of this, but it was written without a change, without per-requirement tests and without a done bar. This change is a **retrofit**: it writes the contract for that existing work so it can be verified by the loop, and it names what the branch still leaves broken. It does not widen scope beyond notification delivery and does not edit `docs/requirements/`.

## What Changes

- **Core**: daily-reminder settings in the core KV with the v1 keys; `fetch_orders` returns the post-fetch `NotificationCommand`s; a headless `run_order_sync_check()` FFI entry.
- **iOS**: `BGAppRefreshTask` background sync, an `Info.plist` generated from `project.yml` (`info:` block), the immediate-display path, reminder settings through the core, picker clamped to the spec window, first XCTest target.
- **Android**: periodic WorkManager worker, boot receiver, POST_NOTIFICATIONS gating, doze-safe alarms, spec channel names and vibration pattern, reminder settings through the core, first JVM unit tests.
- **Both**: a "Hintergrund-Sync" manual trigger in Einstellungen → Diagnose that runs the background body on demand.
- **New in this change** (not on the branch, see "Gaps"): tasks 1.2 (geofence-notice cancel and a single headless implementation), 2.2 (headless iOS runner), 2.3 (iOS `FireNow` actually immediate), 2.4 and 3.3 (permission gate on the reminder toggle), 3.1 (Android single-flight).

### Baseline for the coder

The dispatcher's first step, **before task 1.1 and outside any coder task**, is to merge `origin/fix/notification-pipeline` unmodified into the change branch (history kept, no squash). Every task below then starts from that code. Tasks that are marked *verify* must be satisfied by the merged code and only add the missing `@req` tests; they change production code only if the done bar fails, and the handoff says so.

## Implementation status: what the branch already does

This is the answer to "is the notification system properly fixed?". Short version: **the decision logic and Android plumbing are largely done; iOS delivery has one dead path and one design flaw; four spec'd behaviours are still missing.**

| Area | Status on the branch | Task |
| --- | --- | --- |
| Core daily-reminder settings in KV, v1 keys and formats (`daily_reminder_enabled`, `daily_reminder_time`), garbage ignored | **Implemented**, 2 tests, untagged | 1.1 verify |
| `fetch_orders` returns commands after success, none on failure, errors swallowed | **Implemented**, 2 tests, untagged | 1.2 (verify + gap E) |
| Headless `run_order_sync_check` (no network, cache first) | **Implemented twice**: `OrderStore::run_order_sync_check` and a copy in `ffi.rs` | 1.2 |
| iOS `Info.plist` via xcodegen `info:` (`UIBackgroundModes=[fetch]`, permitted identifier, version pins) | **Implemented**, pinned by a host-app test; needs verification on a real build | 2.1 verify |
| iOS BGAppRefreshTask registration, self-requeue, non-blocking completion, truthful result | **Implemented** (main-queue deadlock fixed in `01b0db9`) | 2.2 verify |
| iOS background body | **Flawed**: builds a whole `AppModel` | 2.2 gap B |
| iOS immediate display path in `NotificationService` | **Dead code**: `execute(.fireNow)` still passes `after: 0.1`, so `after <= 0` is never reached | 2.3 gap A |
| iOS consumes `FireNow` from `runMenuCheck` and the post-fetch commands | **Implemented** | 2.3 verify |
| iOS reminder settings via core, default 11:00, picker window 11:00-13:45 | **Implemented**, clamp logic inline and untested | 2.4 verify |
| Reminder toggle requests permission and aborts with "Berechtigung fehlt" (both shells) | **Missing** | 2.4, 3.3 gap D |
| Android WorkManager worker, boot receiver, schedule at app start | **Implemented** | 3.1 verify |
| Android single-flight menu scrape | **Missing** (iOS has it) | 3.1 |
| Android POST_NOTIFICATIONS gate, spec channel names, vibration, allow-while-idle alarms | **Implemented** | 3.2 verify |
| Android exact alarms | **Unreachable on API 31+ by decision**: `SCHEDULE_EXACT_ALARM` stays undeclared, so reminders use the inexact allow-while-idle fallback (design D8) | none |
| Manual "Hintergrund-Sync" triggers | **Implemented** on both shells | 2.2, 3.1 verify |
| End-to-end proof on a device and an emulator | **Missing** | 4.1 |

## Gaps the branch leaves open (each is a task)

- **A. iOS `FireNow` is not immediate.** `NotificationService.execute(.fireNow)` calls `deliver(after: 0.1)`; the new `after <= 0` branch can never run. The new-menu notification and the 08:45-09:00 cancel reminder must show immediately (new-menu §3.4, cancel-reminder §4.3).
- **B. The iOS background handler constructs a full `AppModel()`.** Its `init` runs auto-login for Gourmet and Ventopay, a billing fetch, the launch analytics event, the permission prompt and geofence setup. A background run therefore scrapes more than the menu check, which `05-platform-services.md` §4.2 forbids ("no network calls to avoid concurrent scraping") and `AppModel.attemptAutoLogin` itself calls ban risk. No request shape changes; the fix removes traffic.
- **C. Android exact alarms cannot be granted.** Decided: accepted as a tradeoff, no permission declared (design D8).
- **D. The reminder toggle never asks for permission** on either shell, although `settings.md` §7.1 and `notifications-daily-reminder.md` §9 require the request, the "Berechtigung fehlt" alert and an unchanged switch. The Android launch-time denial dialog also uses different wording from the spec string.
- **E. Post-fetch hook omits the geofence-notice cancel.** `orders.md` §8 says that when an order exists for today the geofence "no order" notification (`geofence-no-order-reminder`) is cancelled first, then the daily and cancel checks run. The branch emits only the latter two.
- **F. Two copies of the headless check** (`ffi.rs` and `OrderStore`) can drift.
- **Android single-flight** (part of task 3.1): the worker, the manual button and the foreground menu check can scrape concurrently.

Not fixed here, by decision (see Assumptions and Follow-ups): picker 15-minute steps (deliberate deviation, design D8), `docs/requirements/05` §4.5 background modes, `notification.reminderToggled` analytics (absent on both shells before this change).

## Approvals granted by this change

Approving this change grants exactly these edits that `AGENTS.md` otherwise forbids:

- **Dependency manifests**: `src/android/app/build.gradle.kts` gains `androidx.work:work-runtime-ktx:2.9.1` (task 3.1) and `junit:junit:4.13.2` as `testImplementation` (task 3.1). No `Cargo.toml` or `Cargo.lock` changes. `src/ios/project.yml` gains the `info:` block and the `SnackPilotTests` target (task 2.1); it adds no package dependency.
- **Not granted**: any Gourmet or Ventopay request-shape change, edits to `docs/requirements/`, `openspec/specs/` (except normal archive), CI config, force-push, or push to `main` outside the loop's merge.

## Assumptions (change these before approving if wrong)

1. `openspec/specs/` is empty, so the delta spec `notification-pipeline` is an initial set of requirements, not a modification. Requirement ids `REQ-NTF-*` are defined by this change's spec; `docs/requirements/` has no ids, so each requirement cites its source section.
2. The dispatcher merges the fix branch as the baseline (above). If the human prefers cherry-picking or re-implementing, only that first step changes.
3. Shell-owned reminder preferences from v2.1.0 (`UserDefaults` / `SharedPreferences` `daily_reminder_*`) are **not migrated**. They defaulted to 08:00, which the core refused at/after, so no working reminder can be lost.
4. Picker values outside 11:00-13:45 clamp to the **nearest bound** (08:30 gives 11:00, 13:50 gives 13:45; human ruling). The branch clamps hour and minute independently (08:30 becomes 11:30), which is a bug; tasks 2.4 and 3.3 replace it and each pins both cases in a test.
5. Android keeps the exact-alarm fallback (`setAndAllowWhileIdle`) as the shipped behaviour; `SCHEDULE_EXACT_ALARM` is not declared (human ruling). Reminders may fire minutes late under Doze.
6. `UIBackgroundModes=[fetch]` is correct for `BGAppRefreshTask`; `05-platform-services.md` §4.5 lists `location` and `processing`, which v2 deliberately does not set (region monitoring needs no mode; `processing` is for `BGProcessingTask`). The doc is not edited; it is flagged.
7. The core's `FileKv` keys `daily_reminder_enabled` and `daily_reminder_time` (JSON) are authoritative. **Known conflict:** `gpui-kit-ui-rewrite` (design D3, task 3.3) names `daily_reminder_hour` / `daily_reminder_minute`; that change must adopt the core's `daily_reminder_time` JSON when it reaches its task 3.3. This change does not edit it.
8. Real background firing on iOS is OS-scheduled and not deterministic. Acceptance is: registration succeeds on a device, the manual trigger runs the same body, and unit tests pin the contract.

## Capabilities

### New Capabilities

- `notification-pipeline`: how order-driven reminders and the new-menu notification are decided in the core, executed headlessly in the background on iOS and Android, and delivered on both shells.

### Modified Capabilities

None. `openspec/specs/` holds no capabilities today.

## Impact

- **Code**: `src/core/src/{features/orders.rs,ffi.rs,storage/cache.rs}`; iOS `AppModel.swift`, `BackgroundTasks.swift`, `MainTabView.swift`, `NotificationService.swift`, `SnackPilotApp.swift`, `project.yml`, `.gitignore`, `README.md`, `SnackPilotTests/NotificationPipelineTests.swift`; Android `AppViewModel.kt`, `BackgroundSyncWorker.kt`, `BootBroadcastReceiver.kt`, `MainActivity.kt`, `NotificationService.kt`, `SnackPilotApp.kt`, `ui/AppUi.kt`, `AndroidManifest.xml`, `build.gradle.kts`, `NotificationPipelineTest.kt`.
- **Safety**: no Gourmet or Ventopay request shape changes; gap B reduces background traffic. No fixture changes (`docs/fixtures/` and `src/core/tests/fixtures/` stay hash-matched). Tests use demo credentials and fake clocks and reach no real host.
- **Verification hosts**: Linux for core and Android JVM tests; the Mac (`ssh mac`) for every iOS build and test, the booted iPhone 17 Pro simulator, and the connected Dev iPhone; the Android emulator.
- **Interaction**: `gpui-kit-ui-rewrite` assumption 6 puts these background tasks out of its scope, which is consistent; its reminder-key naming conflicts (assumption 7 above).
- **Size**: 10 tasks (2 core, 4 iOS, 3 Android, 1 evidence), each one PR-sized slice of the existing 22-file diff plus the gaps.

## Follow-ups (not tasks of this change)

- A later docs addendum against `docs/requirements/05-platform-services.md` §4.5 recording that v2 declares `UIBackgroundModes=["fetch"]` for `BGAppRefreshTask` instead of v1's `["location", "processing"]`. It needs explicit approval because `AGENTS.md` forbids editing `docs/requirements/` without it; this change does not edit it.
- `gpui-kit-ui-rewrite` adopts the core's `daily_reminder_time` key at its task 3.3 (assumption 7).

## Open questions

None. The earlier questions were ruled on by the human: no `SCHEDULE_EXACT_ALARM` (design D8), any-minute pickers as a deliberate deviation (design D8), the `fetch` mode recorded as a follow-up, and the nearest-bound clamp (design D10).
