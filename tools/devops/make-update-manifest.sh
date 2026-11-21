#!/usr/bin/env bash
# Writes and signs the desktop update manifests for one release (design D4/D8).
#
#   make-update-manifest.sh <version> <macos-dir> <windows-dir> <notes-file> <out-dir>
#
# Writes <out-dir>/SnackPilot-manifest-<platform>-<arch>.json (schema 1, compact,
# sorted keys, ASCII, trailing newline) and the same name plus `.sig`, an Ed25519
# signature in lowercase hex. Each manifest names and hashes the file that is
# published: the macOS job's zip (the receipt names the `.app` it was packed from,
# with a bundle-content checksum) and the Windows installer. The private key is read from
# SNACKPILOT_UPDATE_SIGNING_KEY (PKCS#8 PEM) and never printed, logged or left on
# disk. A missing key, or a platform whose receipt and artifact do not agree,
# fails the run. A macOS receipt whose trust state is not
# `developer-id-notarized-stapled` gets no macOS manifest and a warning: an
# unsigned macOS build cannot be verified by the app.
#
# Progress goes to stderr; stdout stays empty.
# @req REQ-UPD-18 @req REQ-UPD-19 @req REQ-UPD-03
set -euo pipefail

usage() { echo "usage: $0 <version> <macos-dir> <windows-dir> <notes-file> <out-dir>" >&2; }
say() { printf '%s\n' "$*" >&2; }
die() { printf 'make-update-manifest: %s\n' "$*" >&2; exit 1; }

[ $# -eq 5 ] || { usage; exit 1; }
version=$1 macos_dir=$2 windows_dir=$3 notes_file=$4 out_dir=$5

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "invalid version '$version'"
openssl version | awk '{ exit !($2 ~ /^3\./) }' || die "openssl 3.x is required (found $(openssl version))"
command -v python3 >/dev/null 2>&1 || die "python3 is required"
[ -s "$notes_file" ] || die "missing release notes: $notes_file"
# The key is only a file from here on, so nothing below can echo it.
[ -n "${SNACKPILOT_UPDATE_SIGNING_KEY:-}" ] || die "SNACKPILOT_UPDATE_SIGNING_KEY is not set"

umask 077
key_file="$(mktemp "${TMPDIR:-/tmp}/snackpilot-update-key.XXXXXX")" || die "mktemp failed"
trap 'rm -f "$key_file"' EXIT
trap 'exit 130' INT TERM
chmod 600 "$key_file"
printf '%s\n' "$SNACKPILOT_UPDATE_SIGNING_KEY" > "$key_file"
openssl pkey -in "$key_file" -noout 2>/dev/null || die "SNACKPILOT_UPDATE_SIGNING_KEY is not a usable private key"

mkdir -p "$out_dir"

# Reads a receipt and the artifact it names, and prints the manifest JSON as one
# line (D4). Fields used by the manifest: build, checksum, artifact, notes.
manifest_for() { # platform arch(dir) minimum_os dir trust_expected
    python3 - "$version" "$notes_file" "$@" <<'PY'
import hashlib, json, os, sys
version, notes_file, platform, arch, directory, minimum_os, trust_expected, published = sys.argv[1:9]
if not os.path.isdir(directory):
    sys.exit(f"missing {platform} directory: {directory}")
receipt_path = os.path.join(directory, "native-build-receipt.json")
try:
    # utf-8-sig: Windows PowerShell 5 writes a BOM with -Encoding utf8.
    with open(receipt_path, encoding="utf-8-sig") as f:
        receipt = json.load(f)
except (OSError, ValueError) as e:
    sys.exit(f"unreadable {platform} receipt {receipt_path}: {e}")
trust = receipt.get("trust_state")
if trust != trust_expected:
    sys.exit(f"{platform} receipt trust_state is {trust!r}, expected {trust_expected!r}")
if receipt.get("version") != version:
    sys.exit(f"{platform} receipt is for version {receipt.get('version')!r}, not {version!r}")
if published:
    # The macOS receipt names the verified `SnackPilot.app` and carries the
    # bundle-content checksum (docs/requirements/08), but what is published, and
    # what the app downloads, is the zip the macOS job packs from it.
    artifact = os.path.join(directory, published)
    if not os.path.isfile(artifact):
        sys.exit(f"missing {platform} artifact {artifact}")
else:
    named = os.path.basename(str(receipt.get("artifact_path") or ""))
    if not named:
        sys.exit(f"{platform} receipt names no artifact")
    artifact = os.path.join(directory, named)
    if not os.path.isfile(artifact):
        sys.exit(f"missing {platform} artifact {artifact}")
digest = hashlib.sha256()
with open(artifact, "rb") as f:
    for block in iter(lambda: f.read(1 << 20), b""):
        digest.update(block)
sha = digest.hexdigest()
if not published and not (receipt.get("checksum") or "").lower() in ("", sha):
    sys.exit(f"{platform} artifact does not match the receipt checksum")
with open(notes_file, encoding="utf-8-sig") as f:
    notes = f.read()
tag = f"desktop/v{version}"
manifest = {
    "schema": 1, "platform": platform, "architecture": arch, "version": version,
    "build": str(receipt.get("build")), "minimum_os": minimum_os,
    "artifact": os.path.basename(artifact), "size": os.path.getsize(artifact),
    "sha256": sha, "notes": notes, "notes_url": f"https://github.com/radaiko/SnackPilot/releases/tag/{tag}",
    "tag": tag,
}
print(json.dumps(manifest, sort_keys=True, ensure_ascii=True, separators=(",", ":")))
PY
}

# Signs the manifest and writes the lowercase-hex signature next to it.
# Ed25519 signs deterministically, so the raw file only has to be verified.
sign() { # manifest
    local manifest=$1 sig="$1.sig" raw sig_bytes
    raw="$(mktemp "${TMPDIR:-/tmp}/snackpilot-update-sig.XXXXXX")" || die "mktemp failed"
    # shellcheck disable=SC2064 # both paths are expanded now on purpose
    trap "rm -f '$key_file' '$raw'" EXIT
    openssl pkeyutl -sign -rawin -inkey "$key_file" -in "$manifest" -out "$raw" 2>/dev/null
    [ "$(wc -c < "$raw")" -eq 64 ] || die "signing $(basename "$manifest") produced no 64-byte signature"
    openssl pkeyutl -verify -pubin -inkey <(openssl pkey -in "$key_file" -pubout 2>/dev/null) \
        -rawin -in "$manifest" -sigfile "$raw" >/dev/null 2>&1 \
        || die "self-verification of $(basename "$manifest") failed"
    sig_bytes="$(od -An -v -tx1 < "$raw" | tr -d ' \n')"
    [[ "$sig_bytes" =~ ^[0-9a-f]{128}$ ]] || die "signing $(basename "$manifest") produced no 64-byte signature"
    # The hex file must be the same 64 bytes again, not merely the same text.
    python3 -c 'import sys; sys.exit(open(sys.argv[1], "rb").read() != bytes.fromhex(sys.argv[2]))' \
        "$raw" "$sig_bytes" || die "signature hex does not match the signed bytes"
    printf '%s\n' "$sig_bytes" > "$sig"
    rm -f "$raw"
    trap 'rm -f "$key_file"' EXIT
    say "signed $(basename "$manifest")"
}

write_one() { # platform arch minimum_os dir trust_expected [published_artifact_name]
    local manifest="$out_dir/SnackPilot-manifest-$1-$2.json"
    # Written aside first, so a refused artifact leaves no empty manifest behind.
    manifest_for "$1" "$2" "$4" "$3" "$5" "${6:-}" > "$manifest.tmp" \
        || { rm -f "$manifest.tmp"; die "no $1 manifest"; }
    mv "$manifest.tmp" "$manifest"
    sign "$manifest"
}

macos_trust="$(python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8-sig") as f:
    print(json.load(f).get("trust_state"))' "$macos_dir/native-build-receipt.json" 2>/dev/null || true)"
if [ "$macos_trust" = developer-id-notarized-stapled ]; then
    write_one macos arm64 26.0 "$macos_dir" developer-id-notarized-stapled \
        "SnackPilot-$version-macos-arm64.zip"
else
    say "::warning::the macOS build reports trust_state '${macos_trust:-none}': it cannot be verified, no macOS manifest"
fi
write_one windows x64 10.0.22000 "$windows_dir" unsigned

say "manifests: $out_dir"
