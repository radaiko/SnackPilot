#!/usr/bin/env bash
# Build, sign, notarize, staple and verify the macOS release app.
#
# Usage: bash tools/devops/package-macos.sh
#        bash tools/devops/package-macos.sh --bundle-checksum <SnackPilot.app>
#
# The release identity and notary profile are external configuration:
#   SNACKPILOT_SIGNING_IDENTITY  Developer ID Application identity (required)
#   SNACKPILOT_NOTARY_PROFILE    existing notarytool keychain profile (required)
# Optional:
#   SNACKPILOT_PACKAGE_OUT       output directory (default dist/macos)
#   SNACKPILOT_PRODUCT_VERSION   shared version override (default project.yml)
#   SNACKPILOT_BUILD             shared build override (default project.yml)
#   CARGO_TARGET_DIR             cargo target directory (test isolation)
#
# There is no ad-hoc signing, no dry-run and no unsigned fallback: any missing
# tool, identity, rejected submission or failed trust check exits nonzero and
# leaves no receipt. The deliverable is the signed/stapled .app; the temporary
# ZIP is only the notary submission transport and is deleted again.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
UI_DIR="$REPO_ROOT/src/ui"
TEMPLATE="$UI_DIR/desktop/macos/Info.plist.in"
ICON="$UI_DIR/desktop/macos/SnackPilot.icns"

APP_NAME="SnackPilot"
BUNDLE_ID="dev.radaiko.gourmetclient"
TARGET="aarch64-apple-darwin"
MIN_MACOS="26.0"
EXECUTABLE="SnackPilot"
ICON_NAME="SnackPilot"

OUT_DIR="${SNACKPILOT_PACKAGE_OUT:-$REPO_ROOT/dist/macos}"
APP="$OUT_DIR/SnackPilot.app"
RECEIPT="$OUT_DIR/native-build-receipt.json"
NOTARY_ZIP="$OUT_DIR/SnackPilot-notary-submission.zip"
WORK="$OUT_DIR/package-work"

die() { echo "package-macos: $*" >&2; exit 1; }
info() { echo "package-macos: $*"; }

# --- bundle-content manifest (shared by the receipt and ship validation) -----
stat_mode() {
    if stat -f '%Lp' "$1" >/dev/null 2>&1; then
        stat -f '%Lp' "$1"
    else
        stat -c '%a' "$1"
    fi
}

# Sorted relative paths, entry types, modes, symlink targets and file SHA-256
# values. No timestamps, absolute paths or locale-dependent ordering.
bundle_manifest() {
    local app="$1"
    (
        cd "$app"
        find . -mindepth 1 -print | LC_ALL=C sort | while IFS= read -r rel; do
            rel="${rel#./}"
            if [ -L "$rel" ]; then
                printf 'symlink\t%s\t%s\t%s\n' "$rel" "$(stat_mode "$rel")" "$(readlink "$rel")"
            elif [ -d "$rel" ]; then
                printf 'dir\t%s\t%s\t-\n' "$rel" "$(stat_mode "$rel")"
            elif [ -f "$rel" ]; then
                printf 'file\t%s\t%s\t%s\n' "$rel" "$(stat_mode "$rel")" \
                    "$(shasum -a 256 "$rel" | awk '{print $1}')"
            else
                printf 'other\t%s\t%s\t-\n' "$rel" "$(stat_mode "$rel")"
            fi
        done
    )
}

bundle_checksum() {
    local app="$1"
    [ -d "$app" ] || die "bundle not found: $app"
    bundle_manifest "$app" | shasum -a 256 | awk '{print $1}'
}

if [ "${1:-}" = "--bundle-checksum" ]; then
    [ $# -eq 2 ] || die "usage: package-macos.sh --bundle-checksum <SnackPilot.app>"
    bundle_checksum "$2"
    exit 0
fi
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 0
fi
[ $# -eq 0 ] || die "unknown argument: $1 (there is no dry-run or ad-hoc mode)"

# --- inputs -----------------------------------------------------------------
require_tools() {
    local tool
    for tool in "$@"; do
        command -v "$tool" >/dev/null 2>&1 || die "required tool not found: $tool"
    done
}
require_tools cargo codesign xcrun ditto spctl lipo otool file shasum find stat

IDENTITY="${SNACKPILOT_SIGNING_IDENTITY:-}"
PROFILE="${SNACKPILOT_NOTARY_PROFILE:-}"
[ -n "$IDENTITY" ] || die "SNACKPILOT_SIGNING_IDENTITY is not configured; refusing an unsigned release"
[ -n "$PROFILE" ] || die "SNACKPILOT_NOTARY_PROFILE is not configured; refusing an un-notarized release"
case "$IDENTITY" in
    -|*adhoc*|*ad-hoc*|*AdHoc*) die "ad-hoc signing is not a release identity" ;;
esac

yaml_value() {
    awk -v key="$1" '
        $1 == key":" { value=$2; gsub(/"/, "", value); print value; exit }
    ' "$UI_DIR/ios/project.yml"
}

version="${SNACKPILOT_PRODUCT_VERSION:-$(yaml_value MARKETING_VERSION)}"
build="${SNACKPILOT_BUILD:-$(yaml_value CURRENT_PROJECT_VERSION)}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "invalid product version: '$version'"
[[ "$build" =~ ^[0-9]+$ ]] || die "invalid build number: '$build'"
[ "$build" != "0" ] || die "invalid build number: '$build'"
[ "$version" != "0.0.0" ] || die "placeholder product version"

case "$version" in
    *"|"*|*"&"*|*"/"*) die "invalid product version: '$version'" ;;
esac
case "$build" in
    *"|"*|*"&"*|*"/"*) die "invalid build number: '$build'" ;;
esac

for source in "$TEMPLATE" "$ICON"; do
    [ -f "$source" ] || die "missing packaging input: $source"
done
[ "$(head -c 4 "$ICON")" = "icns" ] || die "SnackPilot.icns is not an ICNS resource"

info "release $version ($build), target $TARGET, minimum macOS $MIN_MACOS"
info "identity: $IDENTITY"
info "notary profile: $PROFILE"
info "output: $OUT_DIR"

# --- build and assemble -----------------------------------------------------
rm -rf "$APP" "$WORK" "$RECEIPT"
rm -f "$NOTARY_ZIP"
mkdir -p "$OUT_DIR" "$WORK"
# The temporary notary ZIP never survives a failed or successful run.
cleanup() { rm -f "$NOTARY_ZIP"; }
trap cleanup EXIT

(
    cd "$UI_DIR"
    MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS" \
        SNACKPILOT_PRODUCT_VERSION="$version" \
        SNACKPILOT_BUILD="$build" \
        cargo build --locked --release -p snackpilot-app --bin snackpilot-desktop \
            --target "$TARGET"
)

BIN_DIR="${CARGO_TARGET_DIR:-$UI_DIR/target}/$TARGET/release"
BIN="$BIN_DIR/snackpilot-desktop"
[ -x "$BIN" ] || die "release binary missing: $BIN"

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
install -m 0755 "$BIN" "$APP/Contents/MacOS/$EXECUTABLE"
install -m 0644 "$ICON" "$APP/Contents/Resources/$ICON_NAME.icns"

# Nested runtime code is signed before the outer bundle. Swift/GPUI releases
# can carry dylibs next to the binary; copy them into Frameworks.
shopt -s nullglob
for library in "$BIN_DIR"/*.dylib "$BIN_DIR"/*.so; do
    mkdir -p "$APP/Contents/Frameworks"
    install -m 0755 "$library" "$APP/Contents/Frameworks/$(basename "$library")"
done
shopt -u nullglob

sed \
    -e "s|@BUNDLE_ID@|$BUNDLE_ID|g" \
    -e "s|@EXECUTABLE@|$EXECUTABLE|g" \
    -e "s|@ICON@|$ICON_NAME|g" \
    -e "s|@VERSION@|$version|g" \
    -e "s|@BUILD@|$build|g" \
    -e "s|@MINIMUM_SYSTEM_VERSION@|$MIN_MACOS|g" \
    "$TEMPLATE" > "$APP/Contents/Info.plist"

plist_value() {
    awk -v key="$1" '
        index($0, "<key>" key "</key>") { getline; gsub(/^[ \t]*<string>|<\/string>[ \t]*$/, ""); print; exit }
    ' "$APP/Contents/Info.plist"
}

[ "$(plist_value CFBundleIdentifier)" = "$BUNDLE_ID" ] || die "plist bundle identity mismatch"
[ "$(plist_value CFBundleShortVersionString)" = "$version" ] || die "plist version mismatch"
[ "$(plist_value CFBundleVersion)" = "$build" ] || die "plist build mismatch"
[ "$(plist_value LSMinimumSystemVersion)" = "$MIN_MACOS" ] || die "plist minimum OS mismatch"
[ "$(plist_value CFBundleExecutable)" = "$EXECUTABLE" ] || die "plist executable mismatch"
[ "$(plist_value CFBundleIconFile)" = "$ICON_NAME" ] || die "plist icon mismatch"
[ -f "$APP/Contents/Resources/$ICON_NAME.icns" ] || die "bundle icon missing"

case "$(file "$APP/Contents/MacOS/$EXECUTABLE")" in
    *arm64*) ;;
    *) die "release binary is not arm64" ;;
esac
archs="$(lipo -archs "$APP/Contents/MacOS/$EXECUTABLE")"
case "$archs" in
    *arm64*) ;;
    *) die "lipo reports no arm64 slice: $archs" ;;
esac
case "$archs" in
    *x86_64*|*i386*) die "release bundle carries an unsupported Intel slice: $archs" ;;
esac
minos="$(otool -l "$APP/Contents/MacOS/$EXECUTABLE" | awk '/LC_BUILD_VERSION/{found=1} found && /minos/{print $2; exit}')"
[ "$minos" = "$MIN_MACOS" ] || die "minimum macOS is '$minos', expected $MIN_MACOS"

# --- sign, notarize, staple, verify ----------------------------------------
info "signing nested code"
while IFS= read -r nested; do
    [ -n "$nested" ] || continue
    codesign --force --options runtime --timestamp --sign "$IDENTITY" "$nested"
done < <(find "$APP/Contents/MacOS" "$APP/Contents/Frameworks" -type f \
    \( -name '*.dylib' -o -name '*.so' \) 2>/dev/null | LC_ALL=C sort)

info "signing the app"
codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP"

signing_info="$(codesign -dv --verbose=4 "$APP" 2>&1)" \
    || die "cannot inspect the app signature"
grep -q 'Authority=Developer ID Application' <<<"$signing_info" \
    || die "the app is not signed with a Developer ID Application identity"
grep -q '^TeamIdentifier=..*' <<<"$signing_info" \
    || die "the signature has no team identifier"
grep -Eq 'flags=.*runtime' <<<"$signing_info" \
    || die "the app is not signed with the hardened runtime"
if grep -q 'Signature=adhoc' <<<"$signing_info"; then
    die "ad-hoc signature is not a release"
fi

info "submitting the temporary ZIP to the notary service"
ditto -c -k --keepParent "$APP" "$NOTARY_ZIP"
notary_output="$(xcrun notarytool submit "$NOTARY_ZIP" --keychain-profile "$PROFILE" --wait 2>&1)" \
    || { printf '%s\n' "$notary_output" >&2; die "notarization submission failed"; }
printf '%s\n' "$notary_output"
grep -q 'status: Accepted' <<<"$notary_output" \
    || die "notarization was not accepted"

info "stapling the notarization ticket"
xcrun stapler staple "$APP"
xcrun stapler validate "$APP"

info "verifying trust on the stapled app"
codesign --verify --deep --strict --verbose=2 "$APP"
spctl_output="$(spctl --assess --type execute --verbose=4 "$APP" 2>&1)" \
    || { printf '%s\n' "$spctl_output" >&2; die "Gatekeeper assessment failed"; }
grep -q 'accepted' <<<"$spctl_output" \
    || die "Gatekeeper did not accept the app"
rm -f "$NOTARY_ZIP"

# --- receipt ----------------------------------------------------------------
checksum="$(bundle_checksum "$APP")"
[[ "$checksum" =~ ^[0-9a-f]{64}$ ]] || die "bundle checksum is not SHA-256"

cat > "$RECEIPT" <<EOF
{
  "platform": "macos",
  "architecture": "aarch64",
  "version": "$version",
  "build": "$build",
  "artifact_path": "$APP",
  "trust_state": "developer-id-notarized-stapled",
  "checksum_scheme": "sha256-bundle-manifest-v1",
  "checksum": "$checksum"
}
EOF

info "signed, notarized and stapled: $APP"
info "receipt: $RECEIPT"
info "bundle checksum: $checksum"
