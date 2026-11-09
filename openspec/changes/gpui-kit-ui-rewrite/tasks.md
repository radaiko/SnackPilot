# Tasks

Conventions for every task below:

- One task at a time, in order. A task is done only when its done bar is met and `make check` is green.
- `check:` is always `make check` (after task 1.1 it includes `ui-check`). Platform commands are listed in addition, never instead.
- `host:` says where the work runs. **Linux** is this VM. **Mac** means `ssh mac` (non-interactive: start every remote command with `export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH`; sync first with `make mac-sync`, available from task 1.2). **Emulator** is the Android AVD `headless` (`~/android-sdk/emulator/emulator -avd headless -no-window -no-audio -gpu swiftshader_indirect`).
- Android and iOS install the same application id, so every install step first uninstalls the other build (`adb uninstall dev.radaiko.gourmetclient`, `xcrun simctl uninstall booted dev.radaiko.gourmetclient`).
- Evidence screenshots show demo data only (magic credentials), are saved under `docs/evidence/gpui-kit-ui-rewrite/<task-id>/`, and are named in the handoff. No real credential, cookie or live response is ever captured.
- Tests tag requirements with `// @req REQ-…` on the test function. `deps: none` means `Cargo.toml`, `Gradle` and `project.yml` gain no new dependency.
- `proof:` names the workflow or test that proves the task, as `openspec/config.yaml` requires.
- Spike gates 2.1, 2.2 and 2.3 are go/no-go (design D10). If one fails, stop and report; do not start group 3.

## 1. Foundation

- [x] 1.1 Create the `src/ui` workspace, pin the gpui dependency set and add `ui-check`
  - tier: normal
  - host: Linux
  - files: `src/ui/Cargo.toml`, `src/ui/Cargo.lock`, `src/ui/.gitignore`, `src/ui/rust-toolchain.toml` (only if the pinned gpui needs a newer rustc than the installed one; record the version), `src/ui/model/Cargo.toml`, `src/ui/model/src/lib.rs`, `src/ui/model/tests/pins.rs`, `src/ui/app/Cargo.toml`, `src/ui/app/src/lib.rs`, `Makefile`, `.forgejo/workflows/ui.yml`
  - deps: `gpui-pre =0.3.4` (default-features off), `gpui-kit` git rev `7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7` (default-features off, feature `component`), `gpui-pre-mobile` git rev `0b882efdac7f524e0bb0b1d4c886b2aa752f9f20` (package alias `gpui-mobile`), all in `snackpilot-app` only; `snackpilot-core` by path, `tokio`, `serde`, `serde_json`, `chrono`, `chrono-tz`, `thiserror` in `snackpilot-model` at the same version ranges as `src/core/Cargo.toml`
  - reqs: REQ-UI-01, REQ-UI-02, REQ-UI-08
  - acceptance: `src/ui` is a workspace with `snackpilot-model` (no gpui) and `snackpilot-app` (gpui deps, empty `lib.rs`); `src/core` is unchanged; `make ui-check` runs fmt, clippy `-D warnings` and `cargo test -p snackpilot-model` in `src/ui` and is part of `make check`; `.forgejo/workflows/ui.yml` runs `make ui-check` on `src/ui/**` changes on the `android` runner; `cargo tree -i gpui-pre` in `src/ui` shows exactly one version `0.3.4`; `cargo tree -p snackpilot-model` shows no `reqwest`, `hyper` or `gpui*` directly beneath the crate other than via `snackpilot-core`.
  - tests: `src/ui/model/tests/pins.rs` parses `src/ui/Cargo.toml` and `src/ui/Cargo.lock` and asserts the three pins and a single `gpui-pre` (`@req REQ-UI-02`); a second test asserts the model manifest lists no HTTP client crate (`@req REQ-UI-08`).
  - check: `make check`; additionally `cd src/ui && cargo metadata --locked --format-version 1 >/dev/null`
  - proof: ui workflow (`make ui-check`), core workflow unchanged

- [x] 1.2 Add the worktree-to-Mac rsync script and `make mac-sync`
  - tier: trivial
  - host: Linux, then Mac
  - files: `tools/devops/sync-mac.sh`, `tools/devops/test-sync-mac.sh`, `tools/devops/README.md`, `Makefile`
  - deps: none
  - reqs: REQ-UI-01 (enabler for Mac verification)
  - acceptance: `tools/devops/sync-mac.sh` rsyncs the current worktree root to `mac:~/vm-sync/SnackPilot-gpui/` with `--delete`, excluding `.git`, `target`, `build`, `DerivedData`, `*.xcodeproj`, `.gradle`, `node_modules`, and protecting those paths from remote deletion; `--dry-run` prints the plan; it refuses to run if the destination would be `~/vm-sync/SnackPilot`; `make mac-sync` calls it; README documents the PATH export for non-interactive ssh.
  - tests: `tools/devops/test-sync-mac.sh` runs `sync-mac.sh --dry-run` against a temp tree and asserts the exclusions and the refusal case without contacting the Mac.
  - check: `make check`; additionally `bash tools/devops/test-sync-mac.sh`, then `make mac-sync` followed by `ssh mac 'test -f ~/vm-sync/SnackPilot-gpui/AGENTS.md'`
  - proof: shell test above; `ui` workflow unchanged

## 2. Spikes (go/no-go)

- [x] 2.1 Android spike: a gpui-kit `Root` runs on the `headless` emulator
  - tier: hard
  - host: Linux + Emulator
  - files: `src/ui/app/Cargo.toml`, `src/ui/app/src/lib.rs`, `src/ui/app/src/mobile.rs`, `src/ui/app/src/assets.rs`, `src/ui/app/assets/fonts/Inter-Regular.ttf`, `src/ui/app/assets/fonts/Inter-SemiBold.ttf`, `src/ui/app/assets/fonts/Inter-Bold.ttf`, `src/ui/app/assets/fonts/OFL.txt`, `src/ui/android/**` (new Gradle host project, NativeActivity based, application id `dev.radaiko.gourmetclient`), `tools/devops/ui-android-build.sh`, `tools/devops/doctor.sh`, `openspec/changes/gpui-kit-ui-rewrite/design.md` (append "Spike result: Android"), `docs/evidence/gpui-kit-ui-rewrite/2.1/*.png`
  - deps: `jni` and `ndk-context` only if the gpui-mobile example at rev `0b882efdac7f524e0bb0b1d4c886b2aa752f9f20` needs them in the app crate (record in the spike result); Inter font files (SIL OFL) as assets; toolchain installs outside the repo: `sdkmanager "ndk;30.0.16248370"`, `rustup target add aarch64-linux-android x86_64-linux-android`, `cargo install cargo-ndk`
  - reqs: REQ-UI-01, REQ-UI-02, REQ-UI-03, REQ-UI-06
  - acceptance: following the gpui-mobile example's `android` tree, the host builds an APK whose native library mounts one `gpui_kit::component::Root` (via `gpui_kit::init(cx)`, not `application()`); the root shows "Menüs / Bestellungen / Abrechnung / Einstellungen" and a button with a counter in the bundled Inter font; a tap increments the counter; the app stays alive 10 s; `tools/devops/ui-android-build.sh` builds for `x86_64` (emulator) and `aarch64-linux-android` and `doctor.sh` reports the NDK; the manifest declares `configChanges` so the Activity is not recreated (D11). The "Spike result: Android" section records versions, commands, what the example needed, and a pass or fail verdict.
  - tests: `adb install -r`, `adb shell am start`, `adb shell pidof dev.radaiko.gourmetclient` after 10 s, `adb logcat -d` contains no `FATAL EXCEPTION`, `adb shell input tap <x> <y>` then two screenshots (`adb exec-out screencap -p`) show counter 0 then 1 (`@req REQ-UI-03` for the single root: logcat shows one init).
  - check: `make check`; additionally `tools/devops/ui-android-build.sh` and `cd src/ui/android && ./gradlew :app:assembleDebug --no-daemon`
  - proof: android workflow build (extended in 6.2) plus the emulator evidence in the handoff

- [x] 2.2 iOS embed spike: the same `Root` runs in a simulator via UIKit containment
  - tier: hard
  - host: Linux (edit), Mac (build and run)
  - files: `src/ui/app/src/mobile.rs`, `src/ui/app/src/lib.rs`, `src/ui/ios/project.yml`, `src/ui/ios/Host/App.swift`, `src/ui/ios/Host/Embedding.h`, `src/ui/ios/bootstrap.sh`, `src/ui/ios/UITests/SpikeTapTests.swift`, `src/ui/ios/.gitignore`, `openspec/changes/gpui-kit-ui-rewrite/design.md` (append "Spike result: iOS"), `docs/evidence/gpui-kit-ui-rewrite/2.2/*.png`
  - deps: none beyond 1.1 (reuses the pinned gpui set and the font assets of 2.1); iOS rustup targets `aarch64-apple-ios-sim`, `x86_64-apple-ios`, `aarch64-apple-ios` already installed on the Mac
  - reqs: REQ-UI-01, REQ-UI-03, REQ-UI-06
  - acceptance: following the gpui-mobile example (`App.swift`, `Embedding.h`, `project.yml` at rev `0b882efdac7f524e0bb0b1d4c886b2aa752f9f20`), `bootstrap.sh` builds the app staticlib for `aarch64-apple-ios-sim`, links it with `-lc++`, and `xcodegen` produces a project with bundle id `dev.radaiko.gourmetclient`, deployment target iOS 17.0; the simulator app shows the same root, fonts and counter as 2.1; the host uses `gpui_ios_set_embedded()`, callback registration, `gpui_ios_run_demo()` and one view controller for the app lifetime; a coordinate tap from an XCUITest increments the counter.
  - tests: `xcodebuild -scheme SnackPilot -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' CODE_SIGNING_ALLOWED=NO build`; boot a simulator, `xcrun simctl uninstall`, `install`, `launch`, `xcrun simctl io <udid> screenshot`; `xcodebuild test` runs `SpikeTapTests` (`@req REQ-UI-03` background/foreground via `simctl` keeps the same root: screenshot before and after).
  - check: `make check`; additionally `make mac-sync` and the Mac commands above via `ssh mac`
  - proof: ios workflow build (extended in 6.2) plus the simulator evidence in the handoff

- [x] 2.3 Desktop spike: the same `Root` runs in a macOS window
  - tier: normal
  - host: Linux (edit, best-effort compile), Mac (build and run)
  - files: `src/ui/app/Cargo.toml`, `src/ui/app/src/lib.rs`, `src/ui/app/src/bin/desktop.rs`, `openspec/changes/gpui-kit-ui-rewrite/design.md` (append "Spike result: Desktop"), `docs/evidence/gpui-kit-ui-rewrite/2.3/*.png`
  - deps: none beyond 1.1; desktop platform features of the pinned `gpui-pre`/`gpui-kit` are enabled by target-gated feature tables only
  - reqs: REQ-UI-01, REQ-UI-07
  - acceptance: `snackpilot-desktop` opens a window using `gpui_kit::application()` and mounts the same root and assets; the window is resizable with minimum width 360 and content centred up to 480 px; on Linux `cargo check -p snackpilot-app --bin snackpilot-desktop` is attempted and its result (pass, or missing system libraries) recorded without failing the gate.
  - tests: on the Mac `cargo build -p snackpilot-app --bin snackpilot-desktop` succeeds; `timeout 5 target/debug/snackpilot-desktop` exits 124 (still running) with no panic on stderr; `screencapture -x` screenshot saved if a GUI session exists, otherwise the handoff says so and the human verifies by eye.
  - check: `make check`; additionally the Mac commands above
  - proof: `ui` workflow compile check on the model crate; Mac run evidence

## 3. Model and platform bridge

- [x] 3.1 Model runtime, `Platform` trait and session flows
  - tier: hard
  - host: Linux
  - files: `src/ui/model/src/lib.rs`, `src/ui/model/src/runtime.rs`, `src/ui/model/src/state.rs`, `src/ui/model/src/platform.rs`, `src/ui/model/src/fake.rs`, `src/ui/model/src/session.rs`, `src/ui/model/src/errors.rs`, `src/ui/model/tests/session.rs`, `src/ui/model/tests/errors.rs`, `src/ui/model/tests/ffi_link.rs`
  - deps: none
  - reqs: REQ-SCR-05, REQ-SCR-06, REQ-SCR-09, REQ-PLT-01, REQ-PLT-06, REQ-UI-08
  - acceptance: `AppModel::new(storage_dir, Arc<dyn Platform>)` builds a `SnackPilotCore` with an analytics sink that forwards to `Platform::track`; a tokio runtime owned by the model runs core futures; `State` is readable and `subscribe(callback)` fires after each state change; `login`, `ventopay_login`, `gourmet_logout`, `ventopay_logout` and `attempt_auto_login` port `AppModel.swift` exactly: credentials saved before the attempt, Ventopay first without its own billing fetch, logout keeps credentials, demo credentials also log in Ventopay; `human_error` maps every `CoreError` variant to the German sentences of `AppModel.humanError`; `trait Platform` declares the calls of design D4 plus `capabilities()` (notifications, geofence) returning typed `Result`s; `FakePlatform` records calls. If the core's UniFFI-exported async methods cannot be called from plain Rust, stop and report the smallest core edit instead of changing `src/core`.
  - tests: `session.rs` logs in with the demo credentials against the real core and `FakePlatform` (no network): saves-before-validate (`@req REQ-PLT-01`), auto-login order and single billing fetch (`@req REQ-SCR-09`), logout keeps credentials (`@req REQ-SCR-05`, `@req REQ-SCR-06`), a failing `Platform` call yields a typed error and no panic (`@req REQ-PLT-06`); `errors.rs` has one case per `CoreError` variant; `ffi_link.rs` calls `core_version()` and awaits `fetch_menus` in demo mode (`@req REQ-UI-08`).
  - check: `make check`
  - proof: ui workflow (`make ui-check`)

- [x] 3.2 Model: menus, orders and billing state and actions
  - tier: normal
  - host: Linux
  - files: `src/ui/model/src/menus.rs`, `src/ui/model/src/orders.rs`, `src/ui/model/src/billing.rs`, `src/ui/model/src/format.rs`, `src/ui/model/src/state.rs`, `src/ui/model/src/lib.rs`, `src/ui/model/src/session.rs` (dispatcher-approved: visibility widening + sync_selected_day hook), `src/ui/model/tests/menus.rs`, `src/ui/model/tests/orders.rs`, `src/ui/model/tests/billing.rs`, `src/ui/model/tests/format.rs`
  - deps: none
  - reqs: REQ-SCR-01, REQ-SCR-02, REQ-SCR-03, REQ-SCR-09, REQ-UI-06
  - acceptance: port from `AppModel.swift`: cached-first loading, `sync_selected_day` and `nearest_day`, `prev_day`/`next_day`/`go_to_today`/`select_day`, `toggle`, `clear_pending`, `has_pending_changes`, submit with a `ProgressListener` that updates `order_progress`, the submit-button label logic of §3.1, category ordering `MENÜ I`..`UNKNOWN`, `dish` and `is_ordered` cross-reference, orders split and counts, confirm, single-flight `cancel_order` guard, multi-select batch cancel, cutoff via `is_ordering_cutoff`, billing month selection, source filter, descending merge, Gesamt/Belege/Zuschuss totals, forced reload, and `format_eur` / dates in `de-AT`. If `docs/requirements/` gives no example for a thousands separator in `format_eur`, stop and ask the human instead of guessing.
  - tests: demo-mode tests per scenario of REQ-SCR-01/02/03 (toggle then submit shows phases via a recording listener; batch cancel sends one core call; confirm-banner text for 1 and 2 orders; source-filter totals; empty month); `format.rs` cases from `04-ui-ux.md` §3.3/§4.7 (`€ 12,34`).
  - check: `make check`
  - proof: ui workflow (`make ui-check`)

- [x] 3.3 Model: preferences, reminders, location flow, geofence handling and diagnostics
  - tier: normal
  - host: Linux
  - files: `src/ui/model/src/prefs.rs`, `src/ui/model/src/flows.rs`, `src/ui/model/src/diagnostics.rs`, `src/ui/model/src/state.rs`, `src/ui/model/src/lib.rs`, `src/ui/model/src/session.rs` (dispatcher-approved: kv field, visibility, startup loads, geofence delegation), `src/ui/model/src/fake.rs` (dispatcher-approved: deny flags), `src/ui/model/tests/prefs.rs`, `src/ui/model/tests/flows.rs`
  - deps: none
  - reqs: REQ-UI-05, REQ-SCR-07, REQ-SCR-08, REQ-PLT-02, REQ-PLT-03, REQ-PLT-05, REQ-PLT-06
  - acceptance: theme preference and accent persist under the keys `theme_preference` and `accent_color`, and reminder prefs under `daily_reminder_enabled`, `daily_reminder_hour`, `daily_reminder_minute`, through the core `FileKv` in the model's storage dir, with defaults System, Orange, off (no stored time until first enable; picker default 11:00 per v1 — human decision 2026-11-07, the earlier "08:00" was a spec error); accent change calls `Platform::set_app_icon` only on an explicit user change; `apply_daily_reminder` asks the core for the command and runs it through `Platform::notify`, or sends `CancelPending("daily-order-reminder")` when off; `set_company_location_from_current_position` implements §3.8 / `AppModel` steps 1–3 with the exact German alerts and aborts; `clear_company_location` stops the geofence and clears the core location; `handle_geofence(event)` sets is-at-company, runs `geofence_commands` and refreshes the log; diagnostics activate (12 or 24 h), clear, and compute inactive/active/expired view state; reminder toggle denial leaves the switch off with "Berechtigung fehlt".
  - tests: `FakePlatform`-driven tests per scenario: permission refused (`@req REQ-PLT-03`), reminder denied (`@req REQ-PLT-06`), schedule then cancel same id (`@req REQ-PLT-02`), geofence Exit runs decisions (`@req REQ-PLT-03`), prefs round-trip and independence of theme and accent (`@req REQ-UI-05`), accent triggers icon call once (`@req REQ-PLT-05`), log state machine (`@req REQ-SCR-08`).
  - check: `make check`
  - proof: ui workflow (`make ui-check`)

- [x] 3.4 Android platform bridge: JNI to migrated Kotlin shims
  - tier: hard
  - host: Linux + Emulator
  - files: `src/ui/app/src/platform/mod.rs`, `src/ui/app/src/platform/android.rs`, `src/ui/app/src/lib.rs`, `src/ui/app/Cargo.toml`, `src/ui/android/app/src/main/AndroidManifest.xml`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/PlatformBridge.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/SecureCredentialStore.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/NotificationService.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/AlarmReceiver.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/LocationService.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/GeofenceBroadcastReceiver.kt`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/SnackPilotApp.kt`, `src/ui/android/app/build.gradle.kts`, `src/ui/android/app/src/main/res/**` (launcher icons and aliases are copied in 5.5, not here), `docs/evidence/gpui-kit-ui-rewrite/3.4/*.png`
  - deps: `jni`, `ndk-context` (android target-only, in `snackpilot-app`, only if 2.1 recorded that they are needed); Gradle: `com.google.android.gms:play-services-location:21.3.0`, `com.github.aptabase:aptabase-kotlin:0.0.8`, `androidx.core`, `org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0` at the versions already used in `src/android/app/build.gradle.kts`
  - reqs: REQ-PLT-01, REQ-PLT-02, REQ-PLT-03, REQ-PLT-04, REQ-PLT-06
  - acceptance: the Kotlin shims are moved from `src/android` without behaviour change except that `uniffi.snackpilot_core` types are replaced by small plain data classes at the JNI boundary; `android.rs` implements `Platform` over JNI; async results (permissions, one-shot fix) return through `external fun` entry points matched to request ids; the manifest keeps INTERNET, POST_NOTIFICATIONS, location permissions and both receivers, and the two channels `order-reminders` and `menu-updates` are created with the ids and names of `05-platform-services.md` §5.3; a debug-only intent extra exercises: save and read a credential, fire a notification, schedule one 3 s out, cancel it; every JNI call checks for a pending Java exception and returns a typed error.
  - tests: unit test of the argument/result marshalling in `src/ui/app/src/platform/android.rs` behind `cfg(test)` with a mock JNI-free adapter (`@req REQ-PLT-06`); on the emulator: `adb logcat` shows the credential round trip, a banner screenshot for the fired notification (`@req REQ-PLT-02`), the scheduled notification arriving and a cancelled one not arriving; `adb shell run-as dev.radaiko.gourmetclient cat shared_prefs/SecureStore.xml` shows key `key_v1-gourmet_username` with the §1.5 JSON shape (`@req REQ-PLT-01`, value redacted in the handoff).
  - check: `make check`; additionally `tools/devops/ui-android-build.sh` and `cd src/ui/android && ./gradlew :app:assembleDebug --no-daemon`
  - proof: android workflow build (extended in 6.2) plus emulator evidence

- [x] 3.5 iOS platform bridge: C function table to migrated Swift shims
  - tier: hard
  - host: Linux (edit), Mac (build and run)
  - files: `src/ui/app/src/platform/ios.rs`, `src/ui/app/src/platform/mod.rs`, `src/ui/app/src/lib.rs`, `src/ui/ios/Host/PlatformShims.h`, `src/ui/ios/Shims/KeychainStore.swift`, `src/ui/ios/Shims/NotificationService.swift`, `src/ui/ios/Shims/LocationService.swift`, `src/ui/ios/Shims/Analytics.swift`, `src/ui/ios/Shims/PlatformBridge.swift`, `src/ui/ios/Host/App.swift`, `src/ui/ios/project.yml`, `src/ui/ios/PrivacyInfo.xcprivacy`, `docs/evidence/gpui-kit-ui-rewrite/3.5/*.png`
  - deps: Swift package `Aptabase` `from: "0.3.4"` in `project.yml` (already used by the native app); no new Cargo dependency
  - reqs: REQ-PLT-01, REQ-PLT-02, REQ-PLT-03, REQ-PLT-04, REQ-PLT-06
  - acceptance: shims are moved from `src/ios/SnackPilot` unchanged in behaviour (Keychain service `app:no-auth`, AfterFirstUnlock; `UNUserNotificationCenter` with foreground banner; the 500 m `company` region with skip-if-active guard; `LocationService` singleton created at launch so background region relaunches deliver events); `PlatformBridge.swift` registers the function table at startup; `ios.rs` implements `Platform` over it with request-id completions; the location purpose strings and `INFOPLIST_KEY_*` entries of the native `project.yml` are carried over; no `UIBackgroundModes`, as before; a debug launch argument runs the credential, notification and schedule exercise.
  - tests: unit test of the callback-table marshalling in `ios.rs` using a Rust-side stub table (`@req REQ-PLT-06`); on the simulator: credential round trip through the shim read back via `security`-free app log (value redacted), a fired notification banner screenshot (`@req REQ-PLT-02`, `@req REQ-PLT-01`); geofence entry is device-only and is listed as a human check in the handoff.
  - check: `make check`; additionally `make mac-sync` and `xcodebuild … CODE_SIGNING_ALLOWED=NO build` plus simulator launch via `ssh mac`
  - proof: ios workflow build (extended in 6.2) plus simulator evidence
  - note: approved deviations — logger moved from `mobile.rs` into `ios.rs::install_stderr_logger`; simulator builds use ad-hoc signing (`CODE_SIGN_IDENTITY[sdk=iphonesimulator*] = "-"`) instead of `CODE_SIGNING_ALLOWED=NO` because unsigned simulator binaries fail keychain `SecItemAdd` with -34018 (verified on a fresh install; no entitlements file needed — Xcode strips profile-gated keys under ad-hoc signing); device builds keep signing off for the human/devops release flow. Added beyond the declared files: `UITests/NotificationPermissionTests.swift`, `a-exercise-log.txt`, `d-notification-center.jpg`. Human checks pending: physical-device geofence entry, device v1-credential takeover.
  - note: approved deviations — stderr logger moved from `mobile.rs` into `ios.rs::install_stderr_logger`; simulator builds use ad-hoc signing with **no** entitlements file (unsigned binaries hit keychain -34018; Xcode strips profile-gated entitlements under ad-hoc anyway — fresh-install SecItemAdd round trip verified on the simulator); `docs/devlibs-host-tests.md` documents the Linux X11 link shims. Human checks deferred: physical-device geofence entry and device v1-credential takeover.

- [x] 3.6 Desktop platform: OS keychain credentials and no-op services
  - tier: normal
  - host: Linux (tests with the mock keyring), Mac (real keychain round trip)
  - files: `src/ui/app/src/platform/desktop.rs`, `src/ui/app/src/platform/mod.rs`, `src/ui/app/src/bin/desktop.rs`, `src/ui/app/Cargo.toml`
  - deps: `keyring` (desktop targets only, in `snackpilot-app`; enable only the platform-native store features needed for macOS, and for Linux/Windows if the crate offers them without extra system libraries, else macOS only)
  - reqs: REQ-PLT-07, REQ-PLT-01, REQ-PLT-06
  - acceptance: `DesktopPlatform` stores and reads the four credential keys under service `app:no-auth` in the OS keychain; permissions and geofence report `Unavailable` as a typed result; `track` is a no-op; `Platform::capabilities()` reports no notifications so the Benachrichtigungen row is hidden (the row itself is built in 5.4); the desktop binary uses `DesktopPlatform`.
  - tests: unit tests with the `keyring` mock credential builder (`@req REQ-PLT-07`); on the Mac a one-off run of a small `examples/` or test-only binary saves, reads and deletes a dummy value `test-user`/`test-pass` and the handoff states the result.
  - check: `make check`; additionally `cargo test -p snackpilot-app --lib` on the Mac and `cargo check -p snackpilot-app --bin snackpilot-desktop` on Linux if system libs exist
  - proof: ui workflow compile of the model crate; Mac test run
  - note: Linux has no pkg-config/fontconfig, so the desktop binary check ran on the Mac instead (compiles clean); on Linux no keyring native store is enabled (would need libdbus) — the Linux desktop host is compile-only and tests use the keyring mock. Mac proof (one-shot LaunchAgent in the GUI session, plain SSH hits "User interaction is not allowed"): `save_user=true save_pass=true match_user=true match_pass=true delete_user=true delete_pass=true user_gone=true pass_gone=true`, values never logged. Added beyond the files line: `src/ui/app/examples/keychain_roundtrip.rs`, `src/ui/Cargo.lock`.

## 4. UI foundation

- [x] 4.1 Theme tokens, bundled assets and primitive components
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/theme.rs`, `src/ui/app/src/assets.rs`, `src/ui/app/src/components/mod.rs`, `src/ui/app/src/components/button.rs`, `src/ui/app/src/components/text_field.rs`, `src/ui/app/src/components/card.rs`, `src/ui/app/src/components/chip.rs`, `src/ui/app/src/components/switch.rs`, `src/ui/app/src/components/banner.rs`, `src/ui/app/src/components/spinner.rs`, `src/ui/app/src/gallery.rs`, `src/ui/model/src/tokens.rs`, `src/ui/model/tests/tokens.rs`, `docs/evidence/gpui-kit-ui-rewrite/4.1/*.png`
  - deps: none (gpui-kit `assets` feature enabled in `snackpilot-app`, already within the pinned dependency)
  - reqs: REQ-UI-05, REQ-UI-06
  - acceptance: `tokens.rs` (no gpui) holds the light and dark colour tokens and the five accent primaries from `04-ui-ux.md` §8 and `Theme.swift` / `SnackPilotTheme.kt`; `theme.rs` resolves System/Hell/Dunkel against the OS and exposes the §7 recipes (buttonPrimary, buttonDanger, cardSurface, bannerSurface, tintedBanner, inputField, fab) with the iOS glass versus flat Android/desktop split (`useFlatStyle`); components render the German strings of the spec in Inter; icons map to the §2 table, with substitutions listed in the handoff; a `gallery` screen (debug entry) shows every component in light and dark.
  - tests: `tokens.rs` compares every hex to a table copied from §8 (`@req REQ-UI-05`) and asserts the five accent labels; screenshots of the gallery on the emulator in light and dark, plus a successful iOS simulator build and desktop build (`@req REQ-UI-06`: umlauts visible).
  - check: `make check`; additionally `tools/devops/ui-android-build.sh` and `./gradlew :app:assembleDebug`; Mac build via `make mac-sync`
  - proof: ui workflow (token test), android workflow, emulator evidence
  - note: token table 31 rows pinned against §8 incl. glass{Success,Warning,Error}; rgba() tokens carry 0-255 channels (parser bug fixed: only alpha is 0-1). Kit `Button` Custom variant fades its fill to 20 % alpha and replays caller style over all states, so §7 fills/borders/shadows are painted by a wrapper `Div` (`shell()`); kit `Input` paints its own theme background and a bright dark-mode border — switched off via `bordered(false)`, §7 fill/border come from the wrapper; kit `Switch` label follows the kit global theme, so `switch()` draws its own label from `theme.text_primary`. Gallery debug hook (env var, launch args, Android system property) is `#[cfg(debug_assertions)]`-gated with release stubs. Android emulator screenshot impossible: gfxstream SIGSEGV in vulkan.ranchu.so regardless of app code (e-android-emulator-gpu-crash.txt); iOS-simulator captures (b/c + page-2 shots) carry the visual proof — the page knob (`--gallery-page=bottom`) makes the lower sections fit one screen. Desktop gallery hook launches clean on the Mac (f-desktop-hook.txt) but headless `screencapture` is blocked by TCC screen-recording consent — 4.2 must deliver the 768/1280 px evidence (needs a one-time consent click at the Mac or another capture path). §2 icon mapping is per-screen and lands with the 5.x screens; components take caller-supplied gpui-kit Lucide `Icon`s.

- [x] 4.2 App shell: root, tab bar, navigation stack, dialogs, overlays and safe areas
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/app.rs`, `src/ui/app/src/navigation.rs`, `src/ui/app/src/components/tab_bar.rs`, `src/ui/app/src/components/dialog.rs`, `src/ui/app/src/components/loading_overlay.rs`, `src/ui/app/src/components/toast.rs`, `src/ui/app/src/components/day_navigator.rs`, `src/ui/app/src/lib.rs`, `src/ui/app/src/mobile.rs`, `src/ui/app/src/bin/desktop.rs`, `src/ui/model/src/navigation.rs`, `src/ui/model/tests/navigation.rs`, `docs/evidence/gpui-kit-ui-rewrite/4.2/*.png`
  - deps: none
  - reqs: REQ-UI-03, REQ-UI-04, REQ-UI-07, REQ-SCR-04
  - acceptance: one `Root` hosts the four-tab shell and a push/pop stack with the back affordance of §3.5; the iOS floating glass pill (248×56, icons only, dot indicator) and the Android/desktop bottom bar with labels follow §2; the dialog component provides alert and confirm dialogs (§4.1) with destructive confirm labels; loading overlay (§4.2) and new-menu toast (§4.8) exist; safe-area insets come from the host; the desktop shell centres content at most 480 px; all four tabs show placeholder content from `AppModel` state.
  - tests: `navigation.rs` covers tab switch, re-tap no-op, push/pop and back from a pushed screen (`@req REQ-UI-04`); emulator screenshots of each tab and of an open dialog; iOS pill compiled and shown on the simulator; background and resume keeps the selected tab (`@req REQ-UI-03`).
  - check: `make check`; additionally Android build and Mac simulator build as in 4.1
  - proof: ui workflow, android workflow, emulator evidence
  - note: emulator captures remain impossible (vulkan.ranchu SIGSEGV, recorded in 4.1); iOS-simulator captures of tab0–3, dialog, toast and the background/resume pair carry the visual proof, and the Android release .so plus the Gradle debug APK build green. Desktop 768/1280: TCC consent is granted and the capture pipeline works, but the Mac window server currently returns NULL images for every capture API (diagnosed in `f-desktop-768-1280.txt`); launch is proven and CGWindowList confirms the 768×1312 window — PNGs deferred to a follow-up `render_to_image` capture binary. Review round 1 FAIL → fixed: dialog backdrop now `.occlude()` (§4.1), proven by XCUITest tap pair (`g-dialog-occlusion.txt`: control tap switches tab, same tap with dialog open leaves Menüs selected); review round 2 PASS. Advisories fixed: nav tests retagged `@req REQ-UI-04`, SHELL_MODEL comment corrected, toast hold text ~4.6 s (§4.8), 8 px tab-button hit slop (§2.1). Documented limitations: per-side pill border colours and outline+filled icon pairs (gpui / vendored-lucide limits). Earlier pre-review fixes: loading overlay occludes input, `Dialog::Confirm` takes confirm/cancel labels. Follow-ups: Android hardware Back → `Navigation::pop` and system-back dialog dismissal → Android host task; REQ-UI-07/REQ-SCR-04 stay screenshot-proved (no automated tests). Deviations accepted by review: `pub mod navigation;` in model lib.rs, `DialogOcclusionTests.swift` + `project.yml` outside `files:` (only input route reaching GPUI).

## 5. Screens

- [x] 5.1 Menus tab
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/screens/mod.rs`, `src/ui/app/src/screens/menus.rs`, `src/ui/app/src/components/menu_card.rs`, `src/ui/app/src/components/fab.rs`, `src/ui/app/src/components/swipe.rs`, `src/ui/app/src/app.rs`, `docs/evidence/gpui-kit-ui-rewrite/5.1/*.png`
  - deps: none
  - reqs: REQ-SCR-01
  - acceptance: the tab follows §3.1 with `DayNavigator`, grouped `MenuCard`s, ordered and pending states, refresh and progress banners, error and empty states, the FAB label forms, and the 50 px swipe-between-days gesture with edge resistance; ordering on a cutoff day is blocked; all logic is read from `AppModel` (3.2) and no state is duplicated in the view.
  - tests: model tests of 3.2 cover logic; screenshots on the emulator in demo mode: normal day, pending selection with FAB "Bestellen (1)", progress banner, cutoff day, empty state (`@req REQ-SCR-01`); simulator build compiles.
  - check: `make check`; additionally Android build and Mac simulator build
  - proof: android workflow, emulator evidence
  - note: iOS simulator build and launch verified (402×778 pt); the auth-gate capture is in 5.1/a-ios-menus-auth-gate.png. Filled states (normal day, pending "Bestellen (1)", progress banner, cutoff, empty) need a logged-in demo session: the login UI is 5.4 and the iOS host gets the shell model in 5.7, so those captures are deferred to 5.4/5.7. Android emulator captures stay impossible (vulkan.ranchu SIGSEGV, 4.1 record). Review round 1 fixed the checkmark disc placement and the |dx|>|dy| gesture claim.

- [x] 5.2 Orders tab
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/screens/orders.rs`, `src/ui/app/src/components/order_item.rs`, `src/ui/app/src/components/segment_tabs.rs`, `src/ui/app/src/screens/mod.rs`, `src/ui/app/src/app.rs`, `docs/evidence/gpui-kit-ui-rewrite/5.2/*.png`
  - deps: none
  - reqs: REQ-SCR-02
  - acceptance: the tab follows §3.2 and §4.6: segments with counts, confirm banner with plural text, `OrderItem` cards with dish lookup, cancel confirm dialog ("Bestellung stornieren", "Stornieren"/"Behalten"), dimmed in-flight card, multi-select mode with batch cancel, disabled cancel past the cutoff, empty states.
  - tests: screenshots on the emulator in demo mode after ordering via the Menus tab: upcoming list, confirm banner, cancel dialog, selection mode, past tab, empty state (`@req REQ-SCR-02`); logic covered by 3.2 tests; simulator build compiles.
  - check: `make check`; additionally Android build and Mac simulator build
  - proof: android workflow, emulator evidence
  - note: iOS simulator build/launch verified and the §3.2 auth gate captured (5.2/a-ios-orders-gate.png); filled states (upcoming list, confirm banner, cancel dialog, selection mode, past tab, empty) need the login UI (5.4) and the iOS shell model (5.7), as in 5.1. Android emulator captures stay blocked (vulkan.ranchu SIGSEGV). Review round 1 disabled the cancel button past the cutoff and added the `@req REQ-SCR-02` gate test (`cancel_enabled`).

- [x] 5.3 Billing tab
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/screens/billing.rs`, `src/ui/app/src/components/bill_card.rs`, `src/ui/app/src/components/pull_refresh.rs`, `src/ui/app/src/screens/mod.rs`, `src/ui/app/src/app.rs`, `docs/evidence/gpui-kit-ui-rewrite/5.3/*.png`
  - deps: none
  - reqs: REQ-SCR-03, REQ-UI-06
  - acceptance: the tab follows §3.3 and §4.7: three month tabs, Alle/Kantine/Automaten chips, summary bar with Zuschuss only when greater than zero, newest-first `BillCard` list, `de-AT` EUR amounts, spinner and inline error, pull-to-refresh forcing a fetch.
  - tests: screenshots on the emulator in demo mode: all sources, Kantine filter, month with no data, loading state (`@req REQ-SCR-03`); `de-AT` amounts compared to the 3.2 format tests; simulator build compiles.
  - check: `make check`; additionally Android build and Mac simulator build
  - proof: android workflow, emulator evidence
  - note: iOS simulator build/launch verified and the §3.3 auth gate captured (5.3/a-ios-billing-gate.png); month tabs, chips, summary, BillCard list, empty and loading states need the login UI (5.4) and the iOS shell model (5.7), as in 5.1/5.2. de-AT amounts come from the model's format_eur (covered by its tests). Android emulator captures stay blocked (vulkan.ranchu SIGSEGV). Review round 1 fixed the empty-state copy to "Keine Abrechnungsdaten für diesen Monat"; round 2 PASS. Follow-ups from review (non-blocking): `select_month`/`reload_billing` and the orders equivalents call `block_on` on the UI thread, so the loading spinner cannot paint during a fetch (same pattern in 5.2 — fix when the async host plumbing lands); badge letterSpacing 0.5 and one-line clamp on the Gourmet item description (§4.7) still missing in `bill_card.rs`.

- [x] 5.4 Settings tab and both login screens (text input proof)
  - tier: hard
  - host: Linux + Emulator; Mac build and simulator run
  - files: `src/ui/app/src/screens/settings.rs`, `src/ui/app/src/screens/login.rs`, `src/ui/app/src/components/text_field.rs`, `src/ui/app/src/components/settings_row.rs`, `src/ui/app/src/screens/mod.rs`, `src/ui/app/src/app.rs`, `src/ui/android/app/src/main/AndroidManifest.xml` (soft-input mode only), `src/ui/ios/Host/App.swift` (keyboard handling only), `docs/evidence/gpui-kit-ui-rewrite/5.4/*.png`
  - deps: none
  - reqs: REQ-SCR-04, REQ-SCR-05, REQ-SCR-06, REQ-PLT-01, REQ-PLT-07
  - acceptance: Einstellungen shows the rows and hints of §3.4 (Benachrichtigungen hidden when the platform has no notifications); `login.rs` is one parametrised screen for Kantine and Automaten with the differences of §3.5/§3.6; text fields accept typing, backspace, paste and secure entry with the soft keyboard on Android and iOS, and the keyboard does not cover the focused field; save-before-login, `Speichern...` state, success/failure/empty alerts, Abmelden keeping credentials, Datenschutz dialog. If typing cannot be made to work on either mobile platform, stop and report (design risk list) before 5.5.
  - tests: on the emulator type `demo` and `demo1234!` with `adb shell input text`, save, and screenshot the "Gespeichert" alert, then the Menus tab with demo data (`@req REQ-SCR-05`); wrong-credentials path against demo-mode `FakePlatform`-free core is covered by 3.1 tests; same typing proof on the simulator via XCUITest `typeText` (`@req REQ-SCR-06`); screenshots of Einstellungen with live hints (`@req REQ-SCR-04`).
  - check: `make check`; additionally Android build and Mac simulator build plus XCUITest run
  - proof: android workflow, ios workflow, emulator and simulator evidence
  - note: iOS simulator typing proof passes (LoginTypingTests: system-keyboard taps, keyboard rises on focus and never covers the password field, pixel-diff typing/backspace, secure dots, §3.5 failure alert); `WindowExt::has_focused_input` + `gpui_mobile::show/hide_keyboard` drive the keyboard from focus, `Input.masked(true)` is the secure entry, `--detail=` opens a credential screen. Gaps carried forward: paste not exercised (XCUITest cannot drive the paste menu on a GPUI field; physical-device follow-up); no Android typing run (emulator vulkan.ranchu SIGSEGV, as 5.1–5.3; only cargo ndk check backs Android); on iOS before 5.7 save-before-login, the success alert, the authenticated Abmelden look and the credential prefill rest on code/model tests. Review r1 fixed the danger Abmelden recipe and added `AppModel::saved_credentials` for the §3.5 prefill.

- [x] 5.5 Appearance screen and alternate icons
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/screens/appearance.rs`, `src/ui/app/src/screens/mod.rs`, `src/ui/android/app/src/main/AndroidManifest.xml` (five launcher aliases), `src/ui/android/app/src/main/res/**` (copy of the five launcher icon sets and `ic_launcher_background` values from `src/android/app/src/main/res`), `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/PlatformBridge.kt` (alias toggle), `src/ui/ios/Assets.xcassets/**` (copy of the five AppIcon sets), `src/ui/ios/Shims/AppIconService.swift`, `src/ui/ios/project.yml` (alternate icon build settings), `docs/evidence/gpui-kit-ui-rewrite/5.5/*.png`
  - deps: none
  - reqs: REQ-SCR-07, REQ-UI-05, REQ-PLT-05
  - acceptance: the screen follows §3.7 (three Design options, five light-mode swatches with checkmark ring); both settings apply instantly and persist across a restart; picking an accent calls the platform icon switch, which toggles the Android launcher alias and the iOS alternate icon; startup never changes the icon.
  - tests: emulator screenshots: Hell, Dunkel, a non-default accent, then relaunch showing the choice persisted (`@req REQ-UI-05`, `@req REQ-SCR-07`); `adb shell pm` output shows exactly one enabled launcher alias after switching to Ozean (`@req REQ-PLT-05`); simulator build compiles with the alternate icon names.
  - check: `make check`; additionally Android build and Mac simulator build
  - proof: android workflow, emulator evidence
  - note: iOS simulator screenshot of §3.7 (5.5/a-ios-appearance.png); Android Gradle debug APK builds with the five launcher aliases and the copied icon res, iOS builds with the five AppIcon sets and ASSETCATALOG alternate names. Runtime alias/alternate-icon switching cannot be observed here (emulator vulkan.ranchu SIGSEGV; iOS icon switch shows a system alert simctl cannot drive); the model's set_accent_color is the single icon call site with no startup reconciliation, and a `@req REQ-PLT-05` test pins accent raw values to the alias suffixes. Review r1 fixed the alias mapping that fell back to Orange for every accent.

- [x] 5.6 Notifications screen
  - tier: normal
  - host: Linux + Emulator; Mac build
  - files: `src/ui/app/src/screens/notifications.rs`, `src/ui/app/src/components/time_chips.rs`, `src/ui/app/src/screens/mod.rs`, `src/ui/app/src/app.rs` (dispatch, alert surfacing), `src/ui/model/src/diagnostics.rs` (log email export/send), `src/ui/model/src/session.rs` (take_location_alert, open_app_settings), `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/PlatformBridge.kt` (open app settings, mail compose; already present from 3.4), `src/ui/ios/Shims/PlatformBridge.swift` (open settings, mail compose; already present from 3.5), `docs/evidence/gpui-kit-ui-rewrite/5.6/*.png`
  - deps: none
  - reqs: REQ-SCR-08, REQ-PLT-02, REQ-PLT-03, REQ-PLT-06
  - acceptance: the screen follows §3.8: Bestell-Erinnerung switch and twelve chips 11:00–13:45, Android-only background-activity hint banner, company-location button with the exact alerts and the "Standort entfernen" state, and the three log states with 12/24 h activation, status text with de-AT expiry, "Log per E-Mail senden" opening a mail composer to `aiko@spitzbub.app` with the subject and body of §3.8, and "Log verwerfen".
  - tests: emulator screenshots: switch on with chips, permission-denied alert, location saved state via the debug saved-location argument, log inactive/active/expired states (`@req REQ-SCR-08`); model tests of 3.3 cover the logic; simulator build compiles.
  - check: `make check`; additionally Android build and Mac simulator build
  - proof: android workflow, emulator evidence
  - note: iOS simulator capture of §3.8 (5.6/a-ios-notifications.png: sections, switch, location button, 12/24 h log buttons); `make check` includes the new `@req REQ-SCR-08` email-format test. Gaps: the 12 time chips and the Android-only background hint only render while the reminder is enabled (permission dialog + tap not drivable via simctl), log active/expired states need a time-boxed run, and the mail composer/Android hint were not exercised on a device (emulator SIGSEGV). Review r1 reduced the location status to the exact "Firmenstandort gesetzt" copy and added the information/mail icons.

- [x] 5.7 Bootstrap: cached start, auto-login, broadcast banner, foreground refresh and geofence restore
  - tier: normal
  - host: Linux + Emulator; Mac build and simulator run
  - files: `src/ui/app/src/app.rs`, `src/ui/app/src/mobile.rs`, `src/ui/app/src/bin/desktop.rs`, `src/ui/app/src/platform/ios.rs` (iOS model install + bootstrap), `src/ui/app/src/components/broadcast_banner.rs`, `src/ui/app/src/components/mod.rs`, `src/ui/model/src/lifecycle.rs`, `src/ui/model/src/lib.rs`, `src/ui/model/src/state.rs` (broadcast fields), `src/ui/model/src/session.rs`, `src/ui/model/tests/lifecycle.rs`, `src/ui/android/app/src/main/java/dev/radaiko/snackpilot/PlatformBridge.kt` (lifecycle callbacks; no change needed), `src/ui/ios/Host/App.swift` (lifecycle callbacks; no change needed), `docs/evidence/gpui-kit-ui-rewrite/5.7/*.png`
  - deps: none
  - reqs: REQ-SCR-09, REQ-UI-03, REQ-PLT-03, REQ-PLT-04
  - acceptance: launch order follows REQ-SCR-09: cached snapshots published first, `app.launched` tracked, broadcast fetched, notification permission requested, company geofence restored idempotently, saved credentials auto-logged-in (Ventopay first, one billing fetch), then the saved-credentials refresh; foreground return calls `on_foreground` which refreshes orders, billing and broadcast under a re-entrancy guard; the broadcast banner is dismissible and returns on the next foreground.
  - tests: `lifecycle.rs` with `FakePlatform` and demo core: cache-first before any fetch, single billing fetch, foreground re-entrancy guard, banner dismiss/reappear (`@req REQ-SCR-09`), geofence start called once across two launches (`@req REQ-PLT-03`), `app.launched` forwarded (`@req REQ-PLT-04`); emulator and simulator: relaunch screenshots with cached data, background and resume keeps the root (`@req REQ-UI-03`).
  - check: `make check`; additionally Android build and Mac simulator build and run
  - proof: ui workflow, android workflow, emulator and simulator evidence
  - note: iOS simulator launch with the session model installed shows the demo Menus day (5.7/a-ios-menus-after-bootstrap.png); this also unblocks the filled-state captures deferred from 5.1–5.6. `bootstrap()` implements the REQ-SCR-09 order (cache first, app.launched, broadcast, permission, idempotent geofence restore via a persisted marker, auto-login). Foreground refresh is driven by the shell's window-activation watcher, so every host calls `on_foreground`; the re-entrancy flag clears via RAII. Tests cover cache-first, one billing fetch on auto-login, geofence-once, banner dismiss/reappear and the re-entrant drop. Review r1 wired the foreground watcher and the guard. Gaps: iOS foreground uses the same watcher (not a host callback); Android device run impossible (emulator SIGSEGV); `on_foreground` blocks the render thread during its fetches.

## 6. Cutover

- [ ] 6.1 Delete the native UIs and move the hosts into place
  - tier: normal
  - host: Linux, then Mac
  - files: `src/ios/**` (delete), `src/android/**` (delete), `src/core/scripts/build-apple-xcframework.sh` (keep), `src/core/scripts/build-android-jni.sh` (keep), `tools/devops/run-ios.sh`, `tools/devops/run-android.sh`, `tools/devops/archive-ios.sh`, `tools/devops/ship.sh`, `tools/devops/android-keystore.sh`, `tools/devops/doctor.sh`, `Makefile`, `src/ui/android/keystore.properties.example`, `src/ui/android/README.md`, `src/ui/ios/README.md`
  - deps: none
  - reqs: REQ-UI-09, REQ-UI-01
  - acceptance: before deleting, `git diff --stat` and a checklist confirm every asset and Gradle/Xcode setting needed (versionCode 35, versionName/MARKETING_VERSION 2.1.0, release signing via `keystore.properties`, targetSdk 36, minSdk 29, iOS 17.0, `ITSAppUsesNonExemptEncryption`, `TARGETED_DEVICE_FAMILY 1`, privacy manifest) already exists in `src/ui/ios` and `src/ui/android`; then `src/ios` and `src/android` are removed; devops scripts and Makefile targets (`ios-run`, `android-run`, `ios-archive`, `android-keystore`, `ship`) point at `src/ui/ios` and `src/ui/android` with unchanged behaviour; `grep -R "SwiftUI\|androidx.compose\|uniffi.snackpilot_core" src` finds nothing outside `src/core`; bundle id and application id are unchanged.
  - tests: `make check`; `tools/devops/test-lib.sh` still passes; fresh builds: Android `./gradlew :app:assembleDebug` + install on a wiped emulator + demo login screenshot, iOS simulator build + install on a fresh simulator + demo login screenshot (`@req REQ-UI-09`, `@req REQ-UI-01`).
  - check: `make check`; additionally `make doctor`, Android and Mac builds as above
  - proof: android and ios workflows (after 6.2), final emulator and simulator evidence
  - note: src/ios and src/android deleted (87 files); devops scripts, Makefile and README repointed to src/ui/ios|android; `make check`, test-lib and bash -n green; Mac gradle clean assembleDebug and iOS xcodebuild green. Review r1 carried the shipped v1 settings into the hosts: minSdk 29, versionCode 35, versionName 2.1.0, keystore.properties-driven release signing; iOS MARKETING_VERSION 2.1.0, CURRENT_PROJECT_VERSION 35, ITSAppUsesNonExemptEncryption NO, TARGETED_DEVICE_FAMILY 1 (verified in the built Info.plist). keystore.properties and *.jks are gitignored. Remaining: CI workflow paths and stale old-path mentions are task 6.2; the wiped-emulator install stays blocked by the recorded vulkan.ranchu SIGSEGV.

- [ ] 6.2 CI, docs and process text
  - tier: normal
  - host: Linux
  - files: `.forgejo/workflows/ios.yml`, `.forgejo/workflows/android.yml`, `.forgejo/workflows/ui.yml`, `docs/requirements/08-gpui-desktop.md` (new file only), `AGENTS.md`, `README.md`, `openspec/config.yaml`, `tools/devops/README.md`, `openspec/changes/gpui-kit-ui-rewrite/design.md` (append measured binary sizes and startup times)
  - deps: none
  - reqs: REQ-UI-01, REQ-UI-07, REQ-UI-09
  - acceptance: `ios.yml` builds from `src/ui/ios` and keeps the simulator smoke test and screenshot upload; `android.yml` builds Rust for the Android targets with `cargo-ndk`, then `src/ui/android` debug APK, keeps lint as is, and its path filters include `src/ui/**`; `ui.yml` also compiles `snackpilot-app` for the host where the runner has the libraries; `core.yml` is unchanged; `docs/requirements/08-gpui-desktop.md` records the desktop addendum, the supersession of "iOS and Android only", the accessibility regression, and the measured sizes; `AGENTS.md` and `README.md` describe the single Rust UI, the `src/ui` layout, `make ui-check`, and still contain the GourmetClient credit link; `openspec/config.yaml` context line says "gpui-kit UI over a shared Rust core for iOS, Android and desktop"; no existing file under `docs/requirements/` is modified.
  - tests: `openspec validate --all --strict --no-interactive` (`make spec-lint`); `git diff --name-only` shows only the listed files; workflow YAML is syntax-checked with `python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .forgejo/workflows/*.yml`.
  - check: `make check`
  - proof: first pipeline runs of the `ios`, `android` and `ui` workflows on the merge request
