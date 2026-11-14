#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool adb "install Android platform-tools"
BUNDLE_ID="dev.radaiko.gourmetclient"
cd "$REPO_ROOT"

# SDK/JDK discovery: the build needs a real JDK (javac), not just a JRE, and
# Gradle needs an SDK path. Both are picked up from the environment when set.
export ANDROID_HOME="${ANDROID_HOME:-$HOME/android-sdk}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
[ -x "$HOME/.local/jdk/bin/javac" ] && export JAVA_HOME="${JAVA_HOME:-$HOME/.local/jdk}"

# The emulator's hang watchdog kills its own vCPU threads after 15 s of no
# response, which is normal for software emulation (TCG) on a nested VM: raise
# it, and keep crash reporting out of the way. This VM's nested KVM hangs the
# vCPUs, so TCG is the default here; override with ANDROID_EMU_ACCEL="".
export ANDROID_EMULATOR_WAIT_TIME_BEFORE_KILL="${ANDROID_EMULATOR_WAIT_TIME_BEFORE_KILL:-1800000}"
export ANDROID_EMU_ENABLE_CRASH_REPORTING="${ANDROID_EMU_ENABLE_CRASH_REPORTING:-0}"
ANDROID_EMU_ACCEL="${ANDROID_EMU_ACCEL---accel off}"

# An AVD name is optional: use the first one that exists when none is given.
if [[ -z "${ANDROID_AVD:-}" ]]; then
  ANDROID_AVD="$("${ANDROID_HOME}/emulator/emulator" -list-avds 2>/dev/null | head -1)"
  [[ -n "$ANDROID_AVD" ]] || die "no AVD found — create one with avdmanager, or set ANDROID_AVD"
fi

# newest .so is the freshness marker for the Android bindings
so_marker="$(ls -t src/ui/android/app/src/main/jniLibs/*/libsnackpilot_app.so 2>/dev/null | head -1 || true)"
bootstrap_if_stale tools/devops/ui-android-build.sh "${so_marker:-/nonexistent}"

# Ensure a device/emulator is connected; else start the default AVD.
if [[ -z "$(adb devices | awk 'NR>1 && $2=="device"{print $1}')" ]]; then
  require_tool emulator "install the Android emulator package"
  info "no device — starting AVD '$ANDROID_AVD'"
  ( emulator -avd "$ANDROID_AVD" -no-window -no-audio -no-boot-anim -no-snapshot \
      -gpu swiftshader_indirect -cores 4 -memory 3072 $ANDROID_EMU_ACCEL >/dev/null 2>&1 & )
  adb wait-for-device
  info "waiting for boot to complete (software emulation takes several minutes)"
  boot_tries=0
  until [[ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" == "1" ]]; do
    sleep 5; boot_tries=$((boot_tries + 1))
    [[ $boot_tries -ge 240 ]] && die "emulator '$ANDROID_AVD' did not finish booting after ~20 min — check the AVD"
  done
fi
ok "device ready: $(adb devices | awk 'NR>1 && $2=="device"{print $1; exit}')"

info "building (assembleDebug)"
( cd src/ui/android && ./gradlew :app:assembleDebug )
apk="$(ls -t src/ui/android/app/build/outputs/apk/debug/*.apk | head -1)"

# `gradlew installDebug` uses ddmlib, whose 5 s shell timeout a software-emulated
# guest blows through (ShellCommandUnresponsiveException); adb install with no
# timeout works.
info "installing $(basename "$apk")"
adb install -r -t "$apk"

info "launching"
launch_out="$(adb shell monkey -p "$BUNDLE_ID" -c android.intent.category.LAUNCHER 1 2>&1)"
if grep -qiE 'No activities found|monkey aborted|Error:' <<<"$launch_out"; then
  err "$launch_out"
  die "launch failed — no launchable LAUNCHER activity for $BUNDLE_ID"
fi
ok "SnackPilot launched on device"
