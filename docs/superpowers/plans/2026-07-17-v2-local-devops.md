# v2 Local Devops Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a single `make` entrypoint at the v2 repo root that gives a one-command build-&-run loop per platform and a local ship/release pipeline producing signed installable artifacts.

**Architecture:** A thin `Makefile` dispatches to focused bash scripts under `tools/devops/`. Shared logging, prereq checks, staleness detection, and version-file editing live in `tools/devops/lib.sh`. The run/ship scripts delegate core-binding regeneration to the existing `src/ios/bootstrap.sh` / `src/android/bootstrap.sh`. Ship bumps one shared semver across three files, builds artifacts into `dist/`, and commits + tags locally without pushing.

**Tech Stack:** GNU Make, Bash (macOS/BSD userland — `sed -i ''`), xcodebuild + simctl (iOS), Gradle + adb (Android), keytool (Android signing), awk/sed for version edits.

## Global Constraints

- Store identities MUST NOT change: iOS bundle id + Android applicationId `dev.radaiko.gourmetclient`; display name `SnackPilot`. This tooling never edits them.
- Do NOT alter web-scraping logic (CLAUDE.md). This is build orchestration only; no file under `src/core/src` request code is touched.
- Single shared semver `X.Y.Z` bumped in all three version-carrying files in one commit: `src/core/Cargo.toml` (`[package].version`), `src/ios/project.yml` (`MARKETING_VERSION`), `src/android/app/build.gradle.kts` (`versionName`). Build numbers: iOS `CURRENT_PROJECT_VERSION`, Android `versionCode`, from a local monotonic counter.
- Version validation: strict `^[0-9]+\.[0-9]+\.[0-9]+$`, strictly increasing vs the last line of `.ship-history` (compare with `sort -V`).
- Release commit format: `Release v<version> (<label>)`, `<label>` a comma-separated subset of `ios,android`. Tags: `ios/v<version>` / `android/v<version>`, created locally, NOT pushed.
- iOS export default method `development`; `METHOD=app-store` overrides.
- Scripts are `set -euo pipefail`. All paths below are relative to the v2 repo root (`/Users/radaiko/dev/private/SnackPilot-v2`) unless absolute.
- The Android launcher is an `activity-alias` per accent, so launch via the LAUNCHER category (`monkey`), not a hardcoded activity name.

---

### Task 1: Scaffold — Makefile, lib.sh logging + prereqs, gitignore

**Files:**
- Create: `Makefile`
- Create: `tools/devops/lib.sh`
- Modify: `.gitignore`

**Interfaces:**
- Produces (from `lib.sh`, sourced by every other script): `info msg`, `ok msg`, `err msg`, `die msg` (logging to stderr, `die` exits 1); `require_tool name hint` (dies if `name` not on PATH); `REPO_ROOT` (absolute repo root); `have_tool name` (0/1, no output).
- Produces (Make): targets `help` (default), `doctor`, `clean`.

- [ ] **Step 1: Write `tools/devops/lib.sh`**

```bash
#!/usr/bin/env bash
# Shared helpers for the SnackPilot v2 devops scripts. Source, don't execute.
set -euo pipefail

# Absolute repo root, regardless of caller cwd.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# ── logging (all to stderr so stdout stays clean for captured values) ──
_c_red=$'\033[0;31m'; _c_grn=$'\033[0;32m'; _c_ylw=$'\033[1;33m'; _c_bld=$'\033[1m'; _c_off=$'\033[0m'
info() { printf '%s▸%s %s\n' "$_c_bld" "$_c_off" "$*" >&2; }
ok()   { printf '%s✓%s %s\n' "$_c_grn" "$_c_off" "$*" >&2; }
err()  { printf '%s✗%s %s\n' "$_c_red" "$_c_off" "$*" >&2; }
die()  { err "$*"; exit 1; }

have_tool() { command -v "$1" >/dev/null 2>&1; }

require_tool() {
  local name=$1 hint=${2:-}
  if ! have_tool "$name"; then
    die "required tool '$name' not found on PATH${hint:+ — $hint}"
  fi
}
```

- [ ] **Step 2: Write the `Makefile`**

```makefile
# SnackPilot v2 — local devops entrypoint. Run `make help`.
SHELL := /bin/bash
DEVOPS := tools/devops

.DEFAULT_GOAL := help

.PHONY: help doctor clean ios-run android-run android-keystore ship

help: ## List available targets
	@grep -hE '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "} {printf "  \033[1m%-18s\033[0m %s\n", $$1, $$2}'

doctor: ## Check that required build tools are installed
	@bash $(DEVOPS)/doctor.sh

clean: ## Remove dist/ and generated build artifacts
	@rm -rf dist src/ios/build src/android/app/build && echo "cleaned dist/ + build outputs"

ios-run: ## Build + install + launch on an iOS simulator (DEVICE= to override)
	@bash $(DEVOPS)/run-ios.sh

android-run: ## Build + install + launch on an Android emulator/device
	@bash $(DEVOPS)/run-android.sh

android-keystore: ## One-time: generate the Android release keystore
	@bash $(DEVOPS)/android-keystore.sh

ship: ## Interactive release: bump version, build artifacts, commit + tag (DRY_RUN=1)
	@bash $(DEVOPS)/ship.sh
```

- [ ] **Step 3: Write `tools/devops/doctor.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

info "checking devops prerequisites"
status=0
check() {  # name hint
  if have_tool "$1"; then ok "$1"; else err "$1 missing${2:+ — $2}"; status=1; fi
}
check cargo       "install Rust: https://rustup.rs"
check xcodegen    "brew install xcodegen"
check xcodebuild  "install Xcode + command line tools"
check cargo-ndk   "cargo install cargo-ndk"
check adb         "install Android platform-tools"
check keytool     "comes with a JDK"
if [[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME:-}" ]]; then
  ok "ANDROID_NDK_HOME=$ANDROID_NDK_HOME"
else
  err "ANDROID_NDK_HOME unset or not a directory — export it (see src/android/bootstrap.sh)"; status=1
fi
[[ $status -eq 0 ]] && ok "all prerequisites present" || err "some prerequisites missing"
exit $status
```

- [ ] **Step 4: Add `.gitignore` entries**

Append to `.gitignore`:

```
# Devops
dist/
.ship-history
tools/devops/.build-number
```

- [ ] **Step 5: Verify help, doctor, clean**

Run: `cd /Users/radaiko/dev/private/SnackPilot-v2 && make help`
Expected: a list including `help`, `doctor`, `clean`, `ios-run`, `android-run`, `android-keystore`, `ship` with descriptions.

Run: `make doctor`
Expected: one line per tool; exit 0 if all present, else non-zero with actionable hints. (On this machine the memory says cargo-ndk + NDK are installed; a missing tool is a real report, not a plan failure.)

Run: `make clean`
Expected: prints `cleaned dist/ + build outputs`, exit 0.

- [ ] **Step 6: Commit**

```bash
git add Makefile tools/devops/lib.sh tools/devops/doctor.sh .gitignore
git commit -m "feat(devops): Makefile scaffold + doctor + shared lib"
```

---

### Task 2: Version helpers in lib.sh (unit-tested)

**Files:**
- Modify: `tools/devops/lib.sh`
- Create: `tools/devops/test-lib.sh`

**Interfaces:**
- Consumes: `info/ok/err/die` from Task 1.
- Produces (from `lib.sh`): `validate_semver v` (0 if strict `X.Y.Z`, else 1); `version_gt a b` (0 if a strictly greater than b by `sort -V`); `set_cargo_package_version file v`; `set_yaml_key file key value` (edits `KEY: "..."` in a yaml file); `set_gradle_string file key v` (edits `key = "..."`); `set_gradle_int file key n` (edits `key = <int>`); `verify_file_contains file needle` (dies if absent). These are the exact names Task 6 (ship) calls.

- [ ] **Step 1: Write the failing test `tools/devops/test-lib.sh`**

```bash
#!/usr/bin/env bash
# Unit tests for the pure helpers in lib.sh. Run: bash tools/devops/test-lib.sh
set -uo pipefail
source "$(dirname "$0")/lib.sh"

fails=0
assert()      { if eval "$1"; then echo "ok: $2"; else echo "FAIL: $2"; fails=$((fails+1)); fi; }
assert_not()  { if eval "$1"; then echo "FAIL: $2"; fails=$((fails+1)); else echo "ok: $2"; fi; }

# validate_semver
assert     'validate_semver 2.0.0'      'accepts 2.0.0'
assert     'validate_semver 10.20.30'   'accepts 10.20.30'
assert_not 'validate_semver 2.0'        'rejects 2.0'
assert_not 'validate_semver v2.0.0'     'rejects v2.0.0'
assert_not 'validate_semver 2.0.0-rc1'  'rejects 2.0.0-rc1'

# version_gt
assert     'version_gt 2.0.1 2.0.0'  '2.0.1 > 2.0.0'
assert     'version_gt 2.1.0 2.0.9'  '2.1.0 > 2.0.9'
assert_not 'version_gt 2.0.0 2.0.0'  '2.0.0 !> 2.0.0'
assert_not 'version_gt 2.0.0 2.0.1'  '2.0.0 !> 2.0.1'

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT

# set_cargo_package_version — only the [package] version, not a dep's
cat > "$tmp/Cargo.toml" <<'EOF'
[package]
name = "snackpilot-core"
version = "2.0.0"

[dependencies]
serde = { version = "1.0" }
EOF
set_cargo_package_version "$tmp/Cargo.toml" 2.1.0
assert 'grep -q '"'"'^version = "2.1.0"'"'"' "'"$tmp"'/Cargo.toml"' 'cargo package version bumped'
assert 'grep -q '"'"'version = "1.0"'"'"' "'"$tmp"'/Cargo.toml"'      'cargo dep version untouched'

# set_yaml_key
cat > "$tmp/project.yml" <<'EOF'
    MARKETING_VERSION: "2.0.0"
    CURRENT_PROJECT_VERSION: "1"
EOF
set_yaml_key "$tmp/project.yml" MARKETING_VERSION 2.1.0
set_yaml_key "$tmp/project.yml" CURRENT_PROJECT_VERSION 42
assert 'grep -q '"'"'MARKETING_VERSION: "2.1.0"'"'"' "'"$tmp"'/project.yml"'      'marketing version bumped'
assert 'grep -q '"'"'CURRENT_PROJECT_VERSION: "42"'"'"' "'"$tmp"'/project.yml"'   'project version bumped'

# set_gradle_string / set_gradle_int
cat > "$tmp/build.gradle.kts" <<'EOF'
        versionCode = 1
        versionName = "2.0.0"
EOF
set_gradle_string "$tmp/build.gradle.kts" versionName 2.1.0
set_gradle_int "$tmp/build.gradle.kts" versionCode 42
assert 'grep -q '"'"'versionName = "2.1.0"'"'"' "'"$tmp"'/build.gradle.kts"'  'gradle versionName bumped'
assert 'grep -q '"'"'versionCode = 42'"'"' "'"$tmp"'/build.gradle.kts"'        'gradle versionCode bumped'

echo "---"; [[ $fails -eq 0 ]] && echo "ALL PASS" || { echo "$fails FAILED"; exit 1; }
```

- [ ] **Step 2: Run the test, verify it fails**

Run: `bash tools/devops/test-lib.sh`
Expected: FAIL — `validate_semver: command not found` (helpers not defined yet).

- [ ] **Step 3: Add the helpers to `tools/devops/lib.sh`**

Append:

```bash
# ── version helpers ──
validate_semver() { [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# version_gt A B  → success iff A is strictly greater than B (semver via sort -V)
version_gt() {
  [[ "$1" != "$2" ]] || return 1
  local highest; highest="$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)"
  [[ "$highest" == "$1" ]]
}

verify_file_contains() { grep -q -- "$2" "$1" || die "expected '$2' in $1 after edit — aborting"; }

# Replace [package].version only (leaves dependency versions alone).
set_cargo_package_version() {
  local file=$1 v=$2
  awk -v ver="$v" '
    /^\[package\]/ { inpkg=1 }
    /^\[/ && $0 !~ /^\[package\]/ { inpkg=0 }
    inpkg && /^version[[:space:]]*=/ { sub(/"[^"]*"/, "\"" ver "\"") }
    { print }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  verify_file_contains "$file" "version = \"$v\""
}

# Edit `KEY: "value"` (yaml). Quotes preserved.
set_yaml_key() {
  local file=$1 key=$2 val=$3
  sed -i '' -E "s/(${key}:[[:space:]]*\")[^\"]*(\")/\1${val}\2/" "$file"
  verify_file_contains "$file" "${key}: \"${val}\""
}

# Edit `key = "value"` (kotlin DSL string).
set_gradle_string() {
  local file=$1 key=$2 val=$3
  sed -i '' -E "s/(${key}[[:space:]]*=[[:space:]]*\")[^\"]*(\")/\1${val}\2/" "$file"
  verify_file_contains "$file" "${key} = \"${val}\""
}

# Edit `key = <int>` (kotlin DSL number).
set_gradle_int() {
  local file=$1 key=$2 val=$3
  sed -i '' -E "s/(${key}[[:space:]]*=[[:space:]]*)[0-9]+/\1${val}/" "$file"
  verify_file_contains "$file" "${key} = ${val}"
}
```

- [ ] **Step 4: Run the test, verify it passes**

Run: `bash tools/devops/test-lib.sh`
Expected: every line `ok:`, final line `ALL PASS`, exit 0.

- [ ] **Step 5: Commit**

```bash
git add tools/devops/lib.sh tools/devops/test-lib.sh
git commit -m "feat(devops): version-edit + semver helpers with unit tests"
```

---

### Task 3: iOS build & run

**Files:**
- Modify: `tools/devops/lib.sh` (add `bootstrap_if_stale`)
- Create: `tools/devops/run-ios.sh`

**Interfaces:**
- Consumes: logging + `require_tool` + `REPO_ROOT` (Task 1).
- Produces (lib.sh): `bootstrap_if_stale platform artifact` — reruns `src/<platform>/bootstrap.sh` iff any file under `src/core/src` is newer than `artifact` (or `artifact` is missing).

- [ ] **Step 1: Add `bootstrap_if_stale` to `tools/devops/lib.sh`**

```bash
# Rerun a platform bootstrap only when the core is newer than its built binding artifact.
bootstrap_if_stale() {
  local platform=$1 artifact=$2
  local script="$REPO_ROOT/src/$platform/bootstrap.sh"
  if [[ ! -e "$artifact" ]] || [[ -n "$(find "$REPO_ROOT/src/core/src" -newer "$artifact" -print -quit 2>/dev/null)" ]]; then
    info "core changed (or bindings absent) → running src/$platform/bootstrap.sh"
    ( cd "$REPO_ROOT/src/$platform" && ./bootstrap.sh )
  else
    ok "core bindings for $platform are up to date"
  fi
}
```

- [ ] **Step 2: Write `tools/devops/run-ios.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool xcodebuild "install Xcode"
require_tool xcrun "install Xcode command line tools"

BUNDLE_ID="dev.radaiko.gourmetclient"
IOS_SIM="${IOS_SIM:-iPhone 16 Pro}"
cd "$REPO_ROOT"

bootstrap_if_stale ios "src/ios/Frameworks/SnackPilotCore.xcframework"

# Resolve a target simulator UDID: explicit DEVICE=, else the booted one, else boot IOS_SIM.
udid="${DEVICE:-}"
if [[ -z "$udid" ]]; then
  udid="$(xcrun simctl list devices booted -j | /usr/bin/python3 -c 'import json,sys; d=json.load(sys.stdin)["devices"]; print(next((x["udid"] for v in d.values() for x in v if x.get("state")=="Booted"), ""))')"
fi
if [[ -z "$udid" ]]; then
  info "no booted simulator — booting '$IOS_SIM'"
  udid="$(xcrun simctl list devices available -j | /usr/bin/python3 -c 'import json,sys,os; name=os.environ["IOS_SIM"]; d=json.load(sys.stdin)["devices"]; print(next((x["udid"] for v in d.values() for x in v if x.get("name")==name), ""))')"
  [[ -n "$udid" ]] || die "simulator '$IOS_SIM' not found — set IOS_SIM= to an available device (xcrun simctl list devices available)"
  xcrun simctl boot "$udid"
  open -a Simulator
fi
ok "target simulator: $udid"

info "building (iphonesimulator)"
xcodebuild -project src/ios/SnackPilot.xcodeproj -scheme SnackPilot \
  -sdk iphonesimulator -destination "id=$udid" \
  -derivedDataPath src/ios/build build

app="src/ios/build/Build/Products/Debug-iphonesimulator/SnackPilot.app"
[[ -d "$app" ]] || die "built .app not found at $app"

info "installing + launching"
xcrun simctl install "$udid" "$app"
xcrun simctl launch "$udid" "$BUNDLE_ID"
ok "SnackPilot launched on $udid"
```

- [ ] **Step 3: Verify on a simulator**

Run: `cd /Users/radaiko/dev/private/SnackPilot-v2 && make ios-run`
Expected: bootstrap runs if needed, `BUILD SUCCEEDED`, the app installs and launches on the simulator (login screen visible). Confirm with `xcrun simctl list devices booted` that a device is booted and, if unsure, screenshot with `xcrun simctl io booted screenshot /tmp/snackpilot-ios.png` and inspect it.

- [ ] **Step 4: Commit**

```bash
git add tools/devops/lib.sh tools/devops/run-ios.sh
git commit -m "feat(devops): make ios-run (build + install + launch on simulator)"
```

---

### Task 4: Android build & run

**Files:**
- Create: `tools/devops/run-android.sh`

**Interfaces:**
- Consumes: logging + `require_tool` + `REPO_ROOT` + `bootstrap_if_stale` (Tasks 1, 3).

- [ ] **Step 1: Write `tools/devops/run-android.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool adb "install Android platform-tools"
BUNDLE_ID="dev.radaiko.gourmetclient"
ANDROID_AVD="${ANDROID_AVD:-Medium_Phone_API_36.1}"
cd "$REPO_ROOT"

# newest .so is the freshness marker for the Android bindings
so_marker="$(ls -t src/android/app/src/main/jniLibs/*/libsnackpilot_core.so 2>/dev/null | head -1 || true)"
bootstrap_if_stale android "${so_marker:-/nonexistent}"

# Ensure a device/emulator is connected; else start the default AVD.
if [[ -z "$(adb devices | awk 'NR>1 && $2=="device"{print $1}')" ]]; then
  require_tool emulator "install the Android emulator package"
  info "no device — starting AVD '$ANDROID_AVD'"
  ( emulator -avd "$ANDROID_AVD" -netdelay none -netspeed full >/dev/null 2>&1 & )
  adb wait-for-device
  info "waiting for boot to complete"
  until [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; do sleep 2; done
fi
ok "device ready: $(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')"

info "building + installing (installDebug)"
( cd src/android && ./gradlew :app:installDebug )

info "launching"
adb shell monkey -p "$BUNDLE_ID" -c android.intent.category.LAUNCHER 1 >/dev/null
ok "SnackPilot launched on device"
```

- [ ] **Step 2: Verify on an emulator**

Run: `cd /Users/radaiko/dev/private/SnackPilot-v2 && make android-run`
Expected: bootstrap runs if needed, Gradle `installDebug` succeeds, the app launches (login screen). If unsure, `adb exec-out screencap -p > /tmp/snackpilot-android.png` and inspect.

- [ ] **Step 3: Commit**

```bash
git add tools/devops/run-android.sh
git commit -m "feat(devops): make android-run (build + install + launch on emulator)"
```

---

### Task 5: Android release keystore generation

**Files:**
- Create: `tools/devops/android-keystore.sh`

**Interfaces:**
- Consumes: logging + `require_tool` + `REPO_ROOT` (Task 1).
- Produces: `src/android/snackpilot-release.jks` + `src/android/keystore.properties` (both gitignored by `src/android/.gitignore`), consumed by `src/android/app/build.gradle.kts`.

- [ ] **Step 1: Write `tools/devops/android-keystore.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool keytool "comes with a JDK"
KS="$REPO_ROOT/src/android/snackpilot-release.jks"
PROPS="$REPO_ROOT/src/android/keystore.properties"
ALIAS="snackpilot"

[[ -e "$KS" ]] && die "keystore already exists at $KS — refusing to overwrite (delete it manually to regenerate)"

pass="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 32)"
info "generating release keystore (RSA 2048, 10000-day validity)"
keytool -genkeypair -v \
  -keystore "$KS" -alias "$ALIAS" \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass "$pass" -keypass "$pass" \
  -dname "CN=SnackPilot, OU=SnackPilot, O=SnackPilot, L=Vienna, C=AT"

cat > "$PROPS" <<EOF
storeFile=snackpilot-release.jks
storePassword=$pass
keyAlias=$ALIAS
keyPassword=$pass
EOF
chmod 600 "$PROPS"

ok "wrote $KS and $PROPS (both gitignored)"
err "BACK THESE UP: losing snackpilot-release.jks + keystore.properties means you can never"
err "sign an update for the same signing identity again. Store them outside the repo."
```

- [ ] **Step 2: Verify generation + a signed build (throwaway, then keep or discard)**

Run: `cd /Users/radaiko/dev/private/SnackPilot-v2 && make android-keystore`
Expected: creates `src/android/snackpilot-release.jks` + `src/android/keystore.properties`; prints the back-up warning. Confirm both are gitignored: `git status --porcelain src/android | grep -E 'keystore|jks'` returns nothing.

Run: `cd src/android && ./gradlew :app:assembleRelease && cd -`
Expected: `BUILD SUCCESSFUL`; verify the apk is signed:
`apksigner verify --print-certs src/android/app/build/outputs/apk/release/app-release.apk` (or `jarsigner -verify`) reports a valid signature. (If you don't want to keep this key, delete both files and rerun later.)

- [ ] **Step 3: Commit**

```bash
git add tools/devops/android-keystore.sh
git commit -m "feat(devops): make android-keystore (generate signed-release keystore)"
```

---

### Task 6: Ship pipeline

**Files:**
- Create: `tools/devops/ship.sh`

**Interfaces:**
- Consumes: all lib.sh helpers — logging, `validate_semver`, `version_gt`, `set_cargo_package_version`, `set_yaml_key`, `set_gradle_string`, `set_gradle_int` (Tasks 1–2).
- Env: `DRY_RUN=1` (no file/git/counter mutation, build into temp), `METHOD=` (iOS export method, default `development`; `app-store` needs an Apple Distribution cert, not just the Apple Development one), `IOS_TEAM=` (Apple Team ID; auto-detected from the keychain signing identity's cert OU when unset).

- [ ] **Step 1: Write `tools/devops/ship.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT"

DRY_RUN="${DRY_RUN:-0}"
METHOD="${METHOD:-development}"
HISTORY="$REPO_ROOT/.ship-history"
BUILDNO_FILE="$REPO_ROOT/tools/devops/.build-number"
CARGO="src/core/Cargo.toml"
PROJECT_YML="src/ios/project.yml"
GRADLE="src/android/app/build.gradle.kts"

[[ "$DRY_RUN" == "1" ]] && info "DRY RUN — no files, commits, tags, or counters will change"

# 1. version
last="0.0.0"; [[ -f "$HISTORY" ]] && last="$(tail -1 "$HISTORY")"
info "last shipped version: $last"
read -rp "New version (strict semver, > $last): " VERSION
validate_semver "$VERSION" || die "invalid version '$VERSION' — must be X.Y.Z"
version_gt "$VERSION" "$last" || die "$VERSION must be strictly greater than $last"

# 2. platforms
echo "Platforms:  1) iOS   2) Android   3) both" >&2
read -rp "Select (e.g. 1,2 or 3): " sel
do_ios=0; do_android=0
[[ "$sel" == *1* || "$sel" == *3* ]] && do_ios=1
[[ "$sel" == *2* || "$sel" == *3* ]] && do_android=1
[[ $do_ios -eq 1 || $do_android -eq 1 ]] || die "no platform selected"
label=""; [[ $do_ios -eq 1 ]] && label="ios"; [[ $do_android -eq 1 ]] && label="${label:+$label,}android"
info "shipping v$VERSION ($label)"

# 3. build number (increment; dry-run just previews)
buildno=0; [[ -f "$BUILDNO_FILE" ]] && buildno="$(cat "$BUILDNO_FILE")"
buildno=$((buildno + 1))
info "build number: $buildno"

mkdir -p dist
outdir="dist"; [[ "$DRY_RUN" == "1" ]] && outdir="$(mktemp -d)"

# 4. bump version files (skip on dry-run)
if [[ "$DRY_RUN" != "1" ]]; then
  set_cargo_package_version "$CARGO" "$VERSION"
  set_yaml_key "$PROJECT_YML" MARKETING_VERSION "$VERSION"
  set_yaml_key "$PROJECT_YML" CURRENT_PROJECT_VERSION "$buildno"
  set_gradle_string "$GRADLE" versionName "$VERSION"
  set_gradle_int "$GRADLE" versionCode "$buildno"
  ok "bumped version to $VERSION (build $buildno) in all three files"
else
  info "(dry-run) would bump $CARGO, $PROJECT_YML, $GRADLE to $VERSION / build $buildno"
fi

# 5a. iOS artifact
if [[ $do_ios -eq 1 ]]; then
  require_tool xcodebuild "install Xcode"
  # Auto-detect the Apple team from the signing identity already in the keychain (the cert's
  # OU is the Team ID — NOT the id in the cert name). Override with IOS_TEAM=.
  IOS_TEAM="${IOS_TEAM:-$(security find-certificate -c "Apple Development" -p 2>/dev/null | openssl x509 -noout -subject -nameopt sep_multiline,utf8 2>/dev/null | sed -n 's/^ *OU=//p' | head -1)}"
  [[ -n "$IOS_TEAM" ]] || die "no Apple Development team in keychain — set IOS_TEAM=<teamid> (see: security find-identity -v -p codesigning)"
  info "iOS: regenerating project + archiving (method=$METHOD, team=$IOS_TEAM)"
  ( cd src/ios && ./bootstrap.sh )
  archive="$outdir/SnackPilot.xcarchive"
  xcodebuild -project src/ios/SnackPilot.xcodeproj -scheme SnackPilot \
    -sdk iphoneos -destination "generic/platform=iOS" \
    -archivePath "$archive" -allowProvisioningUpdates \
    DEVELOPMENT_TEAM="$IOS_TEAM" CODE_SIGN_STYLE=Automatic archive
  plist="$outdir/exportOptions.plist"
  cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>method</key><string>$METHOD</string>
  <key>signingStyle</key><string>automatic</string>
  <key>teamID</key><string>$IOS_TEAM</string>
</dict></plist>
EOF
  xcodebuild -exportArchive -archivePath "$archive" \
    -exportOptionsPlist "$plist" -exportPath "$outdir" -allowProvisioningUpdates
  ipa="$(ls "$outdir"/*.ipa | head -1)"
  mv "$ipa" "$outdir/SnackPilot-$VERSION.ipa"
  ok "iOS artifact: $outdir/SnackPilot-$VERSION.ipa"
fi

# 5b. Android artifact
if [[ $do_android -eq 1 ]]; then
  [[ -f "src/android/keystore.properties" ]] || err "no keystore.properties — artifacts will be UNSIGNED (run: make android-keystore)"
  info "Android: bundleRelease + assembleRelease"
  ( cd src/android && ./gradlew :app:bundleRelease :app:assembleRelease )
  cp src/android/app/build/outputs/bundle/release/app-release.aab "$outdir/SnackPilot-$VERSION.aab"
  cp src/android/app/build/outputs/apk/release/app-release.apk "$outdir/SnackPilot-$VERSION.apk"
  ok "Android artifacts: $outdir/SnackPilot-$VERSION.{aab,apk}"
fi

# 6. commit + tag + history (skip on dry-run)
if [[ "$DRY_RUN" == "1" ]]; then
  ok "DRY RUN complete — artifacts in $outdir; no git or counter changes made"
  exit 0
fi
echo "$buildno" > "$BUILDNO_FILE"
git add "$CARGO" "$PROJECT_YML" "$GRADLE"
git commit -m "Release v$VERSION ($label)"
[[ $do_ios -eq 1 ]] && git tag "ios/v$VERSION"
[[ $do_android -eq 1 ]] && git tag "android/v$VERSION"
echo "$VERSION" >> "$HISTORY"
ok "committed Release v$VERSION ($label) and tagged locally"
info "artifacts in dist/. To publish the tags later: git push && git push --tags"
```

- [ ] **Step 2: Verify with a dry run (no mutations)**

Run: `cd /Users/radaiko/dev/private/SnackPilot-v2 && DRY_RUN=1 make ship`
At the prompts enter a version greater than the last (e.g. `2.0.1`) and platform `3`.
Expected: builds the iOS `.ipa` and Android `.aab`/`.apk` into a temp dir, prints `DRY RUN complete`. Then confirm nothing changed:

Run: `git status --porcelain && cat .ship-history 2>/dev/null; cat tools/devops/.build-number 2>/dev/null`
Expected: `git status` clean (no version-file edits), `.ship-history` / `.build-number` absent or unchanged.

- [ ] **Step 3: Verify version validation rejects bad input**

Run: `DRY_RUN=1 make ship`, enter `2.0` (or a version ≤ last).
Expected: aborts with `invalid version` / `must be strictly greater`, exit non-zero, no build started.

- [ ] **Step 4: Commit**

```bash
git add tools/devops/ship.sh
git commit -m "feat(devops): make ship (version bump + artifacts + local commit/tag)"
```

---

### Task 7: Document the devops tooling

**Files:**
- Create: `tools/devops/README.md`
- Modify: `README.md` (v2 repo root — add a "Local devops" pointer)

**Interfaces:** none (docs only).

- [ ] **Step 1: Write `tools/devops/README.md`**

````markdown
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

See `docs/superpowers/specs/2026-07-17-v2-local-devops-design.md` for the design and the Play upload-key continuity caveat.
````

- [ ] **Step 2: Add a pointer to the root `README.md`**

Add a short section (place after the existing build instructions):

```markdown
## Local devops

`make help` at the repo root lists the build/run/ship targets. See
[`tools/devops/README.md`](tools/devops/README.md).
```

- [ ] **Step 3: Verify the docs render and links resolve**

Run: `ls tools/devops/README.md && grep -q "Local devops" README.md && echo ok`
Expected: `ok`.

- [ ] **Step 4: Commit**

```bash
git add tools/devops/README.md README.md
git commit -m "docs(devops): document the make-based local devops tooling"
```

---

## Self-Review

**Spec coverage:**
- One-command build & run → Tasks 3 (iOS), 4 (Android). ✓
- Local ship pipeline (version bump, build number, artifacts, commit/tag, dry-run, METHOD) → Task 6. ✓
- Makefile single entrypoint + help/doctor/clean → Task 1. ✓
- Staleness-gated bootstrap reuse → Task 3 (`bootstrap_if_stale`), used by 3 & 4. ✓
- Version-carrying files + build number counter → Tasks 2 (helpers) + 6 (wiring). ✓
- Android keystore generation → Task 5. ✓
- dist/ + .ship-history + .build-number gitignored → Task 1 step 4; keystore/jks already ignored by `src/android/.gitignore` (verified). ✓
- No push (tags local) → Task 6. ✓
- Docs → Task 7. ✓
- Out-of-scope items (CI, store upload, Play key continuity, icons, privacy) → intentionally absent. ✓

**Placeholder scan:** No TBD/TODO; every code step shows full content; verification steps give exact commands + expected output. ✓

**Type/name consistency:** Helper names used in Task 6 (`set_cargo_package_version`, `set_yaml_key`, `set_gradle_string`, `set_gradle_int`, `validate_semver`, `version_gt`) match their definitions in Task 2. `bootstrap_if_stale` defined in Task 3, reused in Task 4. Bundle id `dev.radaiko.gourmetclient` consistent across run scripts. ✓

**Notes for the implementer:**
- macOS/BSD `sed -i ''` is assumed (iOS builds are macOS-only). Not portable to GNU sed.
- `run-ios.sh` uses `/usr/bin/python3` (present on macOS with Xcode CLT) to parse `simctl -j`. If unavailable, swap for a `jq` or plist-based parse.
- `IOS_SIM` default `iPhone 16 Pro` — adjust to an available device if that name isn't installed (the script errors with guidance).
- If iOS archive signing fails under automatic signing, that's a real signing-setup issue to resolve with the user (the memory notes Apple signing is "set up in Xcode" but archive/distribution may need a one-time provisioning confirmation), not a plan defect.
```
