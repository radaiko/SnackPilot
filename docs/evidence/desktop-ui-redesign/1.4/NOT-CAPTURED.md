# Not captured: task 1.4 Bestellungen and Abrechnung GUI evidence

Blocker: the Android emulator on this host is broken (segfaults and a wedged
KVM device) and this Linux VM cannot link the app crate at all (the dispatcher
and the macOS host do that build). **host reboot required.** No real GUI frame
could be produced, so nothing below exists and no image was copied from another
task or fabricated.

Covered instead:

- `src/ui/model/tests/orders.rs` — `order_preview_does_not_cancel_or_check`,
  `checkbox_toggles_once_without_preview_change`,
  `past_segment_exits_batch_without_cancel`,
  `cancelled_preview_restores_nearest_cursor`.
- `src/ui/model/tests/billing.rs` — `billing_context_clears_hidden_preview`,
  `same_date_amount_receipts_keep_distinct_ids`.
- `src/ui/app/src/app.rs` — `order_row_effect_separates_body_and_checkbox`,
  `order_cancel_state_precedence`,
  `desktop_cancel_claims_once_and_dispatches_one_cancel`,
  `desktop_submit_claims_once_and_dispatches_one_submit`.
- `src/ui/app/src/components/order_item.rs` — `order_columns_match_the_brief`,
  `order_description_falls_back_to_subtitle_then_title`,
  `order_status_words_remain_visible`.
- `src/ui/app/src/components/bill_card.rs` — `bill_columns_match_the_brief`,
  `bill_source_labels_match_the_brief`.
- `src/ui/app/src/components/record_table.rs` — `table_date_formats_de_at_day`,
  `row_space_toggles_only_in_selection_mode`.
- `src/ui/app/src/screens/orders.rs` — `cutoff_repaint_runs_only_on_desktop_cadence`.
- `src/ui/app/src/gallery.rs` — the offline Bestellungen and Abrechnung
  scenarios render the real `desktop_order_*` / `desktop_bill_*` /
  `desktop_summary_line` helpers with static records at 720 and 1280, light and
  dark, so they can be captured as soon as a host with a working window is
  available.

## Required shots, still outstanding

- `orders-720-<light|dark>-none.png` — no preview: `Datum` 80 / `Bestellung`
  rows with category over status, detail prompt `Bestellung auswählen`.
- `orders-1280-<light|dark>-selected.png` — `Datum` 104 / `Bestellung` flex /
  `Status` 104, selected row rule and the full dish plus `Stornieren` in the
  detail.
- `orders-1280-preview-vs-check.png` — one row selected and one row checked in
  the same selection-mode frame: the cursor tint and the 20×20 checkbox are
  visibly different.
- `orders-720-selection-wrap.png` — 44 px check column at compact width with the
  `Bestellung` cell wrapping instead of truncating, and the strip
  `Stornieren ({n})` / `Fertig`.
- `orders-1280-past.png` — `Vergangene ({n})`: no `Auswählen`, no cancel action.
- `orders-1280-cutoff.png` — disabled `Stornieren` with
  `Stornierung nicht mehr möglich.`
- `orders-1280-cancelling.png` — affected row/detail spinner with
  `Wird storniert`; other cancellation entry points inert.
- `orders-1280-unconfirmed.png` — `1 unbestätigte Bestellung` /
  `{n} unbestätigte Bestellungen` with `Bestätigen`.
- `billing-720-<light|dark>-none.png` — month toolbar, source chips, compact
  wrapping summary line, `Datum / Quelle` above `Betrag` 80, prompt
  `Beleg auswählen`.
- `billing-1280-<light|dark>-selected.png` — `Datum` 112 / `Quelle` flex /
  `Betrag` 96, tabular trailing amounts, `Beleg` detail with long date,
  `HH:mm`, source badge and semibold amount.
- `billing-1280-gourmet-lines.png` — Gourmet detail with `{count}x`, full
  wrapping descriptions and trailing line totals at 28/80.
- `billing-1280-ventopay.png` — Ventopay detail with only the non-empty
  restaurant description.
- `billing-1280-zero-negative.png` — zero and negative amounts aligned without
  clipping.
- `billing-1280-empty.png`, `billing-1280-loading.png`,
  `billing-1280-cached-refresh.png`, `billing-1280-error.png` — honest empty
  state, initial spinner only without entries, cached receipts during refresh,
  inline model error.
- `orders-360-before.png`, `billing-390-before.png` — the unchanged phone
  cards/segments/selection strip and month tabs/chips/`BillCard` path, for the
  before/after comparison.
