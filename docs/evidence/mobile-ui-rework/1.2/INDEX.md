# 1.2 — Day Tray shell, shared controls and overlays

- **Change:** `mobile-ui-rework`, task 1.2
- **Branch:** `task/1.2-day-tray-shell`
- **Host that produced this index:** the Linux VM. It cannot link or run the
  gpui app crate (missing X/font system libraries), has no Mac desktop session
  and cannot build iOS or run a working Android emulator. Every row below that
  needs a simulator/device/desktop window is `BLOCKED(host)`, **not** PASS, and
  **no screenshot, frame or action trace was fabricated**. The written status is
  exactly what the commands in this file produced.
- **Approval context:** A1/A2/A17/A18/A19/A20, C1 (in-app toast copy without the
  exclamation mark) and decisions 5/6 were approved by the human on 2026-11-13.

## What this task changed

| Area | File(s) | Change |
| --- | --- | --- |
| Presentation projection | `src/ui/app/src/theme.rs` | `MobilePalette` on `AppTheme`, resolved by `build_mobile_palette` from `snackpilot_model::tokens::mobile` measured on the composited card fill; per-fill ink helpers; compact geometry delegate. No base token, recipe or desktop palette value moved. |
| Shell sizing | `src/ui/app/src/lib.rs` | Narrow desktop reports the live width (obsolete 480 px cap removed) and `compact_column` delegates to the model geometry: `min(viewport − 32, 560)`, 440 form cap, `None` for the wide Ledger. |
| Shell presentation | `src/ui/app/src/app.rs` | Shell heading in the headline role (16 below safe top, 16 above controls), one centred column, safe top applied once, measured obstruction reservation (`obstruction_reservation`), reduced-motion toast wiring, C1 toast copy. |
| Back row | `src/ui/app/src/navigation.rs` | 48 px minimum back row, `ChevronLeft` 24, primary ink, no blind 100 px padding. |
| Shared controls | `components/{card,chip,button,text_field,banner,broadcast_banner,dialog,toast,switch,time_chips,tab_bar}.rs` | Body/meta roles, 44/48 target bounds, option/status r8, accent selection tint + check + resolved boundary, token-derived action fill/ink, fill-only press rule, 44×44 broadcast dismissal, scrollable dialog body with visible actions. |
| Component board | `src/ui/app/src/gallery.rs` | Page `mobile` (`SNACKPILOT_GALLERY_PAGE=mobile`): five accents × light/dark × the host dialect, with actions, pending/check, filters, semantic statuses, active/inactive navigation and a forced focus ring. A component board only — not proof of a running screen. |
| Model parity | `src/ui/model/tests/tokens.rs` | `mobile_rendered_projection_parity`: the rendered composite chain and every projected field resolve to pinned tokens (no second calculator). |
| DESIGN extension | `docs/design/DESIGN.md` §3 | Approved mobile extension + inventory of the components changed here only; §1 base inventory and §2 Desktop Ledger retained. |
| Native registration | `src/ui/ios/UITests/DesktopRegressionTests.swift`, `DialogOcclusionTests.swift` | Registered tests for the compact shell at 360/390/768/1280, the toast copy capture, and modal occlusion / single backdrop resolution. |

## Status list

| # | Item | Status | Evidence / command |
| --- | --- | --- | --- |
| 1 | `make check` (fmt, clippy, core tests, devops lib, spec validation, ui model tests) | **PASS** | `make-check-linux.log` (exit 0, 6/6 OpenSpec items valid, all model suites ok) |
| 2 | Model projection parity + geometry + contrast (`tokens.rs`) | **PASS** | `make-check-linux.log` — includes `mobile_rendered_projection_parity` @req REQ-UI-15 |
| 3 | App crate type-checks, tests included (`cargo check -p snackpilot-app --all-targets`) | **PASS** | `app-crate-check.log` (exit 0, no warnings) |
| 4 | App crate test execution (`cargo test --locked -p snackpilot-app --lib`) | **BLOCKED(host)** | `app-crate-test-link.log`: the linker cannot find `-lxcb -lfontconfig -lfreetype -lxkbcommon -lxkbcommon-x11`. Run on the Mac. Covers `mobile_recipes_consume_composited_projection`, `wide_desktop_recipes_unchanged`, `compact_projection_is_host_safe`, `mobile_shell_preserves_navigation_and_single_root`, `mobile_focus_press_disabled_hit_bounds`, `mobile_long_dialog_resolves_once_and_restores_focus`, `mobile_toast_copy_timing_and_reduced_motion`, `mobile_rendered_projection_parity` (model side already ran). |
| 5 | Mac desktop workflow (app tests + GUI build/run, 360/390/719→720→719 trace) | **BLOCKED(host)** | needs the Mac; `mac-sync` then `cd src/ui && cargo test --locked -p snackpilot-app --lib` and `cargo run -p snackpilot-app --bin snackpilot-desktop` |
| 6 | iOS bootstrap/build/XCUITest (`DesktopRegressionTests`, `DialogOcclusionTests`) | **BLOCKED(host)** | needs the Mac + simulator; `src/ui/ios/bootstrap.sh` then `xcodebuild test -project SnackPilot.xcodeproj -scheme SnackPilot -destination 'platform=iOS Simulator,name=iPhone 17 Pro'` |
| 7 | Android release check → debug rebuild → APK/run | **BLOCKED(host)** | needs a working emulator/device (the VM emulator segfaults) |
| 8 | Shell/overlay/control-state boards before/after at all conventions widths/themes and all five accents | **BLOCKED(host)** | needs a rendered surface; the gallery board exists but no capture was produced |
| 9 | Rendered hit bounds / touch & scroll claim / modal action traces | **BLOCKED(host)** | needs a device; the bounds below are the configured values, not a rendered measurement |
| 10 | Short viewport + ≥150 % type and keyboard reachability proof | **BLOCKED(host)** | needs a device/simulator |
| 11 | Accessibility (VoiceOver/TalkBack/Switch Control) traversal | **BLOCKED(host)** | see *Accessibility gap* below — GPUI exposes no full accessibility tree; not claimed from coordinates |
| 12 | Wide Ledger before/after and 719→720→719 input/containment trace | **BLOCKED(host)** | needs the Mac desktop workflow; the model-side containment tests already pass |

## Configured hit bounds (source values, not a rendered measurement)

| Control | Bound | Where |
| --- | --- | --- |
| Any compact target | ≥44×44 | `tokens::mobile::TARGET`, `button::meets_target`, `chip`, `time_chips`, `switch` row, dialog actions |
| Primary actions (form save, dock, confirm) | 48 high | `tokens::mobile::ACTION_HEIGHT`, `button::mobile_action_metrics` |
| Broadcast dismissal | exactly 44×44 | `components/broadcast_banner.rs` |
| Back row | ≥48 high, `ChevronLeft` 24 | `navigation::back_affordance` |
| Dialog actions | ≥44 high, r8 | `dialog::mobile_dialog_action_height` |
| Focus | 2 px ring + 2 px reserved gap (4 px), never shifting rest | `tokens::mobile::FOCUS_*`, `theme.mobile.focus` |
| Option/status radius | 8 | `tokens::mobile::RADIUS_OPTION/RADIUS_STATUS` |

## Contrast

Text and boundary floors are asserted numerically in the model on the actual
composited fills (`mobile_day_tray_contrast_all_accents_and_composited_fills`,
`mobile_rendered_projection_parity`, both executing here) and in the app
projection test (`mobile_recipes_consume_composited_projection`, execution
BLOCKED(host)). Text ≥4.5:1 and control/selection/focus boundaries ≥3:1 are
verified for all five accents, light/dark, glass/flat, rest and 14 % pressed.
No screenshot-based contrast claim is made.

## Accessibility gap (exact limitation)

GPUI in this pinned stack renders to Metal/GPU without a complete
accessibility tree. The shell/components expose no per-node accessibility
labels, roles, states or reading order, so the copied contract's assistive
requirements — "assistive names use the existing words … selected/busy/disabled
exposed as states", "screen-reader reading order follows
heading/context/status/records/action/navigation", "announce existing status
updates" — **cannot be verified or implemented** with the pinned facilities.
That part of the shared contract is `BLOCKED` and needs human reconciliation;
this task does not add a dependency or claim a coordinate tap as assistive
traversal. Visible names exist in text only (heading text, control labels,
exact German copy).

## Native capture checklist (all BLOCKED(host) until run on the host)

Every capture row is `BLOCKED(host)`; nothing here is PASS and no image exists
in this directory. Record with each produced image: host/method, logical
width/height, physical scale, safe insets, theme and accent, scenario fixture,
selected date/clock/context/scroll, revision and the before/after pair.

| # | Capture | Command |
| --- | --- | --- |
| B1 | Compact shell at 360/390/768/1280, light+dark, glass+flat | iOS/Android evidence run with `--evidence --viewport=WxH --tab=N` (and `SNACKPILOT_GALLERY_PAGE=mobile` for the board) |
| B2 | Component state boards, five accents × light/dark × dialect | gallery page `mobile` |
| B3 | Toast copy/timing, normal and reduced motion | `--evidence --toast` with the OS reduced-motion setting on/off |
| B4 | Long modal with keyboard/large type, actions visible, backdrop resolves once | `--evidence --dialog` + the two new registered Swift tests |
| B5 | Wide Ledger before/after 720/768/1280 and the 719→720→719 trace | Mac desktop workflow |

## Notes on honesty

- The four screens (Menüs/Bestellungen/Abrechnung/Einstellungen) still draw
  their own bodies; their Day Tray layouts, the pending dock and the day
  navigator are tasks 1.3–1.11. This task only supplies the shared projection,
  the shell frame and the shared controls.
- `loading_overlay` is unchanged (initial 36 px spinner card); the cached-refresh
  treatment is the neutral 16 px spinner row in `banner::refresh_banner`.
- No claim is made that a gallery board proves a functioning screen, native
  input, or assistive traversal.
