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
