# Not captured: task 1.3 Menüs GUI evidence

Blocker: the Android emulator on this host is broken and needs a host reboot to
recover, and this Linux VM cannot build the app crate at all (the dispatcher and
the macOS host do that build). No real GUI frame could be produced, so nothing
below exists and no image was copied from another task or fabricated.

Covered instead:

- `src/ui/model/tests/menus.rs` — `desktop_menu_preview_is_read_only`,
  `desktop_menu_explicit_toggle_matches_mobile`,
  `desktop_submit_includes_pending_other_days` (model semantics).
- `src/ui/app/src/app.rs` — `menus_rows_preview_on_desktop_and_toggle_on_phone`,
  `desktop_mutations_block_while_dispatched` (menu-handler wiring).
- `src/ui/app/src/gallery.rs` — the offline Menüs scenario renders the real
  `desktop_menu_row` / `desktop_menu_detail` / `desktop_submit_action` helpers
  with static items at 720 and 1280, light and dark, so it can be captured as
  soon as a host with a working window is available.

## Required shots, still outstanding

- `menus-1280-<light|dark>-none.png` — no preview: grouped rows with the detail
  prompt `Menü auswählen` and no action.
- `menus-1280-<light|dark>-selected-long.png` — selected long dish: full dish
  text, `Allergene: …`, original price string, row selection rule and detail
  action.
- `menus-720-<light|dark>-selected.png` — the same selection at the compact
  width, including the wrapped mixed submit label.
- `menus-<720|768|1280>-pending-rule.png` — an independently pending row
  (`✓ Vorgemerkt`) beside a selected but unpending one: selection is not a
  pending change.
- `menus-1280-mixed-submit.png` — all-days submit strip with
  `Änderungen bestätigen (n)` and a pending cancellation in the same frame.
- `menus-1280-cutoff-unavailable.png` — `Geschlossen` and `Ausverkauft` rows
  with their missing actions, still selectable for preview.
- `menus-1280-ordered-pending-cancel.png` — `Bestellt` with
  `Stornierung vormerken` and `Wird storniert` with `Stornierung aufheben`,
  cancellation row struck through.
- `menus-1280-auth.png`, `menus-1280-empty.png`, `menus-1280-loading.png`,
  `menus-1280-cached-refresh.png`, `menus-1280-error.png`,
  `menus-1280-progress.png` — the toolbar/banner states; cached rows stay
  readable during a refresh, an error keeps `Erneut versuchen`, and a running
  operation shows progress with all mutation entry points inert.
- `trace-menus-row-activation.txt` — action trace of one row activation: no
  toggle, no submit, no refresh, only preview plus cursor.
- `trace-menus-explicit-toggle.txt` — action trace of the detail toggle and of
  one batch submit: exactly one model call each, no duplicate while dispatched.
- `mobile-360-before.png`, `mobile-390-before.png`, `mobile-390-after.png` —
  the unchanged phone card/FAB/day-swipe path, pending FAB included, for the
  before/after comparison.
