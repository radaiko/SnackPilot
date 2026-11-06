#!/usr/bin/env bash
# Unit tests for sync-mac.sh: exclusion/protection plan and the SnackPilot refusal,
# all against a local temp tree — never contacts the Mac. Run: bash tools/devops/test-sync-mac.sh
# @req REQ-UI-01 (Mac sync enabler for the gpui-kit UI rewrite)
set -uo pipefail
dir="$(cd "$(dirname "$0")" && pwd)"

fails=0
assert()     { if eval "$1"; then echo "ok: $2"; else echo "FAIL: $2"; fails=$((fails+1)); fi; }
assert_not() { if eval "$1"; then echo "FAIL: $2"; fails=$((fails+1)); else echo "ok: $2"; fi; }
plan_has()   { case "$out" in *"$1"*) true;; *) false;; esac; }

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
src="$tmp/worktree"; dest="$tmp/mac"
mkdir -p "$src"/{.git,target,build,DerivedData,Foo.xcodeproj,.gradle,node_modules}
echo x > "$src/AGENTS.md"

# --dry-run against a local temp dest: succeeds, prints the plan, copies nothing.
out="$(SNACKPILOT_MAC_SRC="$src" SNACKPILOT_MAC_DEST="$dest/" bash "$dir/sync-mac.sh" --dry-run 2>&1)" && rc=0 || rc=$?
assert '[ "$rc" -eq 0 ]' 'dry-run against a local temp dest succeeds'
assert 'plan_has -- --delete' 'plan has --delete'
for pat in '.git' 'target' 'build' 'DerivedData' '*.xcodeproj' '.gradle' 'node_modules'; do
  assert "plan_has '--exclude=$pat'" "plan excludes and protects $pat"
done
assert_not '[ -e "$dest/AGENTS.md" ]' 'dry-run copies nothing'
assert_not 'plan_has "--delete-excluded"' 'plan never passes --delete-excluded (protects excludes from deletion)'

# Refusal: the mutagen-managed ~/vm-sync/SnackPilot is never a valid destination.
out="$(SNACKPILOT_MAC_DEST='mac:~/vm-sync/SnackPilot' bash "$dir/sync-mac.sh" --dry-run 2>&1)" && rc=0 || rc=$?
assert '[ "$rc" -ne 0 ]' 'refuses mac:~/vm-sync/SnackPilot'
assert 'plan_has mutagen' 'refusal message names the mutagen copy'
out="$(SNACKPILOT_MAC_DEST='mac:~/vm-sync/SnackPilot/' bash "$dir/sync-mac.sh" --dry-run 2>&1)" && rc=0 || rc=$?
assert '[ "$rc" -ne 0 ]' 'refuses mac:~/vm-sync/SnackPilot/ (trailing slash)'
out="$(SNACKPILOT_MAC_DEST='/Users/radaiko/vm-sync/SnackPilot' bash "$dir/sync-mac.sh" --dry-run 2>&1)" && rc=0 || rc=$?
assert '[ "$rc" -ne 0 ]' 'refuses absolute .../vm-sync/SnackPilot'

# The gpui default suffix is accepted (checked without rsyncing: refusal passes, then dry-run runs).
out="$(SNACKPILOT_MAC_SRC="$src" SNACKPILOT_MAC_DEST="$dest/../SnackPilot-gpui" bash "$dir/sync-mac.sh" --dry-run 2>&1)" && rc=0 || rc=$?
assert '[ "$rc" -eq 0 ]' 'a SnackPilot-gpui dest is not refused'

echo
[[ $fails -eq 0 ]] && { echo "all sync-mac tests passed"; exit 0; } || { echo "$fails sync-mac test(s) failed"; exit 1; }
