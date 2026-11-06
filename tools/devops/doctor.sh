#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

info "checking devops prerequisites"
status=0
check() {  # name hint
  if have_tool "$1"; then ok "$1"; else err "$1 missing${2:+ — $2}"; status=1; fi
}
check cargo       "install Rust: https://rustup.rs"
check xcodegen    "brew install xcodegen"
check xcodebuild  "install Xcode + command line tools"
check cargo-ndk   "cargo install cargo-ndk"
check adb         "install Android platform-tools"
check keytool     "comes with a JDK"
if [[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME:-}" ]]; then
  ndk_home="${ANDROID_NDK_HOME%/}"
elif [[ -d "${ANDROID_HOME:-$HOME/android-sdk}/ndk" ]]; then
  ndk_home="$(ls -d "${ANDROID_HOME:-$HOME/android-sdk}"/ndk/*/ 2>/dev/null | sort -V | tail -1 || true)"
  ndk_home="${ndk_home%/}"
else
  ndk_home=""
fi
if [[ -n "$ndk_home" && -d "$ndk_home" ]]; then
  ok "NDK $("$ndk_home/ndk-build" -v 2>&1 | head -1) at $ndk_home"
else
  err "Android NDK not found — export ANDROID_NDK_HOME or run: sdkmanager \"ndk;30.0.16248370\""; status=1
fi
[[ $status -eq 0 ]] && ok "all prerequisites present" || err "some prerequisites missing"
exit $status
