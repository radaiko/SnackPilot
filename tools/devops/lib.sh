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

# ── version helpers ──
validate_semver() { [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; }

# version_gt A B  → success iff A is strictly greater than B (semver via sort -V)
version_gt() {
  [[ "$1" != "$2" ]] || return 1
  local highest; highest="$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)"
  [[ "$highest" == "$1" ]]
}

verify_file_contains() { grep -qF -- "$2" "$1" || die "expected '$2' in $1 after edit — aborting"; }

# Replace [package].version only (leaves dependency versions alone).
set_cargo_package_version() {
  local file=$1 v=$2
  awk -v ver="$v" '
    /^\[package\]/ { inpkg=1 }
    /^\[/ && $0 !~ /^\[package\]/ { inpkg=0 }
    inpkg && /^version[[:space:]]*=/ { sub(/"[^"]*"/, "\"" ver "\"") }
    { print }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
  # verify the bump landed INSIDE [package], not merely somewhere in the file
  awk '
    /^\[package\]/ { inpkg=1 }
    /^\[/ && $0 !~ /^\[package\]/ { inpkg=0 }
    inpkg && /^version[[:space:]]*=/ { print }
  ' "$file" | grep -qF -- "version = \"$v\"" \
    || die "set_cargo_package_version: [package].version not set to $v in $file — aborting"
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

# path_is_stale SOURCE_DIR ARTIFACT → success (0) if ARTIFACT is missing or any file
# under SOURCE_DIR is newer than ARTIFACT (i.e. a rebuild is needed).
path_is_stale() {
  local source_dir=$1 artifact=$2
  [[ ! -e "$artifact" ]] && return 0
  [[ -n "$(find "$source_dir" -newer "$artifact" -print -quit 2>/dev/null)" ]]
}

# Rerun a platform bootstrap only when the core is newer than its built binding artifact.
# Watches Cargo.toml too: core_version() bakes in CARGO_PKG_VERSION at COMPILE time, so a
# version-only bump (every ship) must also force a core rebuild — and the version lives in
# Cargo.toml, outside src/core/src. Without this the shipped app reports a stale core version.
bootstrap_if_stale() {
  local platform=$1 artifact=$2
  if path_is_stale "$REPO_ROOT/src/core/src" "$artifact" \
     || [[ "$REPO_ROOT/src/core/Cargo.toml" -nt "$artifact" ]]; then
    info "core changed (or bindings absent) → running src/$platform/bootstrap.sh"
    ( cd "$REPO_ROOT/src/$platform" && ./bootstrap.sh )
  else
    ok "core bindings for $platform are up to date"
  fi
}

# Echo the Apple Team ID from the keychain signing identity (the cert's OU — NOT the id in
# the cert name). Empty output if no Apple Development identity is present.
detect_ios_team() {
  security find-certificate -c "Apple Development" -p 2>/dev/null \
    | openssl x509 -noout -subject -nameopt sep_multiline,utf8 2>/dev/null \
    | sed -n 's/^ *OU=//p' | head -1
}

# Next monotonic build number: increment the gitignored counter and echo it. Shared by ship
# + ios-archive so every build gets a unique, strictly-increasing CFBundleVersion/versionCode.
next_build_number() {
  local f="$REPO_ROOT/tools/devops/.build-number" n=0
  [[ -f "$f" ]] && n="$(cat "$f")"
  n=$((n + 1))
  echo "$n" > "$f"
  echo "$n"
}

# build_ios_archive BUILDNO [ARCHIVE_DIR] → regenerate the project + archive a signed release
# build, then echo the .xcarchive path (all logging is on stderr). Defaults to Xcode's Archives
# folder so it appears in Organizer. Team auto-detected (IOS_TEAM= overrides).
build_ios_archive() {
  local buildno=$1
  local archdir="${2:-$HOME/Library/Developer/Xcode/Archives/$(date +%Y-%m-%d)}"
  require_tool xcodebuild "install Xcode"
  require_tool xcodegen "brew install xcodegen"
  local team; team="${IOS_TEAM:-$(detect_ios_team)}"
  [[ -n "$team" ]] || die "no Apple team in keychain — set IOS_TEAM=<teamid> (see: security find-identity -v -p codesigning)"
  # >&2: this function is called via $(...) to capture the archive path, so the bootstrap's stdout
  # (now that a version bump makes the core stale) must NOT leak into that capture.
  bootstrap_if_stale ios "$REPO_ROOT/src/ios/Frameworks/SnackPilotCore.xcframework" >&2
  # Always regenerate the Xcode project so project.yml changes (esp. MARKETING_VERSION on a ship)
  # land in the build — bootstrap_if_stale skips xcodegen entirely when the core is unchanged.
  ( cd "$REPO_ROOT/src/ios" && xcodegen generate >&2 )
  mkdir -p "$archdir"
  local archive="$archdir/SnackPilot $(date +%H.%M) build-$buildno.xcarchive"
  info "iOS: archiving build $buildno (team $team)"
  xcodebuild -project "$REPO_ROOT/src/ios/SnackPilot.xcodeproj" -scheme SnackPilot \
    -sdk iphoneos -destination "generic/platform=iOS" \
    -archivePath "$archive" -allowProvisioningUpdates \
    DEVELOPMENT_TEAM="$team" CODE_SIGN_STYLE=Automatic CURRENT_PROJECT_VERSION="$buildno" archive >&2
  echo "$archive"
}
