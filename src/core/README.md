# snackpilot-core

The portable Rust core of SnackPilot v2 — all account-ban-sensitive scraping, caching,
domain logic, notification decisions, and demo data. The SwiftUI (`src/ios`) and Compose
(`src/android`) apps are thin shells over this crate via UniFFI-generated bindings.

Behavior is specified in `docs/requirements/` (extracted + adversarially verified from
v1.4.5). **Do not deviate from `01-gourmet-scraping.md` / `02-ventopay-scraping.md`** — the
exact request sequences are what keep accounts from being banned; the fixture-driven tests
in this crate are the guarantee they still match.

## Layout

```
src/
├── domain/       plain records + enums (cross the FFI as UniFFI Records/Enums)
├── error.rs      CoreError (UniFFI Error)
├── datetime/     Clock trait + Vienna-time + every wire date format
├── http/         Transport trait, reqwest impl, capturing fake, Ventopay cookie jar
├── gourmet/      client + parser + api  (01-gourmet-scraping)
├── ventopay/     client + parser + api  (02-ventopay-scraping)
├── storage/      Kv trait, atomic FileKv, MemoryKv, typed cache
├── features/     menus / orders / billing stores (Zustand-store equivalents)
├── notify/       fingerprint, daily/cancel reminders, geofence, log, menu_check
├── demo/         magic-credential check, LCG PRNG, canned data generators
└── ffi.rs        SnackPilotCore — the UniFFI facade the shells call
```

## Develop

```bash
cd src/core
cargo test                         # fixture-driven contract suite (record & replay)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The 13 sanitized fixtures live canonically in `docs/fixtures/` and are mirrored into
`tests/fixtures/`; `build.rs` fails the build if they drift.

## Build for the apps

The facade exports one object (`SnackPilotCore`) plus plain records; the host injects a
storage directory and credentials and receives records back. Async operations use tokio.

- **iOS**: `./scripts/build-apple-xcframework.sh` → `target/SnackPilotCore.xcframework` +
  `target/bindings-swift/snackpilot_core.swift`. Add both to the Xcode project.
- **Android**: `./scripts/build-android-jni.sh` → `target/jniLibs/<abi>/libsnackpilot_core.so`
  + `target/bindings-kotlin/…/snackpilot_core.kt`. Copy into `app/src/main/jniLibs/` and the
  Kotlin source set.

Both scripts run `uniffi-bindgen` (the `[[bin]]` in this crate) against the compiled library.

## What the shell owns (NOT in this crate)

Secure credential storage (Keychain / Keystore) + v1 credential takeover, background-task
scheduling (BGTaskScheduler / WorkManager), notification *delivery* (the core returns
`NotificationCommand`s; the shell executes them), location/geofencing, permissions, the
TelemetryDeck analytics transport (via the injected `AnalyticsSink`), and all UI
(`04-ui-ux.md`). See `docs/architecture/v2-architecture.md` for the full ownership map.
