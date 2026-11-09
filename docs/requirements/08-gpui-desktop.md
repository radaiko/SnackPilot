# 08 — gpui-kit UI and the desktop addendum

Supersedes the "iOS and Android only" scope of v2: the app now has **one Rust UI**
(`src/ui`, gpui-pre + gpui-kit) that runs on iOS, Android and, as a desktop
addendum, in a macOS window. The native SwiftUI and Compose shells were removed
in task 6.1; `src/core` is unchanged and still owns scraping, parsing, caching,
notifications and demo mode.

## Layout

- `src/ui/model` — `snackpilot-model`: UI-facing model, session flows, platform
  trait, state hub, pure screen logic and its tests (no gpui).
- `src/ui/app` — `snackpilot-app`: the gpui UI (shell, screens, components),
  plus the Android (`android/`, Gradle + NativeActivity), iOS (`ios/`, UIKit
  containment + xcodegen) and desktop (`bin/desktop.rs`) hosts.
- `make check` includes `make ui-check` (fmt, clippy, model tests).

## Desktop

The desktop shell mounts the same `Root` in a resizable window (minimum width
360) with the content centred in a column of at most 480 px; it uses the flat
Android dialect of §7 and has no tab-bar safe area. It is a development and
gallery target, not a shipped platform: `make ship` ships iOS and Android only.

## Known regressions

- **Accessibility**: GPUI renders to Metal/Skia with no accessibility tree.
  XCUITest can only drive raw coordinate taps and the system keyboard (see
  `src/ui/ios/UITests/LoginTypingTests.swift`). VoiceOver/TalkBack parity with
  the v1 native shells is a regression accepted for v2 and tracked as a
  follow-up.
- No native platform alerts: dialogs are the themed §4.1 modal.

## Measured artifacts (task 6.2, 2026-11-09)

- Android release cdylib: 46 MB (arm64-v8a), 44 MB (x86_64), unstripped by
  design; the Gradle debug APK is ~159 MB (all ABIs, unstripped).
- iOS simulator debug staticlib: ~1.2 GB (`libsnackpilot_app.a`), a debug
  artifact; the release archive uses the same crate but optimised.
- Startup: not yet instrumented end to end (no first-frame timestamp hook);
  recorded as a follow-up rather than estimated.
