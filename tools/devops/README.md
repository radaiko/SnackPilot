# Local devops

Single `make` entrypoint at the repo root for the daily build/run loop and local releases.
Run `make help` for the target list; `make doctor` to check prerequisites.

## Build & run
- `make ios-run` — build + install + launch on a simulator. `DEVICE=<udid>` or `IOS_SIM="<name>"` to pick one.
- `make android-run` — build + install + launch on an emulator/device. `ANDROID_AVD="<name>"` to pick the AVD started when none is connected.

Both rerun the platform `bootstrap.sh` only when `src/core/src` changed since the last binding build.

## Release (local)
- `make android-keystore` — one-time: generate the signed-release keystore. Back up `src/android/snackpilot-release.jks` + `keystore.properties`.
- `make ship` — prompt for a new semver + platforms, bump `Cargo.toml` / `project.yml` / `build.gradle.kts` (+ build number), build artifacts into `dist/`, commit `Release vX.Y.Z (label)`, and tag `ios/vX.Y.Z` / `android/vX.Y.Z` locally. It does **not** push (no CI consumes the tags yet) — it prints the push command.
  - `DRY_RUN=1 make ship` — validate + build into a temp dir, no git/file changes.
  - `METHOD=app-store make ship` — iOS export for TestFlight (default `development`).
  - `IOS_TEAM=<id> make ship` — iOS signing team auto-detects from keychain; set this to override.

See `docs/superpowers/specs/2026-07-17-v2-local-devops-design.md` for the design and the Play upload-key continuity caveat.
