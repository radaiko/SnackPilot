# 1.4 — Bestellungen controls, cards, cancellation and batch parity

- **Change:** `mobile-ui-rework`, task 1.4
- **Branch:** `task/1.4-day-tray-orders`
- **Revision:** working tree from `main` (task not committed at capture time).
- **Approval context:** A9/A10 plus the shared approvals in 1.2, approved by the
  human on 2026-11-13. No domain, cutoff or copy amendment: every visible label
  and every model intent is the incumbent one. The screen-owned legacy
  `.p(16)`/`.pb(100)` removal and the `Screen::Orders` entry in
  `legacy_screen_owns_bottom_inset` land with this task, as the dispatcher note
  requires.

## What this task changed

| Area | File(s) | Change |
| --- | --- | --- |
| Compact Bestellungen screen | `src/ui/app/src/screens/orders.rs` | Compact segment bar, stacked confirm banner with a full-width 48 px `Bestätigen`, inline error, scrolling cards, body-role empty texts, 48 px `Auswählen` target and the wrapping counted batch strip. The legacy 16/100 px screen padding is gone; the root is one `flex_1` child of the shell column. The confirm/cancel handlers, single-flight guard, 30 s cutoff cadence and dialog copy are unchanged. |
| Cancellation dialog | `src/ui/app/src/screens/orders.rs` | `cancel_dialog(order, on_confirm)` is the one projection the mobile and desktop handlers use: `Bestellung stornieren`, `"{order.title}" stornieren?`, `Stornieren`, `Behalten`. |
| Compact order card | `src/ui/app/src/components/order_item.rs` | 16 px padding / 8 px gaps / 8 px after, card radii, date+category in the readable metadata role 4 px apart and wrapping, description in the body role with the incumbent `order_description` fallback, status badge at r8 on its semantic surface, separate 44×44 trailing cancel target (neutral rest fill, resolved boundary, readable danger `X` 20, visible-but-inert past the cutoff, 16 px spinner in the same slot while cancelling, no card opacity), and the status/cancel pair moving under the dish below the 12-character dish measure (`mobile_order_actions_below`). Selection mode keeps the whole-card check in a reserved 44 px area; past/read-only rows have no action and no tap. `MobileOrderTrailing` / `mobile_order_trailing` / `mobile_order_card_tappable` are the one projection the builder paints. |
| Compact segments | `src/ui/app/src/components/segment_tabs.rs` | Two equal min-48 targets, body Semibold labels whose counts wrap, `mobile_segment_rule` reserving the 3 px rule in both states, accent rule with primary ink when selected and the readable secondary projection when not. `desktop_segment_toolbar` is unchanged. |
| Shell | `src/ui/app/src/app.rs` | `legacy_screen_owns_bottom_inset` no longer lists `Screen::Orders`; the shell's measured reservation is the screen's one bottom inset, and the in-flow batch strip sits inside that content box above navigation. |
| Gallery | `src/ui/app/src/gallery.rs` | `mobile_orders_case` on page `mobile`: the real compact segment bar (12/9 counts), the stacked confirm banner and cards for confirmed / longest unconfirmed dish / cancelling / cutoff / read-only past / selected check. A component board, not proof of a running screen. |
| Model parity | `src/ui/model/tests/orders.rs` | `batch_cancel_covers_zero_one_and_many_checks`: zero dispatches nothing and tracks nothing, one is one `order.cancelled` count 1, many is one event with the full count. |
| DESIGN extension | `docs/design/DESIGN.md` | Bestellungen inventory (segments, card, banner, strip) plus the screen's legacy-padding removal; base inventory and Desktop Ledger retained. |

## Captures

One full simulator screenshot of the offline demo fixture. No live host,
credential, cookie or broadcast was involved: the evidence store is the
in-memory dummy pair and the model switches to evidence mode before `bootstrap`.
The demo core carries **no order rows** (the demo clock is past the order
cutoff), so this image is the default/empty Bestellungen state, not a populated
one; the missing populated/interaction rows are listed as `BLOCKED` below rather
than recreated.

### `after-orders-default-light-390.png`

| Field | Value |
| --- | --- |
| Host | MacBook, Xcode 17C52, iOS 26.2 simulator SDK; `xcrun simctl install` / `launch` / `io screenshot` |
| Device | iPhone 17 Pro simulator, UDID `C2994587-5046-4970-8DDE-3F003EB81D0D` |
| Launch args | `--evidence --viewport=390x844 --tab=1`, with `SNACKPILOT_EVIDENCE=1` |
| Scale / pixels | 3×, 1206×2622 px = 402×874 pt |
| Logical content | simulated 390 px width → 358 px column; the device centres it, so the gutters are 22 pt, not 16 (a simulated viewport, not a 390 pt device) |
| Insets | 62 pt status band and 34 pt home indicator, both applied by the UIKit host; the GPUI view reports no remaining inset, so the shell adds none |
| Measured in the image | content origin y = 62.0 pt; “Bestellungen” cap top y = 85.3 pt (heading box top 78.0 = origin + 16); segment bar y ≈ 126→174 pt with the 3 px/3 pt selected rule at y = 171.0→174.0 pt painted only on the active segment; equal targets; “Auswählen” ink y = 729.7→741.3 pt inside its 48 pt target above the pill; app background ends at y = 840.0 pt |
| Theme / dialect | Orange, light, iOS glass |
| Fixture / context | offline demo core, dummy evidence pair; `Kommende (0)` / `Vergangene (0)`; device clock 20:56; scroll at the top |
| What it proves | the shell heading and centring, the compact segment geometry (equal targets, reserved 3 pt rule, primary ink on the active segment, readable secondary on the inactive one), the body-role empty text with the exact string, the `Auswählen` target clearing the pill, and the removed 16/100 px legacy inset |
| What it does **not** prove | any card, banner, batch strip or dialog state, any second width/dialect/accent, dark appearance, touch or keyboard input. Those rows are `BLOCKED` below |

Commands:

```sh
make mac-sync
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH && cd ~/vm-sync/SnackPilot-gpui && bash src/ui/ios/bootstrap.sh'
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH && cd ~/vm-sync/SnackPilot-gpui/src/ui/ios && xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot -sdk iphonesimulator -destination "generic/platform=iOS Simulator" build'
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH
  UDID=C2994587-5046-4970-8DDE-3F003EB81D0D
  APP=~/Library/Developer/Xcode/DerivedData/SnackPilot-dvwgztggdojzqlcervvorssxtmtj/Build/Products/Debug-iphonesimulator/SnackPilot.app
  xcrun simctl install $UDID "$APP"
  SNACKPILOT_EVIDENCE=1 xcrun simctl launch $UDID dev.radaiko.gourmetclient --evidence --viewport=390x844 --tab=1
  xcrun simctl io $UDID screenshot orders-390.png'
```

## Status list

| # | Criterion | Status | Evidence / command |
| --- | --- | --- | --- |
| 1 | `make check` (fmt, clippy, core tests, devops lib, spec validation, ui model tests) | **PASS** | `make-check-linux.log` (exit 0) |
| 2 | Mac app-crate tests `cargo test --locked -p snackpilot-app --lib` | **PASS** | `app-crate-tests-mac.log` — 132 passed, 0 failed |
| 3 | `mobile_order_cancel_target_and_readable_states` (REQ-UI-15/16): past row has no action, cutoff keeps an inert target, in-flight owns the slot, selection mode is the check, and the painted danger ink/boundary/disabled pair read on all five accents × light/dark × glass/flat | **PASS** | `app-crate-tests-mac.log` |
| 4 | `mobile_order_actions_move_below_the_dish_at_large_type` (REQ-UI-15) and incumbent description fallback | **PASS** | `app-crate-tests-mac.log` |
| 5 | `mobile_confirm_and_batch_strip_wrap_without_dual_dispatch` (REQ-SCR-02/16): counted copy, ready only with checks and no running flow, one gate claim, exactly one `order.cancelled` with the count, no second dispatch | **PASS** | `app-crate-tests-mac.log` |
| 6 | `mobile_order_description_and_exact_cancel_dialog` (REQ-SCR-02): exact four strings, two actions, index 0 keeps the order, index 1 cancels | **PASS** | `app-crate-tests-mac.log` |
| 7 | `mobile_segments_reserve_the_rule_and_equal_targets` (REQ-SCR-02/16) | **PASS** | `app-crate-tests-mac.log` |
| 8 | Model `batch_cancel_covers_zero_one_and_many_checks` (REQ-SCR-02); existing single-flight, day cutoff, past-segment exit, preview-vs-check independence tests retained | **PASS** | `make-check-linux.log` |
| 9 | `compact_shell_does_not_stack_a_screen_bottom_inset` with Bestellungen on the measured reservation (REQ-UI-03/15) | **PASS** | `app-crate-tests-mac.log` |
| 10 | Default state at the simulated 390 px iOS width, light Orange | **PASS** | `after-orders-default-light-390.png` |
| 11 | Populated upcoming/past rows, confirmed/unconfirmed 1/2/long count, cancelling, cutoff, selection zero/one/many and the exact modal confirm/cancel on a native host | **BLOCKED(scenario)** | `--scenario=populated --tab=1` republishes the demo split, which carries **0 upcoming / 0 past** orders; the demo clock is past the order cutoff, so no order can be placed in evidence mode either. No orders fixture exists in `apply_evidence_scenario` (`src/ui/model/src/lifecycle.rs`). Owner: 1.1 fixture set / 1.12 audit; not waived. |
| 12 | Dark appearance, Android flat dialect, 360 and 768 widths, large type, keyboard, reduced motion | **BLOCKED(host)** | `xcrun simctl ui C2994587-5046-4970-8DDE-3F003EB81D0D appearance dark` followed by the launch above: the simulator reports `dark`, but the app still renders the light palette (stored `System` preference or the host appearance trait; not diagnosed here). No Android emulator or 360/768 point simulator on this host. Commands: `bash tools/devops/ui-android-build.sh --release`, then `bash tools/devops/ui-android-build.sh` and `./gradlew :app:assembleDebug --no-daemon` in `src/ui/android`, install/launch on a dedicated emulator; `--viewport=360x844` / `--viewport=768x1024` for the width rows. |
| 13 | Pre/at 09:00 cutoff and local-midnight split asserted in `src/ui/model/tests/orders.rs` | **BLOCKED(contract)** | `AppModel::new` builds `SnackPilotCore` from `CoreConfig`, which has no clock field (`src/core/src/ffi.rs`), so a model test cannot set a fixture clock; a host-clock test would be the forbidden “host clock hack”. The boundary itself is pinned in `src/core/src/datetime/formats.rs` (`is_ordering_cutoff`: before 09:00 open, at/after 09:00 blocked). The model tests cover the day-level cutoff, the segment split and the past-segment exit without mutation. Needs a fixture-clock seam in a later specification, not a hidden core edit. |
| 14 | Wide Desktop Ledger table/detail/preview and batch-key policy unchanged | **PASS** | `desktop_orders_tab`, `desktop_order_row`, `desktop_order_detail`, `desktop_segment_toolbar`, the desktop handlers and the desktop tests are untouched; `make check` and the app-crate suite cover the wide regression tests. Desktop 720/1280 captures were not taken this round (owner 1.12). |
| 15 | iOS bottom-strip contract gap carried from 1.3 | **BLOCKED(host)** | The app background ends at y = 840.0 pt, the last 34 pt are UIKit's own background (`src/ui/ios/Host/App.swift` contract gap recorded in 1.3). Not introduced here and not waived; carried by 1.12. |
