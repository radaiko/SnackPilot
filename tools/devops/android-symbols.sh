#!/usr/bin/env bash
# Inject native debug symbols (from the prebuilt Rust core .so) into an AAB and re-sign it.
# AGP's debugSymbolLevel only extracts symbols from libs IT builds, not our cargo-ndk jniLibs,
# so we bundle them ourselves at the exact path Play expects, then re-sign with the upload key.
#
# Usage: bash tools/devops/android-symbols.sh <path-to.aab>
set -euo pipefail
source "$(dirname "$0")/lib.sh"

AAB="${1:?usage: android-symbols.sh <aab>}"
[[ -f "$AAB" ]] || die "AAB not found: $AAB"
cd "$REPO_ROOT"

NDK="${ANDROID_NDK_HOME:-$(ls -d "$HOME/Library/Android/sdk/ndk/"* 2>/dev/null | sort -V | tail -1)}"
OBJCOPY="$(ls "$NDK"/toolchains/llvm/prebuilt/*/bin/llvm-objcopy 2>/dev/null | head -1)"
[[ -x "$OBJCOPY" ]] || die "llvm-objcopy not found under ANDROID_NDK_HOME=$NDK"
require_tool jarsigner "comes with a JDK"

props="src/android/keystore.properties"
[[ -f "$props" ]] || die "no $props — can't re-sign"
ksFile=$(sed -n 's/^storeFile=//p' "$props"); ksPass=$(sed -n 's/^storePassword=//p' "$props")
alias=$(sed -n 's/^keyAlias=//p' "$props"); keyPass=$(sed -n 's/^keyPassword=//p' "$props")

work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
meta="BUNDLE-METADATA/com.android.tools.build.debugsymbols"
for abi in "$REPO_ROOT"/src/android/app/src/main/jniLibs/*/; do
  a="$(basename "$abi")"
  for so in "$abi"*.so; do
    [[ -e "$so" ]] || continue
    mkdir -p "$work/$meta/$a"
    # --strip-debug keeps the ELF symbol table (.symtab) → SYMBOL_TABLE level, smaller than full.
    "$OBJCOPY" --strip-debug "$so" "$work/$meta/$a/$(basename "$so").dbg"
    info "symbols: $a/$(basename "$so").dbg"
  done
done

info "removing old signature + injecting symbols"
zip -q -d "$AAB" 'META-INF/*' >/dev/null 2>&1 || true
( cd "$work" && zip -q -r -X "$REPO_ROOT/$AAB" BUNDLE-METADATA )

info "re-signing AAB with the upload key"
jarsigner -keystore "$ksFile" -storepass "$ksPass" -keypass "$keyPass" \
  -digestalg SHA-256 -sigalg SHA256withRSA "$AAB" "$alias" >/dev/null

ok "injected native symbols + re-signed: $AAB"
