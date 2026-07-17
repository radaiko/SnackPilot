#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool xcodebuild "install Xcode"
require_tool xcrun "install Xcode command line tools"

BUNDLE_ID="dev.radaiko.gourmetclient"
export IOS_SIM="${IOS_SIM:-iPhone 17 Pro}"
cd "$REPO_ROOT"

bootstrap_if_stale ios "src/ios/Frameworks/SnackPilotCore.xcframework"

# Resolve a target simulator UDID: explicit DEVICE=, else the booted one, else IOS_SIM.
udid="${DEVICE:-}"
if [[ -z "$udid" ]]; then
  udid="$(xcrun simctl list devices booted -j | /usr/bin/python3 -c 'import json,sys; d=json.load(sys.stdin)["devices"]; print(next((x["udid"] for v in d.values() for x in v if x.get("state")=="Booted"), ""))')"
fi
if [[ -z "$udid" ]]; then
  info "no booted simulator — selecting '$IOS_SIM'"
  udid="$(xcrun simctl list devices available -j | /usr/bin/python3 -c 'import json,sys,os; name=os.environ["IOS_SIM"]; d=json.load(sys.stdin)["devices"]; print(next((x["udid"] for v in d.values() for x in v if x.get("name")==name), ""))')"
  [[ -n "$udid" ]] || die "simulator '$IOS_SIM' not found — set IOS_SIM= to an available device (xcrun simctl list devices available)"
fi
ok "target simulator: $udid"

# Ensure the target is booted (a DEVICE= override or IOS_SIM pick may be shut down).
if ! xcrun simctl list devices booted | grep -q "$udid"; then
  info "booting simulator $udid"
  xcrun simctl boot "$udid" 2>/dev/null || true
  open -a Simulator
fi
xcrun simctl bootstatus "$udid" >/dev/null 2>&1 || true

info "building (iphonesimulator)"
xcodebuild -project src/ios/SnackPilot.xcodeproj -scheme SnackPilot \
  -sdk iphonesimulator -destination "id=$udid" \
  -derivedDataPath src/ios/build build

app="src/ios/build/Build/Products/Debug-iphonesimulator/SnackPilot.app"
[[ -d "$app" ]] || die "built .app not found at $app"

info "installing + launching"
xcrun simctl install "$udid" "$app"
xcrun simctl launch "$udid" "$BUNDLE_ID"
ok "SnackPilot launched on $udid"
