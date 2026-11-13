# 1.3 — Menüs cards, day navigator and pending dock

- **Change:** `mobile-ui-rework`, task 1.3
- **Branch:** `task/1.3-day-tray-menus`
- **Host that produced this index:** the Linux VM. It can type-check the gpui
  app crate, but it cannot link or run it (no X/font/xkb libraries), has no Mac
  desktop session and no working iOS simulator or Android emulator. Every row
  below that needs a rendered surface is `BLOCKED(host)`, **not** PASS, and **no
  screenshot, frame or action trace was fabricated**. The written status is
  exactly what the commands cited here produced.
- **Approval context:** A3–A8 plus the shared approvals in 1.2, approved by the
  human on 2026-11-13. Gate behaviour stays incumbent until 1.9. The screen-owned
  legacy `.p(16)`/`.pb(100)` removal and the Menüs entry in
  `legacy_screen_owns_bottom_inset` land with this task, as the dispatcher note
  requires.

## What this task changed

| Area | File(s) | Change |
| --- | --- | --- |
| Compact Menüs screen | `src/ui/app/src/screens/menus.rs` | Day Tray day navigator, neutral refresh row, inline error with a 44 px retry target, dish-first grouped list, no legacy 16/100 px padding, no absolute FAB. The wide `desktop_menus_tab` path is untouched. |
| Menu card | `src/ui/app/src/components/menu_card.rs` | Dish = `subtitle` (title only when empty), body 16/24 up to four lines, 16 px padding / 8 px gaps / 8 px after, card radii, neutral Semibold category heading, metadata badge at r8/8×4 in the unchanged precedence, accent selection tint + 1 px resolved boundary + inline 24 px `Check` slot, dashed/struck readable pending-cancel, wrapping allergens above the unchanged right-aligned price, no opacity dimming. Extraction of `mobile_dish` / `mobile_badge_label` / `mobile_menu_tappable` keeps the rule testable. |
| Day navigator | `src/ui/app/src/components/day_navigator.rs` | New `mobile_day_navigator`: 48×48 arrows with 20 px Lucide `ChevronLeft`/`ChevronRight`, min-48 centre with the date in the headline role and the ordinal in the metadata role 4 px below; disabled ends use readable disabled ink instead of 0.3 opacity. The incumbent `day_navigator` (wide Ledger) is unchanged. |
| Pending dock | `src/ui/app/src/components/fab.rs` | The absolute FAB column became `mobile_dock`: `DockState` (exact `model.fab_label()` or the exact operation phase), `dock_state` presence rule, `dock_height` recipe projection, `dock_bottom` (8 px above navigation), full-column surface min 48 / r14 iOS r12 Android / 16-12 padding, centred wrapping text, no callback during an operation. |
| Swipe | `src/ui/app/src/components/swipe.rs` | Pure `claims_horizontal` / `swipe_offset` / `swipe_day` keep claim 10 / switch 50 / edge 0.3; reduced motion drops the painted travel only. |
| Shell wiring | `src/ui/app/src/app.rs` | `navigation_top`, the dock's measured height fed into `compact_shell_plan`, the dock painted inside that band (above the busy scrim so its phase stays readable), the compact Menüs screen owns the day navigator, Menüs removed from `legacy_screen_owns_bottom_inset`, and `cached_menu_refresh` keeps nonempty cached cards instead of a blocking overlay. |
| Gallery | `src/ui/app/src/gallery.rs` | `mobile_menus_case` on page `mobile`: the real compact navigator, cards in every state and the longest mixed dock over static items. A component board, not proof of a running screen. |
| Model parity | `src/ui/model/tests/menus.rs` | `mobile_mixed_pending_submit_includes_other_days`: a queued cancellation on one day plus a new order on another are submitted by one activation with the exact mixed label. |
| Registered native test | `src/ui/ios/UITests/DesktopRegressionTests.swift` | `testDayTrayMenusDayNavigationAndMixedDock`: selects across two days, then activates the dock twice, capturing the rendered frames with bounded waits (no `sleep`). |
| DESIGN extension | `docs/design/DESIGN.md` | Menüs/navigator/card/dock/swipe inventory in the approved mobile extension only; base inventory and Desktop Ledger retained. |

## Status list

| # | Item | Status | Evidence / command |
| --- | --- | --- | --- |
| 1 | `make check` (fmt, clippy, core tests, devops lib, spec validation, ui model tests) | **PASS** | `make-check-linux.log` (exit 0, 6/6 OpenSpec items valid, model `menus.rs` 15 tests) |
| 2 | All-days mixed pending submit (`mobile_mixed_pending_submit_includes_other_days`, REQ-SCR-01) | **PASS** | in `make-check-linux.log`: `test mobile_mixed_pending_submit_includes_other_days ... ok` |
| 3 | App crate type-checks with tests (`cargo check -p snackpilot-app --all-targets`) | **PASS** | `app-crate-check.log` (exit 0, no warnings) |
| 4 | App crate test execution (`cargo test --locked -p snackpilot-app --lib`) | **BLOCKED(host)** | the VM cannot link the gpui host: `rust-lld: error: unable to find library -lxcb` (also `-lfontconfig`, `-lfreetype`, `-lxkbcommon`, `-lxkbcommon-x11`). Run on Mac: `cd src/ui && cargo test --locked -p snackpilot-app --lib` |
| 5 | Menüs app tests (`mobile_menu_uses_subtitle_and_empty_fallback`, `mobile_menu_badge_check_and_eligibility_projection`, `mobile_pending_dock_wraps_and_reserves_measured_height`, `mobile_menu_action_dispatches_once_without_extra_fetch`, `mobile_day_navigator_keeps_day_identity_and_targets`, `mobile_swipe_thresholds_and_reduced_motion`, `mobile_cached_refresh_keeps_the_menu_content`, `dock_state_matches_pending_and_progress_conditions`, `dock_height_reserves_wrapped_recipe_height`) | **BLOCKED(host)** (compiled, not linked) | row 4's command; each carries its `@req` tag |
| 6 | Mac desktop workflow (GUI build/run at 360/390/720, app tests) | **BLOCKED(host)** | `make mac-sync`, then `ssh mac`, `export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH`, `cd ~/vm-sync/SnackPilot-gpui/src/ui && cargo build --locked -p snackpilot-app --bin snackpilot-desktop && cargo test --locked -p snackpilot-app --lib` |
| 7 | iOS bootstrap/build/XCUITest (`DesktopRegressionTests`, Menüs scenarios) | **BLOCKED(host)** | `bash src/ui/ios/bootstrap.sh`, then `xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' build`, then `xcodebuild test -project SnackPilot.xcodeproj -scheme SnackPilot -destination 'platform=iOS Simulator,id=<UDID>'` (record the chosen UDID) |
| 8 | Android release check → debug rebuild → APK/run | **BLOCKED(host)** | `bash tools/devops/ui-android-build.sh --release`, then `bash tools/devops/ui-android-build.sh`, then in `src/ui/android` `./gradlew :app:assembleDebug --no-daemon`; install/launch on the dedicated emulator |
| 9 | Menüs before/after matrix at 360/390 both dialects, Orange light/dark (default, long 4-line dish + allergens, selected, ordered, pending-cancel, unavailable, cutoff, empty, initial loading, cached refresh, error, mixed dock, last-card clearance) | **BLOCKED(host)** | 1.1 baseline `../1.1/baseline-menus-populated-light-390.png` exists; the after set needs the iOS/Android evidence run `--evidence --viewport=WxH --tab=0` with `--scenario=populated\|empty\|loading\|refresh\|error\|pending\|progress` |
| 10 | 768 mobile (560 column, no extra day column, dock matches column) and mobile 1280 (560 cap) | **BLOCKED(host)** | same evidence run with `--viewport=768x1024` / `--viewport=1280x1024` |
| 11 | Gold-light and Beere-dark real mixed Menüs stress captures | **BLOCKED(host)** | evidence run plus the appearance accent set via the existing appearance setters; requires a rendered surface |
| 12 | Touch/swipe action trace: select across two days, unselect, mark a cancellation, submit once during repeated taps, record exact phases | **BLOCKED(host)** | `testDayTrayMenusDayNavigationAndMixedDock` (registered) plus a native trace on the simulator/emulator; the phases are asserted in the model test on this VM |
| 13 | Wide desktop Menüs row/detail/preview/actions unchanged (720/768/1280) | **BLOCKED(host)** | Mac desktop GUI run; the code path is untouched and the app tests pin it |
| 14 | ≥150 % effective type / short usable height, and 390 reduced motion on the day swap | **BLOCKED(host)** | device/simulator setting; the reduced-motion rule is unit-tested (`mobile_swipe_thresholds_and_reduced_motion`) |
| 15 | Assistive traversal (VoiceOver/TalkBack/Switch Control) | **BLOCKED(host)** | GPUI exposes no complete accessibility tree in this pinned stack; labels are text only and no coordinate tap is claimed as assistive proof |

## Configured bounds and spacing (source values, not a rendered measurement)

| Item | Value | Where |
| --- | --- | --- |
| Day arrows | 48×48, Lucide 20 | `day_navigator::mobile_day_navigator` |
| Day centre target | min 48, headline 24/32, meta 14/20, gap 4 | same |
| Card padding / gaps / after | 16 / 8 / 8 | `menu_card`, tokens |
| Card radii | 16 iOS, 14 Android | `theme.card_surface()` |
| Dish text | body 16/24, max four lines | `menu_card` |
| Check slot + gap | 24 + 8, inline, never floated | `menu_card` |
| Badge | metadata role, r8, 8 horizontal / 4 vertical | `menu_card` |
| Dock | full column, min 48, r14/r12, 16/12 padding, 8 above navigation | `fab::mobile_dock`, `app::navigation_top` |
| Error retry | min 44 high | `menus::mobile_error_banner` |
| Swipe | claim 10, switch 50, edge 0.3 | `swipe` |

The dock's reserved height is a recipe projection: 12 + 12 padding plus one
24 px body line per wrapped line, floored at 48, with a conservative 0.6 em
advance per character (`fab::dock_height`). Every label the model can produce
fits one line at 328/358/560 px, so the projection reserves 48 in practice and
only grows as a protective bound; the rendered clearance itself remains a native
capture item (row 9).

## Honesty notes

- The shell's obstruction reservation (`app::obstruction_reservation`) is the
  one owner: navigation (only when floating) + the present dock's measured
  height + 16. Menüs no longer paints its own inset, so the horizontal gutter is
  the shell's 16 px and the two never sum.
- A compact Menüs cached refresh keeps the cards, checks and day; the blocking
  overlay is not painted over nonempty cached content (initial load without data
  still shows it). The wide Ledger branch keeps its incumbent overlay.
- During a submit the exact phase is painted in the dock band above the busy
  scrim; the action is absent, so there is no second submit path. The gate and
  the all-days set are the incumbent model behaviour.
- No current-day total, count, filter, gesture, checkout or order semantic was
  added; category order and `SUPPE & SALAT` suppression are unchanged (model
  tests).
- GPUI's missing accessibility tree is reported, not worked around: the
  compact screen's names and states exist as text only.
