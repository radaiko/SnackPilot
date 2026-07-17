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
