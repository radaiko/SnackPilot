# Local devops

Single `make` entrypoint at the repo root for the daily build/run loop and local releases.
Run `make help` for the target list; `make doctor` to check prerequisites.

## Build & run
- `make ios-run` — build + install + launch on a simulator. `DEVICE=<udid>` or `IOS_SIM="<name>"` to pick one.
- `make android-run` — build + install + launch on an emulator/device. `ANDROID_AVD="<name>"` to pick the AVD started when none is connected.

Both rerun the platform `bootstrap.sh` only when `src/core/src` changed since the last binding build.

## Release (local)
- `make ios-archive` — build a signed release archive (auto-incrementing build number, team auto-detected from the keychain) and open it in **Xcode Organizer**. Then: *Distribute App → App Store Connect → Upload* → the build lands in **TestFlight**. `IOS_TEAM=<id>` overrides the team; `ORGANIZER_OPEN=0` skips opening Organizer (CI/tests). Organizer re-signs to the App Store *distribution* identity at the Distribute step.
- `make android-keystore` — one-time: generate the signed-release keystore. Back up `src/android/snackpilot-release.jks` + `keystore.properties`.
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
