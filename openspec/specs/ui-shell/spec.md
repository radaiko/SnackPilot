# ui-shell Specification

## Purpose
Defines the single Rust gpui-kit UI code base that hosts SnackPilot on iOS, Android and desktop, including how it is pinned, mounted, themed and kept away from the network.

## Requirements

### Requirement: REQ-UI-01 One Rust UI code base for every target
The app SHALL render its whole user interface from one Rust gpui-kit code base shared by iOS, Android and desktop. Platform-specific code SHALL be limited to host projects and platform services.

#### Scenario: All targets build from one UI crate
- **WHEN** the iOS simulator app, the Android APK and the desktop binary are built
- **THEN** each links the same UI crate and no screen is implemented a second time in Swift, Kotlin or any other language

### Requirement: REQ-UI-02 Pinned gpui dependency set
The UI workspace SHALL pin `gpui-pre` to exactly `=0.3.4` with default features off, `gpui-kit` to git revision `7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7` and `gpui-pre-mobile` to git revision `0b882efdac7f524e0bb0b1d4c886b2aa752f9f20`. The lockfile SHALL resolve exactly one `gpui-pre` version.

#### Scenario: Lockfile has a single gpui-pre
- **WHEN** the UI lockfile is inspected with `cargo tree -i gpui-pre`
- **THEN** exactly one `gpui-pre` version, `0.3.4`, is present

#### Scenario: Pin drift is rejected
- **WHEN** a manifest changes one of the three pins
- **THEN** the pin test in the model crate fails

### Requirement: REQ-UI-03 Single root view per process
The app SHALL initialise gpui-kit once per process and mount exactly one root view. Backgrounding, resuming, rotation locks and Activity or scene recreation SHALL NOT create a second root or lose in-memory app state.

#### Scenario: Background and resume keep state
- **WHEN** the user selects a day on the Menüs tab, sends the app to the background and returns
- **THEN** the same root is shown with the same selected day and no second view exists

#### Scenario: Android configuration change
- **WHEN** the Android system recreates or reconfigures the host Activity
- **THEN** the app keeps its single root and its state

### Requirement: REQ-UI-04 Four-tab navigation
The app SHALL present the tabs Menüs, Bestellungen, Abrechnung and Einstellungen with the icons of `docs/requirements/04-ui-ux.md` §2. On iOS the bar SHALL be the icon-only floating pill; on Android a bottom bar with labels; on desktop the Android-style bar.

#### Scenario: Tab switch
- **WHEN** the user taps the Bestellungen tab
- **THEN** the Orders screen is shown and the tab is marked active in the bar style of the current platform

#### Scenario: Re-tapping the active tab
- **WHEN** the user taps the already active tab
- **THEN** nothing navigates

### Requirement: REQ-UI-05 Theme and accent
The app SHALL support the System, Hell and Dunkel preferences and the five accents Orange, Smaragd, Beere, Gold and Ozean with the hex values of `04-ui-ux.md` §8, persist both independently, and re-render immediately without restart. System SHALL follow the OS appearance live.

#### Scenario: Preference change applies at once
- **WHEN** the user picks Dunkel on the Darstellung screen
- **THEN** every visible screen re-renders dark and the choice survives an app restart

#### Scenario: Accent is independent of preference
- **WHEN** the user picks Ozean while the preference is Hell
- **THEN** the preference stays Hell and the accent becomes Ozean

### Requirement: REQ-UI-06 Bundled fonts, icons and German text
The app SHALL ship its fonts and icons inside the binary or bundle and register them at startup, so no glyph depends on a system font. All user-facing text SHALL be the German strings of `04-ui-ux.md`, verbatim. Dates, numbers and currency SHALL use `de-AT` formatting.

#### Scenario: Umlauts render
- **WHEN** any screen shows "Menüs", "Abrechnung" or "Bestell-Erinnerung"
- **THEN** every glyph renders from the bundled font on all three targets

#### Scenario: de-AT formatting
- **WHEN** a bill amount of 1234.5 EUR is shown
- **THEN** it reads with the de-AT decimal comma and currency format of the spec

### Requirement: REQ-UI-07 Desktop shell
The app SHALL run as a desktop window on macOS using the same UI crate, with the mobile layout centred in a column of at most 480 logical px, a minimum window width of 360 px, and system light and dark following. Linux and Windows builds SHALL compile where the pinned gpui supports them.

#### Scenario: macOS window
- **WHEN** the desktop binary is started on the Mac with demo credentials
- **THEN** a resizable window shows the four-tab UI with demo menus

#### Scenario: Wide window
- **WHEN** the window is wider than 480 px
- **THEN** the content stays a centred column and no sidebar appears

### Requirement: REQ-UI-08 The core is the only network path
The UI crates SHALL reach Gourmet, Ventopay and the broadcast gist only through the `snackpilot-core` facade and SHALL NOT depend on an HTTP client. No Gourmet or Ventopay request shape SHALL change.

#### Scenario: No HTTP client in the UI
- **WHEN** `cargo tree` is run for the model and app crates
- **THEN** `reqwest` appears only beneath `snackpilot-core` and no UI crate lists an HTTP client directly

#### Scenario: Wire audit unchanged
- **WHEN** `make check` runs the core `wire_audit` test
- **THEN** it passes with no edit under `src/core`

### Requirement: REQ-UI-09 Native UIs removed
When the change is complete the repository SHALL contain no SwiftUI or Jetpack Compose view code and no UniFFI-generated Swift or Kotlin binding use. The iOS bundle id and Android application id `dev.radaiko.gourmetclient` SHALL be unchanged.

#### Scenario: Cutover state
- **WHEN** the repository is searched after cutover
- **THEN** `src/ios` and `src/android` are gone, `src/ui/ios` and `src/ui/android` build the shipped apps, and both identifiers match the pre-change values
