#!/usr/bin/env bash
# Build a signed iOS release archive for App Store / TestFlight and open it in Xcode Organizer.
# Repeatable path: `make ios-archive` → Organizer → Distribute App → App Store Connect → Upload.
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool xcodebuild "install Xcode"
cd "$REPO_ROOT"

# Apple team from the keychain signing identity; override with IOS_TEAM=.
IOS_TEAM="${IOS_TEAM:-$(detect_ios_team)}"
[[ -n "$IOS_TEAM" ]] || die "no Apple Development team in keychain — set IOS_TEAM=<teamid> (see: security find-identity -v -p codesigning)"

bootstrap_if_stale ios "src/ios/Frameworks/SnackPilotCore.xcframework"

# Fresh build number each archive so every TestFlight upload has a unique CFBundleVersion.
build="$(next_build_number)"
day="$(date +%Y-%m-%d)"; hm="$(date +%H.%M)"
archives_dir="$HOME/Library/Developer/Xcode/Archives/$day"
mkdir -p "$archives_dir"
archive="$archives_dir/SnackPilot $hm build-$build.xcarchive"

info "archiving SnackPilot (build $build, team $IOS_TEAM)"
xcodebuild -project src/ios/SnackPilot.xcodeproj -scheme SnackPilot \
  -sdk iphoneos -destination "generic/platform=iOS" \
  -archivePath "$archive" -allowProvisioningUpdates \
  DEVELOPMENT_TEAM="$IOS_TEAM" CODE_SIGN_STYLE=Automatic \
  CURRENT_PROJECT_VERSION="$build" archive

ok "archive created (build $build): $archive"
info "Next: in Organizer → Distribute App → App Store Connect → Upload → it lands in TestFlight."
# Opening the .xcarchive launches Xcode Organizer with it selected. ORGANIZER_OPEN=0 skips (CI/tests).
[[ "${ORGANIZER_OPEN:-1}" == "1" ]] && open "$archive" || true
