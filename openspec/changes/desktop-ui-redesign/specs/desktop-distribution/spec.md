# Desktop distribution delta

## Purpose

Defines installable desktop releases, their supported systems and trust policy, and continuity of local user data without altering mobile release identities or scraping behaviour.

## ADDED Requirements

### Requirement: REQ-DST-01 Supported desktop release artifacts
SnackPilot SHALL ship a macOS 26+ Apple Silicon `SnackPilot.app` and an unsigned Windows 11 x64 installer. Both SHALL run the shared UI and contain the product icon, bundled fonts/assets and matching version/build metadata. Desktop identity SHALL remain `dev.radaiko.gourmetclient`. Linux, Intel Macs, Windows ARM, stores and automatic updates SHALL NOT be advertised as supported by this change.

#### Scenario: macOS installation
- **WHEN** the macOS release is copied to Applications and opened on macOS 26 Apple Silicon
- **THEN** the installed app launches without a development checkout, contains only the supported arm64 CPU slice, has minimum OS 26.0, and About reports the release version/build
- **AND** icon, Inter, Lucide and the shared responsive UI work from the installed bundle

#### Scenario: Windows installation and upgrade
- **WHEN** a standard user runs `SnackPilot-{version}-windows-x64-setup.exe` on Windows 11 x64
- **THEN** a per-user install creates SnackPilot launch and uninstall entries, installs the executable and required runtime/assets without a Rust toolchain, and launches the shared UI
- **AND** a later installer replaces the application files without duplicating the installation or deleting user data

#### Scenario: Unsupported host or incomplete artifact
- **WHEN** packaging has the wrong CPU, OS floor, identity, missing asset/runtime, or inconsistent version/build
- **THEN** release validation fails rather than marking the build distributable

### Requirement: REQ-DST-02 Platform-specific release trust
macOS releases SHALL use Developer ID Application signing, hardened runtime, notarization and stapling through externally configured identities. A missing or failing signing/notary stage SHALL fail the release without unsigned fallback. Windows releases SHALL be unsigned and labelled as such; documentation SHALL explain possible SmartScreen/unknown-publisher warnings without disabling OS protections. No signing secret SHALL enter source, artifacts or logs.

#### Scenario: macOS trust validation
- **WHEN** a macOS release is declared ready
- **THEN** strict bundle signature verification, Developer ID identity and hardened-runtime checks, accepted notarization, staple validation and Gatekeeper assessment succeed on the actual installed app
- **AND** the deliverable is the signed/stapled `.app`, not a DMG or public archive; a temporary ZIP used solely for notary submission is not a distribution artifact

#### Scenario: Fail closed on Mac signing error
- **WHEN** the identity/profile is absent, keychain is inaccessible, notarization is rejected or validation fails
- **THEN** shipping exits nonzero, reports the failed stage without secrets, and does not create success tags/history/counter updates or label an unsigned binary a release

#### Scenario: Windows unsigned policy
- **WHEN** the Windows installer and installed executable are inspected
- **THEN** both have no Authenticode signature, release output says `UNSIGNED`, and no certificate/signing credentials are requested
- **AND** the documented install path does not require disabling SmartScreen, antivirus or other OS protections

### Requirement: REQ-DST-03 Desktop release orchestration
`make ship` SHALL select macOS and Windows independently alongside iOS/Android and include all four in `all`. Releases SHALL share semver/build metadata and use distinct `macos/v{version}` and `windows/v{version}` tags. Desktop failure SHALL prevent success bookkeeping. Dry-run SHALL leave version files, git, release history and build counters unchanged and SHALL NOT install, upload or distribute a release.

#### Scenario: Desktop platform selection
- **WHEN** only macOS, only Windows, a mixed subset, or all platforms are selected
- **THEN** only selected artifacts build, all selected artifacts receive identical version/build metadata, and only selected successful platforms receive local tags after all validation succeeds
- **AND** iOS/Android identifiers and signing/build behaviour remain unchanged, with no automatic push/upload

#### Scenario: Dry-run and unavailable build hosts
- **WHEN** dry-run is invoked for any supported selection
- **THEN** it can preview or build isolated test artifacts but leaves tracked version inputs, git refs/index, history and counters unchanged and performs no signing submission, install, upload or distribution
- **AND** a missing required native tool/host is reported explicitly, never replaced by a claimed successful cross-platform artifact

#### Scenario: Build failure is not a release
- **WHEN** any selected build, package or validation stage fails
- **THEN** no release commit/tag/history/counter is produced, pre-existing user edits remain intact, and temporary version changes owned by that invocation are restored safely

### Requirement: REQ-DST-04 Durable and private desktop state
Desktop cache/preferences SHALL use per-user application data, not development temp or installation directories. Credentials SHALL remain in the OS secure store under service `app:no-auth` and the existing four slots. Install/update/relaunch SHALL preserve data. Desktop notifications/geofence/alarms SHALL stay unavailable, analytics a no-op, and unsupported mail/settings/icon services SHALL NOT be presented as implemented features.

#### Scenario: Durable local data
- **WHEN** the app is relaunched or updated after a user changes appearance or cached data is populated
- **THEN** macOS uses `~/Library/Application Support/dev.radaiko.gourmetclient/` and Windows uses `%LOCALAPPDATA%\dev.radaiko.gourmetclient\` and preserves those settings/cache independently of the installation location
- **AND** missing/unwritable storage produces a truthful failure rather than silent fallback to a temp directory

#### Scenario: Secure-store continuity
- **WHEN** a dummy test credential is saved and the installed release is relaunched or upgraded
- **THEN** macOS Keychain or Windows Credential Manager reads the same dedicated test entry under `app:no-auth`, without placing credentials in local data files, release artifacts or evidence
- **AND** persist-before-validate and logout-keeps-credentials/cookies remain unchanged

#### Scenario: Honest desktop capabilities
- **WHEN** either desktop host starts or displays Einstellungen at any width
- **THEN** it does not request unsupported notification/geofence permissions, hides Benachrichtigungen, sends no analytics, and does not promise an updater, dynamic Dock icon, mail export or a successful no-op system-settings action
- **AND** documentation retains the accepted GPUI accessibility-tree limitation and themed-alert policy
