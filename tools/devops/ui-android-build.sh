#!/usr/bin/env bash
# Build the snackpilot-app Rust cdylib for Android into the Gradle host's
# jniLibs (task 2.1 spike). x86_64 = headless emulator, arm64-v8a = devices.
#
# Usage: tools/devops/ui-android-build.sh [--release] [extra cargo-ndk args]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

export ANDROID_HOME="${ANDROID_HOME:-$HOME/android-sdk}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  # Pick the highest installed NDK (spike pins r30 via sdkmanager).
  ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/*/ 2>/dev/null | sort -V | tail -1 || true)"
  export ANDROID_NDK_HOME
fi
[[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME}" ]] || {
  echo "error: no NDK found under $ANDROID_HOME/ndk — run: sdkmanager \"ndk;30.0.16248370\"" >&2
  exit 1
}

cd "$REPO_ROOT/src/ui"
# The gpui-mobile example sets this in example/android/.cargo/config.toml:
# cosmic-text's fontconfig backend dlopens libfontconfig at runtime instead
# of needing a cross pkg-config sysroot at build time.
export RUST_FONTCONFIG_DLOPEN=on
echo "▸ cargo ndk (x86_64 + arm64-v8a) NDK=$ANDROID_NDK_HOME"
cargo ndk \
  -t x86_64 \
  -t arm64-v8a \
  -o android/app/src/main/jniLibs \
  --platform 31 \
  build -p snackpilot-app \
  "$@"

for abi_dir in android/app/src/main/jniLibs/*/; do
  so="$abi_dir/libsnackpilot_app.so"
  [[ -f "$so" ]] || { echo "error: missing $so" >&2; exit 1; }
  # cargo-ndk also copies dependency dylibs (libgpui_mobile-*, libsnackpilot_core);
  # the app cdylib links them statically and needs only system libs, so keep the
  # APK small by pruning them.
  find "$abi_dir" -name '*.so' ! -name libsnackpilot_app.so -delete
  echo "▸ $so ($(du -h "$so" | cut -f1))"
done
