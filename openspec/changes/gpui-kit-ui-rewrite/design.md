# Design: gpui-kit UI rewrite

See `proposal.md` for motivation, scope, assumptions and granted approvals. Requirement ids (`REQ-UI-*`, `REQ-SCR-*`, `REQ-PLT-*`) are defined in the three delta specs.

## Context

- `src/core` is a standalone crate (`snackpilot-core`, edition 2021, own `Cargo.lock`, crate types `lib`, `cdylib`, `staticlib`). `make check` runs fmt, clippy, tests and spec-lint there only. Platform builds are separate Forgejo jobs: `ios.yml` on the MacBook runner, `android.yml` on the android runner.
- The core already exposes everything the UI needs as a Rust API: `SnackPilotCore` (`src/core/src/ffi.rs`, about 36 methods, async methods use tokio), the snapshots in `domain`, `storage::{Kv, FileKv}`, and the notification decision functions. `AnalyticsSink` and `ProgressListener` are plain Rust traits. A Rust UI can call all of it directly; UniFFI is only needed by Swift and Kotlin.
- The native shells today hold (a) UI, (b) an `AppModel` / `AppViewModel` that republishes core snapshots and orchestrates flows (login, auto-login, submit, location setup, geofence handling, reminder apply), and (c) OS services. Only (a) is replaced; (b) is ported to Rust; (c) stays native behind a bridge.
- Demo mode (`demo`/`demo1234!`) serves complete offline data from the core, so model and UI behaviour can be tested and screenshotted without any live host.
- gpui-kit mobile is experimental: one GPUI view per app lifetime, host calls `gpui_kit::init(cx)` and mounts one `component::Root`, fonts and icons must be registered by the app. The documented iOS path embeds through UIKit containment. Android is an undocumented NativeActivity path.

## Goals / Non-Goals

Goals: one Rust UI; behaviour parity with `04-ui-ux.md`; platform services keep working with the same stored credentials; each risky assumption proven by a spike before screens migrate; a model layer testable by `make check` on Linux.

Non-goals: redesign; new features; changes to the core, to scraping, or to cache formats; removing UniFFI from the core; iOS device signing and App Store delivery (human step); background tasks v1 had but the shells never built; tablet layouts; Linux and Windows polish.

## Decisions

### D1. Two crates in a new `src/ui` workspace; core stays standalone

`src/ui/Cargo.toml` is a workspace with `model` (`snackpilot-model`) and `app` (`snackpilot-app`). Both depend on `snackpilot-core` by path (`../../core`). `snackpilot-model` has no gpui dependency. `snackpilot-app` holds views, theme, assets and the entry points (`lib` + `staticlib` + `cdylib` for mobile, a `[[bin]]` for desktop).

Why: `make check` must stay fast and runnable on any Linux box. If gpui sat in `make check`, every contributor would need Wayland/X11 and graphics libraries. Splitting lets all logic (state, flows, formatting, error mapping, day navigation) be tested in `make check`, while rendering is proven by platform jobs and screenshots.
Alternatives: one flat crate (rejected: pulls gpui into `make check`); a root workspace including core (rejected: moves `src/core/Cargo.lock`, which breaks `core.yml` and `android.yml` cache keys and the "core untouched" promise).

`make check` gains `ui-check` (fmt, clippy `-D warnings`, `cargo test -p snackpilot-model`, `cargo test --test pins`). The Makefile edit is task 1.1.

### D2. The UI links the core as a Rust library, not through UniFFI

`snackpilot-model` calls `SnackPilotCore::new(CoreConfig, Some(sink))` and its methods directly. Futures run on a runtime the model owns (`tokio` multi-thread, started once). Results are published into a `State` behind a mutex and a `subscribe(callback)` hook; `snackpilot-app` adapts that hook to a gpui `Entity` notification on the UI thread.
Why: it deletes the whole Swift/Kotlin codegen path from the UI's critical path and keeps one call convention. The core compiles as `rlib` for the app staticlib; its `cdylib`/`staticlib` types stay (proposal assumption 7), so `core.yml` still builds bindings until a follow-up.
Expected core diff: none. Task 3.1 must confirm that `uniffi::setup_scaffolding!()` and the `#[uniffi::export]` async methods link into a plain Rust consumer; if they do not, the task stops and lists the smallest core edit (named in `files:`) for the human.

### D3. Port `AppModel` / `AppViewModel` to `snackpilot-model`

State and actions mirror `AppModel.swift`: session flags, snapshots, selected day, pending changes, progress phase, billing month, prefs (theme, accent, reminder), location flow, geofence handling, diagnostics. Human-readable German error mapping (`humanError`) moves to Rust with a test per `CoreError` variant. UI prefs (`theme_preference`, `accent_color`, `daily_reminder_*`) are stored through the core's public `FileKv` in the same `storage_dir`, under those key names, so no platform preference store is needed and desktop works the same.
Tests use demo credentials against the real core, so no network is involved and no real host is reached.

### D4. Platform bridge is a Rust trait with three implementations

`trait Platform` in `snackpilot-model`: credentials get/save/clear; `notify(NotificationCommand)`; notification permission request; location authorisation, one-shot fix, geofence start/stop/is-monitoring; `track(event, props)`; `set_app_icon(accent)`; `open_app_settings`; `compose_email`; `capabilities()` (whether notifications and geofence exist, so desktop hides the Benachrichtigungen row). Inbound calls into the model: `on_geofence_event`, `on_foreground`, `on_background`.
Implementations: `FakePlatform` (tests), `DesktopPlatform` (`keyring` for credentials, no-ops elsewhere), and per mobile host:
- iOS: Swift shims (migrated `KeychainStore`, `NotificationService`, `LocationService`, `Analytics`, `AppIconService`) register a table of C function pointers plus a context at startup, as the gpui-mobile example registers its app callbacks; async results return through completion callbacks keyed by request id.
- Android: Kotlin shims (migrated `SecureCredentialStore`, `NotificationService`, `AlarmReceiver`, `LocationService`, `GeofenceBroadcastReceiver`, `SnackPilotApp`) are called through the `jni` crate; results return via `external fun` entry points. The JavaVM and Activity handle come from `ndk-context` if gpui-mobile does not already provide them (task 2.1 records which).
Why not UniFFI foreign traits: a second UniFFI scaffolding in the app library and a Swift plus Kotlin codegen step would keep the toolchain this change wants to remove. Revisit only if task 3.4 or 3.5 hits a hard blocker.
The credential coordinates (REQ-PLT-01) are not re-implemented: the existing Swift and Kotlin code moves as is, so takeover from v1 keeps working.

### D5. Host projects live under `src/ui`; native projects are deleted last

`src/ui/ios` (xcodegen `project.yml`, bundle id `dev.radaiko.gourmetclient`) and `src/ui/android` (Gradle, application id `dev.radaiko.gourmetclient`). The old `src/ios` and `src/android` keep building in CI until task 6.1, so there is always a shipping fallback. Because both hosts share one id, a test device or simulator must uninstall one before installing the other; every Mac and emulator step starts with an uninstall.
Asset migration (alternate icons, permission strings, manifest permissions and aliases, privacy manifest) happens in the task that first needs each asset (3.4, 3.5, 5.5), and 6.1 only deletes.

### D6. Mac builds use rsync of the worktree, not a second mutagen session

The mutagen alpha path is the main checkout. This work runs in a worktree outside it. `tools/devops/sync-mac.sh` (task 1.2) rsyncs the worktree to `mac:~/vm-sync/SnackPilot-gpui/` with `--delete`, excluding `.git`, `target`, `build`, `DerivedData`, `*.xcodeproj` and Gradle caches, and protecting those excluded paths from deletion on the remote so the Mac's cargo cache persists. Remote commands run as `ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH; cd ~/vm-sync/SnackPilot-gpui/src/ui && …'`. It never touches `~/vm-sync/SnackPilot`.
Why: rsync is stateless and scriptable by a coder subagent; a second mutagen session needs daemon configuration outside the repository and can conflict with the first.
Simulator builds use `CODE_SIGNING_ALLOWED=NO`; nothing here needs the locked keychain. Device builds and archives stay human-only.

### D7. Assets

Fonts: Inter Regular, SemiBold, Bold (SIL OFL) in `src/ui/app/assets/fonts/` with the licence file, registered with `cx.text_system().add_fonts(...)` before the root mounts. Icons: gpui-kit `assets` feature with an `AssetSource`. Theme tokens are copied from `Theme.swift` and `SnackPilotTheme.kt` into one Rust module and covered by a test that compares each hex against the table in `04-ui-ux.md` §8.

### D8. Verification matrix

| Task kind | Host | Evidence |
| --- | --- | --- |
| model, bridge trait, pins | Linux | `make check` (includes `ui-check`) |
| Android build and screens | Linux + `headless` AVD | Gradle build, `adb exec-out screencap`, logcat |
| iOS build and screens | Mac via `ssh mac` | `xcodebuild` simulator build, `xcrun simctl io … screenshot`, simulator log |
| desktop | Mac (run), Linux (compile if system libs exist) | build, 5 s run without panic, screenshot if a GUI session exists |
Screenshots go to `docs/evidence/gpui-kit-ui-rewrite/<task-id>/`, show demo data only, and are named in the task handoff. No real credential or live response is ever captured.
Unit tests tag requirements with a `// @req REQ-…` comment on the test function. A UI task covers its requirement with a model-level test where logic exists plus a named screenshot per scenario; gpui rendering itself has no headless unit test.

### D9. Layout sketch

```
src/ui/
  Cargo.toml  Cargo.lock  rust-toolchain.toml (only if gpui needs a newer rustc)
  model/   src/{lib,state,session,menus,orders,billing,prefs,platform,flows,errors}.rs  tests/
  app/     src/{lib,mobile,theme,assets,components/*,screens/*}.rs  src/bin/desktop.rs  assets/
  ios/     project.yml  Host/{App.swift,Embedding.h}  Shims/*.swift  Assets.xcassets
  android/ settings.gradle.kts  app/…  Kotlin shims  res/
```

### D10. Spike gates

2.1, 2.2 and 2.3 each end with a written "Spike result" section appended to this file (what worked, versions, commands, screenshots). A failed gate means: stop dispatching, report the evidence, and let the human choose between fixing upstream, pinning a different rev, or abandoning the change. Nothing after 2.3 is built on an unproven assumption. Gate criteria: Android: app starts on the `headless` AVD, shows a gpui-kit `Root` with German text in a bundled font and responds to a tap, screenshot taken. iOS: the same on a simulator via UIKit containment. Desktop: window opens and renders the same root.

### D11. Lifecycle and single view

The Android host Activity declares `configChanges` for orientation, screen size, keyboard and UI mode so it is never recreated, and the portrait lock from `05-platform-services.md` §7 stays. On iOS the host owns one view controller for the app lifetime. `on_foreground` triggers the REQ-SCR-09 refresh; `on_background` flushes nothing (the core persists on write).

## Risks / Trade-offs

- **Experimental mobile support, undocumented Android path** → gated spikes (D10); native UIs stay until 6.1.
- **gpui text input and IME on mobile** (login fields, soft keyboard, secure entry) may be incomplete → task 5.4 proves typing on both platforms; if unusable the human is told before cutover (task 6.1 depends on it).
- **Accessibility and VoiceOver/TalkBack** are not provided by gpui → recorded as a regression in `08-gpui-desktop.md` (task 6.2); not solved here.
- **Binary size and startup time** grow with wgpu and fonts → measured and recorded in 6.2; no budget is set.
- **Mixed `gpui-pre` versions break types** → REQ-UI-02 pin test.
- **Rust toolchain**: core says `rust-version = 1.78`; gpui-pre may need newer → task 1.1 records the required rustc and adds `src/ui/rust-toolchain.toml` only if needed. Core is built by the same toolchain; `make check` proves it still compiles.
- **Linux gpui build needs system libraries** → only the model crate is in `make check`; the app crate is compiled on Linux for Android targets only.
- **Same bundle id for old and new hosts** → uninstall steps (D5).
- **Large change** → spikes first; tasks are PR-sized; the human may stop after any group.

## Migration Plan

1. Groups 1–2: foundation and spikes, both hosts side by side with the native apps.
2. Groups 3–5: model, bridge, theme, screens; each screen merged with its evidence.
3. Group 6: delete native UIs, update CI and docs, run the full matrix once on a fresh Android emulator and a fresh iOS simulator with demo data.
4. Rollback: before 6.1 the native apps are untouched and still shipped; after 6.1 revert the merge commit.
5. Release builds, signing, and store submission remain human steps (`make ship`, `make ios-archive`), whose scripts are updated in 6.2.

## Open Questions

None blocks the specs. Deferred decisions are listed in the proposal assumptions; the questions put to the human are repeated in the handoff report.

## Spike result: Android (task 2.1)

**Verdict: PASS (go).** A gpui-kit `Root` runs on the `headless` emulator: one
`gpui_kit::component::Root` mounted via `gpui_kit::init(cx)` (never
`application()`), showing "Menüs / Bestellungen / Abrechnung / Einstellungen"
and a counter button in bundled Inter; `adb shell input tap` increments the
counter (evidence `docs/evidence/gpui-kit-ui-rewrite/2.1/counter-0.png` and
`counter-1.png`); the app stays alive past 10 s with no `FATAL EXCEPTION` and
exactly one Root/init log line.

Toolchain: rustc 1.99.1, cargo-ndk 4.1.2, NDK `30.0.16248370`, Gradle wrapper
9.4.1, JDK 17 (Temurin unpacked to `~/jdk` — no sudo available for a system
install). Emulator AVD `headless` (android-36 google_apis x86_64) launched with
`-gpu guest`; wgpu selects the `llvmpipe (LLVM 21.0.0)` Vulkan adapter.

Commands: `bash tools/devops/ui-android-build.sh --release` (exports
`RUST_FONTCONFIG_DLOPEN=on` and passes `--platform 31`, as the example's
`.cargo/config.toml` does) then `cd src/ui/android && ./gradlew
:app:assembleDebug --no-daemon` with `ANDROID_HOME`/`JAVA_HOME` set.

Findings:

- **Release builds are mandatory on this emulator.** A debug build enables
  `InstanceFlags::DEBUG` via `from_build_config()` (wgpu-types 29.0.4), so wgpu
  calls `vkSetDebugUtilsObjectNameEXT`, which SIGSEGVs inside the emulator's
  `vulkan.ranchu.so`. The pinned gpui-mobile example's own debug APK crashes
  identically (`example-*.png` show the pinned example running and its taps
  working once built in release), so this is an emulator/wgpu interaction, not
  an app defect. Release libs are 31 MB per ABI (551 MB debug).
- **`jni` and `ndk-context` are not needed** in the app crate — the example at
  rev `0b882efd` needs only `android-activity` (native-activity) and
  `android_logger`, same as here.
- cargo-ndk copies dependency dylibs (`libgpui_mobile-*`, `libsnackpilot_core`)
  into jniLibs, but the app cdylib links them statically (`llvm-readelf -d`
  shows only system DT_NEEDED entries), so the build script prunes everything
  but `libsnackpilot_app.so` (APK 66 MB). Gradle caches the jniLibs merge —
  run `:app:clean` after the prune if libs were deleted in place.
- Host is a plain `NativeActivity` Java subclass with the framework fullscreen
  theme, no androidx or res files; `android.app.lib_name=snackpilot_app`;
  `configChanges` covers orientation/screenSize/keyboard/uiMode (D11).
- Inter is embedded with `include_bytes!` and registered via
  `cx.text_system().add_fonts(...)`; the kit theme `font_family` is set to
  "Inter" after `Theme::change(Light)`.
- Touch works end to end: gpui-mobile synthesizes MouseDown+MouseUp at the
  down position on a tap (within an 8 px slop), and kit `Button::on_click`
  fires. Taps must target the real device-pixel bounds (scale 2.625) —
  screenshots are downscaled when viewed, so locate widgets by pixel analysis.
- Caveat: this emulator is shared and the real SnackPilot app (versionCode 37)
  was repeatedly reinstalled under the same package by a parallel session; the
  acceptance sequence must run uninstall → install → start → tap → screencap
  in one adb session and re-check `mCurrentFocus`.
- Scope note: `src/ui/Cargo.lock` churns with this task because `snackpilot-app`
  gained its Android deps (`android-activity` native-activity feature,
  `android_logger`); the lock is not in the task's `files:` but follows from the
  listed `src/ui/app/Cargo.toml`. Build outputs (`app/build/`, `.gradle/`,
  `jniLibs/`) are gitignored via `src/ui/.gitignore`.
