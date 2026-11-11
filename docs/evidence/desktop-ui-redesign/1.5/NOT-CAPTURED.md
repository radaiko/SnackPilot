# Task 1.5 evidence — status per acceptance item

Status of this directory: **no image or action trace exists**. Every item that
needs a native host is `BLOCKED(host)`. Nothing here is a PASS inferred from
compilation.

Blocker: **no native Mac session and no working Android emulator in this
session.** macOS screenshots and the native menu/keyboard/action trace require
the Mac host, and captures were explicitly out of scope for this session by
human decision. The Android emulator on this host still segfaults on launch
(KVM wedged, host reboot needed). Nothing was fabricated.

## Acceptance items

| Acceptance item | Status | Proof / blocker |
| --- | --- | --- |
| Debug-only evidence mode selects isolated data and dummy credentials and suppresses live broadcast/auth **before** bootstrap; normal credential slots are never read or written | PASS | `tests/lifecycle.rs::evidence_harness_isolates_startup_and_normal_slots` (`make check`) |
| Evidence mode sends no analytics: the wrapped real platform records no `app.launched` or other tracks, on startup and on resume | PASS | same test (`real.tracks()` empty after `bootstrap` and `on_foreground`) |
| Mobile Datenschutz alert carries the approved Aptabase paragraph only; desktop alone appends its no-statistics sentence | PASS | app test `settings::datenschutz_alert_uses_the_approved_aptabase_copy` (runs in the dispatcher's macOS app-crate run) |
| Settings containment 719 → 720 → 719: `should_create_inputs` stays false (one input instance, unsaved text kept) and the model keeps the same settings detail/service binding; phone keeps exactly one push | PASS (offline) | app test `containment_keeps_one_input_and_model_detail_across_719_720_719` plus model projection tests in `src/ui/model/tests/desktop.rs` |
| Record-list roving cursor (Up/Down/Home/End, no wrap) and stale-handle pruning after a refresh/filter/segment change | PASS (offline) | `components::record_table` tests `roving_cursor_moves_without_wrapping`, `stale_row_handle_is_dropped_when_the_render_set_changes` |
| Dialog restore gate refuses a model cursor whose row the current render no longer shows | PASS (offline) | app test `deleted_dialog_origin_restores_the_nearest_surviving_row` |
| Settings master/detail and both forms at 360/720/1280; long credential titles, 440 px form, `Einstellung auswählen`, Datenschutz dialog, notifications fallback | BLOCKED(host) | needs native desktop captures |
| Unsaved input text visibly preserved across a real 719 → 720 → 719 resize | BLOCKED(host) | needs native desktop capture; the offline gate proves only that the form is not re-entered |
| Appearance groups, notification truthfulness at every desktop width, unchanged mobile controls | BLOCKED(host) | needs native desktop and mobile captures |
| Cmd+1–4, Cmd+comma/R and the Windows Ctrl mapping, Esc precedence, Tab/Shift+Tab, focus order/rings/hover on every enabled control | BLOCKED(host) | needs the native keyboard action trace |
| Enter hands focus from a wide record row to the detail's enabled action or its read-only region | BLOCKED(host) | native keyboard behaviour; the unit test only pins the target choice, not the hand-off |
| Segment Left/Right moves one segment without wrapping and does nothing when focus is elsewhere | BLOCKED(host) | pure decision is unit-tested; the native key wiring is not proven |
| Modal: one resolution, whole-window occlusion, focus trap, Enter uses the focused action, opening key/repeat cannot confirm, Esc/backdrop cancel, refresh/navigation blocked | BLOCKED(host) | needs the native dialog trace |
| Focus restoration after a cancelled order: origin, nearest surviving row, heading fallback | BLOCKED(host) | native focus behaviour; only the decision function is unit-tested |
| Masked password (no copy/cut), Cmd+A/C/X/V and undo/redo in the username field, native Edit priority | BLOCKED(host) | needs the native Edit menu |
| Native Mac menus exactly matching the inventory, checked/enabled state, About/privacy copy, native Quit/Hide/Close, window defaults/work-area clamp/minimum, Dock reopen with one model | BLOCKED(host) | needs the native Mac session |
| Runtime About version/build from real metadata, never `0.0.0` | PASS (offline) | `desktop::tests` product version/build assertions |
| Resize with unsaved fields/pending/checks/receipt/open modal, then background/resume | BLOCKED(host) | needs the native resize/resume trace |
| Host-gated (not rail-gated) dispatch for pointer, shortcut/menu and foreground paths at 360 and 720 | PASS (offline) | model/app unit tests; the native action trace is still missing |
| `DesktopRegressionTests.swift` registered in `SnackPilotUITests.sources` and run on a concrete simulator UDID; mobile before/after comparisons | BLOCKED(host) | needs an iOS simulator or Android emulator |
| Release/debug gallery isolation remains | PASS (offline) | gallery flags and isolated evidence startup unchanged by this task |

## Required but missing

Every capture below is absent; the directory holds only this file.

- Gallery screens and states from the offline harness at 360/720/1280 plus
  390/768 checkpoints, light and dark, with content-area logical width and
  display scale recorded.
- Settings: compact and expanded master/detail, the 440 px form with long
  `Kantine-Zugangsdaten` / `Automaten-Zugangsdaten` titles, the Datenschutz
  alert, the notifications fallback, and the same unsaved text across a real
  719 → 720 → 719 resize.
- Menüs: no preview, selected long dish/allergens, independent pending rule,
  mixed submit strip, cutoff/unavailable, ordered/pending cancel,
  auth/empty/loading/cached-refresh/error/progress.
- Bestellungen: upcoming and past, one and two unconfirmed banners, preview
  plus checked row, cancel dialog/progress/disabled cutoff, compact and
  expanded tables.
- Abrechnung: both receipt sources, source/month reset, negative/zero/large
  amounts, long Gourmet lines, optional Ventopay restaurant text,
  auth/empty/loading/cached-refresh/error.
- Mobile before/after comparisons (iOS simulator or Android emulator/device) at
  360/390 and representative wide device widths.
- Keyboard-only action trace: all tabs, every visible control, both offline
  login forms, appearance groups, the destructive dialog confirmed with Enter
  and cancelled with Esc/backdrop, focus restored after a deletion.
- Native Mac menu inventory, About/privacy, Cmd+Q/Edit/View and Dock reopen.
- Modal geometry (one resolution, whole-window occlusion, short-window
  scrolling) and all ten appearance/accent combinations with focus separate
  from selection.
- Resize with unsaved fields/pending/checks/receipt/open modal, then
  background/resume.

## Inherited 1.2 evidence debt

This task inherits two explicit files from task 1.2 and does not clear them:

- `docs/evidence/desktop-ui-redesign/1.2/NOT-CAPTURED-macos.txt` — the native
  720/1280 shell, compact/expanded selected master/detail and real hover/focus
  captures excluded from that task run by human decision (`DEFERRED-APPROVED`,
  owner: this task's native interaction evidence).
- `docs/evidence/desktop-ui-redesign/1.2/STALE-after-ink-fix.txt` — the
  post-ink-fix recapture of `rail-*`, `shell-{720,768,1280}-*`,
  `controls-all-states-*` and the never-captured selected record-table rule
  (`BLOCKED(host)`, owner: this task's capture run).

Old images in `1.2/` are not current contrast proof.

## What this host could check

`make check` (fmt, clippy, core, shell tests, spec-lint, model tests) is green
here. The app-crate unit tests are **not run on this host**: linking fails on
missing `xcb`, `fontconfig`, `freetype` and `xkbcommon` system libraries. The
dispatcher runs them on macOS; its run after this round is the authoritative
count and the only proof for the offline app tests above.
