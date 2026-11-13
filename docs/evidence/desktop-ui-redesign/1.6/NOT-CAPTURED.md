# Task 1.6 evidence — macOS packaging, durable state and capability gate

Status of this directory: **offline (Linux) implementation evidence only.** No
Mac native run happened in this session; every signature, notarization,
staple, install, relaunch and measurement item below is `BLOCKED(host)` and is
not a PASS inferred from compilation. No output was fabricated.

Blocker: this task ran on the Linux VM. The dispatcher owns `make mac-sync`, the
Mac release build, Developer ID/notarization, the installed artifact, appearance
and keyring continuity, and the size/startup measurements below.

## Acceptance items

| Acceptance item | Status | Proof / blocker |
| --- | --- | --- |
| `SnackPilot.app` is release-built `--locked` for `aarch64-apple-darwin` with `MACOSX_DEPLOYMENT_TARGET=26.0`, stable bundle ID, icon/assets/runtime closure and common version/build | BLOCKED(host) | build recipe and validation are implemented in `tools/devops/package-macos.sh`; the real Mac release build has not run |
| Info.plist, bundle identity, version/build, icon and arm64-only slice are validated | PASS (offline script contract) | `tools/devops/test-package-macos.sh` (wired into `make test-lib`/`make check`) with stubbed tools |
| About/rail consume release metadata without placeholders | BLOCKED(host) execution | `src/ui/app/src/desktop/metadata.rs` tests `release_metadata_is_not_a_placeholder`, `placeholders_are_filtered_from_release_metadata`; the app crate type-checks on Linux but its tests can only link/run on the Mac/desktop host |
| Durable host data resolves macOS Application Support (and Windows `%LOCALAPPDATA%`) with no temp/install fallback; missing/unwritable home fails truthfully; installer replacement preserves data | BLOCKED(host) execution | `src/ui/app/src/desktop/storage.rs` tests `macos_uses_application_support_under_home`, `windows_uses_local_app_data`, `missing_host_variables_fail_closed`, `unwritable_location_fails_truthfully`, `existing_data_survives_install_and_relaunch`, `read_only_directory_is_not_writable`; app-crate tests run on the Mac host |
| Keyring service/slots untouched; dedicated dummy entry survives relaunch/update | BLOCKED(host) execution (mock/real) | `src/ui/app/src/platform/desktop.rs` test `dummy_keyring_entry_survives_relaunch_and_update`; real installed-artifact continuity is also BLOCKED(host) |
| Capability gate avoids unavailable notification/geofence calls during desktop bootstrap without changing mobile startup order | PASS (offline) | `src/ui/model/tests/lifecycle.rs` tests `bootstrap_gates_unavailable_desktop_capabilities`, `bootstrap_keeps_mobile_capability_order` |
| Sign nested code and outer app with Developer ID + hardened runtime, notarize a temporary ZIP, require Accepted, staple and validate trust; no ad-hoc or dry-run fallback | PASS (offline script contract) / BLOCKED(host) for real trust | offline stage-order and fail-closed tests in `test-package-macos.sh`; the real identity, submission, staple and Gatekeeper assessment are BLOCKED(host) |
| Common native-build receipt with `macos`/`aarch64`/version/build/path/verified trust and the `sha256-bundle-manifest-v1` checksum | PASS (offline script contract) | `test-package-macos.sh` receipt fields, fallback checksum and tamper test; no real Mac receipt exists yet |
| No DMG or public ZIP, temporary notary ZIP deleted | PASS (offline script contract) | `test-package-macos.sh` filesystem assertion after the stubbed run |
| `08-gpui-desktop.md` describes the shipping matrix, Desktop Ledger, durable data, menus/state, capability gaps, analytics/accessibility and the measurement method truthfully | PASS (documentation) | updated `docs/requirements/08-gpui-desktop.md`, dated prior measurements retained, Mac measurement explicitly pending |
| Actual installed artifact on macOS 26 arm64: `file`/`lipo`/`otool`/plist inspection, `codesign --verify --deep --strict`, signing/options check, `xcrun stapler validate`, `spctl --assess --type execute`, install/launch | BLOCKED(host) | needs the Mac host and the external Developer ID/notary configuration |
| Appearance/cache and dedicated dummy-keyring continuity through relaunch/update | BLOCKED(host) | needs the installed Mac artifact |
| Artifact size and timed launch-to-first-frame with host/method | BLOCKED(host) | measurement method is documented in `08`; no number has been recorded, so none is claimed |

## Required native evidence (absent)

- Mac release command output: locked `aarch64-apple-darwin` build, package,
  `codesign --verify --deep --strict`, `codesign -dv` Developer ID/runtime,
  `xcrun stapler validate`, `spctl --assess --type execute`.
- Installed `/Applications/SnackPilot.app` launch on macOS 26 Apple Silicon,
  with a screenshot/first-frame capture and the appearance/cache continuity
  check across relaunch and bundle replacement.
- Dummy keyring entry round trip across that relaunch/update using only the
  dedicated non-production account (never a real credential).
- `native-build-receipt.json` from the real artifact and its recomputed
  bundle-manifest checksum.
- `du -sh`/`du -sk` artifact size and 60 fps launch-to-first-frame timing, with
  macOS version, CPU and method.

## Offline checks (this VM)

`make-check-linux.log` is the full `make check` output of this task run
(2026-11-13): core fmt/clippy/tests, shell helper tests, the offline macOS
package contract, OpenSpec validation and the model UI tests, all green.

The shell package contract runs in `make check` via `make test-lib`. The Rust
storage/metadata tests are in the `snackpilot-app` crate and run on the Mac
desktop-app test run (`cargo test --locked -p snackpilot-app --lib`), because
`make ui-check` intentionally covers only the model crate; the model capability
tests run in `make check` via `make ui-check`.
