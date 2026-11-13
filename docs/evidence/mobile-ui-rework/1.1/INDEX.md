# 1.1 — Day Tray baseline and pure projection

- **Change:** `mobile-ui-rework`, task 1.1
- **Branch / revision:** `task/1.1-day-tray-baseline` @ `5cf31f9`
- **Host that produced this index:** the text rows were produced on the Linux VM
  (no iOS build, no working Android emulator — it segfaults). The iOS baseline
  captures below were added afterwards by the dispatcher from the MacBook
  simulator; the Android and desktop rows remain `BLOCKED(host)`.
- **Baseline definition:** this branch's current **pre-Day-Tray incumbent**
  presentation at `5cf31f9`. No presentation edit is part of 1.1.
- **Historical reference only:** the mobile before-images owned by
  `desktop-ui-redesign` 1.2–1.5 are `DEFERRED-APPROVED` under the human decision
  of 2026-11-13 and superseded by this change. The original pre-rework state is
  unavailable and is never recreated, inferred or indexed as a baseline.
- **No image was fabricated.** Every text row was produced by a command on the
  Linux VM; the four iOS PNGs in this directory are real `simctl` screenshots
  from the MacBook simulator, indexed in *Captured baseline* below. Nothing else
  is claimed: dark appearance, Android, desktop and the wider matrices were not
  captured.

## Status summary

| Item | Status | Evidence |
| --- | --- | --- |
| Pure mobile space/type/cap/target/radius/focus/ink helpers appended to the existing token module | PASS | `src/ui/model/src/tokens.rs`, `mobile_day_tray_geometry`, `mobile_day_tray_contrast_all_accents_and_composited_fills`, `mobile_extension_preserves_base_values` |
| Base tokens and the five accents unchanged (append-only) | PASS | `mobile_extension_preserves_base_values` @req REQ-UI-15 |
| Wide Desktop Ledger geometry pinned at 720/768/1280 | PASS | `day_tray_keeps_wide_ledger_geometry` @req REQ-UI-15 |
| Debug evidence isolation (no normal credential slots, no notification/permission/geofence/analytics/live host) | PASS | `mobile_evidence_never_touches_normal_credentials_or_live_hosts` @req REQ-UI-08/REQ-SCR-09 |
| Evidence resize/resume keeps one root and the selected day, pending selection and preview | PASS | `mobile_evidence_resize_and_resume_retains_single_root` @req REQ-UI-03 |
| Debug viewport/scenario selection is explicit and debug-only | BLOCKED(host) (type-checked, not executed here) | `evidence_viewport_and_scenario_selection_is_debug_only` @req REQ-UI-16; run with `cargo test --locked -p snackpilot-app --lib` on the Mac |
| Synthetic scenario fixtures (populated/empty/loading/refresh/error/pending/progress/permission results) | BLOCKED(host) (type-checked, not executed here) | `evidence_fixture_states_apply_only_to_an_isolated_store` @req REQ-UI-16; run with `cargo test --locked -p snackpilot-app --lib` on the Mac |
| `make check` on this VM | PASS | `make-check-linux.log` |
| Release build proves the evidence entry is absent | BLOCKED(host) | `cd src/ui && cargo check -p snackpilot-app --release` fails here on a missing host tool (`pkg-config`, `yeslogic-fontconfig-sys`); the evidence method/knobs are `#[cfg(debug_assertions)]` and the selection functions return `None` in release |
| Registered iOS baseline run (`testDayTrayBaselineIsolationAndViewport`) | PASS | run on the Mac against simulator `C2994587-5046-4970-8DDE-3F003EB81D0D`; asserts the model-installed host and window bounds |
| Android runtime baseline and action trace | BLOCKED(host) | needs a working emulator/device (the VM emulator segfaults); command below |
| iOS runtime baseline (Menüs, Bestellungen, Abrechnung, Einstellungen; light) | PASS | `baseline-*-populated-light-390.png`; dark, 360/768/1280 and non-Menüs fixtures not captured |
| Desktop narrow/wide baseline | BLOCKED(host) | needs the Mac desktop workflow; command below |

## Native capture items (all BLOCKED(host))

Every row below is **BLOCKED(host)**: the Linux VM cannot run iOS builds or a
working Android emulator, and has no Mac desktop session. No row is PASS, and
no image/receipt was invented for any row. The item must be captured on the
host by the dispatcher before 1.2, using the command ids after the table.

Common capture metadata to record with every produced image: host and method,
logical content width/height, physical scale, safe insets, theme (light/dark)
and accent, scenario fixture (`populated`, `empty`, `loading`, `refresh`,
`error`, `pending`, `progress`, `permission-granted`, `permission-denied`) and
selected date/clock/context/scroll, revision `5cf31f9`, and the before/after
pair. Simulated iOS viewport widths must be labelled honestly.

| # | Surface / criterion | Required states | Status | Command |
| --- | --- | --- | --- | --- |
| B1 | Menüs runtime | default/populated long dish, pending, progress, empty, initial loading, cached refresh, error, auth gate | BLOCKED(host) | C1, C2, C3 |
| B2 | Bestellungen runtime | upcoming/past, confirmed/unconfirmed, zero/one/many selections, cutoff, cancel dialog, empty, loading, refresh, error | BLOCKED(host) | C1, C2, C3 |
| B3 | Abrechnung runtime | three months/sources, subsidy present/absent, long receipts, empty, loading, refresh, error, auth gate | BLOCKED(host) | C1, C2, C3 |
| B4 | Einstellungen runtime | both/one/neither authenticated, long account hint, privacy dialog | BLOCKED(host) | C1, C2, C3 |
| B5 | Kantine-Zugangsdaten | idle/prefilled masked, keyboard, busy, blank/failure/success dialogs, session/logout | BLOCKED(host) | C1, C2, C3 |
| B6 | Automaten-Zugangsdaten | same form states, independent session, `Automaten-Sitzung aktiv` | BLOCKED(host) | C1, C2, C3 |
| B7 | Anmeldung gates | signed-out Menüs gate, Bestellungen gate, Abrechnung gate (both signed out), auth error | BLOCKED(host) | C1, C2, C3 |
| B8 | Darstellung | System/Hell/Dunkel and all five accents, selected/inactive/pressed/focus | BLOCKED(host) | C1, C2, C3 |
| B9 | Benachrichtigungen | reminder off/on, location absent/resolving/set/error, log inactive/active/expired, permission result | BLOCKED(host) | C1, C2, C3 |
| B10 | Shared overlays | broadcast banner, toast, long modal with actions visible, loading overlay, no pass-through | BLOCKED(host) | C1, C2, C3 |
| B11 | Matrix at 360 and 390 | iOS glass light/dark, Android flat light/dark, Orange; all B1–B10 | BLOCKED(host) | C1, C2, C3 |
| B12 | Matrix at 768 | both dialects/light/dark, 560 content / 440 forms, platform bar, no split | BLOCKED(host) | C1, C2, C3 |
| B13 | Mobile at 1280 | same 560 cap, no record grid/sidebar | BLOCKED(host) | C1, C2, C3 |
| B14 | Desktop narrow 360/390 and wide 720/768/1280 light/dark | narrow Day Tray, wide Ledger unchanged, native input/keys | BLOCKED(host) | C4 |
| B15 | Runtime populated long orders/receipts (not gallery-only) | populated Menüs/Bestellungen/Abrechnung on a runtime screen | BLOCKED(host) | C1, C2, C3 |
| B16 | Isolation assertion traces | fake platform records zero normal-slot access/notification/permission/analytics on the native run | BLOCKED(host) | C1, C2, C3 |

### Known host gap — resolved in this branch

The iOS host installed no `AppModel` when this index was first written. That gap
was closed here: `src/ui/app/src/platform/ios.rs` now applies the evidence
viewport before `session.bootstrap()` and the scenario after it, so the iOS
runtime renders real fixture states, and an evidence run never reaches the OS
permission shims (no system prompt). The Swift `DesktopRegressionTests` case
therefore exercises the model-installed host. What remains `BLOCKED(host)` is
capture breadth, not capability: Android, the desktop window and the
360/768/1280 matrices.

## Exact host commands

### C1 — Mac: sync, desktop build and app-crate tests

```sh
make mac-sync
ssh mac
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH
cd ~/vm-sync/SnackPilot-gpui/src/ui
cargo build --locked -p snackpilot-app --bin snackpilot-desktop
cargo test --locked -p snackpilot-app --lib
```

Launch an isolated desktop baseline at a selected logical viewport (the
viewport knob sizes the gallery/capture window; the runtime window is the Mac
desktop's own):

```sh
SNACKPILOT_EVIDENCE=1 ./target/debug/snackpilot-desktop --evidence --viewport=390x844 --tab=0
```

### C2 — Mac: iOS simulator build, launch and registered baseline test

```sh
make mac-sync
ssh mac
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH
cd ~/vm-sync/SnackPilot-gpui/src/ui/ios
bash bootstrap.sh
xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot -sdk iphonesimulator \
  -destination 'generic/platform=iOS Simulator' build
# Install/launch the dedicated simulator, then run the registered baseline test:
xcodebuild test -project SnackPilot.xcodeproj -scheme SnackPilot \
  -destination 'platform=iOS Simulator,id=<UDID>' \
  -only-testing:SnackPilotUITests/DesktopRegressionTests/testDayTrayBaselineIsolationAndViewport
```

Record the chosen `<UDID>`. The test launches with
`--evidence --viewport=390x844 --tab=<0..3>`, waits for the window (no fixed
sleep), asserts real window bounds and non-empty captures, and writes each
screenshot to `SPIKE_EVIDENCE_DIR`.

### C3 — Android: release compile check, debug rebuild, install and capture

```sh
bash tools/devops/ui-android-build.sh --release
bash tools/devops/ui-android-build.sh
cd src/ui/android && ./gradlew :app:assembleDebug --no-daemon
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n dev.radaiko.gourmetclient/.SnackPilotActivity
```

Isolated capture knobs (debug APK; the debug Rust library must be present so
the fixture entries exist):

```sh
adb shell setprop debug.snackpilot.scenario populated   # or empty/loading/refresh/error/pending/progress/permission-granted/permission-denied
adb shell setprop debug.snackpilot.viewport 390x844
adb shell am force-stop dev.radaiko.gourmetclient
adb shell am start -n dev.radaiko.gourmetclient/.SnackPilotActivity
adb exec-out screencap -p > baseline-menus-390-android.png
```

`SNACKPILOT_EVIDENCE=1` / `--evidence` remains the isolation switch; a debug
build without it must render the normal shell.

### C4 — Mac: desktop narrow/wide before/after

```sh
make mac-sync && ssh mac
export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH
cd ~/vm-sync/SnackPilot-gpui/src/ui
cargo build --locked -p snackpilot-app --bin snackpilot-desktop
```

Resize the window to 360, 390, 719, 720, 768 and 1280 logical content px and
capture light/dark. At 719→720→719 the wide Ledger must appear/disappear with
the mobile projection, and 720/768/1280 must keep the pinned rail/workspace
numbers from `day_tray_keeps_wide_ledger_geometry`.

## Isolation assertions for every native run

- The model is built through `evidence::model`: isolated storage directory,
  in-memory dummy credentials, normal OS/keychain slots never read or written.
- Evidence bootstrap/foreground suppresses the live broadcast, the system
  permission prompt, the geofence restore and analytics (`track`).
- An isolated evidence run answers both permission shims denied inside the iOS
  bridge (`IosPlatform::request_permission`), so no caller — including
  `-DebugPlatformExercise` — can reach `UNUserNotificationCenter` or CoreLocation
  and change a permission. Proven by
  `platform::ios::tests::evidence_run_never_reaches_os_permissions` @req
  REQ-UI-08/REQ-UI-16 and by the native log line
  `notify_permission: denied, no system prompt in evidence mode`.
- Evidence mode now also suppresses host `notify`, so a capture cannot deliver
  a system notification.
- Failure/permission fixtures are synthetic state; no wrong credentials are
  sent anywhere and no live host is contacted.
- These assertions are executable on this VM in
  `mobile_evidence_never_touches_normal_credentials_or_live_hosts` and
  `evidence_fixture_states_apply_only_to_an_isolated_store`; the native run
  must record the same zero-call trace.

## Files produced by this task

| File | Criterion | Status | Host | Metadata |
| --- | --- | --- | --- | --- |
| `INDEX.md` | baseline/index honesty, status list, exact commands | PASS | Linux VM | revision `5cf31f9`; no image metadata (no image produced) |
| `make-check-linux.log` | `make check` under the task's `check:` | PASS | Linux VM | revision `5cf31f9`, 503 lines, ends with the model test/doc-test summary |

## Captured baseline (dispatcher run, 2026-11-13)

Host: MacBook, iPhone 17 Pro simulator (iOS 26), UDID `C2994587-5046-4970-8DDE-3F003EB81D0D`,
`xcrun simctl` install of the debug build synced from branch `task/1.1-ios-evidence`, launched as
`--evidence --scenario=populated --viewport=390x844 --tab=<0..3>`, simulator appearance switched with
`xcrun simctl ui <udid> appearance <light|dark>`. Physical image size 1206x2622 @ 3x, logical width 390, safe top inset 62 pt,
tab bar floating pill. Revision: the baseline commit on this branch.

Dark appearance was **not** captured: `xcrun simctl ui <udid> appearance dark` did not change the
app's rendering (the light and dark runs produced byte-identical Menüs images), so the four dark PNGs
were deleted rather than mislabelled. Dark stays `BLOCKED(host)`.

These are the **pre-Day-Tray incumbent** screens, captured before any presentation edit. They stand in for
the desktop-deferred mobile before-images: the historical ones are unavailable and are not recreated.

| File | Surface | States observed |
| --- | --- | --- |
| `baseline-menus-populated-light-390.png` | Menüs | category headings, closed dishes, allergens, prices, floating tab bar |
| `baseline-orders-populated-light-390.png` | Bestellungen | segments with counts, empty upcoming list, `Auswählen` action |
| `baseline-billing-populated-light-390.png` | Abrechnung | month/source controls, summary, empty receipts |
| `baseline-settings-populated-light-390.png` | Einstellungen | account rows, hints, chevrons |

Observed in the captures and carried into the rework findings: a large dead band between the safe area and
the first content (Orders/Billing), category headings in accent uppercase above the dish, and the
`filled populated` fixture covering Menüs only — Orders/Billing lists render empty under this scenario.
Remaining rows of the native table above (B5–B16, Android, desktop, 360/768/1280) stay `BLOCKED(host)`.
