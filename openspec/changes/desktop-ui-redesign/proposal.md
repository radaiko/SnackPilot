# Proposal: Shipping desktop workspace

domain: ui

## Why

The desktop host currently puts a phone UI inside a 480 px column and is not a shipping target. Employees need a keyboard- and pointer-operable workspace for comparing meals, orders and receipts, plus installable desktop releases without changing mobile behaviour or scraping.

## What Changes

- Implement **SnackPilot Desktop Ledger**, the fixed responsive design in `docs/design/desktop-ui-redesign.md`: labelled navigation rail and master–detail at desktop content widths **>=720 logical px**, compact geometry below 1024, 960 px workspace maximum above; retain the phone fallback below 720 and native mobile layouts at every width.
- Put navigation, settings-detail kind, stable record previews, list cursors and orders segment in the gpui-free model. Keep GPUI focus/input objects in the view; resizing never recreates forms or performs a domain action.
- Add explicit desktop menu/order actions, order and receipt tables, settings split forms, keyboard routing, hover/focus/disabled states, accessible contrast resolution and modal focus isolation. Existing business actions, formatting, German strings and request sequences remain unchanged.
- Add standard decorated macOS window behaviour and the German native menus/About specified by the brief, at narrow and wide widths.
- Promote **macOS 26+ / Apple Silicon** to shipping, using the existing external Developer ID Application and notarization setup; deliver a signed, hardened-runtime, notarized and stapled **`SnackPilot.app` only**, not a DMG or public ZIP.
- **Human scope amendment:** also ship **Windows 11 x64**, using the same Desktop Ledger UI, as an explicitly **unsigned installer**. This overrides only the brief's macOS-only scope. Windows uses native window controls and Ctrl equivalents for shared app/edit commands, not macOS global menus or Windows-logo shortcuts.
- Extend existing `make ship` with independently selectable `macos` and `windows` plus iOS/Android and all four. Preserve shared version/build identity, mobile release behaviour, no automatic upload, and dry-run's no-version/git/history/counter mutation guarantee.
- Replace development-temp storage with durable per-user desktop application data, retaining native secure-store service/slots and honest unsupported notifications/geofence/analytics behaviour.

### Confirmed release decisions

Human answers in this session: Apple Silicon only; macOS 26 Tahoe and newer; existing Developer ID Application **and notarization available**; signed `.app` only; Windows also required unsigned; Windows 11 x64; Windows distribution **Installer**. Identity/notary profile names stay in external configuration; no credentials were requested or recorded. The brief's proposed DMG is superseded by the explicit `.app` choice, not by an engineering assumption.

### Approval scope for implementation

Planning writes only this change directory plus the human-approved narrow Aptabase About/privacy copy amendment in `docs/design/desktop-ui-redesign.md`, and makes no commit. Approval of implementation must cover the exact task file lists, including:

- `docs/requirements/08-gpui-desktop.md` shipping amendment; scope-only supersession pointers in `00-overview.md`, `04-ui-ux.md` and `07-release.md` without rewriting historical v1 content or mobile contracts.
- `Makefile` shipping entry points and shell-test wiring; desktop packaging under `src/ui/desktop/` and `tools/devops/`; README/devops documentation and the incumbent-system inventory `docs/design/DESIGN.md`.
- No new runtime crates or changes to dependency manifests, lockfiles, CI, `src/core`, scraping requirements, or request shapes. Inno Setup 6 is a Windows **build tool only**, explicitly named in task 1.7. If the pinned stack cannot support a required native feature, stop for approval rather than upgrading dependencies or restoring Tauri/Velopack.
- `openspec/specs/` remains untouched during implementation; normal reviewed delta sync/archive is separate. No push, merge, release execution or signing-secret access is authorised by this planning request.
- Approved in-flight contract repair: task 1.5's purpose-scoped keyboard/focus, settings projection/command policy, explicit UITest registration, application/reopen and isolated offline evidence harness additions are listed in `tasks.md`; durable storage remains in 1.6. Evidence statuses/deferrals preserve mandatory behaviour and release trust/install gates. Privacy follows self-hosted Aptabase under authoritative `07-release.md`, not historical TelemetryDeck copy; no protected requirement edit is made by this repair.

## Capabilities

### New Capabilities

- `desktop-distribution`: supported desktop OS/CPU matrix, real packaged artifacts, signing policy, release orchestration, durable local data and install/update evidence (`REQ-DST-01`–`REQ-DST-04`).

### Modified Capabilities

- `ui-shell`: replace obsolete desktop-only bottom-bar/480 px requirements (`REQ-UI-04`, `REQ-UI-07`); add model-owned workspace selection, keyboard/focus/modal rules, desktop recipes and macOS menu integration (`REQ-UI-10`–`REQ-UI-14`).
- `ui-screens`: retain incumbent mobile/domain contracts while explicitly defining wide-desktop presentations for all four destinations, settings forms, dialogs and unsupported notification fallback (`REQ-SCR-01`–`REQ-SCR-04`, `REQ-SCR-08`); narrowly gate unsupported desktop notification/geofence bootstrap work while retaining mobile ordering (`REQ-SCR-09`).

## Impact

- Code: `src/ui/model` navigation/selection/command policy and tests; shared `src/ui/app` shell, screens, theme, components, desktop entry and host integration. One UI and one model/session, no duplicate platform screens.
- Release: Mac-native bundle/sign/notary tooling, Windows-native MSVC build and Inno Setup 6 installer, host-specific artifact validation, Linux offline packaging/orchestration tests; no desktop updater, store upload or Linux shipping.
- Verification: every implementation task runs `make check`; additional native Mac desktop build/run and iOS bootstrap/xcodebuild, Android build/run and mobile regression screenshots, and Windows 11 native build/install evidence. A Windows verification host is not identified in this checkout and is an execution prerequisite, not evidence to invent.
- Safety: no network client added to UI, no live-host tests, no real credentials/screenshots/cookies. `dev.radaiko.gourmetclient`, keyring service `app:no-auth`, persist-before-validate, logout retention, cutoffs, device-local order split and de-AT formats stay intact. GPUI's missing accessibility tree remains an explicit accepted gap; keyboard work does not claim VoiceOver/TalkBack parity.
