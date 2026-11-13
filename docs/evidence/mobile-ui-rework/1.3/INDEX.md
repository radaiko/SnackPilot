# 1.3 — Menüs cards, day navigator and pending dock

- **Change:** `mobile-ui-rework`, task 1.3
- **Branch:** `task/1.3-day-tray-menus`
- **Revision:** branch HEAD `086e0b8` (the task commit) plus this review fix
  round in the working tree, uncommitted at capture time. Every image and command
  below was produced from that tree.
- **Approval context:** A3–A8 plus the shared approvals in 1.2, approved by the
  human on 2026-11-13. Gate behaviour stays incumbent until 1.9. The
  screen-owned legacy `.p(16)`/`.pb(100)` removal and the Menüs entry in
  `legacy_screen_owns_bottom_inset` land with this task, as the dispatcher note
  requires.

## What this task changed

| Area | File(s) | Change |
| --- | --- | --- |
| Compact Menüs screen | `src/ui/app/src/screens/menus.rs` | Day Tray day navigator, neutral refresh row, inline error with a 44 px retry target, dish-first grouped list, no legacy 16/100 px padding, no absolute FAB. The wide `desktop_menus_tab` path is untouched. |
| Menu card | `src/ui/app/src/components/menu_card.rs` | Dish = `subtitle` (title only when empty), body 16/24 up to four lines, 16 px padding / 8 px gaps / 8 px after, card radii, neutral Semibold category heading, metadata badge at r8/8×4 in the unchanged precedence, accent selection tint + 1 px resolved boundary + inline 24 px `Check` slot, dashed/struck readable pending-cancel, wrapping allergens above the unchanged right-aligned price, no opacity dimming. `mobile_dish` / `mobile_badge_label` / `mobile_menu_tappable` / `mobile_card_treatment` / `mobile_card_name_slots` / `mobile_card_footer` are the one projection the builder paints. |
| Day navigator | `src/ui/app/src/components/day_navigator.rs` | `mobile_day_navigator` is a contained control group (`mobile_day_navigator_recipe`): option radius r8, the projected card fill, a neutral boundary that reads at ≥3:1, 4 px inset, bare 48×48 arrow targets with the 20 px Lucide `ChevronLeft`/`ChevronRight` and no disc behind them, min-48 centre with the date in the headline role and the ordinal in the metadata role 4 px below. Disabled ends use the readable disabled ink instead of 0.3 opacity. `day_edge` is the one edge rule. The incumbent `day_navigator` (wide Ledger) is unchanged. |
| Pending dock | `src/ui/app/src/components/fab.rs` | `mobile_dock`: `DockState` (exact `model.fab_label()` or the exact operation phase), `dock_state` presence rule, a real measurement of the label (`dock_label_lines`: the label is shaped with the window's text system at the width the dock leaves for text), `dock_height_for_lines`, and `dock_bottom` (8 px above navigation). The painted dock is given exactly the measured height, so the reservation and the painted band cannot disagree. Full-column surface min 48 / r14 iOS r12 Android / 16-12 padding, centred wrapping text, no callback during an operation. |
| Swipe | `src/ui/app/src/components/swipe.rs` | Pure `claims_horizontal` / `swipe_offset` / `swipe_day` keep claim 10 / switch 50 / edge 0.3; reduced motion drops the painted travel only. |
| Shell wiring | `src/ui/app/src/app.rs` | `safe_area()` applies exactly the inset the host platform reports for the view (no device fallback); `compact_frame` composes the one safe top with the heading's 16 px; `compact_content_bottom` keeps the dock's measured height in the content inset on both navigation dialects; the dock's measured height is computed once and handed to both the plan and the painted dock; Menüs removed from `legacy_screen_owns_bottom_inset`; `cached_menu_refresh` keeps nonempty cached cards instead of a blocking overlay. |
| Gallery | `src/ui/app/src/gallery.rs` | `mobile_menus_case` on page `mobile`: the real compact navigator, cards in every state and the mixed dock over static items. A component board, not proof of a running screen. |
| Model parity | `src/ui/model/tests/menus.rs` | `mobile_mixed_pending_submit_includes_other_days`: a queued cancellation on one day plus a new order on another are submitted by one activation with the exact mixed label. |
| Registered native test | `src/ui/ios/UITests/DesktopRegressionTests.swift` | `testDayTrayMenusDayNavigationAndMixedDock`: selects across two days, then activates the dock twice, capturing the rendered frames with bounded waits (no `sleep`). Registered, not run in this round (see row 12). |
| DESIGN extension | `docs/design/DESIGN.md` | Menüs/navigator/card/dock/swipe inventory, the measured-inset shell rule and the measured dock height in the approved mobile extension only; base inventory and Desktop Ledger retained. |

## Captures — the only rendered evidence this task has

Both images are full simulator screenshots of the offline demo fixture. No live
host, credential, cookie or broadcast was involved: the evidence store is the
in-memory dummy pair and the model switches to evidence mode before `bootstrap`.

### `after-menus-populated-light-390.png` — dispatcher capture, before this fix round

| Field | Value |
| --- | --- |
| Host | MacBook, Xcode 17C52, iOS 26.2 simulator SDK |
| Device | iPhone 17 Pro simulator, UDID `C2994587-5046-4970-8DDE-3F003EB81D0D` |
| Launch args | **not recorded** in the dispatcher handoff. The image is the offline demo core (dummy evidence pair) and its ≈22 pt gutter is the centred 390 px evidence viewport on a 402 pt screen, so an evidence run with `--viewport=390x844` is the consistent reading, not a recorded fact. |
| Scale / pixels | 3×, 1206×2622 px = 402×874 pt |
| Insets | the UIKit host applied the 62 pt status band, and the shell added the assumed 59 pt device fallback on top of it |
| Revision | `086e0b8` |
| What it shows | the two presentation defects the review found: the heading sat 78 pt below the content origin (`59 + 16` plus the host's own 62 pt band) and the navigator was a full-bleed square band with the legacy grey circle discs |

### `after-menus-fixed-light-390.png` — this fix round

| Field | Value |
| --- | --- |
| Host | MacBook, Xcode 17C52, iOS 26.2 simulator SDK; `xcrun simctl install`/`launch`/`io screenshot` |
| Device | iPhone 17 Pro simulator, UDID `C2994587-5046-4970-8DDE-3F003EB81D0D` |
| Launch args | `--evidence --viewport=390x844` |
| Scale / pixels | 3×, 1206×2622 px = 402×874 pt |
| Logical content | 390 px simulated width → 358 px column; the device centres it, so the gutters are 22 pt, not 16 (labelled as a simulated viewport, not a 390 pt device) |
| Insets | 62 pt status band and 34 pt home indicator, both applied by the UIKit host; the GPUI view reports no remaining inset, so the shell adds none (`safe_area()`) and the pill sits 8 pt above the view bottom |
| Measured in the image | content origin y = 62.0 pt; “Menüs” cap top y = 84.3 pt, i.e. heading box top 78.0 = content origin + 16; navigator group y ≈ 127→191 pt (56 pt = 4 + 48 + 4, r8, neutral boundary); app background ends at y = 840.0 pt, the last 34 pt are UIKit's own background (contract gap below) |
| Theme / dialect | Orange, light, iOS glass |
| Fixture / context | offline demo core with the dummy evidence pair; selected day `Fr., 13. Nov.` (`5 / 10`); device clock 20:33; scroll at the top; every dish shows `Geschlossen` (the demo clock is past the order cutoff) |
| What it proves | the safe inset is applied once, the navigator is a contained control with bare arrows, the dish-first card hierarchy (badge, dish, allergens above the right-aligned price) and the pill's clearance |
| What it does **not** prove | any state other than the default populated/cutoff one, any second width/dialect/theme/accent, a pending selection or dock, touch or swipe input. Those rows are `BLOCKED(host)` below — this image is not a substitute for them. |

### `after-menus-pending-dock-light-390.png` — pending dock, this fix round

| Field | Value |
| --- | --- |
| Host / device / scale / insets / theme | as above (MacBook; iPhone 17 Pro simulator UDID `C2994587-5046-4970-8DDE-3F003EB81D0D`; 3× 1206×2622 px; measured 0/0 inside the view, 62/34 consumed by UIKit; Orange light iOS glass) |
| Launch args | `--evidence --viewport=390x844 --scenario=pending` |
| Revision | as above |
| Fixture / context | offline demo core, dummy evidence pair; day `Fr., 13. Nov.` (5/10); one dish queued on the selected day; clock 20:35 |
| Measured in the image | the dock band is exactly **48.0 pt** tall (720.0→767.7 pt), its bottom edge sits 8 pt above the pill's top edge (view bottom 840.0 − 72.0), and the action fill is (184, 68, 21); the selected card shows the accent tint, the resolved 1 px boundary and the inline 24 px check slot; the `Geschlossen` badge sits above the dish and is not confusable with that check |
| What it proves | the measured dock path runs at runtime (the label is shaped, not estimated), the dock is reserved and painted at the same height, and the pending selection/check/badge projection renders |
| What it does **not** prove | a wrapped two-line label, the all-days mixed label after a cancellation, repeated-tap suppression or touch input; those stay `BLOCKED(host)` in rows 11-12 |

## Status list

| # | Item | Status | Evidence / command |
| --- | --- | --- | --- |
| 1 | `make check` (fmt, clippy, core tests, devops lib, spec validation, ui model tests) | **PASS** | `make-check-linux.log` (exit 0) |
| 2 | All-days mixed pending submit (`mobile_mixed_pending_submit_includes_other_days`, REQ-SCR-01) | **PASS** | in `make-check-linux.log`, model `menus.rs` |
| 3 | App crate type-checks with tests (`cargo check -p snackpilot-app --all-targets`) | **PASS** | `app-crate-check.log` (exit 0, no warnings) |
| 4 | App crate test execution (`cargo test --locked -p snackpilot-app --lib`) on the Mac | **PASS** | 127 passed, 0 failed (Mac, `~/vm-sync/SnackPilot-gpui`, after `make mac-sync`) |
| 5 | Menüs app tests: `mobile_menu_uses_subtitle_and_empty_fallback`, `mobile_menu_badge_precedence_and_eligibility`, `mobile_menu_card_check_slot_and_footer_order`, `mobile_menu_card_pending_cancel_and_line_cap`, `mobile_pending_dock_reserves_the_band_it_paints`, `mobile_menu_action_dispatches_once_without_extra_fetch`, `mobile_day_navigator_is_a_contained_control_with_bare_arrow_targets`, `dock_state_matches_pending_and_progress_conditions`, `dock_height_follows_the_measured_line_count_of_its_recipe`, `mobile_heading_is_inset_once_below_the_measured_safe_top`, `compact_shell_does_not_stack_a_screen_bottom_inset`, `mobile_cached_refresh_keeps_the_menu_content` | **PASS** | row 4; each carries its `@req` tag |
| 6 | The three rewritten tests fail on the behaviour they replace | **PASS** | on the Mac copy, restoring `mobile_menu_tappable(|| selected)`, the old 16 px content inset and the old disc/zero-radius navigator recipe made `mobile_menu_badge_precedence_and_eligibility`, `mobile_pending_dock_reserves_the_band_it_paints` (`left: -40.0` — the dock covered the last card) and `mobile_day_navigator_is_a_contained_control_with_bare_arrow_targets` fail; the tree was re-synced and re-run green |
| 7 | iOS simulator capture of the compact Menüs surface (default populated, Orange light, iOS glass) | **PASS** | `after-menus-fixed-light-390.png` (fields above) |
| 8 | Mac desktop workflow (GUI build/run at 360/390/720) | **BLOCKED(host)** | no interactive GUI session from this VM; `make mac-sync` + `ssh mac` + `cargo build --locked -p snackpilot-app --bin snackpilot-desktop` works, the rendered narrow/wide desktop run does not |
| 9 | iOS XCUITest run (`DesktopRegressionTests`, Menüs scenarios) | **BLOCKED(host)** | `xcodebuild test -project SnackPilot.xcodeproj -scheme SnackPilot -destination 'platform=iOS Simulator,id=C2994587-5046-4970-8DDE-3F003EB81D0D'` was not executed in this round; the test is registered |
| 10 | Android release check → debug rebuild → APK/run | **BLOCKED(host)** | no Android emulator/device attached to this host |
| 11 | Menüs matrix at 360/390 both dialects, Orange light/dark (default, long 4-line dish + allergens, selected, ordered, pending-cancel, unavailable, cutoff, empty, initial loading, cached refresh, error, mixed dock, last-card clearance) | **BLOCKED(host)** | only two 390 iOS glass light captures exist (default, pending); 360, both dark appearances, the Android flat dialect and every other state need the evidence run `--evidence --viewport=WxH --scenario=populated\|empty\|loading\|refresh\|error\|pending\|progress` plus an Android device |
| 12 | Touch/swipe action trace: select across two days, unselect, mark a cancellation, submit once during repeated taps, record exact phases | **BLOCKED(host)** | `testDayTrayMenusDayNavigationAndMixedDock` is registered but not run; the phases are asserted in the model test on this VM |
| 13 | 768 mobile (560 column, no extra day column, dock matches column) and mobile 1280 (560 cap) | **BLOCKED(host)** | same evidence run at `--viewport=768x1024` / `1280x1024`; no capture in this round |
| 14 | Gold-light and Beere-dark real mixed Menüs stress captures | **BLOCKED(host)** | needs the appearance accent set plus a rendered pending dock |
| 15 | Wide desktop Menüs row/detail/preview/actions unchanged (720/768/1280) | **BLOCKED(host)** | Mac desktop GUI run; the code path is untouched and the app tests pin it |
| 16 | ≥150 % effective type / short usable height, and 390 reduced motion on the day swap | **BLOCKED(host)** | device/simulator setting; the reduced-motion rule is unit-tested (`mobile_swipe_thresholds_and_reduced_motion`) |
| 17 | Assistive traversal (VoiceOver/TalkBack/Switch Control) | **BLOCKED(host)** | GPUI exposes no complete accessibility tree in this pinned stack; labels are text only and no coordinate tap is claimed as assistive proof |

## Configured values (source and measured)

| Item | Value | Where |
| --- | --- | --- |
| Day arrows | bare 48×48 targets, Lucide 20 | `day_navigator::mobile_day_navigator_recipe` |
| Day group | r8 option radius, projected card fill, neutral boundary ≥3:1, 4 px inset | same |
| Day centre target | min 48, headline 24/32, meta 14/20, gap 4 | same |
| Card padding / gaps / after | 16 / 8 / 8 | `menu_card`, tokens |
| Card radii | 16 iOS, 14 Android | `theme.card_surface()` |
| Dish text | body 16/24, max four lines (`mobile_dish_max_height`) | `menu_card` |
| Check slot + gap | 24 + 8, inline, never floated | `mobile_card_name_slots` |
| Badge | metadata role, r8, 8 horizontal / 4 vertical | `menu_card` |
| Dock | full column, min 48, r14/r12, 16/12 padding, 8 above navigation | `fab::mobile_dock`, `app::navigation_top` |
| Dock height | measured: shaped wrapped line count × 24 + 24 padding, floor 48 | `fab::dock_label_lines`, `dock_height_for_lines` |
| Error retry | min 44 high | `menus::mobile_error_banner` |
| Swipe | claim 10, switch 50, edge 0.3 | `swipe` |
| Shell top inset | exactly the platform's measured view inset (0 when the host already applied it) | `app::safe_area`, `compact_frame` |

## Contract gap and honesty notes

- **Contract gap (outside task 1.3's files): the 34 pt strip under the pill.**
  `src/ui/ios/Host/App.swift` anchors the GPUI view's bottom to
  `view.keyboardLayoutGuide.topAnchor`. That guide excludes the home-indicator
  band, so the last 34 pt of the window (y = 840→874 pt in the capture) is the
  container's UIKit background, not the app background, and nothing in `app.rs`
  can paint it. Fixing it needs the host file, which is not in this task's
  `files:` list: anchor the view to `view.bottomAnchor` (keeping the keyboard
  guide rule for the keyboard case) and let the shell reserve the measured
  `safeAreaInsets.bottom`, or give the container the app background.
- The shell's obstruction reservation (`app::obstruction_reservation`) is the one
  owner: navigation (only when floating) + the present dock's measured height +
  16. The plan's value now serves both navigation dialects — an in-flow bar's own
  height is already excluded from it, and the dock is painted *over* the content
  8 px above that bar, so the old "16 px only in flow" left the dock covering the
  last card on the Android dialect (that is what `mobile_pending_dock_reserves_the_band_it_paints`
  fails on when the old branch is restored).
- Menüs no longer paints its own inset, so the horizontal gutter is the shell's
  16 px (22 pt on the 402 pt device with the simulated 390 viewport) and the two
  never sum.
- A compact Menüs cached refresh keeps the cards, checks and day; the blocking
  overlay is not painted over nonempty cached content (initial load without data
  still shows it). The wide Ledger branch keeps its incumbent overlay.
- During a submit the exact phase is painted in the dock band above the busy
  scrim; the action is absent, so there is no second submit path.
- `mobile_menu_tappable` is the incumbent model gate
  `ordered || (available && !cutoff)` again: a pending selection does not
  re-enable an unavailable or closed dish, and the test pins that.
- No current-day total, count, filter, gesture, checkout or order semantic was
  added; category order and `SUPPE & SALAT` suppression are unchanged (model
  tests).
- GPUI's missing accessibility tree is reported, not worked around: the compact
  screen's names and states exist as text only.
