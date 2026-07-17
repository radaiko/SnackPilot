#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

require_tool keytool "comes with a JDK"
KS="$REPO_ROOT/src/android/snackpilot-release.jks"
PROPS="$REPO_ROOT/src/android/keystore.properties"
ALIAS="snackpilot"

[[ -e "$KS" ]] && die "keystore already exists at $KS — refusing to overwrite (delete it manually to regenerate)"

pass="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 32)" || true
info "generating release keystore (RSA 2048, 10000-day validity)"
keytool -genkeypair -v \
  -keystore "$KS" -alias "$ALIAS" \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass "$pass" -keypass "$pass" \
  -dname "CN=SnackPilot, OU=SnackPilot, O=SnackPilot, L=Vienna, C=AT"

cat > "$PROPS" <<EOF
storeFile=$KS
storePassword=$pass
keyAlias=$ALIAS
keyPassword=$pass
EOF
chmod 600 "$PROPS"

ok "wrote $KS and $PROPS (both gitignored)"
err "BACK THESE UP: losing snackpilot-release.jks + keystore.properties means you can never"
err "sign an update for the same signing identity again. Store them outside the repo."
