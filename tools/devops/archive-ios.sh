#!/usr/bin/env bash
# Build a signed iOS release archive for App Store / TestFlight and open it in Xcode Organizer.
# Repeatable path: `make ios-archive` → Organizer → Distribute App → App Store Connect → Upload.
# (`make ship` runs the same archive step for iOS; use this when you only want a new iOS build.)
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT"

build="$(next_build_number)"
archive="$(build_ios_archive "$build")"
ok "archive created (build $build): $archive"
info "Next: in Organizer → Distribute App → App Store Connect → Upload → it lands in TestFlight."
# Opening the .xcarchive launches Xcode Organizer with it selected. ORGANIZER_OPEN=0 skips (CI/tests).
[[ "${ORGANIZER_OPEN:-1}" == "1" ]] && open "$archive" || true
