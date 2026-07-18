# 07 — Release, distribution & CI

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This doc owns `src/app/app.json` (store-identity/build parts), `src/app/eas.json`,
`ship.sh`, `.github/workflows/release.yml`, `.github/workflows/security-audit.yml`,
`.github/dependabot.yml`, `.github/icon.png`, `tools/icon-tools/**`,
`docs/app-store-release.md`, and `docs/privacy.html` (see `appendix-source-map.md`).
Runtime behavior of app.json entries (permissions, plugins, background tasks) is owned by
`05-platform-services.md`; the runtime alternate-app-icon feature is owned by
`03-features/themes.md` — this doc covers only how the icon assets are produced and which
store-level identity/config must survive into v2.

---

## 1. Store identities — MUST NOT change in v2

v2 ships as a normal store update over the installed v1 app. The update path (and the
credential-takeover design in `05-platform-services.md`) only works if the app identity is
byte-identical:

| Field | Exact value | Provenance |
|---|---|---|
| iOS bundle identifier | `dev.radaiko.gourmetclient` | (v1: src/app/app.json:18) |
| Android application ID (package) | `dev.radaiko.gourmetclient` | (v1: src/app/app.json:32) |
| App display name | `SnackPilot` | (v1: src/app/app.json:3) |
| URL scheme | `snackpilot` | (v1: src/app/app.json:5) |
| App Store Connect app ID (ascAppId) | `6753957109` | (v1: src/app/eas.json:21; docs/app-store-release.md:61) |
| Expo/EAS account | `radaiko` | (v1: docs/app-store-release.md:7,62) |
| Apple Developer account | *not named in source* — an Apple Developer Program membership exists (docs/app-store-release.md:5) but no account/team identity is in the repo; see Open question #2 | (inference) |
| iOS export compliance | `ITSAppUsesNonExemptEncryption: false` | (v1: src/app/app.json:21) |
| iOS device family | `ios.supportsTablet: false` (iPhone-only — set v2's iOS `TARGETED_DEVICE_FAMILY` to iPhone) | (v1: src/app/app.json:17) |

The bundle ID predates the rename to SnackPilot (the project was forked from
GourmetClient); it intentionally stays `dev.radaiko.gourmetclient` forever.

v1-only identities, retired with the RN toolchain (not carried to v2):

| Field | Value | Provenance |
|---|---|---|
| Expo slug | `GourmetApp` | (v1: src/app/app.json:4) |
| Expo owner | `radaiko` | (v1: src/app/app.json:118) |
| EAS project ID | `efb12eb3-0729-4ea2-a3db-8026d95db7d3` | (v1: src/app/app.json:115; docs/app-store-release.md:63) |
| Desktop Velopack pack ID | `dev.radaiko.snackpilot` | (v1: .github/workflows/release.yml:98) — retired, see §8 |

## 2. Versioning scheme

- **One semver `X.Y.Z` version shared by all platforms.** Enforced by `ship.sh`, which
  bumps the same version string in all version-carrying files in one commit
  (v1: ship.sh:88-108). In v1 those files are `src/app/app.json` (`expo.version`),
  `src/desktop/src-tauri/tauri.conf.json`, `src/desktop/package.json`, and
  `src/desktop/src-tauri/Cargo.toml`; the desktop three are dropped in v2, but the
  "single version, bumped everywhere in one release commit" rule carries over
  (v2: iOS project version, Android `versionName`, Rust core crate version).
- **Version validation:** `ship.sh` requires strict semver `^[0-9]+\.[0-9]+\.[0-9]+$` and
  strictly-increasing versions against a local, gitignored `.ship-history` file
  (last line = last shipped version; compared with `sort -V`) (v1: ship.sh:18-43,128-129).
- **Release commit message format:** `Release v${VERSION} (${PLATFORM_LABEL})` where
  `PLATFORM_LABEL` is a comma-separated subset of `desktop, ios, android`, e.g.
  `Release v1.4.5 (desktop,ios,android)` (v1: ship.sh:76-82,117; commit 6997c44).
- **Per-platform git tags trigger CI:** `ios/v${VERSION}`, `android/v${VERSION}`,
  `desktop/v${VERSION}` (v1: ship.sh:79-81; .github/workflows/release.yml:5-8). Platforms
  are selectable per release — the tag history shows versions shipped to only some
  platforms (e.g. `android/v1.4.4` does not exist while `ios/v1.4.4` does).
- **Build numbers (v1 mechanism):** EAS manages them remotely — `eas.json` sets
  `cli.appVersionSource: "remote"` and the `production` build profile sets
  `autoIncrement: true`, so the iOS `buildNumber` / Android `versionCode` auto-increment
  on every EAS build (v1: src/app/eas.json:4,15; docs/app-store-release.md:18). The
  `"buildNumber": "1"` in app.json is a dead local value superseded by the remote source
  (v1: src/app/app.json:19). The generated `ios/Info.plist` is gitignored; EAS reads the
  user-facing version from app.json (v1: ship.sh:90). **v2 must reimplement
  auto-incrementing build numbers itself** (e.g. CI run number or a counter), since EAS
  is gone.
- Note: `src/app/package.json` `"version": "1.0.0"` is never bumped and is meaningless
  (v1: src/app/package.json:3).
- v2 starts at version `2.0.0` (docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md:146).

## 3. Release process per store

### 3.1 Orchestration: `ship.sh` (interactive, run locally)

Exact v1 flow (v1: ship.sh:1-134):

1. Read last shipped version from `.ship-history`; prompt for a strictly higher semver.
2. Prompt for platform selection: `1) Desktop  2) iOS  3) Android  4) All`
   (comma-separated multi-select).
3. Bump the version in all 4 version files (§2) — always all files, even for a
   single-platform release, to keep versions in sync.
4. `git add` the 4 files, commit as `Release v${VERSION} (${PLATFORM_LABEL})`, create one
   tag per selected platform, `git push` then `git push origin <tags>`.
5. Append the version to `.ship-history`.

CI (release.yml) does all building/submitting; nothing is built locally. v2 needs the
same shape minus desktop: bump versions, commit, tag `ios/vX.Y.Z` and/or
`android/vX.Y.Z`, push, let CI build and submit.

### 3.2 iOS — App Store (v1 mechanism: EAS)

Tag `ios/v*` triggers the `ios` job (v1: .github/workflows/release.yml:135-157):
`ubuntu-latest`, Node 20, `expo/expo-github-action@v9` with `eas-version: latest` and
`token: ${{ secrets.EXPO_TOKEN }}`, `npm ci` in `src/app`, then:

```
eas build --platform ios --profile production --non-interactive
eas submit --platform ios --profile production --latest --non-interactive
```

Submission target: App Store Connect app `6753957109` (v1: src/app/eas.json:19-22).
Submission lands in TestFlight; promotion to the App Store is manual in App Store
Connect: wait ~15-30 min processing, test via TestFlight, create a new version, select
the build, Submit for Review; Apple review typically 1-3 days
(v1: docs/app-store-release.md:36-44).

Manual fallback commands (same result as CI, run from `src/app`):
`eas build --platform ios --profile production`, `eas submit --platform ios --latest`,
or combined `eas build --platform ios --auto-submit`; status via `eas build:list`,
signing via `eas credentials` (v1: docs/app-store-release.md:13-34,46-54). Signing
credentials are stored EAS-side, not in the repo.

**v2 replacement:** EAS is unavailable (no Expo). v2 CI must produce a signed IPA
(Xcode/xcodebuild on a macOS runner) and upload to App Store Connect (e.g.
`xcrun altool`/App Store Connect API key or fastlane), preserving: trigger on `ios/v*`
tag, TestFlight-first flow, same ascAppId/bundle ID.

### 3.3 Android — Google Play (v1 mechanism: EAS)

Tag `android/v*` triggers the `android` job (v1: .github/workflows/release.yml:159-186):
same Node/EAS setup as iOS, plus the Google Play service-account key is written from a
secret before submitting:

```
echo "$GOOGLE_PLAY_KEY" > google-play-key.json     # secret GOOGLE_PLAY_SERVICE_ACCOUNT_KEY, repo root
eas build --platform android --profile production --non-interactive
eas submit --platform android --profile production --latest --non-interactive
```

Submit config (v1: src/app/eas.json:23-26):

- `serviceAccountKeyPath`: `../../google-play-key.json` (resolved from `src/app` → repo root)
- `track`: `alpha` — **v1 submits to the Play alpha (closed testing) track**, not
  production. Promotion beyond alpha happens manually in the Play Console (no doc in
  repo describes it; see open questions).

**v2 replacement:** Gradle build (`bundleRelease` AAB) with release signing, upload to
the Play `alpha` track via the Google Play Developer API using the same service-account
key secret, triggered on `android/v*` tags. Package `dev.radaiko.gourmetclient` and Play
app-signing continuity are mandatory for the update path.

### 3.4 EAS build profiles (v1 mechanism, for reference)

(v1: src/app/eas.json:6-17) — three profiles: `development`
(`developmentClient: true`, `distribution: internal`), `preview`
(`distribution: internal`), `production` (`autoIncrement: true`). Only `production` is
used by release CI. CLI floor: `">= 16.32.0"` (v1: src/app/eas.json:3).

## 4. v1 CI inventory (what exists today)

Only two workflows exist; **there is no CI workflow that runs the Jest test suite or
type-checks** — tests are local-only in v1 (v1: .github/workflows/ contains exactly
`release.yml` and `security-audit.yml`).

### 4.1 `release.yml`

Trigger: push of tags `desktop/v*`, `ios/v*`, `android/v*`
(v1: .github/workflows/release.yml:3-8). Jobs:

- `desktop-build` + `desktop-release` — retired, see §8.
- `ios` — §3.2.
- `android` — §3.3.

Secrets consumed: `EXPO_TOKEN`, `GOOGLE_PLAY_SERVICE_ACCOUNT_KEY` (mobile);
`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_APP_PASSWORD`,
`APPLE_TEAM_ID` (desktop signing/notarization — retired with desktop, but Apple signing
material of this kind will be needed again for v2's native iOS CI builds); and
`GITHUB_TOKEN` (the auto-provided GitHub Actions token, used by the retired desktop-release
job for `gh release create` — not needed in v2, which produces no GitHub Releases).

### 4.2 `security-audit.yml`

(v1: .github/workflows/security-audit.yml:1-62)

- Triggers: push to `main` and PRs touching `**/package.json`, `**/package-lock.json`,
  `**/Cargo.toml`, `**/Cargo.lock`, or the workflow file itself; weekly schedule
  `cron: "0 6 * * 1"` (Mondays 06:00 UTC, "weekly sweep so new advisories surface even
  without code changes"); manual `workflow_dispatch`. Concurrency group
  `${{ github.workflow }}-${{ github.ref }}` with `cancel-in-progress: true`.
- `npm-audit` job: matrix over `dir: [src/app, src/desktop, tools/icon-tools]`
  (`fail-fast: false`), Node 20, runs
  `npm audit --package-lock-only --audit-level=moderate` in each dir. Checkout with
  `persist-credentials: false`; actions pinned to full commit SHAs.
- `cargo-audit` job: installs `cargo-audit` via `taiki-e/install-action`, runs
  `cargo audit` in `src/desktop/src-tauri`.

**v2 equivalent:** keep the weekly + manifest-triggered advisory sweep. Rust core:
`cargo audit` against the core crate's `Cargo.lock`. `tools/icon-tools` npm audit stays
if the tool is kept (§6). `src/app`/`src/desktop` npm audits are retired with those
trees. Add whatever the native ecosystems offer for Swift/Gradle dependencies (no v1
precedent — v1 had no CocoaPods/Gradle manifests of its own under audit).

## 5. v2 CI requirements

From the approved v2 design (docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md:147-148),
v2 CI (GitHub Actions) must provide, beyond the §4.2 audit equivalent:

1. **Rust core: build + full test suite on both target architectures** (the
   fixture-driven test suite is the safety net for the account-ban-critical request
   shapes — see `06-testing.md` and the SAFETY-CRITICAL notes in `01-`/`02-`). This is
   a strict upgrade over v1, which ran no tests in CI.
2. **iOS app build** (macOS runner; at minimum a compile check on PRs, signed
   store build on `ios/v*` tags per §3.2).
3. **Android app build** (compile check on PRs, signed AAB + Play upload on
   `android/v*` tags per §3.3).
4. Release workflows produce the store artifacts; the tag-triggered model of §2/§3
   carries over unchanged.

## 6. Icon generation pipeline

Dev-only tooling in `tools/icon-tools/` (npm package `snackpilot-icon-tools`,
deliberately isolated from `src/app` "so sharp is not shipped to EAS builds"
(v1: tools/icon-tools/package.json:5)). Two-stage pipeline, npm scripts `generate`,
`render`, `all` (tsx runner; deps: `sharp ^0.35.3`) (v1: tools/icon-tools/package.json:6-17).

### 6.1 Stage 1 — `generate-icons.ts` (SVG synthesis)

(v1: tools/icon-tools/generate-icons.ts)

Five accent variants with exact colors (v1: tools/icon-tools/generate-icons.ts:4-10):

| id | color | gradientEnd |
|---|---|---|
| `orange` | `#D4501A` | `#B84415` |
| `emerald` | `#2E7D4F` | `#236B3F` |
| `berry` | `#A62547` | `#8C1E3B` |
| `golden` | `#C08B1A` | `#A07415` |
| `ocean` | `#2563A8` | `#1E528C` |

For each variant it writes two 1024×1024 SVGs into `src/app/assets/icons/`:

- `icon-{id}.svg` — full icon: rounded-rect background `rx="224"` filled with a
  white→`#F0F0F2` diagonal gradient; two faint "plate" circles (r=320 stroke-opacity
  0.08 width 6; r=260 stroke-opacity 0.05 width 3, stroked in the accent color); crossed
  fork (rotated −30°) and knife (rotated +30°) filled with a `color`→`gradientEnd`
  diagonal gradient (exact path/rect geometry in
  v1: tools/icon-tools/generate-icons.ts:12-54).
- `adaptive-icon-{id}.svg` — glyph-only (transparent background) fork+knife for
  Android adaptive icons (v1: tools/icon-tools/generate-icons.ts:56-85).

### 6.2 Stage 2 — `render-icons.ts` (PNG rendering)

(v1: tools/icon-tools/render-icons.ts) Renders every `*.svg` in `src/app/assets/icons/`
to a same-named 1024×1024 PNG via sharp, and additionally re-renders the **orange**
variant to `src/app/assets/icon.png` and `src/app/assets/adaptive-icon.png` (the default
app icon is orange).

### 6.3 How the outputs are wired into the app (v1 mechanism)

- Default icon: `./assets/icons/icon-orange.png`; Android adaptive icon foreground
  `./assets/icons/adaptive-icon-orange.png` with `backgroundColor: "#F0F0F2"`;
  notification icon `./assets/icons/icon-orange.png` with accent `#FF6B35`
  (v1: src/app/app.json:8,33-36,59-63).
- Alternate icons `emerald`, `berry`, `golden`, `ocean` are registered via the
  `@g9k/expo-dynamic-app-icon` plugin, each with iOS PNG + Android adaptive
  foreground/`#F0F0F2` background (v1: src/app/app.json:65-96). Runtime switching is
  owned by `03-features/themes.md`.
- Splash: `./assets/splash-icon.png`, `resizeMode: "contain"`, background `#ffffff`
  (v1: src/app/app.json:11-15) — **not** produced by icon-tools; it is a checked-in
  binary asset, carried over by copying (appendix-source-map.md: `assets/**`).
- Repo branding: `.github/icon.png` is a symlink to `../src/app/assets/icon.png`
  (v1: .github/icon.png).

**v2:** keep the tool as-is (it already writes plain SVG/PNG; only the output path and
the wiring change — iOS asset catalogs + alternate-icon entries, Android mipmap +
adaptive-icon XML + `activity-alias` per the v2 design doc §4). Alternatively copy the
rendered PNGs from `main`; the SVG geometry above is the source of truth if icons ever
need regeneration.

## 7. Privacy policy hosting

- The store-facing privacy policy is `docs/privacy.html` — a single self-contained
  German-language HTML page ("Datenschutzerklärung – SnackPilot"), created for App Store
  review (v1: docs/privacy.html; git f0e937d "Add German privacy policy page for App
  Store review").
- **Hosting:** GitHub Pages for the repo serves the `main` branch, `/docs` path, at
  `https://radaiko.github.io/SnackPilot/` — so the policy URL is
  `https://radaiko.github.io/SnackPilot/privacy.html`. (Verified via the GitHub Pages
  API for radaiko/SnackPilot; this is repo *settings*, not in source — no file in the
  repo references the URL.)
- Content that must stay accurate in v2 (v1: docs/privacy.html:48-137): operator
  Aiko Radlingmayr, contact `aiko@spitzbub.app`; credentials stored **only locally**
  (iOS/Android Secure Store); anonymous usage analytics; demo mode (`demo` / `demo1234!`)
  makes **no** network connections to Gourmet/Ventopay; GDPR legal bases Art. 6(1)(f)
  (analytics) and Art. 6(1)(b) (credential transmission); footer "Stand: Februar 2026".
- **v2 analytics divergence (was TelemetryDeck):** v2 switched the analytics provider to a
  **self-hosted Aptabase** instance (anonymous by design — no user/device IDs, no IDFA,
  no cross-app tracking; GDPR Recital 26). The v2 privacy policy + in-app "Datenschutz"
  summary must describe Aptabase, NOT TelemetryDeck, and the "network connections" list is
  now `alaclickneu.gourmet.at`, `my.ventopay.com`, and the Aptabase ingest host
  (`hetzner-server-1.ibex-dory.ts.net`, reached via Tailscale Funnel). See
  `03-features/analytics.md` and the vault doc "Snackpilot - Aptabase Self-Host".
- **Store disclosures (both platforms):** declare *Product interaction / App activity*,
  **not linked to identity, not used for tracking**, purpose analytics — App Store privacy
  label and Play Data Safety. The iOS privacy manifest (`src/ios/SnackPilot/PrivacyInfo.xcprivacy`)
  already reflects this.
- v2 must: keep the page hosted at a stable URL (same URL is simplest — the file lives
  on `main`, which v2 eventually replaces, so the v2 branch must carry
  `docs/privacy.html`), update the subtitle "iOS, Android & Desktop App"
  (v1: docs/privacy.html:49) to drop Desktop, and re-check the storage wording
  (v1 mentions "Desktop: lokaler Speicher", v1: docs/privacy.html:63).
- Note: the in-app "Datenschutz" link does **not** open this page; it shows a native
  alert with a short German analytics summary (v1: src/app/app/(tabs)/settings.tsx:138-149,
  owned by `03-features/settings.md`). The hosted page is referenced only in the store
  listings.

## 8. Desktop / Velopack release — RETIRED in v2

Documented for the record only; the desktop target is dropped
(appendix-source-map.md "Dropped in v2"). v1 behavior
(v1: .github/workflows/release.yml:11-133):

- Tag `desktop/v*` built Tauri bundles on a 3-OS matrix (macos/windows/ubuntu-latest,
  main exe `snack-pilot` / `snack-pilot.exe`), with Linux system deps
  (libwebkit2gtk-4.1-dev etc.), macOS Developer ID signing (keychain import of
  `APPLE_CERTIFICATE` p12) and notarization (`xcrun notarytool store-credentials` with
  `APPLE_ID`/`APPLE_APP_PASSWORD`/`APPLE_TEAM_ID`).
- Packaged the bare release executable with Velopack:
  `vpk pack --packId "dev.radaiko.snackpilot" --packTitle "SnackPilot"
  --packVersion "$VERSION" --mainExe <exe> --outputDir releases`, on macOS signed with
  `Developer ID Application: Aiko Radlingmayr (555V4MNLK3)` / matching Installer
  identity and the notary profile.
- A follow-up job created a GitHub Release named `v${VERSION}` at tag
  `desktop/v${VERSION}` with `--generate-notes`, uploading all Velopack artifacts; the
  installed apps self-updated from GitHub Releases via Velopack.

Consequences of retiring it: no more GitHub Releases are produced by CI (mobile stores
are the only distribution channel), the `desktop/v*` tag namespace is dead, and the
desktop-only secrets in §4.1 are unused until v2's native iOS signing needs Apple
certificates again (different cert type: Apple Distribution vs Developer ID).

## 9. Dependency automation

v1 Dependabot config, all schedules `weekly` on `monday` (v1: .github/dependabot.yml):

| Ecosystem | Directory | Grouping / rules |
|---|---|---|
| npm | `/src/app` | `open-pull-requests-limit: 10`; group `npm-security` (security-updates, all); group `npm-minor-patch` (version-updates, minor+patch). `ignore` rules suppress SDK-coupled majors: `expo*`, `@expo/*`, `jest-expo` (semver-major); `react-native` (major **and** minor — 0.x versioning); `react-native-*`, `@react-native-*`, `react`, `react-dom` (semver-major). Rationale comment: Expo SDK packages must be upgraded together via `npx expo install`, and `exclude-patterns` alone is insufficient because excluded packages would get individual PRs (v1: .github/dependabot.yml:3-44). |
| npm | `/src/desktop` | single group `npm-all` (v1: .github/dependabot.yml:46-55) — retired |
| npm | `/tools/icon-tools` | single group `npm-all` (v1: .github/dependabot.yml:57-66) |
| cargo | `/src/desktop/src-tauri` | single group `cargo-all` (v1: .github/dependabot.yml:68-77) — retired |
| github-actions | `/` | ungrouped (v1: .github/dependabot.yml:79-84) |

**v2 requirements:** keep weekly Dependabot (or equivalent) for: `cargo` on the Rust
core crate (single grouped PR is fine), `github-actions` at `/`, npm for
`/tools/icon-tools` if kept, plus new ecosystems for the native shells — `gradle` for
the Android app and `swift` (Swift Package Manager) for the iOS app if it has SPM
dependencies. The elaborate Expo ignore-rules are RN-specific and die with v1.

## 10. Discrepancies and notes

- **CLAUDE.md's Velopack instructions are stale vs CI:** CLAUDE.md describes
  `vpk pack --packDir "./src-tauri/target/release/bundle"` and a manual
  `vpk upload github` step, while the actual workflow packs a temp dir containing only
  the bare executable and uploads via `gh release create`
  (v1: .github/workflows/release.yml:93-104,124-133). The code (workflow) wins. Moot in
  v2 (retired), recorded for accuracy.
- **`docs/app-store-release.md` covers iOS only** and predates the tag-triggered CI; the
  actual v1 release path is `ship.sh` → tags → `release.yml` (§3.1). The manual EAS
  commands remain a valid fallback.
- **No test/lint CI exists in v1** — the "178 tests" in CLAUDE.md run only locally. v2
  explicitly upgrades this (§5).
- app.json's `web` section (metro bundler, favicon) is dropped with the web target.

## 11. Open questions (not determinable from source)

1. Play Console state beyond the `alpha` track: whether releases are promoted to
   production, which testing tracks/testers exist, and the Play app-signing setup
   (Google-managed vs uploaded key). Only `track: "alpha"` is in source
   (v1: src/app/eas.json:25).
2. iOS distribution-signing details (team ID / certificates for the App Store build)
   live in EAS-managed credentials, not in the repo; the desktop Developer ID team
   `555V4MNLK3` (v1: .github/workflows/release.yml:104) is presumably the same Apple
   team, but that is inference, not source.
3. The exact privacy-policy URL entered in App Store Connect / Play Console listings is
   store-side configuration; `https://radaiko.github.io/SnackPilot/privacy.html` is
   derived from the live GitHub Pages settings, not from any file in the repo.
4. Store listing content (descriptions, screenshots, categories, age ratings) is not in
   the repo at all.
