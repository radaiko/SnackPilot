#!/usr/bin/env bash
# Prepare codesigning + notarization on a CI runner (macOS), from secrets.
#
#   bash tools/devops/ci-signing.sh
#
# Builds a throwaway keychain holding the Developer ID Application identity,
# makes `codesign` able to use it without a GUI prompt, and stores the notary
# credentials as the profile `snackpilot-ci`. Both are thrown away with the
# runner's temporary keychain when the job ends.
#
# Inputs (all required, all from the job's secrets):
#   SNACKPILOT_DEV_ID_P12          base64 of the exported .p12 identity
#   SNACKPILOT_DEV_ID_P12_PASSWORD its export password
#   SNACKPILOT_ASC_KEY             base64 of the App Store Connect API key (.p8)
#   SNACKPILOT_ASC_KEY_ID          the key's Key ID
#   SNACKPILOT_ASC_ISSUER_ID       the issuer UUID from App Store Connect
#
# Exports SNACKPILOT_NOTARY_PROFILE=snackpilot-ci into $GITHUB_ENV, so the
# packaging step can run tools/devops/package-macos.sh unchanged.
#
# An App Store Connect API key is used rather than an Apple ID + app-specific
# password: it is revocable on its own, carries no account credential, and does
# not break when the account's password or 2FA changes.
set -euo pipefail

die() { echo "ci-signing: $*" >&2; exit 1; }
info() { echo "ci-signing: $*"; }

: "${SNACKPILOT_DEV_ID_P12:?the Developer ID .p12 is not configured}"
: "${SNACKPILOT_DEV_ID_P12_PASSWORD:?the .p12 password is not configured}"
: "${SNACKPILOT_ASC_KEY:?the App Store Connect API key is not configured}"
: "${SNACKPILOT_ASC_KEY_ID:?the API key id is not configured}"
: "${SNACKPILOT_ASC_ISSUER_ID:?the API issuer id is not configured}"

WORK="$(mktemp -d)"
KEYCHAIN="$WORK/signing.keychain-db"
PASSWORD="$(openssl rand -hex 24)"
P12="$WORK/identity.p12"
ASC_KEY="$WORK/AuthKey.p8"

cleanup() {
    security delete-keychain "$KEYCHAIN" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

info "creating a temporary keychain"
security create-keychain -p "$PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 3600 "$KEYCHAIN"
security unlock-keychain -p "$PASSWORD" "$KEYCHAIN"
# Codesign must be able to read the key with no login session (no dialog).
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$PASSWORD" "$KEYCHAIN" >/dev/null

info "importing the Developer ID identity"
printf '%s' "$SNACKPILOT_DEV_ID_P12" | base64 --decode > "$P12"
security import "$P12" -k "$KEYCHAIN" -P "$SNACKPILOT_DEV_ID_P12_PASSWORD" \
    -T /usr/bin/codesign -T /usr/bin/security >/dev/null
rm -f "$P12"

identity_count="$(security find-identity -v -p codesigning "$KEYCHAIN" | grep -c 'Developer ID Application' || true)"
[ "$identity_count" -ge 1 ] || die "the imported .p12 holds no Developer ID Application identity"
info "identity available: $identity_count"

security list-keychains -d user -s "$KEYCHAIN" >/dev/null

info "storing the notary credentials as profile snackpilot-ci"
printf '%s' "$SNACKPILOT_ASC_KEY" | base64 --decode > "$ASC_KEY"
xcrun notarytool store-credentials snackpilot-ci \
    --key "$ASC_KEY" \
    --key-id "$SNACKPILOT_ASC_KEY_ID" \
    --issuer "$SNACKPILOT_ASC_ISSUER_ID" >/dev/null
rm -f "$ASC_KEY"

if [ -n "${GITHUB_ENV:-}" ]; then
    echo "SNACKPILOT_NOTARY_PROFILE=snackpilot-ci" >> "$GITHUB_ENV"
fi
info "ready: identity imported, notary profile snackpilot-ci stored"
