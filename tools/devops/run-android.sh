#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool adb "install Android platform-tools"
BUNDLE_ID="dev.radaiko.gourmetclient"
ANDROID_AVD="${ANDROID_AVD:-Medium_Phone_API_36.1}"
cd "$REPO_ROOT"

# newest .so is the freshness marker for the Android bindings
so_marker="$(ls -t src/android/app/src/main/jniLibs/*/libsnackpilot_core.so 2>/dev/null | head -1 || true)"
bootstrap_if_stale android "${so_marker:-/nonexistent}"

# Ensure a device/emulator is connected; else start the default AVD.
if [[ -z "$(adb devices | awk 'NR>1 && $2=="device"{print $1}')" ]]; then
  require_tool emulator "install the Android emulator package"
  info "no device — starting AVD '$ANDROID_AVD'"
  ( emulator -avd "$ANDROID_AVD" -netdelay none -netspeed full >/dev/null 2>&1 & )
  adb wait-for-device
  info "waiting for boot to complete"
  boot_tries=0
  until [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; do
    sleep 2; boot_tries=$((boot_tries + 1))
    [[ $boot_tries -ge 90 ]] && die "emulator '$ANDROID_AVD' did not finish booting after ~180s — check the AVD"
  done
fi
ok "device ready: $(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')"

info "building + installing (installDebug)"
( cd src/android && ./gradlew :app:installDebug )

info "launching"
launch_out="$(adb shell monkey -p "$BUNDLE_ID" -c android.intent.category.LAUNCHER 1 2>&1)"
if grep -qiE 'No activities found|monkey aborted|Error:' <<<"$launch_out"; then
  err "$launch_out"
  die "launch failed — no launchable LAUNCHER activity for $BUNDLE_ID"
fi
ok "SnackPilot launched on device"
