#!/usr/bin/env bash
# Upload a signed .aab to Google Play (default: the `alpha` track), unattended.
#
#   SNACKPILOT_PLAY_SA=~/ci/play-sa.json tools/devops/play-upload.sh dist/SnackPilot-X.Y.Z.aab [TRACK]
#
# Uses the Google Play Developer API with a service account key (JSON). The service
# account must be invited in Play Console → Users and permissions with release
# permission for the app. Nothing is printed that is secret.
set -euo pipefail
source "$(dirname "$0")/lib.sh"

AAB="${1:-}"
TRACK="${2:-alpha}"
PKG="dev.radaiko.gourmetclient"
SA="${SNACKPILOT_PLAY_SA:-$HOME/ci/play-sa.json}"

[[ -n "$AAB" ]] || die "usage: play-upload.sh <file.aab> [track]"
[[ -f "$AAB" ]] || die "no such bundle: $AAB"
[[ -f "$SA" ]] || die "no service account key at $SA — set SNACKPILOT_PLAY_SA=<path>"
require_tool curl "install curl"
require_tool openssl "install openssl"
require_tool python3 "install python3"

json() { python3 -c "import sys,json;d=json.load(sys.stdin);print($1)"; }

email="$(json 'd["client_email"]' < "$SA")"
kpf="$(mktemp)"; trap 'rm -f "$kpf"' EXIT
python3 - "$SA" "$kpf" <<'PY'
import json,sys
key=json.load(open(sys.argv[1]))["private_key"]
open(sys.argv[2],"w").write(key)
PY
chmod 600 "$kpf"

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }
now="$(date +%s)"
hdr="$(printf '%s' '{"alg":"RS256","typ":"JWT"}' | b64url)"
clm="$(printf '{"iss":"%s","scope":"https://www.googleapis.com/auth/androidpublisher","aud":"https://oauth2.googleapis.com/token","iat":%s,"exp":%s}' \
        "$email" "$now" "$((now + 3600))" | b64url)"
sig="$(printf '%s.%s' "$hdr" "$clm" | openssl dgst -sha256 -sign "$kpf" | b64url)"
token="$(curl -sf https://oauth2.googleapis.com/token \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data-urlencode 'grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer' \
  --data-urlencode "assertion=$hdr.$clm.$sig" | json 'd["access_token"]')" \
  || die "Google refused the service account token (invite the service account in Play Console)"

api="https://androidpublisher.googleapis.com/androidpublisher/v3/applications/$PKG"
auth=(-H "Authorization: Bearer $token")

edit="$(curl -sf -X POST "${auth[@]}" "$api/edits" | json 'd["id"]')" || die "cannot open a Play edit"
info "editing $PKG (edit $edit)"

upload="$(curl -sf -X POST "${auth[@]}" \
  -H 'Content-Type: application/octet-stream' \
  --data-binary "@$AAB" \
  "https://androidpublisher.googleapis.com/upload/androidpublisher/v3/applications/$PKG/edits/$edit/bundles?uploadType=media")" \
  || die "bundle upload failed (wrong upload key? Play rejects a bundle signed by a key it does not know)"
code="$(printf '%s' "$upload" | json 'd["versionCode"]')"
ok "uploaded $AAB as versionCode $code"

# status `completed` rolls the build out fully on the chosen track; keep promotion to
# production a manual decision in the Play Console.
name="$(basename "$AAB" .aab)"
curl -sf -X PUT "${auth[@]}" -H 'Content-Type: application/json' \
  --data "{\"releases\":[{\"name\":\"$name\",\"versionCodes\":[\"$code\"],\"status\":\"completed\"}]}" \
  "$api/edits/$edit/tracks/$TRACK" >/dev/null || die "cannot set track $TRACK (does it exist?)"
ok "track $TRACK → versionCode $code"

curl -sf -X POST "${auth[@]}" "$api/edits/$edit:commit" >/dev/null || die "commit failed"
ok "committed — Play Console → Testing → $TRACK"
