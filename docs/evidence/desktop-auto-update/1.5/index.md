# Task 1.5 evidence — Updates detail, launch and hourly tick wiring, privacy sentences

Status: **app-crate tests PASS on the Mac (9 named tests plus the screen's
own 6; suite 201 passed, 2 pre-existing failures, three runs); the desktop
binary builds; `make check` PASS on Linux. Every screenshot is
`BLOCKED(host)`.** No output below is fabricated; every line is copied from
a run in this session. Evidence is text only — no screenshot or binary is in
the repository.

Hosts: Linux VM (Ubuntu 24.04) for `make check`; the MacBook (macOS, arm64) over
`ssh mac`, in `~/vm-sync/SnackPilot-gpui`, for the app-crate tests, the desktop
build, the formatting of the app crate and the native gallery run.

## App-crate tests (Mac)

All nine tests the task names:

```
test app::tests::updates_detail_round_trips_through_the_model ... ok
test app::tests::updates_row_hidden_when_unsupported ... ok
test app::tests::launch_check_runs_once_after_bootstrap ... ok
test app::tests::hourly_tick_requests_periodic_check ... ok
test app::tests::quit_requested_quits ... ok
test app::tests::buttons_dispatch_once_and_disable_while_running ... ok
test screens::settings::tests::datenschutz_mentions_github_check_only_when_supported ... ok
test screens::settings::tests::mobile_datenschutz_text_unchanged ... ok
test app::tests::settings_detail_round_trips_through_the_model ... ok
```

`settings_detail_round_trips_through_the_model` is the existing app test whose
`Advanced` assertion changed to the D7 mapping (`Advanced` ⇄ `Updates`).

The screen module's own tests (`cargo test --locked -p snackpilot-app --lib
screens::updates`):

```
test screens::updates::tests::datenschutz_sentence_only_where_supported ... ok
test screens::updates::tests::failure_rows_are_distinguished ... ok
test screens::updates::tests::only_the_installer_handoff_quits ... ok
test screens::updates::tests::updates_detail_renders_the_model_view ... ok
test screens::updates::tests::buttons_disable_while_running ... ok
test screens::updates::tests::gallery_lists_every_detail_state ... ok
test result: ok. 6 passed; 0 failed
```

The whole app-crate lib suite on the Mac, three consecutive runs:

```
run 1: test result: FAILED. 201 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
run 2: test result: FAILED. 201 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
run 3: test result: FAILED. 201 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
    app::tests::mobile_pending_dock_reserves_the_band_it_paints
    components::menu_card::tests::mobile_menu_status_chip_words_and_readable_tones
```

Both failures are the two pre-existing ones named in the brief (a mobile dock
measurement and a mobile menu-card chip test); neither file belongs to this
task and neither is touched here.

Desktop build, Mac:

```
cargo build --locked -p snackpilot-app --bin snackpilot-desktop
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

`cargo fmt -p snackpilot-app -- --check` is clean; `cargo clippy -p
snackpilot-app --all-targets` reports nothing in `screens/updates.rs` (its
remaining findings are the pre-existing ones in `app.rs`, `settings.rs`,
`gallery.rs`, `lib.rs`, `components/` and `platform/` that this task does not
touch).

## Action trace

A review found that only the first button of a row dispatched: `action_dispatch`
and the click closure compared the pressed action with `view.actions.first()`,
so "Diese Version überspringen" and "Jetzt prüfen" beside "Herunterladen", and
"Jetzt prüfen" beside "Im Finder zeigen", were inert. Fixed: both dispatch on
the clicked `action` (`action_dispatch(action, view, state)`), which is also
refused when the current frame no longer offers it.

`buttons_dispatch_once_and_disable_while_running` now presses **every** button of
the D7 rows that have actions — idle, available (Herunterladen, Diese Version
überspringen, Jetzt prüfen), skipped, ready Windows, ready macOS, handed off
macOS (Im Finder zeigen, Jetzt prüfen), failed download, failed check — each
once, on a fresh model whose state is that row, and asserts the dispatch equals
that button's own action. It then presses again while an operation runs
(`Downloading`, `Verifying`: the row offers nothing, every action refused and
the state unchanged) and a stale button of a state the page has left. With the
`first()` comparison temporarily restored the test fails:

```
test app::tests::buttons_dispatch_once_and_disable_while_running ... FAILED
panicked at app/src/app.rs:3428:
assertion `left == right` failed: available: Skip
  left: None
 right: Some(Skip)
```

The detail renders an operation's row with its buttons still in place but
disabled (`desktop_action_state` with `ControlState::Disabled`), so the row keeps
its width and the click makes no model call; the buttons a running state does
not offer at all (downloading, verifying) are absent and their old frames
dispatch nothing.

## Cadence trace

`launch_check_runs_once_after_bootstrap`: a demo model over the fake platform
(reporting the Windows host) with scripted update services; `bootstrap()`
twice, then the shell's own post-bootstrap call. Assertions: exactly **1**
discovery request, to `https://api.github.com/repos/radaiko/SnackPilot/releases?per_page=100`;
`update_last_check() == NOW`; the host target was read twice (one gate in
`update_supported()`, one in the check) and never for a second discovery; the
check ends in the contained `Failed(Check)`; `busy` stays false, `error_text`
is `None` and no location alert appears — the failure reaches nothing outside
`State.update`.

`hourly_tick_requests_periodic_check`: the first tick (no last attempt) makes
one discovery; a tick at 23 h 59 min makes none (request count unchanged,
`update_last_check()` unchanged); a tick at 24 h + 1 ms makes one more.

The tests use the app's real call sites (`screens::updates::launch_update`,
`tick_update` → `AppModel::check_for_update(Launch|Periodic, now)`); the model's
own cadence rule and switch-off case are 1.3's (`src/ui/model/tests/update.rs`,
`launch_checks_once_when_switch_on`, `launch_does_nothing_when_switch_off`,
`periodic_tick_waits_24_hours`).

## Shell wiring

- `Detail::Updates` ⇄ `SettingsDetail::Advanced` both ways
  (`model_settings_detail`, `detail_from_model`); the desktop detail pane maps
  `Advanced` to the Updates screen, no longer to the empty "Einstellung
  auswählen" prompt.
- The "Updates" row appears in the **wide master list** (from 720 on) and in the
  **narrow row list** (below 720) only while `model.update_supported()`; a
  model-less render shows neither. The narrow row list is the pushed
  `settings_tab` (the `Screen::Settings` path in `app.rs`), which a desktop
  below 720 takes; it now takes the model and appends the same row with
  `IconName::RefreshCw`, opening `Detail::Updates` like every other row. A phone
  host never reports update support, so its list is unchanged.
- `AppShell::new` calls `launch_update` once after the host's `bootstrap`, and
  spawns the hourly `UPDATE_TICK` (`1 h`) timer that calls `tick_update`, next to
  the existing `AUTO_REFRESH` timer.
- The render frame quits the app on the change into
  `HandedOff { QuitRequested }` only (`quit_after_handoff(previous, current)`
  with the shell's stored `last_update_state`), so a paint of the same state
  never quits again.
- No `reqwest` in the app crate (1.3's pin `app_manifest_lists_no_http_client`
  still passes) and no process spawning in the new code; the only process
  spawning in the crate remains 1.4's `desktop/update.rs`.

## Privacy

`docs/privacy.html`: two changes in one contiguous hunk (see
[`privacy-html.diff`](privacy-html.diff) — the amended clause and the new
paragraph are adjacent, so git emits a single hunk):
one new `<p>` as the second paragraph of section 2.3 with the approved sentence,
and the clause `Es bestehen keine Verbindungen zu sonstigen Dritten` amended to
name GitHub Releases as the single exception. No other sentence, heading or
section changes; umlauts stay HTML entities like the rest of the page.

Desktop Datenschutz text: `UPDATE_PRIVACY_SENTENCE` is appended only when
`model.update_supported()`; the mobile body is `PRIVACY_PARAGRAPH` byte for
byte (`mobile_datenschutz_text_unchanged`) and the desktop body without
support equals `privacy_message()` unchanged.

## Layout and keyboard proof

The gallery has one page for the detail: `SNACKPILOT_GALLERY_PAGE=updates`
(debug builds): every row of the D7 table — idle, checking, up to date,
available, skipped, downloading, verifying, ready (Windows), ready (macOS),
handed off (Windows), handed off (macOS), failed check, failed download, failed
verify, failed handoff — rendered from the real detail at 360, 720 and 1280
logical px in the model's geometry, light and dark. `gallery_lists_every_detail_state`
pins that each name really produces its state and a non-empty line, and that
every scenario model is offline (`update_supported()` false in evidence mode).
The app-level capture knob `--update-state=<row>` (with `--detail=updates`)
shows one such row full-window.

The native gallery run works: `SNACKPILOT_GALLERY=1
SNACKPILOT_GALLERY_PAGE=updates SNACKPILOT_GALLERY_SECTION=light
SNACKPILOT_WINDOW=720x800 ./target/debug/snackpilot-desktop` starts and renders
on the Mac (it was stopped by `timeout`). The screen still runs after the splash
frame, so the capture has to be taken once the launch mark clears.

**Captures: BLOCKED(host).** `screencapture -x` on the Mac fails with
`could not create image from display` (screen-recording permission for the SSH
process), so no pixel evidence exists for: each of the 15 rows at 360/720/1280,
light and dark; the no-horizontal-scroll check at those widths; the unclipped
focus ring; the disabled-button contrast; and the keyboard pass over the detail
(Tab switch → actions row, Space/Enter). The keyboard order is implemented (the
switch is a kit `Switch` with native Tab/Enter/Space activation; each action is
a `desktop_action` with its 44 px target and focus frame, skipped when
disabled), but it is not witnessed on screen here.

## Windows host

No Windows host exists, so the Windows half (installer handoff, the app's quit,
the dark captures on Windows) is `BLOCKED(host)`; 1.4 recorded the same for its
Windows run.

## Offline checks (this VM)

`make check` of the final tree is in [`make-check-linux.log`](make-check-linux.log)
(exit 0: core, shell helpers, spec validation `Totals: 7 passed, 0 failed`, and
the ui workspace fmt/clippy/model tests).
