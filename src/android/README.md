# SnackPilot Android

Native Jetpack Compose shell over the portable Rust core (`../core`) via UniFFI/JNI. The app
owns only UI, navigation, and (later) platform services — Keystore, notifications,
WorkManager, geofencing. All scraping, caching, and domain logic lives in the core; see
`docs/architecture/v2-architecture.md`.

## Build

```bash
export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/<version>
./bootstrap.sh                 # builds the JNI libs + Kotlin bindings from the core
./gradlew :app:assembleDebug
```

`bootstrap.sh` produces two git-ignored things — the per-ABI `.so` libraries and the
generated `snackpilot_core.kt`. Re-run it whenever the core changes.

## Layout

```
app/build.gradle.kts            app module (applicationId dev.radaiko.gourmetclient, minSdk 29)
app/src/main/
├── AndroidManifest.xml
├── res/values/themes.xml
└── java/dev/radaiko/snackpilot/
    ├── MainActivity.kt         ComponentActivity + Compose entry
    ├── AppViewModel.kt         AndroidViewModel over SnackPilotCore
    └── ui/AppUi.kt             login gate → 4-tab scaffold → day-grouped Menüs list
app/src/main/jniLibs/<abi>/libsnackpilot_core.so   (copied from the core)
app/src/main/java/uniffi/snackpilot_core/…         (copied from the core)
```

## Status

Vertical slice: FFI wired over JNI — `coreVersion()` and `demoMenuSnapshot()` render
through Compose. Menüs is the only live tab; the others are placeholders pending their
screens. Login talks to the live Gourmet server; the demo credentials (and the "Demo-Menüs
anzeigen" button) render canned data offline and never reach the server. A DEBUG-only
`uiTestDemo` intent extra drives the same path for headless verification.

## Identity

`applicationId = dev.radaiko.gourmetclient` matches v1 so a Play update lands in place,
enabling best-effort credential takeover (07-release §2). The Kotlin source package is
`dev.radaiko.snackpilot`.
