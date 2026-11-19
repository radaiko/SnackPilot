#!/usr/bin/env bash
# Build a signed iOS release archive, export the App Store .ipa and open Organizer.
# Repeatable path: `make ios-archive` → dist/SnackPilot-<version>.ipa, then upload it
# (`xcrun altool --upload-app -f … -t ios --apiKey … --apiIssuer …` or Transporter).
# (`make ship` runs the same steps for iOS; use this when you only want a new iOS build.)
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT"

version="$(grep -m1 'MARKETING_VERSION:' src/ui/ios/project.yml | sed -E 's/.*"([^"]+)".*/\1/')"
build="$(next_build_number)"
archive="$(build_ios_archive "$build")"
ok "archive created (build $build): $archive"
ipa="$(export_ios_ipa "$archive" "$REPO_ROOT/dist" "SnackPilot-$version")"
ok "ipa: $ipa"
info "Upload: xcrun altool --upload-app -f '$ipa' -t ios --apiKey <id> --apiIssuer <issuer>"
# Opening the .xcarchive launches Xcode Organizer with it selected. ORGANIZER_OPEN=1 opts in.
[[ "${ORGANIZER_OPEN:-0}" == "1" ]] && open "$archive" || true
