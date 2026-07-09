#!/usr/bin/env bash
# Regenerate everything the iOS app needs from the Rust core, then the Xcode project.
#
# Produces (all git-ignored — this script is the source of truth):
#   Frameworks/SnackPilotCore.xcframework      from src/core
#   SnackPilot/Generated/snackpilot_core.swift from src/core
#   SnackPilot.xcodeproj                        from project.yml
#
# Prereqs: Rust + iOS targets, Xcode, xcodegen (brew install xcodegen).
# Run from src/ios/:  ./bootstrap.sh
set -euo pipefail
cd "$(dirname "$0")"                     # -> src/ios
CORE="../core"

echo "==> building the core XCFramework + Swift bindings"
(cd "$CORE" && ./scripts/build-apple-xcframework.sh)

echo "==> copying artifacts into the app"
rm -rf Frameworks/SnackPilotCore.xcframework
mkdir -p Frameworks SnackPilot/Generated
cp -R "$CORE/target/SnackPilotCore.xcframework" Frameworks/
cp "$CORE/target/bindings-swift/snackpilot_core.swift" SnackPilot/Generated/

echo "==> generating the Xcode project"
xcodegen generate

echo "==> done. Open SnackPilot.xcodeproj or:"
echo "    xcodebuild -scheme SnackPilot -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' build"
