# v2 Local Devops — Design

> Local build/run/ship tooling for the v2 native apps (SwiftUI iOS + Compose Android over
> the shared Rust core). Scope: the daily dev loop as one command, plus a local release
> pipeline that produces installable/uploadable artifacts. Explicitly *not* cloud CI or
> store-upload automation.

## Goal

Two capabilities, one coherent tool:

1. **One-command build & run** per platform — bootstrap core bindings (only when stale),
   build, install, and launch on a simulator/emulator, replacing the current multi-step
   `bootstrap.sh` + manual `xcodebuild`/`gradlew`.
2. **Local ship/release pipeline** — a v1-`ship.sh`-equivalent adapted to native: bump the
   single shared semver across all three version-carrying files in one commit, tag
   `ios/vX.Y.Z` / `android/vX.Y.Z` locally, and produce signed installable artifacts
   (`.ipa` / `.aab` / `.apk`) into `dist/`.

Grounded in `docs/requirements/07-release.md` (§1 identities, §2 versioning, §3 per-store
process). The tag-triggered CI that §3–§5 anticipate does not exist yet; this pipeline
stops at local commit + tag + artifact and prints the push command rather than pushing.

## Constraints & context

- **Signing today:** iOS uses Xcode automatic signing (already set up). Android has no
  keystore (v1 shipped via EAS-managed signing). This tool generates a fresh local Android
  release keystore for sideload/testing artifacts.
- **Play continuity caveat (out of scope, flagged):** v1 shipped through EAS with Play App
  Signing — Google holds the app-signing key, EAS held the upload key. A brand-new local
  keystore builds fine for sideload, but pushing a v2 update *over v1* on Play later
  requires recovering the EAS upload key or resetting the upload key via Play Console. That
  is a store-side step for publish time; it does not block local work.
- **Version-carrying files** (bumped together, spec §2):
  - `src/core/Cargo.toml` → `version`
  - `src/ios/project.yml` → `MARKETING_VERSION` (user-facing) + `CURRENT_PROJECT_VERSION` (build number)
  - `src/android/app/build.gradle.kts` → `versionName` (user-facing) + `versionCode` (build number)
- **Store identities MUST NOT change** (spec §1): bundle/application id
  `dev.radaiko.gourmetclient`, display name `SnackPilot`. The tool never touches these.
- **Do not alter scraping logic** (CLAUDE.md). This tool is build orchestration only; it
  touches no `src/core/src` request code.

## Layout

```
Makefile                      # v2 repo root — single entrypoint; `make help` is the default target
tools/devops/
├── lib.sh                    # shared: colors/logging, prereq checks, version read/write, staleness check
├── run-ios.sh                # bootstrap-if-stale → build → install → launch on a simulator
├── run-android.sh            # bootstrap-if-stale → installDebug → launch on emulator/device
├── ship.sh                   # interactive release pipeline
├── android-keystore.sh       # one-time: generate release keystore + keystore.properties
└── .build-number             # gitignored monotonic build-number counter
dist/                         # gitignored — built .ipa / .aab / .apk land here
```

The Makefile is a thin dispatcher; all non-trivial logic lives in the `tools/devops/*.sh`
scripts so each has one clear purpose and can be read/run on its own. The existing
`src/ios/bootstrap.sh` and `src/android/bootstrap.sh` remain the source of truth for
core-binding regeneration; the run/ship scripts call them rather than duplicating them.

## Targets

| Target | Purpose |
|---|---|
| `make help` | List targets with one-line descriptions (default target) |
| `make doctor` | Verify tools present: xcodegen, xcodebuild, cargo, cargo-ndk, `ANDROID_NDK_HOME`, adb, keytool |
| `make ios-run` | Build + install + launch on a booted simulator (auto-boot a default if none; `DEVICE=` override) |
| `make android-run` | Build + install + launch on a running emulator/device (auto-start default AVD if none) |
| `make android-keystore` | One-time: generate `snackpilot-release.jks` + write gitignored `keystore.properties` |
| `make ship` | Interactive release pipeline (below); `DRY_RUN=1` and `METHOD=` supported |
| `make clean` | Remove `dist/` and generated build artifacts |

## Staleness check (shared)

`lib.sh` exposes a helper that reruns a platform `bootstrap.sh` only when any file under
`src/core/src` is newer than the built binding artifact
(`src/ios/Frameworks/SnackPilotCore.xcframework` for iOS, the newest
`src/android/app/src/main/jniLibs/*/libsnackpilot_core.so` for Android). This keeps
`ios-run` / `android-run` fast on the common case where only shell/UI code changed, while
still rebuilding the core when it actually changed. A forced rebuild is always available by
running the platform `bootstrap.sh` directly.

## Device selection

- **iOS:** default to the currently booted simulator. If none is booted, boot a configured
  default (env `IOS_SIM`, default a recent iPhone simulator) and wait for boot. `DEVICE=`
  overrides the target. If the default is unavailable, fail with guidance rather than
  guessing.
- **Android:** default to the running emulator / attached device reported by `adb devices`.
  If none, start a configured default AVD (env `ANDROID_AVD`) and wait for `sys.boot_completed`.

## `make ship` flow

Interactive, adapted from v1 `ship.sh` (spec §3.1):

1. Read the last version from gitignored `.ship-history` (last line; `0.0.0` if absent).
   Prompt for a new version; require strict semver `^[0-9]+\.[0-9]+\.[0-9]+$` and
   strictly-increasing vs the last (compared with `sort -V`).
2. Prompt platform selection: iOS / Android / both.
3. Bump the user-facing version in all three files in one pass (kept in sync even for a
   single-platform ship, per spec §2). Each edit is anchored and verified to have changed
   exactly one line; a miss aborts the ship before any commit.
4. Increment the `.build-number` counter and set iOS `CURRENT_PROJECT_VERSION` +
   Android `versionCode` to it (spec §2: v2 reimplements auto-incrementing build numbers
   locally now that EAS is gone).
5. Build the selected platforms' artifacts into `dist/`:
   - **iOS:** rerun `src/ios/bootstrap.sh` (picks up the new `project.yml`), then
     `xcodebuild archive` → `xcodebuild -exportArchive` with an export-options plist.
     Default export method `development` (installs on registered devices via the existing
     Xcode automatic signing); `METHOD=app-store` switches to a store `.ipa` for TestFlight
     when distribution signing is ready. Output: `dist/SnackPilot-<version>.ipa`.
   - **Android:** `./gradlew bundleRelease` (`.aab` for Play) + `assembleRelease` (`.apk`
     for sideload), signed with the keystore. Output:
     `dist/SnackPilot-<version>.aab` + `dist/SnackPilot-<version>.apk`.
6. Commit the bumped files as `Release v<version> (<label>)` where `<label>` is a
   comma-separated subset of `ios,android` (spec §2 format, minus desktop). Create one
   local tag per selected platform: `ios/v<version>` / `android/v<version>`. Append the
   version to `.ship-history`. **Do not push** — no CI consumes the tags yet; print the
   `git push && git push --tags` command for when the user chooses to.

**`DRY_RUN=1`**: perform version validation and the build into a temp dir, but make no file
edits, no commit, no tag, and no `.ship-history` / `.build-number` mutation — so the whole
pipeline is safe to exercise end-to-end before a real ship.

## `make android-keystore`

Runs `keytool -genkeypair` non-interactively (RSA 2048, 10000-day validity, a fixed
`-dname`), generating `src/android/snackpilot-release.jks` with a strong randomly-generated
password, and writes the gitignored `src/android/keystore.properties`
(`storeFile`/`storePassword`/`keyAlias`/`keyPassword`) that `app/build.gradle.kts` already
reads. Refuses to overwrite an existing keystore. Prints a prominent warning to back up the
`.jks` + `keystore.properties` (losing them means no future update to the same signing
identity). `.gitignore` already blocks `keystore.properties`/`*.jks`/`*.keystore`.

## Error handling

- Every script is `set -euo pipefail`; `lib.sh` provides `info`/`ok`/`err` logging.
- `doctor` (and each run/ship script's preamble) checks required tools up front and fails
  with an actionable message (what to install) rather than a deep stack trace.
- `ship` validates version + verifies each file edit *before* any git mutation; a failure
  in the build phase leaves the working tree with the bumped files but no commit/tag, so it
  can be retried or reverted cleanly.
- Missing Android keystore → `assembleRelease`/`bundleRelease` produce an unsigned artifact
  (existing gradle behavior); `ship` warns and points at `make android-keystore`.

## Verification

Devops scripts are integration-verified by running them, not unit tests:

- `make doctor` — reports tool availability truthfully on this machine.
- `make ios-run` — launches the app on a simulator; confirm it runs (demo path).
- `make android-run` — installs + launches on the emulator; confirm it runs.
- `make ship DRY_RUN=1` (both platforms) — exercises validation + both builds into a temp
  dir, produces the artifacts, makes no git/file mutations (verify `git status` clean and
  `.ship-history` unchanged afterward).
- `make android-keystore` on a throwaway path — verify it generates the keystore + a valid
  `keystore.properties`, then that `assembleRelease` produces a *signed* apk.

## Out of scope

- GitHub Actions CI (core tests, PR compile checks, tag-triggered store builds) — spec
  §4–§5; a later, separate effort.
- TestFlight / Play Store upload automation (`xcrun altool` / Play Developer API).
- The Play upload-key continuity resolution (store-side, at publish time).
- Icon-generation pipeline (`tools/icon-tools`, spec §6) — already handled; unrelated.
- Privacy-policy carry-over (spec §7) — tracked separately in the v2 status.
```
