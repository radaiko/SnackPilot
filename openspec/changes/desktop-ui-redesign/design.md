# Design: Desktop Ledger and desktop releases

## Context

See `proposal.md` for motivation, confirmed release answers and approval boundaries. Domain is **ui**: packaging/host work enables the requested shipping UI, not a core rewrite.

The approved `docs/design/desktop-ui-redesign.md` is the visual and per-screen contract, including exact geometry, tokens, copy, states, keyboard map and evidence matrix. Do not repeat or reinterpret its taste decisions. Two explicit human amendments override its release proposals: **Windows 11 x64 also ships unsigned as an installer**, and **macOS 26+ arm64 ships as a signed `.app`, not DMG**. All other visual decisions remain fixed. Apply the desktop screen dialect to both desktop hosts; native macOS menu/window semantics stay macOS-only.

Pre-implementation baseline (historical inventory used to plan this change, not current implementation reality):

- `src/ui/app/src/lib.rs::DesktopFrame` unconditionally caps the app at 480; the desktop default is 420×760, minimum height zero. `app.rs` paints the bottom bar regardless of desktop width.
- The gpui-free `Navigation` exists, but the instance, settings `Detail` enum and `orders_past` live in `AppShell`; filters/pending sets already live in `State`. Login `InputState` entities are created in `open_detail` and must not be recreated on resize.
- Existing model methods expose all required menu/order/billing operations, stable billing IDs and formatting. Several synchronous model methods call `block_on`; calling them on the render thread prevents the required in-flight UI from painting. The existing `busy` confirm path is not an atomic command claim, unlike single cancel. The desktop command boundary must prevent repeated dispatch and let cached content remain readable.
- `bin/desktop.rs` constructs one model at `temp_dir()/snackpilot-desktop`. `DesktopPlatform` already uses keyring (`apple-native`, `windows-native` target features exist) and truthful capability bits, but unsupported mail/settings/icon calls currently succeed as no-ops. UI must not advertise them; no new platform service is approved.
- `tools/devops/ship.sh` only selects iOS/Android; shared semver lives in the core package/iOS project/Android Gradle inputs and a local counter. Its current failure cleanup uses broad file checkout; extending shipping must preserve user work rather than reuse that unsafe cleanup for new paths.
- `make check` includes formatting for both UI crates but only model clippy/tests, not app unit tests, platform compilation, signing or installer tests. Those require the additional commands below. `docs/design/DESIGN.md` does not exist yet and is an implementation deliverable, not a file this spec session writes.

## Goals / Non-Goals

**Goals:** separate stable selection from mutation, project one model into narrow/wide containers, keep keyboard/menu/pointer dispatch equivalent, and make actual native installed artifacts the shipping proof.

**Non-goals:** new core/domain semantics, request changes, HTTP dependencies, mobile redesign/tablet dialect, Linux shipping, Windows ARM/Intel Mac support, Tauri/Velopack, an updater, OS notification/geofence/mail integration, navigation persistence across process termination, or repairing the GPUI accessibility tree.

## Decisions

### D1. A pure workspace model, not host-owned navigation

Extend `src/ui/model/src/navigation.rs` and add `desktop.rs` for pure host/width projection, stable selection reconciliation and keyboard command policy as needed. `State`/`AppModel` owns the navigation instance, selected settings kind, previews, orders segment and list cursors. Keep GPUI `FocusHandle`, scroll handles, modal view resources and `InputState` entities in `AppShell`. Never persist a preview identity as a row index; the nearest surviving focus target is computed from previous ordered keys only during reconciliation.

Use one shared navigation intent for bottom tab, rail, shortcut and native menu; same-tab activation is a no-op. A changed tab resets the departing navigation detail/preview/cursor but not pending menu changes. A day/month/source/segment change invalidates only its context's preview. Esc clears preview without forgetting list cursor, so next arrow/row activation can reopen it. Foreground data publication reconciles preview IDs; rendering alone never resets them.

Task 1.5 owns the approved 720 px settings projection correction: compact and expanded desktop contain the selected settings detail without a push; phone projection retains exactly one push. Prove this with a 719→720→719 model-stack test, keeping the same view-owned inputs and unsaved text.

Rejected: independent wide/narrow navigation trees or storing the new state only in a desktop host. Those break resize/resume and cannot be covered by model tests. No new dependency is needed.

### D2. Project the shell, reuse record/form content

Determine layout from **host AND content width**, not theme.flat or OS appearance. `Phone` applies to all mobile widths and desktop <720; `DesktopCompact` is [720,1024); `DesktopExpanded` is >=1024. Put dimensions in named desktop tokens, with geometry assertions at 719/720/768/1023/1024/1280. The brief's exact rail/gutter/split formulas win; remove the outer 480 cap only for wide desktop and paint overlays outside the rail/workspace cap.

Add shared `DesktopNavigationRail`, `SplitWorkspace`, `RecordTable`, header/toolbar/action-strip compositions within `components`. Existing screen functions choose containment/presentation rather than duplicating screens or business logic. Menu rows preview only; the existing toggle is an explicit detail button. Order body previews, checkbox toggles batch set with propagation stopped. Read-only receipt detail reuses existing item/formatting data. Settings passes a containment mode to the same credential/appearance content; resize does not call form entry/prefill. In-flight operations dispatch outside the render thread using the existing model/runtime and publish state back; do not add sleeps, extra scrapes or another client.

Dispatch is host-gated, not rail-gated: 1.2 supplies shared infrastructure, 1.3/1.4 wire menu/order/billing, and 1.5 completes credentials, keyboard/menu refresh, foreground and narrow desktop paths. Test pointer, shortcut/menu and foreground dispatch at 360 and 720. Height never changes the width-selected layout; short windows only add reachability/scroll obligations.

Rejected: a parallel desktop screen tree/data-table library, new detail routes for narrow menu/order/billing, or width-only mobile switching.

### D3. Extend the incumbent theme, scoped to desktop

Create `docs/design/DESIGN.md` as an inventory of existing palette, five accents, Inter scale/assets, spacing/radii/elevation, motion, Lucide mapping and shared components, followed by the Desktop Ledger extension. Do not replace the approved brief or change mobile token values. Pure geometry/contrast/state-selection helpers can live in the model's new `desktop.rs` so `make ui-check` covers them; gpui conversion stays in `theme.rs`.

Use the brief's desktop token names and recipes, actual-fill WCAG calculations and linear-sRGB hover mixes. Keep selected-row rule/tint separate from hover inset boundary and keyboard ring. All desktop interactive surfaces, including narrow legacy controls, use shared feedback/focus recipes; passive data is not a button. Reserve ring space and >=44 px targets, wrap toolbars/actions/long text, scroll independent pane bodies and short modal messages. Keep toast timings, immediate state transitions and reduced-motion semantics exactly as specified.

Rejected: globally raising contrast/changing recipes for mobile, screen-local colours/spacing, or introducing another font/component system.

### D4. One command policy; view-owned focus and single-resolution dialogs

Use typed shared intents with pure enablement/scope decisions tested in the model; desktop adapters handle platform keys and GPUI focus. Shared commands use Cmd on Mac and Ctrl on Windows; do not intercept Windows-logo or native close/quit/editing keys. This Windows mapping is an explicit engineering assumption for human review, not a change to macOS bindings. Every operation is claimed once before background dispatch; key repeat, double activation and a dialog-opening key cannot dispatch it twice. Guard at the actual command/mutation entry point, not with an artificial delay.

Focus order/traversal follows the brief. Each rail/list/option group is one Tab stop with a roving cursor; focus moves to revealed rows without scrolling the other pane. Inputs receive native editing first, masking prevents password copy/cut, Return saves once only if eligible. Modal focus is captured on open, initially OK/confirm, cycles only inside actions, Enter uses focused action, Esc/backdrop cancels index 0. Restore the origin or nearest surviving row/heading. Underlying tab/settings/refresh/back intents disable while modal; native quit/hide/minimise remain available. Keep the same dialog API, texts and destructive labels.

Navigation chords remain available while editing unless a modal is open; native editing keys retain priority, without view-only eligibility exceptions. Wide-record-list roving focus and nearest-surviving-row restoration are behaviour required in 1.5, not host blockers waived by a capture deferral.

Rejected: separate keyboard and pointer eligibility, one key handler that captures arrows globally, separate native alert system, or repeat suppression by timing hacks.

### D5. Native Mac menus/window behaviour through the pinned host facilities

Use the existing pinned GPUI/gpui-kit native actions/menu facilities, adding only a desktop host module within `src/ui/app/src/desktop/` if needed. Follow the brief's German menu inventory verbatim: SnackPilot / Ablage / Bearbeiten / Darstellung / Fenster. No Rust dependency changes or upstream edits are preapproved. If a required host capability cannot be implemented with the pinned facilities, report the exact gap and request approval instead of silently claiming support.

Menus read the same model/command enablement; About uses one themed alert and real release metadata. `init_kit`, shared assets and one installed model remain process-wide; reopening after Mac window close mounts at most one live main window using that session. Default 1180×780/minimum 360×480, work-area clamping, native decorations and live appearance. Window bounds can be host-persisted where supported; no domain navigation moves into host-only storage. Gallery/debug arguments remain debug-gated.

About/privacy copy follows authoritative `docs/requirements/07-release.md`: self-hosted Aptabase, not the historical TelemetryDeck paragraph. Retain the GourmetClient credit and truthful desktop-only no-statistics sentence. The human approved only these narrow copy amendments to the brief and affected deltas.

Windows gets standard native decorated/resizable window controls, shared keyboard/pointer behaviour and built-in OS close/edit semantics; a Mac-style global menu or new custom toolbar is not implied. Windows-specific native menu design beyond this is not part of the amendment.

### D6. Native artifacts, deliberate trust policies

macOS: package the locked `aarch64-apple-darwin` release binary with deployment target `MACOSX_DEPLOYMENT_TARGET=26.0` into `SnackPilot.app`; `Info.plist` contains stable bundle ID, icon, minimum OS and shared version/build. Reuse existing Orange icon assets to generate `.icns` with system tools; fonts/Lucide remain bundled/embedded as today. Validate dynamic libraries and asset completeness so no checkout path is required. Sign actual nested code then the outer app with the external Developer ID Application identity and hardened runtime. Use external notary profile, temporary `ditto` ZIP for submission only, await acceptance, staple **the app**, then verify `codesign --verify --deep --strict`, signature identity/options, `xcrun stapler validate` and `spctl --assess --type execute`. Never use ad-hoc fallback. Final deliverable is the `.app` directory; future transport format is not invented.

Windows: native `x86_64-pc-windows-msvc` release build on Windows 11, with MSVC/Windows SDK prerequisites. Use **Inno Setup 6** (build-time only) and a checked-in `.iss` for per-user `%LOCALAPPDATA%\Programs\SnackPilot`, stable app ID `dev.radaiko.gourmetclient`, German install UI, Start Menu/uninstall entries, Windows 11 minimum and x64-only install mode. No administrator requirement, certificate or Authenticode stage. Bundle any required redistributable runtime in an approved redistributable form or statically link where the pinned toolchain supports it; dependencies must be resolved from actual PE import inspection, not by copying arbitrary system DLLs. No new runtime crate is authorised. Installer includes no user data; upgrade/uninstall removes application files, not user settings/credentials. Document unknown-publisher/SmartScreen risk without instructing users to disable protections. Output `SnackPilot-{version}-windows-x64-setup.exe`, labelled `UNSIGNED` with checksum.

Rejected: restoring Velopack/updater/Tauri, DMG contrary to the human's `.app` selection, Windows signing by inference, or declaring cross-compilation to be install proof. The Windows host is a required execution resource not identified in this repo; task 1.7 stops if unavailable.

### D7. Durable storage and narrow release-script extension

Use `~/Library/Application Support/dev.radaiko.gourmetclient/` on Mac and `%LOCALAPPDATA%\dev.radaiko.gourmetclient\` on Windows. Resolve with existing host/std facilities; no new directory crate. Fail truthfully on missing/unwritable application-data path; do not fall back to temp. Keyring service/slots already match and remain unchanged. Development temp cache is not authoritative state to import, and no credentials are read from it. Package replacement never touches application data. Exercise dedicated dummy keyring accounts only, never real credential slots, for release evidence.

Extend `ship.sh`/`lib.sh`, don't add a second release orchestrator. Preserve existing iOS/Android selectors (including the legacy both option) and add named `macos`, `windows`, `all` selection with explicit combinations. Use `macos/v…` and `windows/v…` tags, not the retired generic desktop updater namespace. Version/build enter native packages through invocation parameters/generated metadata, derived from the existing common release version/build inputs; do not change `src/core/Cargo.toml`, mobile manifests, or crate pins as part of implementing the tooling. Runtime About must not use the app crate's placeholder `0.0.0`.

Builders run natively: Mac `.app` via the synced tree, Windows build/installer via a provided Windows working copy/native command. `make ship` orchestrates selected native commands, rejects unavailable hosts/tools/artifact mismatches, and never fabricates a Windows artifact on Mac/Linux. A configured external Windows build command can supply the native build; keep addresses/secrets outside source, validate returned platform/version/build/checksum before success. No host provisioning or CI modification is authorised. Preserve no auto-push/upload. Fail all validation before commit/tags/history/counter; restore only edits owned by the invocation, never `git checkout` user work. Offline stubs exercise each selector and failure stage. Dry-run bypasses mutation and external distribution/notarization submission, may produce isolated unsigned test artifacts explicitly labelled nonrelease, and cannot report them as signed releases.

Tasks 1.6/1.7 generate one native-build receipt contract consumed by `ship.sh` in 1.8: platform, architecture, version, build, artifact path, trust state, checksum scheme and checksum. Mac trust is `developer-id-notarized-stapled` after verification; Windows is `unsigned`. Windows hashes the installer with SHA-256. A Mac `.app` uses SHA-256 of a deterministic bundle-content manifest (sorted relative paths, entry types, modes, symlink targets and file SHA-256 values; no timestamps/absolute paths), not a public ZIP. Consumer tests recompute checksums and reject mismatched fields/trust/artifacts before bookkeeping. Task 1.8 audits the cumulative diff read-only; its edits stay within its own allowlist, with separately approved follow-up contracts for integration defects.

## Verification plan

Every task: `make check`. Additional commands are not substitutes:

- Mac sync/build/run: `make mac-sync`; remote commands start `export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH`, use `~/vm-sync/SnackPilot-gpui/`; in `src/ui`, `cargo build --locked -p snackpilot-app --bin snackpilot-desktop`, `cargo test --locked -p snackpilot-app --lib`, then launch debug with offline fixture/demo data in the GUI and exercise real input/menu/resize. A bounded 5-second no-panic run proves liveness only, not appearance.
- iOS: after sync, run `bash src/ui/ios/bootstrap.sh`; in `src/ui/ios`, `xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' build`; use the established simulator ad-hoc signing from project settings (unsigned simulators cannot prove keychain). Install/launch a concrete simulator and run relevant XCUITests with `-destination 'platform=iOS Simulator,id=<UDID>'`.
- Android: compile the Rust library with `bash tools/devops/ui-android-build.sh --release` as a check, then rebuild without `--release` (`bash tools/devops/ui-android-build.sh`) before `./gradlew :app:assembleDebug --no-daemon` from `src/ui/android` and captures. The debug APK must contain the debug library for gallery/debug evidence entries. Install/launch on a working emulator/device for regression evidence. Do not delete a user's installed app/data without permission; use dedicated test devices/simulators/profiles.
- Windows: from `src/ui`, `cargo build --locked --release -p snackpilot-app --bin snackpilot-desktop --target x86_64-pc-windows-msvc`, native app tests, `ISCC.exe` via the packaging script, then install/upgrade/uninstall and real keyboard/pointer/resize smoke on Windows 11 x64.
- Actual screenshots for four roots/reachable details at desktop **360/720/1280**, plus **390/768** checkpoints; before/after mobile at corresponding widths, light/dark and all five accents. Follow the brief's populated/empty/auth/loading/cached-refresh/error/selection/disabled/long-content matrix; focused/hovered controls and 720's 263 px detail are essential. Store fixture/demo-only images, action traces and command output in `docs/evidence/desktop-ui-redesign/<task-id>/`; annotate scale/content widths, do not relabel phone images desktop. Tests never reach real hosts; fixture startup disables/isolates live broadcast as well as auth requests.
- Final release: Mac install to Applications, notarization/trust validation, relaunch/update and dummy keychain/settings continuity; Windows unsigned installer on clean Windows 11 profile, launch/edit/resize, update and uninstall preserving state. Record actual artifact size and launch-to-first-frame timing with method/host, not extrapolations. `make check` is not a release trust or visual PASS.

Task 1.5 supplies a debug-only isolated startup/foreground harness: select isolated data and dummy credentials and suppress live broadcast/auth before bootstrap; never read or write normal credential slots. No core/manifest changes. Register `DesktopRegressionTests.swift` explicitly in `SnackPilotUITests.sources`, regenerate the project and run on a concrete simulator UDID. The `tasks.md` evidence-status rule and named 1.2–1.4 debt remain binding: a human-approved deferral is not PASS, does not consume a fix round, and never waives behaviour or mandatory release trust/install proof.

## Risks / Trade-offs

- Pinned gpui native menu/focus/reopen features may be incomplete → test actual Mac early in task 1.5, stop for approval if impossible; don't upgrade pins or fake menus.
- Existing blocking model calls freeze UI and allow repeat races → run existing flows off render thread, claim once before dispatch, test delayed offline responses/duplicate input; request order and domain eligibility stay untouched.
- Desktop screenshot/Android emulator failures occurred in the archived change → native capture/device evidence is a hard done bar here; a render snapshot helps geometry but does not prove native input. Report missing evidence as blocked, not pass.
- Available Mac signing/notary setup is human-confirmed, not tested → resolve references externally, fail closed if unavailable; no secret/certificate bodies in evidence.
- No Windows verification host is identified → human must provide a Windows 11 x64 build/test environment before 1.7 can finish. No paid cloud allocation, remote access configuration or CI bootstrap is implicit.
- Unsigned Windows downloads can trigger OS reputation warnings → explicitly accepted unsigned distribution, honest release instructions; no promise of warning-free installation.
- Accessibility tree remains absent → keep limitation explicit and do not equate keyboard coverage with screen-reader support.

## Migration Plan

1. Add model invariants and desktop tokens/shell with existing mobile branches preserved.
2. Adapt menu/order/billing/settings containment and route keyboard/menu/pointer to the same guarded intents.
3. Introduce durable desktop paths and native Mac/Windows artifacts; prove clean install and update continuity.
4. Wire shipping selectors, failure/dry-run tests and the approved `08` amendment plus supersession pointers. Run the complete automated/native/visual matrix and one review; do not mark tasks complete without their evidence.
5. Rollback is a reviewed revert of this change through the normal loop, not deletion of users' application data or keychain entries. No scraping/cache format migration is included.

## Open Questions

No unresolved product compatibility, signing-policy or distribution-format decisions. External signing identity/notary-profile references and the address/access method of the provided Windows native build host are execution configuration, deliberately not recorded here; absence blocks the corresponding task rather than loosening its acceptance.
