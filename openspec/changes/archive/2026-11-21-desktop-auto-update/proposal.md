# Proposal: Desktop update notify, verify, hand off

domain: architectural

## Why

Desktop users (macOS 26 Apple Silicon, Windows 11 x64) install SnackPilot from GitHub Releases and have no way to learn that a newer build exists. The v1 Velopack updater was retired. This change adds a small, honest updater: it notices a newer signed release, shows its notes, downloads and verifies the artifact, then hands installation to the platform. It never replaces its own running process and adds no native FFI.

It is `architectural` because it introduces a trust boundary (a compiled-in public key and a CI-held private key), the first non-scraping network path in the core, and a new dependency pair.

## What Changes

- **Core (`snackpilot-core`, new `update` module, plain Rust API, not exported through UniFFI):** semver ordering, manifest parsing, Ed25519 verification, GitHub release discovery over the existing `Transport` trait, and a streaming verified download behind a new narrow `Downloader` trait (the `Transport` response body is a `String` and cannot carry a binary installer). Fakes (`CapturingTransport`, `FakeDownloader`) keep every test offline.
- **Model (`snackpilot-model`):** the update state machine, cadence (launch, then every 24 h, manual), the persisted switch and skipped version, German copy and action policy as a pure projection, and `Platform` hooks with default bodies so mobile hosts are untouched. No HTTP in the model; no `reqwest` in the app crate.
- **Desktop host (`snackpilot-app`):** macOS bundle trust checks and Finder reveal; Windows installer handoff and quit; staging directories.
- **UI:** a new "Updates" settings detail on the existing `Advanced` slot (desktop only).
- **CI:** one new step in the `publish` job of `.forgejo/workflows/desktop.yml` plus `tools/devops/make-update-manifest.sh`, producing `SnackPilot-manifest-{macos-arm64,windows-x64}.json` and `.json.sig` from the real artifacts, signed with `openssl` (Ed25519) using the Forgejo secret `SNACKPILOT_UPDATE_SIGNING_KEY`.

### Out of scope

Silent or background installation, delta updates, auto-restart into the new version, a second distribution channel, mobile updates, key rotation or revocation, Linux, ETag caching, an update badge outside Einstellungen.

### Assumptions (confirm before dispatch)

1. The artifact is exactly what `desktop.yml` already uploads: `SnackPilot-{version}-macos-arm64.zip` (a `ditto --keepParent` ZIP holding `SnackPilot.app`) and `SnackPilot-{version}-windows-x64-setup.exe` (NSIS, per user, unsigned).
2. The GitHub release list is read page 1 only (`per_page=100`); desktop releases stay within the newest 100 releases of the repository.
3. The switch defaults to **on** (human decision D), and an automatic check contacts only `api.github.com`/`github.com`.
4. A missing signing key fails the publish; an unsigned macOS build simply gets no macOS manifest.
5. The signature file is lowercase hex of the raw 64-byte Ed25519 signature; the public key is compiled in as 32 raw bytes written as hex. This avoids `base64`, `hex` and `semver` crates.
6. The macOS staging directory is user-visible (`~/Downloads/SnackPilot-{version}/`) because the user drags the app; the Windows staging directory is a fresh directory under the OS temp directory.
7. `desktop-ui-redesign` tasks 1.6 and 1.7 land first; this change edits files they also touch (`desktop/mod.rs`, `platform/desktop.rs`).
8. Settings carries the only notification surface (the row hint shows "Version X verfügbar"); no rail or toast badge.

## Capabilities

### New Capabilities

- `desktop-update`: discovery, manifest and artifact integrity, macOS bundle trust, honest handoff, state machine and cadence, the Updates detail, signed manifests in the release, disclosure and the desktop requirement document (`REQ-UPD-01`–`REQ-UPD-21`, one behaviour each).

### Modified Capabilities

None in `openspec/specs/`. `REQ-DST-01`/`REQ-DST-04` (still an in-flight delta of `desktop-ui-redesign`) say automatic updates are not promised; this change supersedes that wording and the human reconciles it when that change is archived (prerequisite P1 in `tasks.md`). No delta is written against them because they are not yet in `openspec/specs/`.

### Human decisions recorded

- **A** Discovery reads only `desktop/vX.Y.Z` tags, as shipped. `desktop-ui-redesign`'s proposed `macos/v…`/`windows/v…` tags are not adopted for discovery, with no dual-namespace tolerance; if that change renames release tags, this one breaks and must be revised first.
- **B** `docs/requirements/08-gpui-desktop.md` is amended inside this change (task 1.7): the published artifacts and the updater replace "no public ZIP" and "no updater".
- **C** `ed25519-dalek` 2.x (no default features) and `sha2` 0.10 are approved in `snackpilot-core`, with no feature flag, compiled for every consumer.
- **D** The automatic check defaults on; `docs/privacy.html` gets one factual sentence about the GitHub request (task 1.5).

## Impact

- Code: `src/core/src/update/**` (new), `src/core/build.rs`, `src/core/Cargo.toml`, both lockfiles; `src/ui/model/src/{update.rs,platform.rs,fake.rs,state.rs,session.rs,lib.rs}`; `src/ui/app/src/{desktop,platform,screens,app.rs}`; tools, workflow, README.
- Dependencies (named on task 1.1, approved): `ed25519-dalek` 2.x and `sha2` 0.10 only.
- Security: the update path never touches the scraping clients; `src/core/tests/wire_audit.rs` and every Gourmet/Ventopay request shape stay unchanged. The only hosts contacted are GitHub's. No credential, cookie or token is sent, logged or stored.
- Docs: `docs/requirements/08-gpui-desktop.md` (task 1.7, approved) and `docs/privacy.html` (one added paragraph, task 1.5).
- Privacy: the desktop Datenschutz text and `docs/privacy.html` state the GitHub request; the mobile text is unchanged.
- Verification: `make check` for core and model; app-crate tests, the macOS trust run against a real release ZIP and the Windows handoff need the Mac and Windows hosts and are evidence tasks, not inferred from compilation.
