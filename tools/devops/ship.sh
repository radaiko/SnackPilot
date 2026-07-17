#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT"

DRY_RUN="${DRY_RUN:-0}"
METHOD="${METHOD:-development}"
HISTORY="$REPO_ROOT/.ship-history"
BUILDNO_FILE="$REPO_ROOT/tools/devops/.build-number"
CARGO="src/core/Cargo.toml"
PROJECT_YML="src/ios/project.yml"
GRADLE="src/android/app/build.gradle.kts"

[[ "$DRY_RUN" == "1" ]] && info "DRY RUN — no files, commits, tags, or counters will change"

# 1. version
last="0.0.0"; [[ -f "$HISTORY" ]] && last="$(tail -1 "$HISTORY")"
info "last shipped version: $last"
read -rp "New version (strict semver, > $last): " VERSION
validate_semver "$VERSION" || die "invalid version '$VERSION' — must be X.Y.Z"
version_gt "$VERSION" "$last" || die "$VERSION must be strictly greater than $last"

# 2. platforms
echo "Platforms:  1) iOS   2) Android   3) both" >&2
read -rp "Select (e.g. 1,2 or 3): " sel
do_ios=0; do_android=0
[[ "$sel" == *1* || "$sel" == *3* ]] && do_ios=1
[[ "$sel" == *2* || "$sel" == *3* ]] && do_android=1
[[ $do_ios -eq 1 || $do_android -eq 1 ]] || die "no platform selected"
label=""; [[ $do_ios -eq 1 ]] && label="ios"; [[ $do_android -eq 1 ]] && label="${label:+$label,}android"
info "shipping v$VERSION ($label)"

# 3. build number (increment; dry-run just previews)
buildno=0; [[ -f "$BUILDNO_FILE" ]] && buildno="$(cat "$BUILDNO_FILE")"
buildno=$((buildno + 1))
info "build number: $buildno"

mkdir -p dist
outdir="dist"; [[ "$DRY_RUN" == "1" ]] && outdir="$(mktemp -d)"

# 4. bump version files (skip on dry-run)
if [[ "$DRY_RUN" != "1" ]]; then
  set_cargo_package_version "$CARGO" "$VERSION"
  set_yaml_key "$PROJECT_YML" MARKETING_VERSION "$VERSION"
  set_yaml_key "$PROJECT_YML" CURRENT_PROJECT_VERSION "$buildno"
  set_gradle_string "$GRADLE" versionName "$VERSION"
  set_gradle_int "$GRADLE" versionCode "$buildno"
  ok "bumped version to $VERSION (build $buildno) in all three files"
else
  info "(dry-run) would bump $CARGO, $PROJECT_YML, $GRADLE to $VERSION / build $buildno"
fi

# 5a. iOS artifact
if [[ $do_ios -eq 1 ]]; then
  require_tool xcodebuild "install Xcode"
  # Auto-detect the Apple team from the signing identity already in the keychain (the cert's
  # OU is the Team ID — NOT the id in the cert name). Override with IOS_TEAM=.
  IOS_TEAM="${IOS_TEAM:-$(security find-certificate -c "Apple Development" -p 2>/dev/null | openssl x509 -noout -subject -nameopt sep_multiline,utf8 2>/dev/null | sed -n 's/^ *OU=//p' | head -1)}"
  [[ -n "$IOS_TEAM" ]] || die "no Apple Development team in keychain — set IOS_TEAM=<teamid> (see: security find-identity -v -p codesigning)"
  info "iOS: regenerating project + archiving (method=$METHOD, team=$IOS_TEAM)"
  ( cd src/ios && ./bootstrap.sh )
  archive="$outdir/SnackPilot.xcarchive"
  xcodebuild -project src/ios/SnackPilot.xcodeproj -scheme SnackPilot \
    -sdk iphoneos -destination "generic/platform=iOS" \
    -archivePath "$archive" -allowProvisioningUpdates \
    DEVELOPMENT_TEAM="$IOS_TEAM" CODE_SIGN_STYLE=Automatic archive
  plist="$outdir/exportOptions.plist"
  cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>method</key><string>$METHOD</string>
  <key>signingStyle</key><string>automatic</string>
  <key>teamID</key><string>$IOS_TEAM</string>
</dict></plist>
EOF
  xcodebuild -exportArchive -archivePath "$archive" \
    -exportOptionsPlist "$plist" -exportPath "$outdir" -allowProvisioningUpdates
  ipa="$(ls "$outdir"/*.ipa | head -1)"
  mv "$ipa" "$outdir/SnackPilot-$VERSION.ipa"
  ok "iOS artifact: $outdir/SnackPilot-$VERSION.ipa"
fi

# 5b. Android artifact
if [[ $do_android -eq 1 ]]; then
  [[ -f "src/android/keystore.properties" ]] || err "no keystore.properties — artifacts will be UNSIGNED (run: make android-keystore)"
  info "Android: bundleRelease + assembleRelease"
  ( cd src/android && ./gradlew :app:bundleRelease :app:assembleRelease )
  cp src/android/app/build/outputs/bundle/release/app-release.aab "$outdir/SnackPilot-$VERSION.aab"
  cp src/android/app/build/outputs/apk/release/app-release.apk "$outdir/SnackPilot-$VERSION.apk"
  ok "Android artifacts: $outdir/SnackPilot-$VERSION.{aab,apk}"
fi

# 6. commit + tag + history (skip on dry-run)
if [[ "$DRY_RUN" == "1" ]]; then
  ok "DRY RUN complete — artifacts in $outdir; no git or counter changes made"
  exit 0
fi
echo "$buildno" > "$BUILDNO_FILE"
git add "$CARGO" "$PROJECT_YML" "$GRADLE"
git commit -m "Release v$VERSION ($label)"
[[ $do_ios -eq 1 ]] && git tag "ios/v$VERSION"
[[ $do_android -eq 1 ]] && git tag "android/v$VERSION"
echo "$VERSION" >> "$HISTORY"
ok "committed Release v$VERSION ($label) and tagged locally"
info "artifacts in dist/. To publish the tags later: git push && git push --tags"
