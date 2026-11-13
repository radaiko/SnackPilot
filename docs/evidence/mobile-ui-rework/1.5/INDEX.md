# 1.5 — Abrechnung filters, summary and read-only receipts

- **Change:** `mobile-ui-rework`, task 1.5
- **Branch:** `task/1.5-day-tray-billing`
- **Revision:** working tree from `main` (task not committed at capture time).
- **Approval context:** A11/A12 and A17 plus the shared approvals in 1.2,
  approved by the human on 2026-11-13. No cache, fetch, merge, totals, formatter
  or month-identity redesign: every value and intent is the incumbent one.
  The screen-owned legacy `.p(16)`/`.pb(100)` removal and the `Screen::Billing`
  entry in `legacy_screen_owns_bottom_inset` land with this task, as the
  dispatcher note requires.
- **Fixture:** the offline demo core behind the isolated evidence store
  (`SNACKPILOT_EVIDENCE=1`, dummy credential pair, network/auth/broadcast
  suppressed before bootstrap). Month `2026-10` carries 32 receipts
  (22 Kantine, 10 Automaten), subsidy `€ 33,00`. No live host, credential,
  cookie jar or broadcast was touched; the demo timestamps are synthetic
  (Kantine receipts at `00:00`).

## What this task changed

| Area | File(s) | Change |
| --- | --- | --- |
| Compact Abrechnung | `src/ui/app/src/screens/billing.rs` | Month choices (three equal min-48 targets, body-16 full core labels wrapping to a second line, accent rule + readable accent ink when selected), source options on the shared `chip::chip` recipe (min 44/r8/12 px side padding/8 gap/wrapping/check 20 + tint), one `banner_surface` summary (`Gesamt` metadata above the body value, then a wrapping `Belege`/conditional `Zuschuss` metadata line 16 apart), a neutral 16 px `Aktualisiere...` cached-refresh row, the quiet centred spinner 36 for an absent selected-context cache, the inline exact model error and the receipt list. The month/source controls stay painted during loading. The legacy `.p(16)`/`.pb(100)` is gone and the root is one `flex_1` child of the shell column. |
| Selected-context guard | `src/ui/app/src/screens/billing.rs` | `billing_context_matches`/`billing_context_cached` compare the published `GourmetMonthlyBilling`/`VentopayMonthlyBilling` `month_key`s with the selected `MonthOption.key`; a month switch still in flight publishes the previous month, so its rows are withheld (`BillingView::Initial`) instead of painted under the new label. A missing/failed/signed-out source is not a mismatch and simply contributes nothing, so the other source's total stays truthful. `billing_owns_progress` keeps the shell's blocking overlay off the compact surface, so the quiet initial spinner36 and the cached-refresh row are the only progress feedback there. |
| Compact receipt | `src/ui/app/src/components/bill_card.rs` | `card_surface`, padding16, gap8, 8 px after the card. Date body Semibold, time and the neutral uppercase `KANTINE`/`AUTOMATEN` metadata on one wrapping line, amount trailing in the body role `TEXT_PRIMARY`, neutral hairline, Kantine lines with the 28 px quantity metadata column, full wrapping description and trailing metadata amount, and the supplied non-empty Automaten restaurant only. Both sources are `ReceiptInteraction::ReadOnly`: no pointer affordance (`.cursor_default()`), no tab stop, no handler. `mobile_receipt_lines` never invents an item for Automaten. |
| Pull refresh | `src/ui/app/src/components/pull_refresh.rs` | `min_h(0)` only, so the wrapper shrinks inside the shell's measured content box. The 60 px trigger is unchanged. |
| Shell | `src/ui/app/src/app.rs` | `legacy_screen_owns_bottom_inset` no longer lists `Screen::Billing` (the shell's measured reservation is the screen's one bottom inset), and `billing_owns_progress` keeps the blocking loading overlay off the compact Abrechnung surface (its own quiet spinner36 or neutral refresh row paint instead). The wide Ledger overlay and every other surface keep the incumbent behaviour. |
| Gallery | `src/ui/app/src/gallery.rs` | `mobile_billing_case` on page `mobile`: the real month row, source row, summary surface and cards over static receipts (long Kantine description, large credit, Automaten with and without a restaurant line) plus the exact empty text. A component board, not proof of a running screen. |
| Model / format pins | `src/ui/model/tests/billing.rs`, `src/ui/model/tests/format.rs` | `mobile_summary_matches_existing_filtered_totals` (3 months × 3 filters against the stored source months, `format_eur` strings, month containment), `mobile_billing_partial_source_preserves_truthful_context` (selected-month identity, one-service truth, no previous-month rows under a new label), `mobile_billing_samples_keep_the_incumbent_formatter` (full dates/months, zero/negative/large/sub-cent EUR). Formatter source untouched. |
| Swift | `src/ui/ios/UITests/DesktopRegressionTests.swift` | `testDayTrayBillingMonthsSourcesAndReceipts` (billing scenarios only) launches the isolated 390-wide viewport, taps the second month and then the Kantine/Automaten options, and captures each frame. |
| DESIGN extension | `docs/design/DESIGN.md` | Abrechnung inventory (months, options, summary, card, loading/refresh, context guard) and the screen's legacy-padding removal; base inventory and Desktop Ledger retained. |

## Captures

All four are the same running screen, isolated synthetic fixture, Orange light,
iOS glass dialect. Host: MacBook, Xcode 17C52, iOS 26.2 simulator SDK; iPhone
17 Pro simulator UDID `C2994587-5046-4970-8DDE-3F003EB81D0D`, physical scale 3×,
device 402×874 pt. The launch requests `--viewport=390x844`, so the app lays out
a simulated 390 pt width (`358` px column) centred on a 402 pt device — the
gutter is 22 pt on screen, not 16: a **simulated** width, labelled as such. Safe
insets are applied by the UIKit host (62 pt status band, 34 pt home indicator);
the GPUI view reports no remaining inset, so the shell adds none.

| File | Scenario | What it proves |
| --- | --- | --- |
| `after-billing-default-light-390.png` | `--evidence --viewport=390x844 --tab=2`, month `November 2026`, filter `Alle` | shell heading, three month targets, wrapping full labels, the option recipe with the check + tint on `Alle`, the one summary (`Gesamt € 57,40`, `Belege 20`, `Zuschuss € 15,00`), Automaten cards with the neutral uppercase source, the supplied restaurant line, the trailing body amount, and the removed 16/100 px inset (the last card ends above the floating pill). |
| `action-second-month.png` | after tapping `Oktober 2026` | the selected rule/label move to the second choice, the context switch republishes that month: `Gesamt € 110,00`, `Belege 32`, `Zuschuss € 33,00`, newest-first Kantine receipts with the 28 px `1x` column and line amounts — the previous month is never painted under the new label. |
| `action-kantine.png` | after tapping `Kantine` | the check/tint move to `Kantine`; `Gesamt € 94,80`, `Belege 22`, subsidy still `€ 33,00`; only Kantine receipts remain. |
| `action-automaten.png` | after tapping `Automaten` | check/tint on `Automaten`; `Gesamt € 15,20`, `Belege 10`, **no** `Zuschuss` line (Kantine side filtered out); only Automaten receipts, including one without a restaurant line (header only). |

Approximate measured bands in the 1206×2622 px captures (3×; pt = px / 3):
content origin y = 62.0 pt; `Abrechnung` cap top ≈ 86 pt; month row ≈ 130→192 pt
(two-line labels inside the ≥48 pt target); source row ≈ 201→240 pt (44 pt
targets); summary card ≈ 253→375 pt (padding 16); first receipt card top ≈ 390 pt;
iOS pill band starts ≈ 790 pt, the last visible card ends above it. These are
visual measurements on the capture, not layout assertions.

Commands:

```sh
make mac-sync
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH && cd ~/vm-sync/SnackPilot-gpui && bash src/ui/ios/bootstrap.sh'
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH && cd ~/vm-sync/SnackPilot-gpui/src/ui/ios && xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot -sdk iphonesimulator -destination "generic/platform=iOS Simulator" build'
ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH && cd ~/vm-sync/SnackPilot-gpui/src/ui/ios && xcodebuild test -project SnackPilot.xcodeproj -scheme SnackPilot -destination "platform=iOS Simulator,id=C2994587-5046-4970-8DDE-3F003EB81D0D" -only-testing:SnackPilotUITests/DesktopRegressionTests/testDayTrayBillingMonthsSourcesAndReceipts'
```

The registered test reported `Executed 1 test, with 0 failures` and wrote the
four screenshots to `/tmp/desktop-regression-15/` on the Mac; they are copied
here as the files above, from the final revision (the last code change only
touched which surface owns the loading overlay, which no capture shows).

## Defects found while capturing (fixed inside this task's files)

1. **Render panic in the first billing build.** `month_select`/`source_select`
   read the shell entity (`shell.read(cx)`) while `billing_tab` was still being
   rendered, so every Abrechnung frame aborted with
   `cannot read snackpilot_app::app::AppShell while it is already being
   updated`. They now take the model that `billing_tab` already holds. The
   Abrechnung screen had never been rendered on a device since the shell rework,
   so no test covered it; this is verified by the running captures above (a
   render-path entity read is not reachable from a `cargo test --lib` unit
   test).
2. **The registered XCUITest file did not compile.** A pre-existing
   `for _ in 0..2` (Swift needs `0..<2`) in the Menüs test from task 1.3 made
   `SnackPilotUITests` fail to build, so no registered test had actually run.
   Fixed with the one-character operator correction; the purpose-restricted
   style change is recorded here as a contract observation for the 1.12 audit.
3. **Month labels overflowed** the first capture: the row wrapped its children
   at their intrinsic width. Now three equal `flex_1` targets with a
   `w_full/min_w_0` label, so a full `September 2026` takes a second line
   instead of being clipped — visible in `action-second-month.png`.

## Status list

| # | Criterion | Status | Evidence / command |
| --- | --- | --- | --- |
| 1 | `make check` (fmt, clippy, core tests, devops lib, spec validation, ui model tests) | **PASS** | `make-check-linux.log` (exit 0) |
| 2 | Mac app-crate tests `cargo test --locked -p snackpilot-app --lib` | **PASS** | `app-crate-tests-mac.log` — 134 passed, 0 failed |
| 3 | `mobile_billing_controls_survive_loading_and_context_change` (REQ-SCR-03/15/16): filters survive a loading state, a context cache shows the neutral refresh, a mismatch is withheld, the shell exemption applies to Abrechnung only, the summary equals the filtered totals and the subsidy stays conditional | **PASS** | `app-crate-tests-mac.log` |
| 4 | `mobile_receipt_lines_wrap_and_remain_read_only` (REQ-SCR-03/15/16): full Kantine lines, no invented Automaten item, optional restaurant, read-only for both sources, neutral uppercase labels, 28 px quantity column, incumbent EUR formatter | **PASS** | `app-crate-tests-mac.log` |
| 5 | Model `mobile_summary_matches_existing_filtered_totals` and `mobile_billing_partial_source_preserves_truthful_context` (REQ-SCR-03) | **PASS** | `make-check-linux.log` |
| 6 | `format.rs` regression samples (REQ-UI-06), formatter source untouched | **PASS** | `make-check-linux.log` |
| 7 | `compact_shell_does_not_stack_a_screen_bottom_inset` with Abrechnung on the measured reservation (REQ-UI-03/15) | **PASS** | `app-crate-tests-mac.log` |
| 8 | Three months, three sources, subsidy present (Kantine/All) and absent (Automaten), mixed source receipts, default state at the simulated 390 iOS width, light Orange | **PASS** | the four captures above |
| 9 | `testDayTrayBillingMonthsSourcesAndReceipts` on the simulator (month switch, then Kantine, then Automaten) | **PASS** | the four captures above; `Executed 1 test, with 0 failures` |
| 10 | Initial loading (quiet spinner 36 over still-present filters), cached refresh (neutral 16 row over records/totals) and error-with-cache on a native host | **BLOCKED(scenario)** | a phone-host fetch is synchronous inside the activation, so neither loading frame is reachable on the simulator here, and no evidence scenario publishes `billing_loading` or a billing error (`apply_evidence_scenario` has no billing branch; adding one belongs to the 1.1 fixture set). The projection is asserted by test 3 — including that the shell's blocking overlay stays off this surface. Owner: 1.1 fixture set / 1.12 audit; not waived. |
| 11 | Dark appearance, Android flat dialect, 360/768/1280 and large-type/keyboard matrices | **BLOCKED(host)** | only one 402×874 iPhone 17 Pro simulator is available; no 768/1280-point simulator device and no Android emulator/device in this session (same limitation recorded by 1.3/1.4). Owner: 1.12 evidence audit. |
| 12 | Empty month, auth gate only when both services are signed out, both/one-service auth, failed-source error | **BLOCKED(host, scenario)** | an empty month needs a month with no receipts (the demo months all carry rows) and the gate needs a signed-out launch; both are scenario/fixture work owned by 1.1/1.12. The gate code path itself is unchanged. |
| 13 | Wide Desktop Ledger unchanged at 720/768/1280 and narrow desktop 360/390 | **BLOCKED(host)** | same simulator/device limitation as row 11; the desktop composition (`desktop_billing_tab`, table, detail, summary line) is untouched by this diff and its app tests still pass. |
| 14 | Numeric contrast on the rendered fills (text ≥4.5:1, boundaries ≥3:1) | **PASS (projections)** | the mobile ink/boundary helpers are the tested 1.1/1.2 tokens (`mobile_secondary_on`, `mobile_ink_on`, `mobile_status_on`, `selection_*`); no new colour is introduced. A per-pixel measurement of these captures is not claimed. |
| 15 | Accessibility tree / VoiceOver traversal | **BLOCKED(host)** | GPUI renders without an accessibility tree (documented since 1.1 and 1.2); the cards install no handler by construction, so no focus node is fabricated. Owner: 1.12 audit |
