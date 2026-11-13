# 08 — gpui-kit UI and the desktop addendum

Supersedes the "iOS and Android only" scope of v2: the app now has **one Rust UI**
(`src/ui`, gpui-pre + gpui-kit) that runs on iOS, Android and, as a shipped
desktop platform, in a macOS window. The native SwiftUI and Compose shells were
removed in task 6.1; `src/core` is unchanged and still owns scraping, parsing,
caching, notifications and demo mode.

## Layout

- `src/ui/model` — `snackpilot-model`: UI-facing model, session flows, platform
  trait, state hub, pure screen logic and its tests (no gpui).
- `src/ui/app` — `snackpilot-app`: the gpui UI (shell, screens, components),
  plus the Android (`android/`, Gradle + NativeActivity), iOS (`ios/`, UIKit
  containment + xcodegen) and desktop (`bin/desktop.rs`) hosts.
- `src/ui/desktop/macos` — the release `Info.plist` template and the
  `SnackPilot.icns` generated from the existing Orange app icon.
- `make check` includes `make ui-check` (fmt, clippy, model tests); the app
  crate, native hosts and `tools/devops/test-package-macos.sh` run in their
  documented loops.

## Shipping matrix

| Platform | Artifact | Minimum | Architecture | Trust |
| --- | --- | --- | --- | --- |
| macOS | `SnackPilot.app` | macOS 26.0 | Apple Silicon (`arm64`) | Developer ID Application, hardened runtime, notarized and stapled |
| Windows | `SnackPilot-{version}-windows-x64-setup.exe` (task 1.7) | Windows 11 | x64 | **UNSIGNED**, labelled as such |
| iOS / Android | existing App Store / Play artifacts | unchanged | unchanged | unchanged |

Desktop identity is `dev.radaiko.gourmetclient`. Linux, Intel Macs, Windows ARM,
App Store distribution and automatic updates are not supported by this change
and are not advertised. The macOS deliverable is the signed and stapled `.app`
itself; no DMG and no public ZIP is produced. A temporary ZIP exists only as the
notary submission transport and is deleted again.

## Desktop

The desktop shell mounts the same `Root` in a resizable native window (minimum
360×480, default 1180×780, clamped to the visible work area) with one model and
one main window. The **SnackPilot Desktop Ledger** is the approved desktop
extension of the incumbent design system: a labelled 176/192 px rail at 720 px
and wider, a persistent master–detail workspace, desktop-only table density,
roving keyboard focus and the exact native menu inventory. Both layouts share
one responsive UI code path; the mobile layouts at the same widths are
unchanged. A Dock reopen restores the existing window and session instead of
starting a second one.

Keyboard: Cmd+1–4, Cmd+, and Cmd+R (Ctrl equivalents on Windows), Esc
precedence, standard Tab/arrow/Enter/Space behaviour and native Bearbeiten
editing keys. Menus dispatch the same guarded intents as on-screen controls and
are disabled while a modal owns the session. Quit, Hide, Close, Minimise and
Zoom keep their native behaviour.

About and the rail footer read the shared product version/build inputs (the iOS
`MARKETING_VERSION`/`CURRENT_PROJECT_VERSION` that `make ship` bumps) or the
packaging override, never the app crate's `0.0.0` placeholder. The macOS
release validates the same version/build in its `Info.plist`, receipt and About
surface.

## Durable desktop data

A shipping desktop build stores its cache and preferences per user, outside the
installed application:

- macOS: `~/Library/Application Support/dev.radaiko.gourmetclient/`
- Windows: `%LOCALAPPDATA%\dev.radaiko.gourmetclient\`

A missing home/`LOCALAPPDATA`, an unsupported host or an unwritable directory
stops startup with the real reason; the app never falls back to a temp
directory, the install location or the development evidence directory. An
installer replacing the `.app`/installation directory therefore cannot
overwrite user data, and a relaunch reads the same state.

Credentials remain in the OS secure store under service `app:no-auth` and the
existing four slots (`gourmet_username`, `gourmet_password`,
`ventopay_username`, `ventopay_password`). Persist-before-validate,
logout-keeps-credentials/cookies and cookie retention are unchanged. Desktop
evidence runs use a dedicated non-production dummy keyring entry instead of the
four normal slots.

## Desktop capability gaps

Desktop declares notifications and geofence unavailable, so bootstrap does not
request those OS permissions and Einstellungen does not present them as working
features at any width. A stale/debug route shows the honest unavailable copy.
Analytics is a no-op: no `app.launched` or other event leaves the desktop host.
Mail export, system-settings actions and dynamic Dock icons are not implemented
and are not promised; there is no updater. The accepted GPUI limitation that the
rendered surface has no accessibility tree still applies, and native system
alerts remain replaced by the themed modal.

## Known regressions

- **Accessibility**: GPUI renders to Metal/Skia with no accessibility tree.
  XCUITest can only drive raw coordinate taps and the system keyboard (see
  `src/ui/ios/UITests/LoginTypingTests.swift`). VoiceOver/TalkBack parity with
  the v1 native shells is a regression accepted for v2 and tracked as a
  follow-up. The desktop host inherits the limitation.
- No native platform alerts: dialogs are the themed §4.1 modal.
- Desktop has no notification, geofence, analytics, updater or dynamic icon
  implementation; see the capability gaps above.

## Release trust and packaging (macOS)

`make package-macos` builds `snackpilot-desktop` with `--locked --release` for
`aarch64-apple-darwin` and `MACOSX_DEPLOYMENT_TARGET=26.0`, assembles
`SnackPilot.app` with the shared icon/assets/Inter/Lucide runtime closure and the
common version/build, signs nested code and then the outer app with the external
Developer ID Application identity and hardened runtime, submits a temporary ZIP
with the external `notarytool` keychain profile, requires `Accepted`, staples the
app and validates `codesign --verify --deep --strict`, `stapler validate` and
`spctl --assess --type execute`. A missing identity/profile, ad-hoc signing,
rejected notarization or failed trust check exits nonzero without a receipt;
there is no dry-run or unsigned fallback. The build writes the common
`native-build-receipt.json` (`macos`, `aarch64`, version, build, artifact path,
`developer-id-notarized-stapled` only after verification, and the SHA-256 of the
deterministic bundle-content manifest over sorted relative paths, entry types,
modes, symlink targets and file hashes) for `make ship` to consume. No signing
secret is read into, stored in or printed from the repository, artifact or
evidence.

## Measured artifacts (task 6.2, 2026-11-09)

- Android release cdylib: 46 MB (arm64-v8a), 44 MB (x86_64), unstripped by
  design; the Gradle debug APK is ~159 MB (all ABIs, unstripped).
- iOS simulator debug staticlib: ~1.2 GB (`libsnackpilot_app.a`), a debug
  artifact; the release archive uses the same crate but optimised.
- Startup: not yet instrumented end to end (no first-frame timestamp hook);
  recorded as a follow-up rather than estimated.

## macOS release measurements (task 1.6, pending)

No macOS release measurement has been captured yet. These are **not** copied
from the mobile figures and must be recorded from the real signed/stapled
artifact on the Mac host:

- Size: `du -sk SnackPilot.app` (KiB) and `du -sh SnackPilot.app`, after
  stapling, with the macOS version and CPU recorded.
- Launch to first frame: screen recording at 60 fps from launch, counting frames
  until the first painted window content, on the installed app on macOS 26
  Apple Silicon; host and method recorded with the result. Not estimated from
  mobile startup.
