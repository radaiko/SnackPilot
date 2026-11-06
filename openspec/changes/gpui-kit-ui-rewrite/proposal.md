# Proposal: One gpui-kit UI for iOS, Android and desktop

domain: architectural

## Why

SnackPilot has two hand-written UIs (about 2,600 lines SwiftUI, about 2,760 lines Compose) over one Rust core, and no desktop app. Every screen change is made twice and the two drift. The human wants one Rust UI code base, built with the gpui-kit framework and its experimental mobile support, that runs on iOS, Android and desktop. The business logic already lives in `src/core`, so the UI is the only part left to unify. This is a spike-first, full-replacement rewrite on branch `spec/gpui-kit-mobile-rewrite`; the native UIs are deleted at the end, not kept in parallel.

## What Changes

- **New** `src/ui/` Cargo workspace with two crates: `snackpilot-model` (no gpui; app state, actions, platform trait; unit-testable on Linux) and `snackpilot-app` (gpui-kit views, theme, assets, entry points for iOS, Android and desktop).
- **New** host projects `src/ui/ios` (xcodegen, thin Swift host + platform shims) and `src/ui/android` (Gradle, NativeActivity host + Kotlin platform shims).
- **New** desktop target (macOS verified; Linux/Windows compile best-effort). The v1 "desktop dropped" decision in `docs/requirements/` is superseded for this change only.
- **New** platform bridge: a Rust trait covering credential storage, notification delivery, location and geofence, analytics, app-icon switching and mail. Swift/Kotlin keep implementing the OS calls; only the UI moves.
- **Reused unchanged** `src/core`. The UI links it as a Rust library (not via UniFFI). Expected core diff: none. Any core edit a task needs must be named in that task's `files:` and called out in its handoff.
- **BREAKING (internal)** `src/ios/SnackPilot/*.swift` views and `src/android/.../ui/*.kt` are removed at cutover. Bundle id and package `dev.radaiko.gourmetclient` do not change.
- **Spike gates**: tasks 2.1 (Android), 2.2 (iOS) and 2.3 (desktop) are go/no-go. If a gate fails, the change stops and the human decides; the native UIs remain the shipping UI.

### Approvals granted by this change

AGENTS.md requires explicit approval for these edits. Approving this change grants them for exactly the items listed here and nothing else:

- **Dependency manifests** (`Cargo.toml`, `Cargo.lock`) under `src/ui/` only, for exactly these crates: `gpui-pre =0.3.4`, `gpui-kit` (git rev `7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7`), `gpui-pre-mobile` (git rev `0b882efdac7f524e0bb0b1d4c886b2aa752f9f20`), `jni`, `ndk-context`, `keyring`, `tokio`, plus the core's own existing deps for `snackpilot-model` where needed (`serde`, `serde_json`, `chrono`, `chrono-tz`, `thiserror`). Task `deps:` lines are the authority; a dependency not named in a task's `deps:` is not approved. `src/core/Cargo.toml` and `src/core/Cargo.lock` are not touched.
- **CI config**: `.forgejo/workflows/ios.yml` and `.forgejo/workflows/android.yml` (task 6.2) and one new workflow `.forgejo/workflows/ui.yml` (task 1.1 adds the model job; 6.2 extends it). `core.yml`, `specs.yml` and `issue-sync.yml` are not touched.
- **`docs/requirements/`**: one new file, `docs/requirements/08-gpui-desktop.md`, recording the desktop and gpui addendum (task 6.2). No existing file there is edited. Where `04-ui-ux.md` says "iOS and Android only", `08` records the supersession.
- **Not granted**: any change to a Gourmet or Ventopay request shape, `openspec/specs/` edits outside normal archive, force-push, or push to `main` outside the loop's merge.

### Assumptions (change these before approving if wrong)

1. Fonts: bundle Inter (SIL OFL 1.1) Regular, SemiBold and Bold; icons come from gpui-kit's `assets` feature. A glyph missing from `04-ui-ux.md` §2 is mapped to the nearest available icon and recorded in task 4.1.
2. Desktop uses the mobile layout (single column, four tabs) in a resizable window, content centred with a maximum width of 480 logical px. No sidebar.
3. Desktop credential storage uses the OS keychain through the `keyring` crate. Desktop has no notifications, geofence or alarms; the Benachrichtigungen row is hidden on desktop.
4. The Mac is reachable over `ssh mac` for the whole change, and simulator builds need no signing.
5. The `headless` AVD can run a Vulkan or GLES surface well enough for gpui. If not, task 2.1 fails its gate (see design D10).
6. Background tasks that v1 had but the native shells never built (BGTaskScheduler order sync, WorkManager menu check) stay out of scope.
7. UniFFI annotations and the `cdylib`/`staticlib` crate types stay in `src/core` for now. Removing them is a follow-up change.

## Capabilities

### New Capabilities

- `ui-shell`: one gpui-kit code base hosting the whole app on iOS, Android and desktop: dependency pinning, single root view, tab navigation, theming, bundled assets, desktop shell, core-only networking, removal of the native UIs.
- `ui-screens`: behaviour of every screen and shared component, reproducing `docs/requirements/04-ui-ux.md`: menus, orders, billing, settings, both logins, appearance, notifications, app bootstrap.
- `platform-bridge`: contract between the Rust UI and the OS services that stay native: credentials, notifications, geofence and location, analytics, app icon, desktop fallbacks.

### Modified Capabilities

None. `openspec/specs/` holds no capabilities today.

## Impact

- **Code**: new `src/ui/**`; migrated shims from `src/ios/SnackPilot/{KeychainStore,NotificationService,LocationService,Analytics,AppIconService}.swift` and `src/android/.../{SecureCredentialStore,NotificationService,AlarmReceiver,LocationService,GeofenceBroadcastReceiver,SnackPilotApp}.kt`; deletion of the SwiftUI and Compose UI files; `Makefile`, `tools/devops/*`, three workflow files, `AGENTS.md` and README text updated.
- **Dependencies**: see "Approvals granted". Biggest risk is mixing `gpui-pre` versions; the lockfile must hold exactly one.
- **Toolchain**: Android NDK `30.0.16248370`, `cargo-ndk`, rustup targets `aarch64-linux-android`, `x86_64-linux-android`, `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`. Installed by task steps, not by the repo.
- **Verification hosts**: Linux VM for model tests, Android build and emulator; the MacBook (`ssh mac`) for iOS simulator and the macOS desktop run.
- **Safety**: no scraping code moves. UI crates must not depend on `reqwest`, `hyper` or any HTTP client (REQ-UI-08), so request shapes cannot change by construction.
- **Size**: this is a large change (22 tasks). It is gated: tasks after 2.3 are only dispatched when all three spikes pass.
