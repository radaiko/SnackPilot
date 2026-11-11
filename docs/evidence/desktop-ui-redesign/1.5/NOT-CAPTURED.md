# Task 1.5 evidence — NOT CAPTURED

Status: **BLOCKED** for native capture. No image or action trace in this
directory is a substitute for the required evidence; nothing here is a PASS.

Blocker: **host reboot / native Mac run required.** The Android emulator on
this host segfaults on launch (KVM wedged, host reboot needed); macOS
screenshots and the native menu/keyboard run require the Mac host, and captures
were explicitly out of scope for this session by human decision. Nothing was
fabricated.

## Required but missing

Desktop gallery (offline, debug build) at 360/720/1280 plus 390/768 checkpoints,
light and dark, display scale and content width annotated:

- settings master/detail at 720 and 1280, long `Kantine-Zugangsdaten` /
  `Automaten-Zugangsdaten` titles, the 440 px form, `Einstellung auswählen`;
- the same form at 360 (pushed, back affordance) and the same input text across
  719 → 720 → 719 (unsaved username, no re-prefill);
- Datenschutz alert at 360/720/1280 with the desktop-only no-statistics
  paragraph;
- the debug/stale notifications route fallback
  (`Benachrichtigungen sind auf diesem Gerät nicht verfügbar.`) at every desktop
  width and its absence from the settings master;
- every appearance/accent combination with keyboard focus separate from
  selection, wrapping at compact detail width;
- focused/hovered/disabled states for rail rows, settings rows, privacy,
  form fields, Save/Logout, dialog actions and day controls;
- desktop confirm dialog (Bestellung stornieren / Behalten / Stornieren)
  occluding rail and workspace, focus ring on the confirm, short-window scroll.

Native Mac action trace (keyboard only):

- Cmd+1–4 and Cmd+comma/R through all tabs, active-tab no-op, refresh once;
- Esc precedence: dialog → batch → preview/settings detail → nothing, with
  pending menu changes intact;
- Tab/Shift+Tab, rail Up/Down/Home/End, settings and appearance group arrows,
  Enter/Space activation, Return submitting Save once;
- masked password copy/cut unavailable, Cmd+A/C/X/V/undo/redo in the username
  field;
- both offline dummy credential forms (dummy values only), appearance groups,
  destructive dialog confirmed with Enter, cancelled with Esc and backdrop,
  Tab trapped inside the modal, refresh/navigation blocked while it is open,
  focus restored after dismissal;
- resize with unsaved fields, pending menu changes, selected record/receipt and
  an open modal; background/resume;
- real Cmd+Q/About/Edit/Darstellung/Fenster menus, About version/build and the
  GourmetClient credit, Quit/Hide/Close native semantics, Dock reopen showing
  one session.

Mobile before/after screenshots and the `DesktopRegressionTests` run
(`xcodebuild test -destination 'platform=iOS Simulator,id=<UDID>'`) are also
missing. The test file is now part of the `SnackPilotUITests` target's explicit
`project.yml` source list, so it needs a real simulator run on a Mac before its
result can be claimed.

## Known gaps carried in this handoff

- **Record-list roving focus.** Wide Menüs/Bestellungen/Abrechnung rows already
  paint the desktop focus ring and are pointer targets, but Up/Down/Home/End do
  not move a list cursor between them: every row is its own tab stop. Wiring
  the roving cursor needs the list owners to hand each row a shell-owned
  `FocusHandle`, and those calls live in `screens/{menus,orders,billing}.rs`,
  which are outside task 1.5's file allowlist (the settings master, rail and
  appearance groups already do exactly this from `app.rs`).
- **Dialog focus restore after a deletion.** The shell now records the record
  whose control held focus and, when the confirmed flow deleted it, restores
  the screen heading instead of the unmounted handle. Landing on the *nearest
  surviving row* needs the same row handle as above; the model already
  reconciles the list cursor to that row.

## What this host could check

`make check` (fmt, clippy, core, shell tests, spec-lint, model tests) and
`cargo check -p snackpilot-app --lib/--tests` type-check the desktop code and
its unit tests. App test **linking** fails here on missing `xcb`, `fontconfig`,
`freetype`, `xkbcommon` system libraries, so app unit tests are not executed on
this host.
