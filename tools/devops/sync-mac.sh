#!/usr/bin/env bash
# rsync the current worktree to the Mac so non-interactive `ssh mac` builds see it.
# Usage: sync-mac.sh [--dry-run]   (tests: SNACKPILOT_MAC_SRC= / SNACKPILOT_MAC_DEST= overrides)
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool rsync

SRC="${SNACKPILOT_MAC_SRC:-$REPO_ROOT}"
DEST="${SNACKPILOT_MAC_DEST:-mac:~/vm-sync/SnackPilot-gpui/}"

dry=0
case "${1:-}" in
  --dry-run) dry=1 ;;
  "") ;;
  *) die "usage: sync-mac.sh [--dry-run]" ;;
esac

# ~/vm-sync/SnackPilot is the mutagen-managed copy — never a --delete target.
dest_path="${DEST#*:}"          # strip "host:" if present
dest_path="${dest_path%/}"      # strip a trailing slash
case "$dest_path" in
  ~/vm-sync/SnackPilot|*/vm-sync/SnackPilot)
    die "refusing to sync to '$DEST': ~/vm-sync/SnackPilot is the mutagen-managed copy — use a distinct directory (default: ~/vm-sync/SnackPilot-gpui/)" ;;
esac

# Excluded paths are also protected from --delete: rsync never removes excluded files
# on the destination unless --delete-excluded is passed (and it never is here). These
# are generated on the Mac and must survive. --exclude (not --filter -p) so the Mac's
# openrsync (protocol 29) accepts the rules.
PROTECTED=(.git target build DerivedData '*.xcodeproj' .gradle node_modules)
filters=()
for p in "${PROTECTED[@]}"; do filters+=(--exclude="$p"); done

cmd=(rsync -a --delete "${filters[@]}")
(( dry )) && cmd+=(--dry-run -v --itemize-changes)
cmd+=("$SRC/" "$DEST")

if (( dry )); then
  info "plan: ${cmd[*]}"
else
  # rsync only creates the leaf of a remote path; ensure the parent exists first.
  if [[ "$DEST" == *:* && "$DEST" != /* ]]; then
    ssh "${DEST%%:*}" "mkdir -p $dest_path"
  fi
fi

info "syncing $SRC → $DEST"
"${cmd[@]}"
(( dry )) || ok "mac copy up to date at $DEST"
