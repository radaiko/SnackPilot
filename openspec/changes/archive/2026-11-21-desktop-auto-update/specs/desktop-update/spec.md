# Desktop update delta

## Purpose

Defines how the macOS and Windows desktop apps learn that a newer release exists, fetch and verify it, and hand installation to the platform without ever replacing their own running process. It adds a new capability and does not touch the scraping specs or any Gourmet/Ventopay request shape. It supersedes the "no updater / no automatic updates" wording of `desktop-distribution` (`REQ-DST-01`, `REQ-DST-04`), which is reconciled when `desktop-ui-redesign` is archived.

## ADDED Requirements

### Requirement: REQ-UPD-01 Release discovery
The desktop app SHALL discover updates only through the GitHub Releases of `radaiko/SnackPilot`, using one unauthenticated GET of the release list and never `releases/latest`. It SHALL ignore drafts, prereleases and every tag that is not exactly `desktop/vX.Y.Z`, and SHALL select the highest tag by numeric semver comparison. No request SHALL carry credentials or any Gourmet or Ventopay value.

#### Scenario: Highest desktop tag wins over other releases
- **WHEN** the release list contains `desktop/v2.3.0`, `desktop/v2.10.0`, `ios/v9.0.0` and a prerelease `desktop/v2.11.0`
- **THEN** the app selects `desktop/v2.10.0` and requests exactly the manifest and signature assets of that release after the list request

#### Scenario: Requests are anonymous
- **WHEN** a check runs
- **THEN** every request is a GET without body, `Authorization` header or cookie, and none targets a Gourmet or Ventopay host

### Requirement: REQ-UPD-02 Manifest applicability
For the selected release the app SHALL fetch the manifest `SnackPilot-manifest-{platform}-{architecture}.json` and its detached signature `….json.sig` for the running host (`macos`/`arm64`, `windows`/`x64`). It SHALL offer an update only when the manifest version is strictly greater than the installed version, platform, architecture and tag match the host and release, and the host OS meets the minimum OS. Release notes SHALL come from the signed manifest.

#### Scenario: Same or older version is up to date
- **WHEN** the highest desktop release carries version 2.2.0 and the installed version is 2.2.0 or 2.3.0
- **THEN** the result is "up to date" and no artifact is offered

#### Scenario: Release without a manifest for this host
- **WHEN** the highest desktop release has no manifest asset for the running platform and architecture
- **THEN** the check fails with a typed error, no update is offered and the app stays usable

#### Scenario: Manifest that does not belong to the release
- **WHEN** a correctly signed manifest names a platform, architecture, version or tag different from the host or from the chosen release tag
- **THEN** it is rejected and nothing is offered

#### Scenario: Minimum OS not met
- **WHEN** the manifest minimum OS is higher than the known host OS version
- **THEN** nothing is offered, while an unknown host OS version does not block the offer

### Requirement: REQ-UPD-03 Manifest signature
The app SHALL verify an Ed25519 signature over the exact manifest bytes, using a public key compiled into the binary, before it reads any manifest field. An absent, malformed or invalid signature SHALL fail the check. Failure messages SHALL be German, plain and free of URLs, hashes and keys.

#### Scenario: Tampered manifest
- **WHEN** one byte of the manifest or of the signature differs from what was signed
- **THEN** verification fails before any field is used and no download starts

#### Scenario: Manifest signed by another key
- **WHEN** the signature is valid for a different public key
- **THEN** it is rejected exactly like a tampered manifest

#### Scenario: Malformed signature file
- **WHEN** the signature is not hex of exactly 64 bytes
- **THEN** the check fails as an invalid signature

### Requirement: REQ-UPD-04 Artifact integrity
The artifact address SHALL come from the release asset list (an `https://github.com/radaiko/SnackPilot/releases/download/` address whose file name equals the signed manifest `artifact`), never from the manifest alone. The download SHALL stop at the manifest `size`, and its byte count and SHA-256 SHALL match the manifest. Any failure SHALL delete the staged download and leave nothing installable.

#### Scenario: Valid manifest and artifact
- **WHEN** the signature verifies and the downloaded file has the manifest size and SHA-256
- **THEN** the artifact is accepted as verified for the next platform step

#### Scenario: Tampered or truncated artifact
- **WHEN** the downloaded bytes differ in size or hash from the signed manifest
- **THEN** the staging directory is removed, the state is "download failed" with a message that nothing was installed, and no handoff action is offered

#### Scenario: Address outside the release download prefix
- **WHEN** a release asset address does not start with the download prefix or does not end with the expected file name
- **THEN** it is rejected and nothing is downloaded

### Requirement: REQ-UPD-05 macOS bundle identity
After the signature and SHA-256 checks the macOS app SHALL extract the ZIP into a new empty directory and SHALL require exactly one `SnackPilot.app` whose `CFBundleIdentifier` is `dev.radaiko.gourmetclient` and whose `CFBundleShortVersionString` equals the manifest version.

#### Scenario: Wrong identity or version
- **WHEN** the extracted bundle has another identifier or version than expected
- **THEN** it is rejected and the extraction and ZIP are deleted

#### Scenario: Not exactly one bundle
- **WHEN** the ZIP contains no `SnackPilot.app` or more than one
- **THEN** it is rejected and the extraction and ZIP are deleted

### Requirement: REQ-UPD-06 macOS bundle trust
The extracted bundle SHALL pass `codesign --verify --deep --strict`, carry a Developer ID signature of team `555V4MNLK3`, have a valid stapled ticket (`stapler validate`) and pass `spctl --assess --type execute`. These tools SHALL be invoked by absolute path. A failing step SHALL delete the extraction and the ZIP and the app SHALL report that the update was discarded.

#### Scenario: Genuine release passes
- **WHEN** the verified ZIP contains the Developer-ID-signed, notarized and stapled bundle of the manifest version
- **THEN** all four checks pass and the update becomes ready for handoff

#### Scenario: Wrong team or missing staple
- **WHEN** the bundle is signed by a different team or has no stapled ticket
- **THEN** verification fails, nothing is kept and no Finder action is offered

### Requirement: REQ-UPD-07 Installation is delegated and never claimed
Installation SHALL be delegated to the platform: the app SHALL NOT replace its own files, SHALL NOT claim an installation it did not perform and SHALL word a finished handoff only as the update having been prepared. A failed handoff SHALL show a typed German failure, keep the verified files and allow a retry.

#### Scenario: Handoff failure
- **WHEN** starting the installer or revealing the bundle fails
- **THEN** a typed German failure is shown, the verified files stay, and the user can retry

#### Scenario: No installed claim
- **WHEN** any handoff succeeds
- **THEN** no text states that the update is installed

### Requirement: REQ-UPD-08 Windows installer handoff
On Windows the app SHALL start the verified installer from its per-update staging directory once, without elevation, silent switches or an install-directory override (the installer already defaults to `HKCU\Software\SnackPilot\InstallDir`), and SHALL quit immediately afterwards.

#### Scenario: Windows handoff
- **WHEN** the user chooses to install a verified Windows update
- **THEN** the installer process is started once with no arguments, the app quits, and no installed-version claim is made

### Requirement: REQ-UPD-09 macOS Finder handoff
On macOS the app SHALL reveal the verified extracted `SnackPilot.app` in Finder, keep running and leave its own bundle unmodified.

#### Scenario: macOS handoff
- **WHEN** the user chooses "Im Finder zeigen" for a verified macOS update
- **THEN** Finder reveals the extracted app, the state reads that the update is shown and not yet installed, and the running app is not modified

### Requirement: REQ-UPD-10 Update state machine
The model SHALL own one update state machine with the states idle, checking, up to date, available, skipped, downloading, verifying, ready, handed off and failed, SHALL change state only through explicit transitions and SHALL run at most one operation at a time.

#### Scenario: Single operation
- **WHEN** a check or download is running and another check or download is requested
- **THEN** the second request is ignored and the number of core calls is unchanged

### Requirement: REQ-UPD-11 Check cadence and switch
Checks SHALL run on every launch and then once per 24 hours of runtime while "Automatisch nach Updates suchen" is on (default on, persisted in the existing KV store), and on demand through "Jetzt prüfen" regardless of the switch.

#### Scenario: Launch and daily cadence
- **WHEN** the switch is on, the app launches, and 23 h 59 min later a periodic tick arrives
- **THEN** exactly one check ran at launch and none ran at the tick; a tick after 24 h runs one more

#### Scenario: Switch off
- **WHEN** the switch is off and the app launches
- **THEN** no update request is made, while "Jetzt prüfen" still runs one check

### Requirement: REQ-UPD-12 Explicit download and skipped versions
Downloads SHALL start only on explicit user action. Skipping a version SHALL persist and hide that version from automatic checks only.

#### Scenario: Available does not download
- **WHEN** a check finds an update
- **THEN** the state is available and no download request has been made

#### Scenario: Skipped version
- **WHEN** the user skips version 2.3.0 and an automatic check finds 2.3.0 again
- **THEN** the state is skipped; a manual check offers 2.3.0 again and clears the skip

### Requirement: REQ-UPD-13 Update traffic is gated and failures are contained
Update traffic SHALL NOT occur in evidence mode, on mobile or on hosts without update support. A failed automatic check SHALL be silent outside the Updates detail, and no update failure SHALL block ordering, billing or any other screen.

#### Scenario: Unsupported host
- **WHEN** the platform reports no update support (mobile, evidence mode, unsupported desktop)
- **THEN** no request is made and the Updates entry is not shown

#### Scenario: Failed automatic check
- **WHEN** an automatic check fails because the network is down
- **THEN** only the update state changes: no dialog or toast appears and other screens keep working

### Requirement: REQ-UPD-14 Updates entry in Einstellungen
Einstellungen SHALL show an "Updates" entry on supported desktop hosts only, opening the existing `Advanced` settings detail and projected like every other detail at all desktop widths.

#### Scenario: Entry hidden when unsupported
- **WHEN** the host reports no update support
- **THEN** Einstellungen has no Updates entry and the Advanced detail is unreachable

### Requirement: REQ-UPD-15 Updates detail content
The Updates detail SHALL show the installed version and build, the switch "Automatisch nach Updates suchen", the action "Jetzt prüfen" and a German state line for every state of the state machine.

#### Scenario: Every state has a line
- **WHEN** the state is any of idle, checking, up to date, available, skipped, downloading, verifying, ready, handed off or failed
- **THEN** a non-empty German state line and the matching actions are shown

### Requirement: REQ-UPD-16 Offer, download and handoff flow
For an available update the detail SHALL show the release notes and their link before offering "Herunterladen" and "Diese Version überspringen". A download SHALL show progress and a verified update SHALL offer the platform handoff action. Actions SHALL be disabled while an operation runs.

#### Scenario: Available update shows notes first
- **WHEN** the state is available with notes
- **THEN** the notes and "Herunterladen" are visible, and no download has started

#### Scenario: Progress and ready
- **WHEN** a download is running and then verified
- **THEN** a percentage progress line is shown, then the platform-specific ready line and its single handoff action

### Requirement: REQ-UPD-17 Updates detail layout
The Updates detail SHALL use the existing detail pane geometry and controls and SHALL not scroll horizontally at 360, 720 and 1280 logical pixels.

#### Scenario: Three widths
- **WHEN** the detail is shown at 360, 720 and 1280 logical pixels
- **THEN** all text wraps, no control is clipped and there is no horizontal scrolling

### Requirement: REQ-UPD-18 Manifests generated in the release workflow
The desktop release workflow SHALL, in the publish job after both platform builds and before publishing, generate one manifest per platform from the real artifacts and attach manifest and signature to the same GitHub release. The manifest SHALL carry schema version, platform, architecture, version, build, minimum OS, artifact file name, size, SHA-256, release notes body, release notes URL and tag.

#### Scenario: Manifest matches artifact
- **WHEN** the manifest step runs for a Windows installer
- **THEN** the manifest `size` and `sha256` equal those of the uploaded file and the signature verifies with the public key compiled into the app

### Requirement: REQ-UPD-19 Signing key custody
The manifest SHALL be signed with the Ed25519 private key held only in the Forgejo secret `SNACKPILOT_UPDATE_SIGNING_KEY`. A missing key SHALL fail the publish, and a macOS build whose receipt trust state is not `developer-id-notarized-stapled` SHALL get no macOS manifest. No key material SHALL appear in the repository, a log or an artifact.

#### Scenario: Missing signing key
- **WHEN** the signing secret is empty
- **THEN** the step exits nonzero, nothing is published and the log contains no key material

#### Scenario: Unsigned macOS build
- **WHEN** the macOS receipt reports an unsigned trust state
- **THEN** no macOS manifest or signature is produced and the release notes still say the build is unsigned

### Requirement: REQ-UPD-20 Disclosure of the GitHub request
The desktop Datenschutz text and `docs/privacy.html` SHALL state that the desktop app asks GitHub, unauthenticated, at launch and every 24 hours whether a newer version exists. The mobile Datenschutz text SHALL stay unchanged.

#### Scenario: Desktop text
- **WHEN** the desktop Datenschutz dialog is shown on a host with update support
- **THEN** it contains the GitHub sentence

#### Scenario: Mobile text unchanged
- **WHEN** the mobile Datenschutz dialog is shown
- **THEN** its text is byte-identical to before this change

### Requirement: REQ-UPD-21 Desktop requirements document
`docs/requirements/08-gpui-desktop.md` SHALL describe the published desktop artifacts and the updater behaviour, and SHALL NOT state that no updater or no public ZIP exists.

#### Scenario: Reader of the requirement document
- **WHEN** the document is searched for "no updater" and "no public ZIP"
- **THEN** neither statement remains, and it names the signed manifest, Ed25519 and SHA-256 checks, the 24 hour cadence, the unauthenticated GitHub contact and that the macOS bundle is never self-replaced
