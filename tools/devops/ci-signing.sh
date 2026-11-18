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
# Writes SNACKPILOT_NOTARY_KEY / _KEY_ID / _ISSUER to $SNACKPILOT_ENV_FILE (and
# to $GITHUB_ENV when set), so tools/devops/package-macos.sh can notarize with
# the key.
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
ASC_KEY="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/snackpilot-notary-$(openssl rand -hex 6).p8"

cleanup() {
    security delete-keychain "$KEYCHAIN" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

step() {
    # Runs one command and names it on failure, without echoing its arguments
    # (some carry the keychain password).
    local desc="$1"; shift
    local out
    if ! out=$("$@" 2>&1); then
        printf '%s\n' "$out" >&2
        die "$desc failed"
    fi
}

info "creating a temporary keychain"
step "create-keychain" security create-keychain -p "$PASSWORD" "$KEYCHAIN"

# The runner is a LaunchDaemon user with no login keychain, so the calls that
# only tune the session are best-effort; the ones that carry the identity are
# not.
security default-keychain -s "$KEYCHAIN" >/dev/null 2>&1 || true
security set-keychain-settings -lut 3600 "$KEYCHAIN" >/dev/null 2>&1 \
    || info "note: no login keychain here, so settings were not applied"
EXISTING_KEYCHAINS="$(security list-keychains -d user 2>/dev/null | tr -d '"' | tr -s ' ' ' ' || true)"
if [ -n "$EXISTING_KEYCHAINS" ]; then
    # shellcheck disable=SC2086
    security list-keychains -d user -s "$KEYCHAIN" $EXISTING_KEYCHAINS >/dev/null 2>&1 \
        || info "note: could not extend the keychain search list"
else
    security list-keychains -d user -s "$KEYCHAIN" >/dev/null 2>&1 \
        || info "note: could not set the keychain search list"
fi

step "unlock-keychain" security unlock-keychain -p "$PASSWORD" "$KEYCHAIN"

info "importing the Developer ID identity"
printf '%s' "$SNACKPILOT_DEV_ID_P12" | base64 --decode > "$P12"
# -A as well as -T: this keychain exists for one job and is deleted with it, and
# a daemon session has no dialog to answer an access prompt with.
step "import identity" security import "$P12" -k "$KEYCHAIN" -P "$SNACKPILOT_DEV_ID_P12_PASSWORD" \
    -T /usr/bin/codesign -T /usr/bin/security -A
rm -f "$P12"

identity_count="$(security find-identity -v -p codesigning "$KEYCHAIN" 2>/dev/null | grep -c 'Developer ID Application' || true)"
[ "$identity_count" -ge 1 ] || die "the imported .p12 holds no Developer ID Application identity"
info "identity available: $identity_count"

security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$PASSWORD" "$KEYCHAIN" >/dev/null 2>&1 \
    || info "note: partition list not set; the -A import still lets codesign read the key"

# notarytool can store a profile, but that needs the *default* keychain, which a
# LaunchDaemon user does not have ("keychainLocked(default)"). Passing the key
# straight to the submission avoids the keychain altogether.
info "preparing the notary key"
printf '%s' "$SNACKPILOT_ASC_KEY" | base64 --decode > "$ASC_KEY"
chmod 600 "$ASC_KEY"

# Written to a file as well: a step that calls this script and then packages in
# the same shell cannot see $GITHUB_ENV, which only applies to later steps.
ENV_FILE="${SNACKPILOT_ENV_FILE:-}"
if [ -z "$ENV_FILE" ]; then
    ENV_FILE="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/snackpilot-ci.env"
fi
{
    echo "SNACKPILOT_NOTARY_KEY=$ASC_KEY"
    echo "SNACKPILOT_NOTARY_KEY_ID=$SNACKPILOT_ASC_KEY_ID"
    echo "SNACKPILOT_NOTARY_ISSUER=$SNACKPILOT_ASC_ISSUER_ID"
} > "$ENV_FILE"
chmod 600 "$ENV_FILE"
if [ -n "${GITHUB_ENV:-}" ]; then
    cat "$ENV_FILE" >> "$GITHUB_ENV"
fi
info "environment written to $ENV_FILE"
info "ready: identity imported, notary key prepared"
