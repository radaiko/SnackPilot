#!/usr/bin/env bash
# Build the macOS release .app WITHOUT signing, for the CI pipeline.
#
#   bash tools/devops/package-macos-unsigned.sh
#
# This is the explicit-UX-exception path: CI cannot reach the Developer ID
# identity or the notary profile (they live in the Mac's login keychain, which
# a runner session cannot unlock), so CI produces an artifact that Gatekeeper
# blocks until the user removes the quarantine flag. `make package-macos`
# remains the signed, notarized, stapled release; this script never claims to
# be one and writes `trust_state: unsigned` into its receipt.
#
# Optional:
#   SNACKPILOT_PACKAGE_OUT       output directory (default dist/macos)
#   SNACKPILOT_PRODUCT_VERSION   shared version override (default project.yml)
#   SNACKPILOT_BUILD             build override (default project.yml)
#   CARGO_TARGET_DIR             cargo target directory
#
# Output: dist/macos/SnackPilot.app and
#         dist/macos/SnackPilot-{version}-macos-arm64.zip (the .app plus a
#         note that it is unsigned)
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
ZIP=""   # set once the version is known

die() { echo "package-macos-unsigned: $*" >&2; exit 1; }
info() { echo "package-macos-unsigned: $*"; }

for tool in cargo ditto file shasum; do
    command -v "$tool" >/dev/null 2>&1 || die "missing tool: $tool"
done
command -v lipo >/dev/null 2>&1 || die "missing tool: lipo (install Xcode command line tools)"
command -v otool >/dev/null 2>&1 || die "missing tool: otool (install Xcode command line tools)"

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

for source in "$TEMPLATE" "$ICON"; do
    [ -f "$source" ] || die "missing packaging input: $source"
done
[ "$(head -c 4 "$ICON")" = "icns" ] || die "SnackPilot.icns is not an ICNS resource"

ZIP="$OUT_DIR/SnackPilot-$version-macos-arm64.zip"
NOTE="$OUT_DIR/UNSIGNED.txt"

info "release $version ($build), target $TARGET, minimum macOS $MIN_MACOS"
info "trust: UNSIGNED (CI build; signed releases come from make package-macos)"
info "output: $OUT_DIR"

rm -rf "$APP" "$ZIP" "$NOTE" "$RECEIPT"
mkdir -p "$OUT_DIR"

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

# Any dylib shipped next to the binary belongs in the bundle (gpui releases can
# carry one); an unsigned bundle is loaded straight from Frameworks.
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

case "$(lipo -archs "$APP/Contents/MacOS/$EXECUTABLE")" in
    *arm64*) ;;
    *) die "release binary is not arm64" ;;
esac

# The legacy notary ZIP is a transport; this one is the download. ditto keeps
# the bundle's symlinks and resource forks intact, which unzip does not.
cat > "$NOTE" <<'NOTE_EOF'
SnackPilot for macOS (Apple Silicon)

This build is UNSIGNED: it was produced by CI, which has no access to the
Developer ID identity or the notary profile. macOS therefore blocks it on the
first launch (Gatekeeper: "cannot be opened because the developer cannot be
verified").

To run it anyway, either right-click SnackPilot.app -> Open, or remove the
quarantine flag once:

    xattr -dr com.apple.quarantine /Applications/SnackPilot.app

A signed, notarized and stapled release comes from `make package-macos`.
NOTE_EOF

info "packing the bundle"
STAGE="$OUT_DIR/SnackPilot-$version"
rm -rf "$STAGE"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/SnackPilot.app"
cp "$NOTE" "$STAGE/UNSIGNED.txt"
ditto -c -k --sequesterRsrc --keepParent "$STAGE" "$ZIP"
rm -rf "$STAGE"
[ -s "$ZIP" ] || die "packaging the bundle produced no archive"

checksum="$(shasum -a 256 "$ZIP" | awk '{print $1}')"
[[ "$checksum" =~ ^[0-9a-f]{64}$ ]] || die "archive checksum is not SHA-256"

cat > "$RECEIPT" <<EOF
{
  "platform": "macos",
  "architecture": "aarch64",
  "version": "$version",
  "build": "$build",
  "artifact_path": "$ZIP",
  "trust_state": "unsigned",
  "checksum_scheme": "sha256-archive",
  "checksum": "$checksum"
}
EOF

info "artifact: $ZIP"
info "sha256:   $checksum"
info "Gatekeeper blocks this build until the download is un-quarantined:"
info "  xattr -d com.apple.quarantine SnackPilot.app   (or right-click → Open)"
