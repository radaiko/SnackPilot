#!/usr/bin/env bash
# Build the snackpilot-core XCFramework + Swift bindings for the iOS app (src/ios).
#
# Prereqs (run once):
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   xcode-select --install
#
# Output:
#   src/core/target/SnackPilotCore.xcframework   — link this into the Xcode project
#   src/core/target/bindings-swift/              — snackpilot_core.swift + FFI header/modulemap
#
# Run from src/core/:  ./scripts/build-apple-xcframework.sh
set -euo pipefail
cd "$(dirname "$0")/.."   # -> src/core

LIB=libsnackpilot_core.a
OUT=target
BINDINGS="$OUT/bindings-swift"

echo "==> building device + simulator static libs (release)"
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
cargo build --release --target x86_64-apple-ios

echo "==> generating Swift bindings"
rm -rf "$BINDINGS"
cargo run --release --bin uniffi-bindgen -- generate \
  --library "target/aarch64-apple-ios/release/$LIB" \
  --language swift --out-dir "$BINDINGS"

# UniFFI emits <name>FFI.modulemap; XCFramework needs it named module.modulemap.
cp "$BINDINGS/snackpilot_coreFFI.modulemap" "$BINDINGS/module.modulemap"

echo "==> fat simulator lib (arm64 + x86_64)"
mkdir -p "$OUT/sim"
lipo -create \
  "target/aarch64-apple-ios-sim/release/$LIB" \
  "target/x86_64-apple-ios/release/$LIB" \
  -output "$OUT/sim/$LIB"

echo "==> assembling XCFramework"
rm -rf "$OUT/SnackPilotCore.xcframework"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/$LIB" -headers "$BINDINGS" \
  -library "$OUT/sim/$LIB" -headers "$BINDINGS" \
  -output "$OUT/SnackPilotCore.xcframework"

echo "==> done: $OUT/SnackPilotCore.xcframework + $BINDINGS/snackpilot_core.swift"
