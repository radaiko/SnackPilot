# Task 1.4 evidence — desktop host: macOS bundle trust, installer handoff, staging

Status: **tests PASS on the Mac; macOS native runs (a) and (b) PASS; Windows
native run (c) is `BLOCKED(host)`.** No output below is fabricated; every line
is copied from a run in this session. Evidence is text only — no artifact,
screenshot or binary is in the repository.

Hosts: Linux VM (Ubuntu 24.04) for the stubbed tests; the MacBook (macOS 26.5.2,
build 25F84, arm64, Xcode at `/Applications/Xcode.app`) over `ssh mac`, in
`~/vm-sync/SnackPilot-gpui` (branch `task/desktop-auto-update-desktop-host`).

## App-crate tests (Mac)

`cd src/ui && cargo test --locked -p snackpilot-app --lib desktop::update`

```
running 13 tests
test desktop::update::tests::codesign_output_parser_extracts_team_and_authority ... ok
test desktop::update::tests::update_target_is_none_off_supported_hosts ... ok
test desktop::update::tests::windows_handoff_quits_after_spawn ... ok
test desktop::update::tests::macos_missing_staple_is_rejected ... ok
test desktop::update::tests::macos_sequence_runs_in_order_with_absolute_paths ... ok
test desktop::update::tests::handoff_failure_keeps_verified_files ... ok
test desktop::update::tests::macos_wrong_team_is_rejected ... ok
test desktop::update::tests::windows_handoff_spawns_installer_with_no_arguments ... ok
test desktop::update::tests::staging_dir_is_fresh_and_empty ... ok
test desktop::update::tests::macos_wrong_bundle_id_or_version_is_rejected ... ok
test desktop::update::tests::stale_staging_cleanup_only_touches_snackpilot_update_dirs ... ok
test desktop::update::tests::macos_requires_exactly_one_app_bundle ... ok
test desktop::update::tests::macos_failure_removes_staging ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 175 filtered out; finished in 0.01s
```

The same 13 tests also pass on the Linux VM (`/tmp` staging, stub runner for
every tool), so the sequence, the parsers, both handoffs and the cleanup are
not Mac-only.

The whole app-crate lib suite on the Mac:

```
running 189 tests
...
failures:
    app::tests::mobile_pending_dock_reserves_the_band_it_paints
    components::menu_card::tests::mobile_menu_status_chip_words_and_readable_tones

test result: FAILED. 187 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

Both failures are pre-existing and unrelated to this task (a mobile dock layout
measurement, `left: -56.0` vs `right: 0.0`, and a mobile menu-card chip test):
a clean copy of the branch tip on the Mac with this task's three source changes
reverted fails the same two tests, and neither file belongs to the change.
`cargo test -p snackpilot-app --lib desktop::update` — the run this task owns —
is green, and `make check` (which does not run app-crate tests, design D9) is
green.

Also on the Mac: `cargo fmt --all --check` clean, and
`cargo clippy -p snackpilot-app --all-targets -- -D warnings` reports nothing
in `desktop/update.rs`, `desktop/mod.rs` (new code) or `platform/desktop.rs`;
its only findings are pre-existing ones in `app.rs`, `lib.rs`,
`components/day_navigator.rs` and the old `on_reopen` closure in
`desktop/mod.rs`, all untouched here.

One test was added after the native runs, in `platform/desktop.rs`:
`launch_cleanup_runs_through_the_platform_hook` (it fails if the launch call
does not clear a stale `snackpilot-update-*` directory on Windows or removes
another name).

## (a) Real macOS release ZIP through `verify_update_artifact` — PASS

Artifact: `SnackPilot-2.1.0-macos-arm64.zip` from the public release
`desktop/v2.1.0` (`https://github.com/radaiko/SnackPilot/releases/download/desktop/v2.1.0/SnackPilot-2.1.0-macos-arm64.zip`),
downloaded on the Mac into a scratch `mktemp -d` directory and deleted after
the run. A temporary example binary (since deleted, never committed) called
the real `verify_macos_artifact` with `SystemRunner` — the shipped sequence of
`/usr/bin/ditto`, `/usr/libexec/PlistBuddy` (twice), `/usr/bin/codesign
--verify --deep --strict`, `/usr/bin/codesign -dv --verbose=4`, `/usr/bin/xcrun
stapler validate`, `/usr/sbin/spctl --assess --type execute`:

```
host target: Some(("macos", "arm64"))
zip: 10813164 bytes, sha256 c632eaf009753ca7a8839500dc25593baa0451cb170dd22fd68320f806ebaa7b
(a) verify_update_artifact PASS: prepared=Some("/Users/radaiko/Downloads/snackpilot-update-1.4-evidence.GIsj/extracted/SnackPilot.app") staging kept=true
```

What the shipped parsers read from that bundle:

```
Identifier=dev.radaiko.gourmetclient
Authority=Developer ID Application: Aiko Radlingmayr (555V4MNLK3)
Authority=Developer ID Certification Authority
Authority=Apple Root CA
Notarization Ticket=stapled
TeamIdentifier=555V4MNLK3
dev.radaiko.gourmetclient        # CFBundleIdentifier
2.1.0                            # CFBundleShortVersionString
```

This is the risk named in design D6 ("Mac verification can pass in tests with
stubs and fail on real bundles"): the real CI ZIP, with its signed nested code
and resource forks, passes all seven steps unchanged.

## (b) Tampered ZIP and ad-hoc re-signed ZIP — PASS

Same run.

- **One byte changed, hash and size check — re-implemented comparison, not the
  core path:** the one changed byte keeps the ZIP the same length but changes
  its SHA-256. The comparison below was done by the temporary example binary
  itself (length against the signed `size`, SHA-256 against the signed
  `sha256`); it is **not** `snackpilot_core::update::download_verified` running.
  The `remove_dir_all` that cleared the staging directory was also the example's
  own, on the same condition — so this line proves the byte change is detectable
  by the signed size and hash, not that core's `download_verified` rejects it:

  ```
  (b1) tampered download: len=10813164 signed size=10813164 hash=79272ed303165c1623747c6155fdfcb7d64d07cbf9840060dfc5c5506bf91ccf
  (b1) tampered download matches the signed size and sha256: false
  (b1) staging exists=false
  ```

  The signed value is `c632eaf0…a7b` from (a). Core's own removal of a staging
  directory on a size or hash error is `src/core/src/update/download.rs`
  (task 1.2), covered there by that task's tests, and no test in this task
  exercises it.
- **Ad-hoc re-signed bundle, team check:** the genuine bundle was extracted,
  stripped of its signature (`codesign --remove-signature`) and signed ad hoc
  (`codesign --force --deep --sign -`), repacked with `/usr/bin/ditto -c -k
  --keepParent` into a ZIP of the same name inside its own staging directory:

  ```
  (b2) ditto extract: exit status: 0
  (b2) codesign remove: exit status: 0
  (b2) codesign adhoc: exit status: 0
  (b2) stapler validate after ad hoc sign: exit status: 65
  (b2) codesign -dv: CodesignReport { team: None, authorities: [] }
  (b2) ditto pack the re-signed bundle: exit status: 0
  (b2) team check FAILED (System("codesign -dv team or authority")): staging exists=false
  ```

  The re-signed bundle still carries `CFBundleIdentifier=dev.radaiko.gourmetclient`
  and version 2.1.0 and still passes `codesign --verify --deep --strict`, so the
  sequence reaches the report step and only the missing team
  (`TeamIdentifier=not set`) and authority lines reject it; `verify_macos_artifact`
  then deletes the staging directory.

## (c) Windows installer handoff — BLOCKED(host)

No Windows host exists in this environment, so the real unsigned
`SnackPilot-2.1.0-windows-x64-setup.exe`, its start, the app's exit and the
installer's default directory from `HKCU\Software\SnackPilot\InstallDir` for a
non-default install are **not proven here**. Nothing is inferred from the fact
that the Windows code path compiles. Covered offline by
`windows_handoff_spawns_installer_with_no_arguments` and
`windows_handoff_quits_after_spawn` (stubbed spawn point: exactly one spawn, the
verified path, no argument, `QuitRequested` only after the spawn, a typed
failure and kept files when the spawn fails).

## Acceptance items

| Item | Status | Proof |
| --- | --- | --- |
| `update_target` is `Some` only for macOS arm64 and Windows x64, no network | PASS | `supported_target()` is pure `cfg!` (`Some(("macos", "arm64"))` on the Mac, `None` on Linux); `update_target_is_none_off_supported_hosts` |
| `update_staging_dir`: macOS `~/Downloads/SnackPilot-{version}/`, fresh and empty | PASS (unit test) | `staging_dir_is_fresh_and_empty` proves the `~/Downloads/SnackPilot-{version}/` shape and its freshness against an injected home. The native (a)/(b) runs did **not** call `update_staging_dir`; they used a scratch `mktemp -d ~/Downloads/snackpilot-update-1.4-evidence.XXXX` directory they created themselves |
| `verify_update_artifact`: macOS sequence of REQ-UPD-05/06 via `CommandRunner`, absolute tool paths, staging deleted on any failure | PASS | `macos_sequence_runs_in_order_with_absolute_paths`, `macos_wrong_team_is_rejected`, `macos_missing_staple_is_rejected`, `macos_wrong_bundle_id_or_version_is_rejected`, `macos_failure_removes_staging` (every step, and a runner that cannot start a tool), `macos_requires_exactly_one_app_bundle`; native runs (a) and (b) |
| Windows `verify_update_artifact` only checks the staged installer | PASS (stubbed) | `windows_handoff_spawns_installer_with_no_arguments` (a foreign name is discarded) |
| `hand_off_update`: Windows spawn once, no arguments, then `QuitRequested`; macOS `/usr/bin/open -R`, then `Revealed` | PASS (macOS path stubbed; Windows BLOCKED(host)) | `windows_handoff_spawns_installer_with_no_arguments`, `windows_handoff_quits_after_spawn`, `handoff_failure_keeps_verified_files` |
| Failed handoff keeps the verified files | PASS | `handoff_failure_keeps_verified_files` |
| Stale `snackpilot-update-*` removed at launch on Windows; nothing under `~/Downloads` deleted except a failed staging | PASS | `stale_staging_cleanup_only_touches_snackpilot_update_dirs` and `launch_cleanup_runs_through_the_platform_hook` (driven through `desktop::launch_update_cleanup`, the one call `desktop/mod.rs` adds); the cleanup is a no-op off Windows; the only `remove_dir_all` of a staging directory is the failed verification and the creation-time reset of the version's own staging |
| No `reqwest`, no new crate, no FFI, no `/S`, no `/D=` | PASS | only `std::process`, `std::fs`, `std::io` are used; the app manifest is unchanged |

## Offline checks (this VM)

`make check` output of the final run is in [`make-check-linux.log`](make-check-linux.log) (exit 0).
