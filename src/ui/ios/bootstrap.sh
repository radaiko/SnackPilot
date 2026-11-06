#!/usr/bin/env bash
# SnackPilot iOS spike host (task 2.2): build the Rust staticlib for the Apple
# simulator, then generate SnackPilot.xcodeproj with xcodegen.
#
# Run from anywhere, on the Mac:  bash src/ui/ios/bootstrap.sh
# Then: xcodebuild -project SnackPilot.xcodeproj -scheme SnackPilot \
#        -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
#        CODE_SIGNING_ALLOWED=NO build
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"   # src/ui/ios
UI="$HERE/.."                           # src/ui (the cargo workspace root)

export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"

# The example links the app as a staticlib; the manifest stays cdylib+lib for
# Android and host tests, so override the crate type here instead of Cargo.toml.
# rustc emits it into deps/ (cargo only copies manifest crate types out of deps/),
# so copy it next to the other artifacts where project.yml expects it.
for target in "${RUST_IOS_TARGETS:-aarch64-apple-ios-sim}"; do
  echo "==> cargo staticlib: snackpilot-app for $target"
  ( cd "$UI" && cargo rustc -p snackpilot-app --lib --target "$target" -- --crate-type=staticlib )
  out="$UI/target/$target/debug"
  cp "$out/deps/libsnackpilot_app.a" "$out/libsnackpilot_app.a"
  ls -la "$out/libsnackpilot_app.a"
done

echo "==> xcodegen generate"
cd "$HERE"
xcodegen generate
