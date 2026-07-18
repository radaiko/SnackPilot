#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT"

DRY_RUN="${DRY_RUN:-0}"
HISTORY="$REPO_ROOT/.ship-history"
BUILDNO_FILE="$REPO_ROOT/tools/devops/.build-number"
CARGO="src/core/Cargo.toml"
PROJECT_YML="src/ios/project.yml"
GRADLE="src/android/app/build.gradle.kts"

# Revert the version-file bumps if we fail after editing them but before the commit lands.
bumped=0; committed=0
cleanup() {
  local rc=$?
  if [[ "$bumped" == "1" && "$committed" == "0" ]]; then
    err "ship failed (exit $rc) — reverting version-file bumps in the working tree"
    git checkout -- "$CARGO" "$PROJECT_YML" "$GRADLE" 2>/dev/null || true
  fi
  exit $rc
}
trap cleanup EXIT

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

# Fail fast if a target tag already exists, so a collision can't orphan a Release commit later.
for _plat in ${label//,/ }; do
  git rev-parse -q --verify "refs/tags/$_plat/v$VERSION" >/dev/null 2>&1 \
    && die "tag $_plat/v$VERSION already exists — bump the version or delete the tag first"
done

# 3. build number (increment; dry-run just previews)
buildno=0; [[ -f "$BUILDNO_FILE" ]] && buildno="$(cat "$BUILDNO_FILE")"
buildno=$((buildno + 1))
info "build number: $buildno"

if [[ "$DRY_RUN" == "1" ]]; then outdir="$(mktemp -d)"; else mkdir -p dist; outdir="dist"; fi

# 4. bump version files (skip on dry-run)
if [[ "$DRY_RUN" != "1" ]]; then
  set_cargo_package_version "$CARGO" "$VERSION"
  set_yaml_key "$PROJECT_YML" MARKETING_VERSION "$VERSION"
  set_yaml_key "$PROJECT_YML" CURRENT_PROJECT_VERSION "$buildno"
  set_gradle_string "$GRADLE" versionName "$VERSION"
  set_gradle_int "$GRADLE" versionCode "$buildno"
  bumped=1
  ok "bumped version to $VERSION (build $buildno) in all three files"
else
  info "(dry-run) would bump $CARGO, $PROJECT_YML, $GRADLE to $VERSION / build $buildno"
fi

# 5a. iOS artifact
if [[ $do_ios -eq 1 ]]; then
  require_tool xcodebuild "install Xcode"
  # Archive a signed release build; upload happens via Xcode Organizer → App Store Connect.
  if [[ "$DRY_RUN" == "1" ]]; then
    archive="$(build_ios_archive "$buildno" "$outdir")"
    ok "iOS archive (dry-run): $archive"
  else
    archive="$(build_ios_archive "$buildno")"
    ok "iOS archive build $buildno: $archive"
    info "opening Xcode Organizer → Distribute App → App Store Connect → Upload → TestFlight"
    open "$archive"
  fi
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
committed=1
[[ $do_ios -eq 1 ]] && git tag "ios/v$VERSION"
[[ $do_android -eq 1 ]] && git tag "android/v$VERSION"
echo "$VERSION" >> "$HISTORY"
ok "committed Release v$VERSION ($label) and tagged locally"
info "artifacts in dist/. To publish the tags later: git push && git push --tags"
