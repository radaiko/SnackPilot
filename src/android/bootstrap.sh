#!/usr/bin/env bash
# Regenerate everything the Android app needs from the Rust core.
#
# Produces (all git-ignored — this script is the source of truth):
#   app/src/main/jniLibs/<abi>/libsnackpilot_core.so       from src/core (cargo-ndk)
#   app/src/main/java/uniffi/snackpilot_core/snackpilot_core.kt  from src/core
#
# Prereqs: Rust + Android targets, cargo-ndk, Android NDK.
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   cargo install cargo-ndk
#   export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/<version>
# Run from src/android/:  ./bootstrap.sh
set -euo pipefail
cd "$(dirname "$0")"                     # -> src/android
CORE="../core"

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    # Best-effort default: newest NDK under the standard SDK path.
    CAND=$(ls -d "$HOME/Library/Android/sdk/ndk/"* 2>/dev/null | sort -V | tail -1 || true)
    [ -n "$CAND" ] && export ANDROID_NDK_HOME="$CAND"
fi
echo "==> ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-<unset>}"

echo "==> building JNI libs + Kotlin bindings from the core"
(cd "$CORE" && ./scripts/build-android-jni.sh)

echo "==> copying artifacts into the app"
rm -rf app/src/main/jniLibs app/src/main/java/uniffi
mkdir -p app/src/main/jniLibs app/src/main/java/uniffi/snackpilot_core
cp -R "$CORE/target/jniLibs/"* app/src/main/jniLibs/
cp "$CORE/target/bindings-kotlin/uniffi/snackpilot_core/snackpilot_core.kt" \
   app/src/main/java/uniffi/snackpilot_core/

echo "==> done. Build with:  ./gradlew :app:assembleDebug"
