# Design: Desktop update notify, verify, hand off

## Decisions

### D1 Layering (constrains every task)

```
snackpilot-app (gpui)  --renders-->  snackpilot-model  --calls-->  snackpilot-core::update
 desktop/update.rs: macOS codesign/spctl, Finder, Windows installer spawn      Transport + Downloader + crypto
```

- The model never does HTTP (`REQ-UI-01`/`REQ-UI-08`, `src/ui/model/tests/pins.rs`). It calls `snackpilot_core::update` functions and receives `Arc<dyn Transport>`/`Arc<dyn Downloader>` services; production defaults are core's `ReqwestTransport::new(false, true)` and `ReqwestDownloader`.
- The app crate never links `reqwest`: task 1.3 adds a pin test over `src/ui/app/Cargo.toml` next to the existing model pin.
- The update API is **not** `#[uniffi::export]`ed and mobile code paths do not call it. It compiles into the core on all targets (pure Rust).
- Everything that runs a process (`codesign`, `spctl`, `open`, the installer) lives behind `Platform` hooks with default bodies, so `FakePlatform` drives the model and Android/iOS bridges compile unchanged.

### D2 Core `update` module

`src/core/src/update/`: `mod.rs` (re-exports, `UpdateError`), `version.rs`, `manifest.rs`, `key.rs`, `discover.rs`, `download.rs`, `fake.rs`.

```
pub const RELEASES_URL: &str = "https://api.github.com/repos/radaiko/SnackPilot/releases?per_page=100";
pub const DOWNLOAD_PREFIX: &str = "https://github.com/radaiko/SnackPilot/releases/download/";
pub struct HostTarget { pub platform: &'static str /* "macos"|"windows" */, pub architecture: &'static str /* "arm64"|"x64" */, pub os_version: Option<String> }
pub struct Manifest { schema, platform, architecture, version, build, minimum_os, artifact, size: u64, sha256, notes, notes_url, tag }
pub enum Discovery { UpToDate { latest: String }, Available(Candidate) }
pub struct Candidate { pub manifest: Manifest, pub artifact_url: String }
pub async fn discover(t: &dyn Transport, host: &HostTarget, installed: &str, key: &[u8; 32]) -> Result<Discovery, UpdateError>;
pub trait Downloader: Send + Sync { fn download<'a>(&'a self, url: &'a str, dest: &'a Path, max_bytes: u64, progress: &'a (dyn Fn(u64, Option<u64>) + Send + Sync)) -> Pin<Box<dyn Future<Output = Result<(), UpdateError>> + Send + 'a>>; }
pub async fn download_verified(d: &dyn Downloader, c: &Candidate, staging: &Path, progress: &(dyn Fn(u64, Option<u64>) + Send + Sync)) -> Result<PathBuf, UpdateError>;
```

- `UpdateError` is its own enum (not a `CoreError` variant, so `CoreError` and its UniFFI shape stay untouched): `Network`, `NoRelease`, `NoManifest`, `SignatureInvalid`, `ManifestMismatch`, `Unsupported`, `SizeMismatch`, `HashMismatch`, `Io`. `user_message()` returns plain German; no URL, hash or key.
- **Discovery request sequence is fixed and asserted**: (1) `GET RELEASES_URL`, (2) `GET` manifest asset URL, (3) `GET` signature asset URL. No body, no `Authorization`, no custom headers. `ReqwestTransport` already adds `Accept` and a User-Agent; do not modify it, and do not touch `http/` at all.
- Tag namespace (decision A): only `desktop/vX.Y.Z`, as `desktop.yml` ships. `desktop-ui-redesign`'s proposed `macos/v…`/`windows/v…` split is not adopted for discovery and there is no dual-namespace tolerance; if release tags are renamed, this updater stops finding releases and must be revised first.
- Tag filter: exactly `desktop/v<digits>.<digits>.<digits>`; `draft` or `prerelease` true is skipped. Compare numerically per component (`2.10.0 > 2.9.0`). Versions with other shapes are ignored, never an error.
- Asset URLs come from the release JSON `browser_download_url`; each must start with `DOWNLOAD_PREFIX` and end with `/<expected asset name>`. Verify the exact encoding of the tag segment (`desktop%2Fv…` vs `desktop/v…`) against a real release in evidence and pin it in a fixture; the prefix check deliberately stops at `download/`.
- Verification order inside `discover`: signature over the exact response bytes first, then JSON parse, then field checks (`schema == 1`, platform, architecture, `tag` equals the chosen tag, `version` equals the tag version, `version > installed`, `minimum_os <= host.os_version` when known). Unknown extra fields are ignored; a different `schema` is `Unsupported`.
- Signature file: ASCII lowercase hex of 64 bytes plus optional trailing newline. Key: `UPDATE_PUBLIC_KEY: [u8; 32]` in `key.rs`. The real value is supplied by the human before dispatch (prerequisite P2); tests use a key derived in-test from a fixed seed through the same `verify_manifest(bytes, sig_hex, &key)` function.
- `ReqwestDownloader`: its own `reqwest::Client` (https only, redirect limit 5, connect timeout 15 s, no total timeout, `tokio::time::timeout` of 30 s per chunk via `Response::chunk`), streams to `dest`, aborts when `max_bytes` is exceeded, reports progress with the `Content-Length` when present. It sends its own User-Agent `SnackPilot-desktop-update`; no file under `src/core/src/http/` changes.
- `download_verified` (pure over the trait): rejects an existing `staging` that is not an empty directory, downloads to `staging/<artifact>.part`, renames only after size and SHA-256 match, and **removes `staging` entirely on any error**. SHA-256 comparison is case-insensitive hex of the whole file.

### D3 Public key, private key, signing format

- Private key: Ed25519 PKCS#8 PEM in Forgejo secret `SNACKPILOT_UPDATE_SIGNING_KEY`, read only by the manifest step. The script writes it to a `mktemp` file with mode 600, removes it in a `trap`, never uses `set -x`, never echoes it.
- Signing: `openssl pkeyutl -sign -rawin -inkey <key> -in manifest.json` (OpenSSL 3.x; Ed25519 is deterministic), output converted to lowercase hex. The script fails with a clear message when `openssl version` is below 3.
- Public key: derived once by the human (`openssl pkey -in key.pem -pubout -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n'`), 64 hex chars, pasted into `src/core/src/update/key.rs` by task 1.1 from the value the human records in this file under "Key" below. A test pins that the compiled key is not all-zero and not the test key.
- Key rotation is out of scope: a rotated key needs an app release first. State it in the README (task 1.6).

Key (public half of the Forgejo secret, 64 hex chars): `5b030e2946e5da06d179fa786504e21f4353b93b47187b3ec7df2579fdfedb44`. The private half is stored as `SNACKPILOT_UPDATE_SIGNING_KEY` (Ed25519 PKCS#8 PEM) and was generated with OpenSSL 3.0; a sign/verify round trip with `openssl pkeyutl -rawin` confirmed the pair before it was stored.

### D4 Manifest (schema 1)

Written by `tools/devops/make-update-manifest.sh` with Python `json.dumps(obj, sort_keys=True, ensure_ascii=True, separators=(",", ":")) + "\n"`:

```
{"architecture":"x64","artifact":"SnackPilot-2.3.0-windows-x64-setup.exe","build":"42","minimum_os":"10.0.22000","notes":"…","notes_url":"https://github.com/radaiko/SnackPilot/releases/tag/desktop/v2.3.0","platform":"windows","schema":1,"sha256":"<64 hex>","size":12345678,"tag":"desktop/v2.3.0","version":"2.3.0"}
```

`minimum_os`: macOS `26.0` (the `MINIMUM_MACOS_VERSION` in `desktop/metadata.rs`), Windows `10.0.22000`. `notes` is the workflow's `dist/notes/body.md` (the trust-state paragraph; GitHub's auto-generated change list is appended later to the release body and is not in the manifest). `build` comes from the platform's `native-build-receipt.json` (read with `utf-8-sig`: PowerShell's `Set-Content -Encoding utf8` writes a BOM on Windows PowerShell 5). Output files: `dist/update/SnackPilot-manifest-macos-arm64.json`, `…windows-x64.json` and `….json.sig`.

### D5 State machine (model, `src/ui/model/src/update.rs`)

```
Idle -> Checking -> UpToDate | Available | Skipped | Failed(Check)
Available -> Downloading -> Verifying -> Ready -> HandedOff | Failed(Handoff)
Downloading|Verifying -> Failed(Download|Verify)
Failed|UpToDate|Skipped|Available|HandedOff -> Checking (explicit check)
```

- One `UpdateOp` guard (atomic) so only one check/download runs; a second request is a no-op that makes **no** core call.
- Public model API: `update_supported()`, `update_state()` (via `State.update`), `update_view() -> UpdateView`, `set_auto_update_check(bool)`, `check_for_update(UpdateTrigger::{Launch, Periodic, Manual}, now_ms)`, `start_update_download()`, `skip_update()`, `hand_off_update()`. Network work is spawned on the model runtime (`handle().spawn`), never `block_on` from the UI thread; state changes flow through the hub like every other model state.
- KV keys: `update_auto_check_enabled` (`"true"`/`"false"`; absent means true), `update_last_check_ms`, `update_skipped_version`. `update_last_check_ms` is written on every attempt, so a failing network does not retry more often than every 24 h.
- Cadence rule (pure, tested): `Launch` runs when the switch is on; `Periodic` runs when the switch is on and `now_ms - last >= 24 h`; `Manual` always runs and clears the skip. The host calls `check_for_update(Launch)` once after `bootstrap` and `Periodic` from an hourly host timer (same pattern as `AUTO_REFRESH` in `app.rs`).
- Gates, all before any network: `update_supported()` (`Platform::update_target()` is `Some`), not `evidence_mode()`, not already running.
- Windows needs no extra verification step; macOS runs `Platform::verify_update_artifact` between hash success and `Ready`.
- `update_view()` returns `{ installed: "Version 2.2.0 (41)", state_line, notes: Option<String>, notes_url: Option<String>, switch_on, actions: Vec<UpdateAction>, progress: Option<f32>, row_hint }` and carries all German copy (below), so `make check` pins it.

### D6 Platform hooks (default bodies keep mobile and Linux unchanged)

```
fn update_target(&self) -> Option<UpdateHostInfo> { None }                 // platform, arch, os_version, installed version/build
fn update_staging_dir(&self, version: &str) -> Result<PathBuf, PlatformError> { Err(unsupported) }
fn verify_update_artifact(&self, artifact: &Path, manifest: &VerifiedManifest) -> Result<VerifiedUpdate, PlatformError> { Err(unsupported) }
fn hand_off_update(&self, update: &VerifiedUpdate) -> Result<HandoffOutcome, PlatformError> { Err(unsupported) }
```

`HandoffOutcome::{QuitRequested, Revealed}`; on `QuitRequested` the host quits the app. `DesktopPlatform` returns `Some` only on macOS arm64 and Windows x64.

macOS (`src/ui/app/src/desktop/update.rs`): `/usr/bin/ditto -x -k <zip> <staging>/extracted`; exactly one top-level `SnackPilot.app`; `/usr/libexec/PlistBuddy` reads `CFBundleIdentifier` (`dev.radaiko.gourmetclient`) and `CFBundleShortVersionString` (equals manifest version); `/usr/bin/codesign --verify --deep --strict`; `/usr/bin/codesign -dv --verbose=4` output must contain `TeamIdentifier=555V4MNLK3` and an `Authority=Developer ID Application` line; `/usr/bin/xcrun stapler validate`; `/usr/sbin/spctl --assess --type execute`. A `CommandRunner` trait wraps process execution so Linux unit tests stub it; the pure output parsers are tested without macOS. On any failure the whole staging directory is deleted. Handoff: `/usr/bin/open -R <extracted>/SnackPilot.app`.

Windows: handoff is `std::process::Command::new(<staging>/<artifact>).spawn()` with no arguments (no `/S`, no `/D=`; NSIS `InstallDirRegKey HKCU "Software\SnackPilot" "InstallDir"` already makes the installer default to the user's chosen directory), then `QuitRequested`. The OS version for the minimum-OS check comes from `cmd /C ver` parsed as `10.0.<build>`; unparseable means "unknown", which does not block. Stale `snackpilot-update-*` temp directories are removed at launch.

### D7 Updates detail contract (desktop only; one screen, one task)

Surface: `Detail::Updates` maps to `SettingsDetail::Advanced`. The detail replaces `None | Some(Advanced) => None` in `desktop_settings_tab` and the placeholder path in `app.rs`. Desktop master rows gain **"Updates"** (hint: `view.row_hint`, i.e. the installed version, or "Version 2.3.0 verfügbar"); the narrow (<720) row list gains the same row with `IconName::RefreshCw`. Rows are shown only when `model.update_supported()`.

Detail body, top to bottom, inside the existing detail pane padding (`pad` 16 or 24), single column, scrolls vertically, no horizontal scroll at 360/720/1280:

1. Title "Updates" (same title style as Darstellung).
2. Row "Installierte Version": `Version {version} ({build})`.
3. Switch row "Automatisch nach Updates suchen"; hint "Beim Start und alle 24 Stunden. Es wird nur GitHub kontaktiert."
4. State line (one paragraph, wraps).
5. Notes block (Available only): heading "Neu in Version {v}", the manifest notes text, link "Alle Änderungen auf GitHub" (opens `notes_url` through the existing link handling, or shows it as text if the host has none).
6. Progress bar (Downloading only) with percent text.
7. Action row (buttons wrap): see table. Primary first.

| State | State line | Actions |
| --- | --- | --- |
| Idle | `Noch nicht geprüft.` | Jetzt prüfen |
| Checking | `Suche nach Updates …` | Jetzt prüfen (disabled) |
| UpToDate | `SnackPilot ist auf dem neuesten Stand. Zuletzt geprüft: {date}, {time}.` | Jetzt prüfen |
| Available | `Version {v} ist verfügbar.` | Herunterladen · Diese Version überspringen · Jetzt prüfen |
| Skipped | `Version {v} wird übersprungen. „Jetzt prüfen“ zeigt sie wieder an.` | Jetzt prüfen |
| Downloading | `Version {v} wird geladen … {p} %` (no total: `Version {v} wird geladen …`) | (all disabled) |
| Verifying | `Download wird überprüft …` | (all disabled) |
| Ready (Windows) | `Version {v} ist bereit zur Installation. SnackPilot wird dafür beendet.` | Installieren und beenden |
| Ready (macOS) | `Version {v} ist geprüft. Zum Installieren ziehst du die App in den Ordner „Programme“.` | Im Finder zeigen |
| HandedOff (macOS) | `Im Finder angezeigt. Ziehe SnackPilot in „Programme“ und starte die App neu. Das Update ist noch nicht installiert.` | Im Finder zeigen · Jetzt prüfen |
| Failed(Check) | `Die Suche nach Updates ist fehlgeschlagen. {reason}` | Jetzt prüfen |
| Failed(Download) | `Der Download ist fehlgeschlagen. {reason}` | Herunterladen |
| Failed(Verify) | `Das Update konnte nicht überprüft werden und wurde verworfen. Es wurde nichts installiert.` | Jetzt prüfen |
| Failed(Handoff) | `Die Installation konnte nicht gestartet werden. {reason}` | erneut Primäraktion |

`{date}`/`{time}` use the model's existing de-AT formatters; `{reason}` is `UpdateError::user_message()`. Dates and numbers keep `de-AT`. The Windows ready line is the only place the app announces it will quit.

Out of scope for this screen: confirmation dialogs (the handoff button is itself explicit), toasts, badges outside Einstellungen.

### D8 Release workflow

One new step "Sign the update manifests" in the `publish` job, after "Write the release notes" and before "Publish on GitHub". Environment: `SNACKPILOT_UPDATE_SIGNING_KEY: ${{ secrets.SNACKPILOT_UPDATE_SIGNING_KEY }}` plus `VERSION`. The step calls `tools/devops/make-update-manifest.sh` (inputs: version, `dist/desktop-macos`, `dist/desktop-windows`, `dist/notes/body.md`, output `dist/update`). The publish loop adds `dist/update/*.json` and `dist/update/*.json.sig` to `assets`, with the existing "each pattern must match" rule for the Windows manifest and signature, and for the macOS pair only when the receipt trust state is `developer-id-notarized-stapled`. The step reads the secret as an environment variable in the shell, not in an `if` (the workflow header documents that Forgejo hides secrets from expressions). Run order inside the script: compute hashes, write manifests, sign, **self-verify with the derived public key** and fail if it does not verify.

### D9 Tests and fixtures

- `docs/fixtures/update/` mirrored byte-for-byte in `src/core/tests/fixtures/update/`; `build.rs` compares `["gourmet", "ventopay", "update"]`. Fixtures: `releases-list.json` (mixed `desktop/v*`, `ios/v*`, draft, prerelease), `releases-no-manifest.json`, `manifest-windows-x64.json`, `manifest-macos-arm64.json` (bytes only); task 1.6 adds `openssl-manifest.json`, `openssl-manifest.json.sig`, `openssl-public-key.hex`, produced by the real script with a throwaway key that is then discarded.
- Tests are tagged `// @req REQ-UPD-xx`. Core tests in `src/core/tests/update.rs` and inline unit tests; model tests in `src/ui/model/tests/update.rs`; shell tests in `tools/devops/test-update-manifest.sh`, wired into `make test-lib`.
- `make check` runs core and model tests and shell tests but **not** app-crate tests; app-crate tests and the native runs are named explicitly in tasks 1.4 and 1.5.

## Risks

- Download URL shape for a tag containing `/` (D2): proven by fixture plus one real-release evidence line.
- Mac verification can pass in tests with stubs and fail on real bundles (resource forks, `._*` files — see the `--sequesterRsrc` fix in `desktop.yml`): task 1.4 requires a run against a real CI ZIP.
- `ed25519-dalek` and `sha2` compile into the mobile core artifacts too (decision C: approved, no feature flag); the size impact is small.
