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
  ok "ANDROID_NDK_HOME=$ANDROID_NDK_HOME"
else
  err "ANDROID_NDK_HOME unset or not a directory — export it (see src/android/bootstrap.sh)"; status=1
fi
[[ $status -eq 0 ]] && ok "all prerequisites present" || err "some prerequisites missing"
exit $status
