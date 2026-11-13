#!/usr/bin/env bash
# Offline tests for tools/devops/package-macos.sh.
#
# Every Apple/Rust tool is replaced by a stub on PATH: nothing is built, signed,
# notarized or contacted. The tests assert the release contract the real Mac run
# must satisfy: arm64/minimum OS/plist/version/assets, signing and hardened
# runtime, accepted notarization, stapled trust, the receipt checksum, and that
# every missing or failing stage stops the release without a receipt or ZIP.
#
# Run: bash tools/devops/test-package-macos.sh   (also part of `make test-lib`)
# @req REQ-DST-01 @req REQ-DST-02 @req REQ-DST-04 @req REQ-UI-14
set -uo pipefail
unset SNACKPILOT_SIGNING_IDENTITY SNACKPILOT_NOTARY_PROFILE \
    SNACKPILOT_PRODUCT_VERSION SNACKPILOT_BUILD

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT="$ROOT/tools/devops/package-macos.sh"
IDENTITY="Developer ID Application: SnackPilot Test (TEAM123456)"
PROFILE="snackpilot-notary-profile"

fails=0
pass() { echo "ok: $1"; }
fail() { echo "FAIL: $1"; fails=$((fails + 1)); }
check() { local name="$1"; shift; if "$@"; then pass "$name"; else fail "$name"; fi; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
log="$tmp/tools.log"

write_stub() {
    local name="$1"
    cat > "$tmp/bin/$name"
    chmod +x "$tmp/bin/$name"
}

# --- stubs ------------------------------------------------------------------
write_stub cargo <<'STUB'
#!/usr/bin/env bash
echo "cargo $*" >> "$TOOLS_LOG"
echo "cargo-env MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-unset}" >> "$TOOLS_LOG"
target=""
previous=""
for arg in "$@"; do
    [ "$previous" = "--target" ] && target="$arg"
    previous="$arg"
done
[ -n "$target" ] || { echo "cargo stub: no --target" >&2; exit 2; }
[ "${CARGO_FAIL:-0}" = 1 ] && exit 1
dir="${CARGO_TARGET_DIR:?}/$target/release"
mkdir -p "$dir"
printf '#!/bin/sh\nexit 0\n' > "$dir/snackpilot-desktop"
chmod +x "$dir/snackpilot-desktop"
printf 'nested runtime\n' > "$dir/libsnackpilot-extra.dylib"
STUB

write_stub codesign <<'STUB'
#!/usr/bin/env bash
echo "codesign $*" >> "$TOOLS_LOG"
case "$1" in
    --force)
        [ "${CODESIGN_SIGN_FAIL:-0}" = 1 ] && exit 1
        exit 0 ;;
    -dv)
        if [ "${CODESIGN_ADHOC:-0}" = 1 ]; then
            echo "Identifier=dev.radaiko.gourmetclient" >&2
            echo "Signature=adhoc" >&2
            echo "flags=0x2(adhoc)" >&2
        else
            echo "Identifier=dev.radaiko.gourmetclient" >&2
            echo "Authority=Developer ID Application: SnackPilot Test (TEAM123456)" >&2
            echo "TeamIdentifier=TEAM123456" >&2
            echo "flags=0x10000(runtime)" >&2
        fi
        exit 0 ;;
    --verify)
        [ "${CODESIGN_VERIFY_FAIL:-0}" = 1 ] && exit 1
        exit 0 ;;
esac
exit 0
STUB

write_stub xcrun <<'STUB'
#!/usr/bin/env bash
echo "xcrun $*" >> "$TOOLS_LOG"
case "$1 $2" in
    "notarytool submit")
        if [ "${NOTARY_REJECT:-0}" = 1 ]; then
            echo "Submission ID received: 0000"
            echo "status: Invalid"
            echo "message: The archive is invalid." >&2
            exit 1
        fi
        echo "Submission ID received: 0000"
        echo "status: Accepted"
        exit 0 ;;
    "stapler staple")
        [ "${STAPLER_FAIL:-0}" = 1 ] && exit 1
        exit 0 ;;
    "stapler validate")
        [ "${STAPLER_FAIL:-0}" = 1 ] && exit 1
        exit 0 ;;
esac
exit 0
STUB

write_stub ditto <<'STUB'
#!/usr/bin/env bash
echo "ditto $*" >> "$TOOLS_LOG"
last="${!#}"
: > "$last"
exit 0
STUB

write_stub spctl <<'STUB'
#!/usr/bin/env bash
echo "spctl $*" >> "$TOOLS_LOG"
if [ "${SPCTL_FAIL:-0}" = 1 ]; then
    echo "$4: rejected" >&2
    exit 1
fi
last="${!#}"
echo "$last: accepted"
exit 0
STUB

write_stub lipo <<'STUB'
#!/usr/bin/env bash
echo "lipo $*" >> "$TOOLS_LOG"
echo "${LIPO_ARCH:-arm64}"
exit 0
STUB

write_stub otool <<'STUB'
#!/usr/bin/env bash
echo "otool $*" >> "$TOOLS_LOG"
cat <<OUT
Load command 8
      cmd LC_BUILD_VERSION
  cmdsize 32
 platform 1
    minos ${OTOOL_MINOS:-26.0}
      sdk 26.0
OUT
exit 0
STUB

write_stub file <<'STUB'
#!/usr/bin/env bash
echo "file $*" >> "$TOOLS_LOG"
echo "$2: Mach-O 64-bit executable ${FILE_ARCH:-arm64}"
exit 0
STUB

run_case() {
    local out="$1"
    shift
    local -a envs=()
    while [ $# -gt 0 ] && [ "$1" != "--" ]; do
        envs+=("$1")
        shift
    done
    [ "${1:-}" = "--" ] && shift
    : > "$log"
    env \
        SNACKPILOT_PACKAGE_OUT="$out" \
        CARGO_TARGET_DIR="$tmp/target" \
        TOOLS_LOG="$log" \
        PATH="$tmp/bin:$PATH" \
        "${envs[@]}" \
        bash "$SCRIPT" "$@"
}

project_value() {
    awk -v key="$1" '$1 == key":" { value=$2; gsub(/"/, "", value); print value; exit }' \
        "$ROOT/src/ui/ios/project.yml"
}

plist_value() {
    awk -v key="$1" '
        index($0, "<key>" key "</key>") { getline; gsub(/^[ \t]*<string>|<\/string>[ \t]*$/, ""); print; exit }
    ' "$2"
}

log_line() { grep -n -- "$1" "$log" | head -1 | cut -d: -f1; }

receipt_field() {
    grep -o "\"$2\": \"[^\"]*\"" "$1" | head -1 | sed 's/.*": "//; s/"$//'
}

# --- success ----------------------------------------------------------------
out="$tmp/out-ok"
if run_case "$out" \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" \
    SNACKPILOT_NOTARY_PROFILE="$PROFILE" > "$tmp/success.log" 2>&1; then
    pass "a fully configured package succeeds"
else
    fail "a fully configured package succeeds"
    sed 's/^/    /' "$tmp/success.log"
fi

version="$(project_value MARKETING_VERSION)"
build="$(project_value CURRENT_PROJECT_VERSION)"
app="$out/SnackPilot.app"
receipt="$out/native-build-receipt.json"
plist="$app/Contents/Info.plist"

check "the .app bundle and executable exist" test -x "$app/Contents/MacOS/SnackPilot"
if [ "$(head -c 4 "$app/Contents/Resources/SnackPilot.icns")" = "icns" ]; then
    pass "the icon is an ICNS resource"
else
    fail "the icon is an ICNS resource"
fi
if [ -f "$plist" ] && [ "$(plist_value CFBundleIdentifier "$plist")" = "dev.radaiko.gourmetclient" ]; then
    pass "Info.plist keeps the stable bundle identity"
else
    fail "Info.plist keeps the stable bundle identity"
fi
[ -f "$plist" ] && [ "$(plist_value CFBundleShortVersionString "$plist")" = "$version" ] \
    && pass "Info.plist carries the common version" || fail "Info.plist carries the common version"
[ -f "$plist" ] && [ "$(plist_value CFBundleVersion "$plist")" = "$build" ] \
    && pass "Info.plist carries the common build" || fail "Info.plist carries the common build"
[ -f "$plist" ] && [ "$(plist_value LSMinimumSystemVersion "$plist")" = "26.0" ] \
    && pass "Info.plist requires macOS 26.0" || fail "Info.plist requires macOS 26.0"
[ -f "$plist" ] && [ "$(plist_value CFBundleIconFile "$plist")" = "SnackPilot" ] \
    && pass "Info.plist names the product icon" || fail "Info.plist names the product icon"

check "the receipt is written" test -f "$receipt"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" platform)" = "macos" ] \
    && pass "receipt platform is macos" || fail "receipt platform is macos"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" architecture)" = "aarch64" ] \
    && pass "receipt architecture is aarch64" || fail "receipt architecture is aarch64"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" version)" = "$version" ] \
    && pass "receipt version matches the build input" || fail "receipt version matches the build input"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" build)" = "$build" ] \
    && pass "receipt build matches the build input" || fail "receipt build matches the build input"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" trust_state)" = "developer-id-notarized-stapled" ] \
    && pass "receipt trust is only claimed as developer-id-notarized-stapled" || fail "receipt trust is only claimed as developer-id-notarized-stapled"
[ -f "$receipt" ] && [ "$(receipt_field "$receipt" checksum_scheme)" = "sha256-bundle-manifest-v1" ] \
    && pass "receipt uses the bundle-manifest checksum scheme" || fail "receipt uses the bundle-manifest checksum scheme"
[ -f "$receipt" ] && [[ "$(receipt_field "$receipt" checksum)" =~ ^[0-9a-f]{64}$ ]] \
    && pass "receipt checksum is SHA-256" || fail "receipt checksum is SHA-256"

# No public/distribution archive survives; only the temporary notary transport
# existed and it is deleted.
if find "$out" -name '*.zip' | grep -q .; then
    fail "no public ZIP or leftover notary archive"
else
    pass "no public ZIP or leftover notary archive"
fi

# The release flags are part of the contract, not incidental: a dropped
# `--locked`/`--release`/`--target`, deployment target, hardened-runtime signing
# option or a notary submission without the configured profile/wait fails here.
check "cargo runs the locked release build for aarch64-apple-darwin" \
    grep -Fqx 'cargo build --locked --release -p snackpilot-app --bin snackpilot-desktop --target aarch64-apple-darwin' "$log"
check "the release build pins MACOSX_DEPLOYMENT_TARGET=26.0" \
    grep -Fqx 'cargo-env MACOSX_DEPLOYMENT_TARGET=26.0' "$log"
check "nested code is signed with the hardened runtime and a secure timestamp" \
    grep -Eq '^codesign --force --options runtime --timestamp --sign .*libsnackpilot-extra\.dylib$' "$log"
check "the outer app is signed with the hardened runtime and a secure timestamp" \
    grep -Eq '^codesign --force --options runtime --timestamp --sign .*SnackPilot\.app$' "$log"
check "notarization uses the configured keychain profile and waits for the verdict" \
    grep -Eq "^xcrun notarytool submit .* --keychain-profile $PROFILE --wait$" "$log"

# Stage order: nested code before the outer app, then notarize, staple, verify.
nested="$(log_line 'libsnackpilot-extra.dylib')"
outer="$(grep -n 'codesign --force .*/SnackPilot.app$' "$log" | head -1 | cut -d: -f1)"
submit="$(log_line 'xcrun notarytool submit')"
staple="$(log_line 'xcrun stapler staple')"
staple_validate="$(log_line 'xcrun stapler validate')"
verify="$(log_line 'codesign --verify')"
assess="$(log_line 'spctl --assess')"
[ -n "$nested" ] && [ -n "$outer" ] && [ "$nested" -lt "$outer" ] \
    && pass "nested code is signed before the outer app" || fail "nested code is signed before the outer app"
[ -n "$outer" ] && [ -n "$submit" ] && [ "$outer" -lt "$submit" ] \
    && pass "signing happens before notarization" || fail "signing happens before notarization"
[ -n "$submit" ] && [ -n "$staple" ] && [ "$submit" -lt "$staple" ] \
    && pass "notarization is accepted before stapling" || fail "notarization is accepted before stapling"
[ -n "$staple" ] && [ -n "$staple_validate" ] && [ "$staple" -lt "$staple_validate" ] \
    && pass "the stapled app is validated" || fail "the stapled app is validated"
[ -n "$verify" ] && [ -n "$staple_validate" ] && [ "$staple_validate" -lt "$verify" ] \
    && pass "strict signature verification follows stapling" || fail "strict signature verification follows stapling"
[ -n "$verify" ] && [ -n "$assess" ] && [ "$verify" -lt "$assess" ] \
    && pass "Gatekeeper assessment follows signature verification" || fail "Gatekeeper assessment follows signature verification"

# The receipt checksum is recomputable and changes with bundle contents.
checksum="$("$SCRIPT" --bundle-checksum "$app")"
[ "$checksum" = "$(receipt_field "$receipt" checksum)" ] \
    && pass "the receipt checksum matches a fresh bundle manifest" || fail "the receipt checksum matches a fresh bundle manifest"
echo tampered > "$app/Contents/Resources/tampered.txt"
[ "$checksum" != "$("$SCRIPT" --bundle-checksum "$app")" ] \
    && pass "changed bundle contents change the checksum" || fail "changed bundle contents change the checksum"
rm "$app/Contents/Resources/tampered.txt"

# Entry modes and symlink targets are part of the manifest too, and the mode
# query must work on a BSD and on a GNU host: `-f '%Lp'` is a filesystem query
# on GNU, so probing BSD first can read a constant instead of the entry mode.
chmod 0700 "$app/Contents/MacOS/SnackPilot"
[ "$checksum" != "$("$SCRIPT" --bundle-checksum "$app")" ] \
    && pass "changed file modes change the checksum" || fail "changed file modes change the checksum"
chmod 0755 "$app/Contents/MacOS/SnackPilot"
[ "$checksum" = "$("$SCRIPT" --bundle-checksum "$app")" ] \
    && pass "the manifest is stable across identical bundle contents" || fail "the manifest is stable across identical bundle contents"

ln -s SnackPilot.icns "$app/Contents/Resources/SnackPilot-legacy.icns"
[ "$checksum" != "$("$SCRIPT" --bundle-checksum "$app")" ] \
    && pass "an added symlink changes the checksum" || fail "an added symlink changes the checksum"
rm "$app/Contents/Resources/SnackPilot-legacy.icns"

# A GNU host where the BSD probe does not fail: `stat -f` answers a
# file-system query, which is constant no matter how the entry's mode changes.
# Both hosts are emulated, so the scenario says the same thing on Linux and
# macOS: `$tmp/gnu-bin/stat` is the GNU stat and `$tmp/bsd-bin/stat` the BSD one
# behind it. The GNU stub must answer `-c` itself from the entry's mode; passing
# it through to the BSD stat fails, and the constant `-f` answer would then hide
# every mode.
mkdir -p "$tmp/gnu-bin" "$tmp/bsd-bin"
cat > "$tmp/bsd-bin/stat" <<'STUB'
#!/usr/bin/env bash
[ "${1:-}" = "-f" ] || exit 1
exec python3 -c 'import os,sys; print("%o" % (os.stat(sys.argv[1]).st_mode & 0o7777))' "$3"
STUB
chmod +x "$tmp/bsd-bin/stat"
cat > "$tmp/gnu-bin/stat" <<STUB
#!/usr/bin/env bash
if [ "\$1" = "-f" ]; then
    echo "ext2/ext3"
    exit 0
fi
if [ "\$1" = "-c" ]; then
    exec python3 -c 'import os,sys; print("%o" % (os.stat(sys.argv[1]).st_mode & 0o7777))' "\${!#}"
fi
exec "$tmp/bsd-bin/stat" "\$@"
STUB
chmod +x "$tmp/gnu-bin/stat"
gnu_path="$tmp/gnu-bin:$tmp/bsd-bin:$PATH"
gnu_checksum="$(PATH="$gnu_path" "$SCRIPT" --bundle-checksum "$app")"
chmod 0700 "$app/Contents/MacOS/SnackPilot"
[ "$gnu_checksum" != "$(PATH="$gnu_path" "$SCRIPT" --bundle-checksum "$app")" ] \
    && pass "modes are read on a GNU host where the BSD probe does not fail" \
    || fail "modes are read on a GNU host where the BSD probe does not fail"
chmod 0755 "$app/Contents/MacOS/SnackPilot"

# DRY_RUN is an orchestration flag, not a packaging mode: the package script
# still signs and submits.
out_dry="$tmp/out-dry-run"
if run_case "$out_dry" \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" \
    SNACKPILOT_NOTARY_PROFILE="$PROFILE" \
    DRY_RUN=1 > "$tmp/dry-run.log" 2>&1; then
    grep -q 'xcrun notarytool submit' "$log" \
        && pass "DRY_RUN does not skip signing or notary submission" \
        || fail "DRY_RUN does not skip signing or notary submission"
else
    fail "DRY_RUN does not skip signing or notary submission"
fi

# --- fail-closed cases ------------------------------------------------------
expect_failure() {
    local name="$1"
    local out="$tmp/out-$2"
    shift 2
    if run_case "$out" "$@" >/dev/null 2>&1; then
        fail "$name"
    else
        if [ -f "$out/native-build-receipt.json" ]; then
            fail "$name (a failed package must not write a receipt)"
        else
            pass "$name"
        fi
    fi
    if find "$out" -name '*.zip' 2>/dev/null | grep -q .; then
        fail "$name (left a notary ZIP behind)"
    fi
}

: > "$log"
if run_case "$tmp/out-no-identity" SNACKPILOT_NOTARY_PROFILE="$PROFILE" >/dev/null 2>&1; then
    fail "a missing signing identity fails the release"
else
    pass "a missing signing identity fails the release"
fi
[ -s "$log" ] && fail "missing identity must stop before any tool runs" || pass "missing identity stops before any tool runs"

: > "$log"
if run_case "$tmp/out-no-profile" SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" >/dev/null 2>&1; then
    fail "a missing notary profile fails the release"
else
    pass "a missing notary profile fails the release"
fi
[ -s "$log" ] && fail "missing profile must stop before any tool runs" || pass "missing profile stops before any tool runs"

expect_failure "an ad-hoc identity is rejected" adhoc \
    SNACKPILOT_SIGNING_IDENTITY="-" SNACKPILOT_NOTARY_PROFILE="$PROFILE"
expect_failure "a rejected notarization fails closed" notary-reject \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" NOTARY_REJECT=1
expect_failure "a failed signing stage fails closed" sign-fail \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" CODESIGN_SIGN_FAIL=1
expect_failure "a failed staple/verify stage fails closed" staple-fail \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" STAPLER_FAIL=1
expect_failure "a failed Gatekeeper assessment fails closed" spctl-fail \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" SPCTL_FAIL=1
expect_failure "an ad-hoc or non-Developer-ID signature fails closed" false-trust \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" CODESIGN_ADHOC=1
expect_failure "a non-arm64 slice fails closed" wrong-arch \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" LIPO_ARCH=x86_64
expect_failure "a wrong minimum macOS fails closed" wrong-minos \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" OTOOL_MINOS=15.0
expect_failure "a placeholder version fails closed" placeholder-version \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" \
    SNACKPILOT_PRODUCT_VERSION=0.0.0
expect_failure "a zero build fails closed" zero-build \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" SNACKPILOT_BUILD=0
expect_failure "a cargo build failure fails closed" cargo-fail \
    SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" SNACKPILOT_NOTARY_PROFILE="$PROFILE" CARGO_FAIL=1

# The script has no dry-run or ad-hoc mode at all.
: > "$log"
if run_case "$tmp/out-arg" SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" \
    SNACKPILOT_NOTARY_PROFILE="$PROFILE" -- --dry-run >/dev/null 2>&1; then
    fail "no dry-run flag is accepted"
else
    pass "no dry-run flag is accepted"
fi
[ -s "$log" ] && fail "an unsupported flag must stop before any tool runs" || pass "an unsupported flag stops before any tool runs"

# A missing required tool is reported explicitly.
mv "$tmp/bin/codesign" "$tmp/codesign.stub"
if run_case "$tmp/out-no-tool" SNACKPILOT_SIGNING_IDENTITY="$IDENTITY" \
    SNACKPILOT_NOTARY_PROFILE="$PROFILE" >/dev/null 2>&1; then
    fail "a missing required tool fails closed"
else
    pass "a missing required tool fails closed"
fi
mv "$tmp/codesign.stub" "$tmp/bin/codesign"

# --- result -----------------------------------------------------------------
echo "---"
if [ "$fails" -eq 0 ]; then
    echo "ALL PASS"
else
    echo "$fails FAILED"
    exit 1
fi
