# SnackPilot iOS

Native SwiftUI shell over the portable Rust core (`../core`) via UniFFI. The app owns only
UI, navigation, and (later) platform services — Keychain, notifications, background tasks,
location. All scraping, caching, and domain logic lives in the core; see
`docs/architecture/v2-architecture.md`.

## Build

```bash
brew install xcodegen        # once
./bootstrap.sh               # builds the core XCFramework + bindings, generates the project
open SnackPilot.xcodeproj
```

`bootstrap.sh` produces three git-ignored things — the XCFramework, the generated
`snackpilot_core.swift`, and `SnackPilot.xcodeproj`. Re-run it whenever the core changes.

## Layout

```
project.yml                     xcodegen spec (app target, iOS 17, links the XCFramework)
SnackPilot/
├── SnackPilotApp.swift         @main entry
├── AppModel.swift              @MainActor shell state over SnackPilotCore
├── RootView.swift              login gate → tabs
├── LoginView.swift             Gourmet login + offline demo
├── MainTabView.swift           4-tab shell (Menüs / Bestellungen / Abrechnung / Einstellungen)
├── MenusView.swift             day-grouped menu list
└── Generated/snackpilot_core.swift   UniFFI bindings (copied from the core)
Frameworks/SnackPilotCore.xcframework  (copied from the core)
```

## Status

Vertical slice: FFI wired and verified on the simulator — `coreVersion()` and
`demoMenuSnapshot()` render through SwiftUI. Menüs is the only live tab; the others are
placeholders pending their screens. Login talks to the live Gourmet server; the demo
credentials (and the "Demo-Menüs anzeigen" button) render canned data offline and never
reach the server.
