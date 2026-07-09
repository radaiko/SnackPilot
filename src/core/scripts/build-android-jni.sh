#!/usr/bin/env bash
# Build the snackpilot-core JNI libraries + Kotlin bindings for the Android app (src/android).
#
# Prereqs (run once):
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   cargo install cargo-ndk
#   # Android NDK installed; set ANDROID_NDK_HOME.
#
# Output:
#   src/core/target/jniLibs/<abi>/libsnackpilot_core.so   — copy into app/src/main/jniLibs/
#   src/core/target/bindings-kotlin/                      — uniffi/snackpilot_core/snackpilot_core.kt
#
# Run from src/core/:  ./scripts/build-android-jni.sh
set -euo pipefail
cd "$(dirname "$0")/.."   # -> src/core

OUT=target
JNILIBS="$OUT/jniLibs"
BINDINGS="$OUT/bindings-kotlin"

echo "==> building JNI shared libs via cargo-ndk (arm64-v8a, armeabi-v7a, x86_64)"
cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o "$JNILIBS" \
  build --release

echo "==> generating Kotlin bindings"
rm -rf "$BINDINGS"
cargo run --release --bin uniffi-bindgen -- generate \
  --library "$JNILIBS/arm64-v8a/libsnackpilot_core.so" \
  --language kotlin --out-dir "$BINDINGS"

echo "==> done: $JNILIBS/<abi>/libsnackpilot_core.so + $BINDINGS/uniffi/snackpilot_core/snackpilot_core.kt"
