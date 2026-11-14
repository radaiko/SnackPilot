# Local devops

Single `make` entrypoint at the repo root for the daily build/run loop and local releases.
Run `make help` for the target list; `make doctor` to check prerequisites.

## Build & run
- `make ios-run` — build + install + launch on a simulator. `DEVICE=<udid>` or `IOS_SIM="<name>"` to pick one.
- `make android-run` — build + install + launch on an emulator/device. `ANDROID_AVD="<name>"` to pick the AVD started when none is connected (otherwise the first AVD on the host is used).

### Android emulator on this VM

This VM is itself a KVM guest, and the emulator's nested-KVM vCPUs hang within
seconds (`detected a hanging thread 'QEMU2 CPU0'`), so `run-android.sh` starts it
with software emulation (`ANDROID_EMU_ACCEL`, default `-accel off`). Two other
things are required and are set by the script:

- `ANDROID_EMULATOR_WAIT_TIME_BEFORE_KILL` is raised (default 30 min). The
  emulator's hang watchdog otherwise kills its own vCPUs after 15 s, which a
  slow software-emulated boot routinely exceeds.
- `ANDROID_HOME=$HOME/android-sdk` and a **JDK** (not just a JRE): Temurin 17 in
  `~/.local/jdk` is picked up automatically, or install `openjdk-17-jdk` and
  export `JAVA_HOME`. The distro's `java-17-openjdk-amd64` here is JRE-only and
  fails `:app:compileDebugJavaWithJavac` with "does not provide the required
  capabilities: [JAVA_COMPILER]".

Expect a cold boot of ~5–6 minutes and an app start of ~40 s; the guest is slow
enough that Android shows "… isn't responding" dialogs. `adb shell settings put
global hide_error_dialogs 1` silences them, and the app itself renders correctly.

Build the Rust library with `--release` for emulator runs
(`bash tools/devops/ui-android-build.sh --release` before `assembleDebug`):
the debug `.so` makes the guest unusably slow.

Both rerun the platform `bootstrap.sh` only when `src/core/src` changed since the last binding build.

## Mac sync
- `make mac-sync` — rsync the current worktree to `mac:~/vm-sync/SnackPilot-gpui/` so Mac-side builds run against this checkout. `--delete` keeps it clean, but `.git`, `target`, `build`, `DerivedData`, `*.xcodeproj`, `.gradle` and `node_modules` are excluded **and protected from deletion** (they are generated on the Mac). `tools/devops/sync-mac.sh --dry-run` prints the plan; `SNACKPILOT_MAC_SRC=` / `SNACKPILOT_MAC_DEST=` override source/destination (the tests use a temp dir). Syncing to `~/vm-sync/SnackPilot` (the mutagen-managed copy) is refused.
- Non-interactive ssh on the Mac has **no user PATH**: start every remote command with `export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH`, e.g. `ssh mac 'export PATH=/opt/homebrew/bin:$HOME/.cargo/bin:$PATH; cargo --version'`.
- `bash tools/devops/test-sync-mac.sh` — unit tests for the sync script (dry-run against a temp tree, no Mac contact).

## Release (local)
- `make package-macos` — build, sign, notarize, staple and verify the macOS
  release `SnackPilot.app` (see **macOS release** below). Mac host only.
- `make ios-archive` — build a signed release archive (auto-incrementing build number, team auto-detected from the keychain) and open it in **Xcode Organizer**. Then: *Distribute App → App Store Connect → Upload* → the build lands in **TestFlight**. `IOS_TEAM=<id>` overrides the team; `ORGANIZER_OPEN=0` skips opening Organizer (CI/tests). Organizer re-signs to the App Store *distribution* identity at the Distribute step.
- `make android-keystore` — one-time: generate the signed-release keystore. Back up `src/ui/android/snackpilot-release.jks` + `keystore.properties`.
- `make ship` — one command for the next release of both platforms. Prompts for a new semver
  (must be > the last) + platforms, bumps `Cargo.toml` / `project.yml` / `build.gradle.kts` + the
  shared build number (iOS `CURRENT_PROJECT_VERSION` / Android `versionCode`, auto-incrementing),
  then:
  - **iOS** → archives a signed build into Xcode Archives and opens **Organizer** (Distribute App →
    App Store Connect → Upload → TestFlight) — same path as `make ios-archive`.
  - **Android** → signed `dist/SnackPilot-X.Y.Z.aab` + `.apk` (upload the AAB to Play).
  - Commits `Release vX.Y.Z (label)` and tags `ios/vX.Y.Z` / `android/vX.Y.Z` locally (no push —
    prints the push command).
  - `DRY_RUN=1 make ship` — validate + build to a temp dir, no git/file/counter changes, no Organizer.
  - `IOS_TEAM=<id> make ship` — override the auto-detected Apple team.
  - Note: a version can only be shipped once (tag pre-check) — the next `make ship` needs a *new*
    version. Native Android debug symbols are intentionally NOT bundled (Play rejects re-signed AABs;
    the warning is non-blocking).

See `docs/superpowers/specs/2026-07-17-v2-local-devops-design.md` for the design and the Play upload-key continuity caveat.

## macOS release

`make package-macos` is the macOS desktop release path (task 1.6). It runs only
on the Mac host, produces the signed/stapled `SnackPilot.app` (no DMG and no
public ZIP) and writes `dist/macos/native-build-receipt.json` for `make ship`.

Prerequisites, all externally configured by the human:

- macOS 26 Apple Silicon with Xcode command-line tools (`cargo`, `codesign`,
  `xcrun notarytool`, `xcrun stapler`, `ditto`, `spctl`, `lipo`, `otool`).
- The Rust target `aarch64-apple-darwin` in the pinned toolchain.
- A **Developer ID Application** identity in the login keychain, exported as
  `SNACKPILOT_SIGNING_IDENTITY` (the exact `codesign` identity string).
- An existing notarytool keychain profile, exported as
  `SNACKPILOT_NOTARY_PROFILE`. Create/update it once with
  `xcrun notarytool store-credentials <profile> --apple-id ... --team-id ...`;
  keep the Apple ID/password/team values in the password manager, never in this
  repository, a shell history file or evidence.

The script builds `--locked --release -p snackpilot-app --bin
snackpilot-desktop --target aarch64-apple-darwin` with
`MACOSX_DEPLOYMENT_TARGET=26.0`, fills `src/ui/desktop/macos/Info.plist.in`
with the shared version/build and the stable `dev.radaiko.gourmetclient`
identity, copies the `SnackPilot.icns` generated from the existing Orange icon,
verifies the arm64 slice/minimum OS/plist/assets, signs nested code then the app
with hardened runtime, submits a temporary ZIP to the notary service, requires
`Accepted`, staples and validates `codesign --verify --deep --strict`,
`stapler validate` and `spctl --assess --type execute`. A missing
identity/profile, ad-hoc identity, rejected notarization or failed verification
exits nonzero and writes no receipt; there is no dry-run or unsigned fallback.
`SNACKPILOT_PACKAGE_OUT` and `CARGO_TARGET_DIR` only relocate build output.

After packaging, install and relaunch the real artifact on macOS 26 arm64:

```sh
ditto dist/macos/SnackPilot.app /Applications/SnackPilot.app
open /Applications/SnackPilot.app
```

Verify durable state and the dedicated non-production keyring entry across a
relaunch (and after replacing `/Applications/SnackPilot.app` with a rebuilt
bundle): appearance/cache must survive and the test credential must still read
under service `app:no-auth`. Never record a real credential in evidence.

`bash tools/devops/test-package-macos.sh` runs the same script offline with
stubbed Apple/Rust tools (part of `make test-lib` and `make check`): wrong
architecture/minimum OS/plist/version/assets, ad-hoc or missing identity,
rejected notarization, failed signing/staple/Gatekeeper, changed bundle content
and any dry-run flag all fail without a receipt. Actual signature/notary,
install/relaunch and artifact-size/startup evidence are recorded under
`docs/evidence/desktop-ui-redesign/1.6/`.
