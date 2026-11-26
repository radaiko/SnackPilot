#!/usr/bin/env bash
# Offline tests for tools/devops/make-update-manifest.sh and the desktop workflow's
# signing step. The signing key is a throwaway one generated here; nothing is
# published and no network is touched.
#
# Run: bash tools/devops/test-update-manifest.sh   (also part of `make test-lib`)
# @req REQ-UPD-18 @req REQ-UPD-19 @req REQ-UPD-03
set -uo pipefail
unset SNACKPILOT_UPDATE_SIGNING_KEY

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT="$ROOT/tools/devops/make-update-manifest.sh"
WORKFLOW="$ROOT/.github/workflows/desktop.yml"
DOCS_FIXTURES="$ROOT/docs/fixtures/update"

fails=0
pass() { echo "ok: $1"; }
fail() { echo "FAIL: $1"; fails=$((fails + 1)); }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

for tool in openssl python3; do
    command -v "$tool" >/dev/null 2>&1 || { echo "SKIP: $tool not on PATH"; exit 0; }
done

# --- a throwaway Ed25519 key (never leaves $tmp) ---------------------------
openssl genpkey -algorithm Ed25519 -out "$tmp/throwaway.pem" 2>/dev/null \
    || { echo "FAIL: could not generate a throwaway key"; exit 1; }
chmod 600 "$tmp/throwaway.pem"
THROWAWAY_KEY="$(cat "$tmp/throwaway.pem")"
THROWAWAY_PUB="$tmp/throwaway.pub.pem"
openssl pkey -in "$tmp/throwaway.pem" -pubout -out "$THROWAWAY_PUB" 2>/dev/null

# --- a fake artifact tree shaped like the publish job's `dist/` ------------
version="2.4.0"
make_tree() { # root [macos_trust_state]
    local root="$1" trust="${2:-developer-id-notarized-stapled}"
    mkdir -p "$root/desktop-macos" "$root/desktop-windows" "$root/notes"
    printf 'macos zip payload\n' > "$root/desktop-macos/SnackPilot-$version-macos-arm64.zip"
    printf 'windows installer payload\n' > "$root/desktop-windows/SnackPilot-$version-windows-x64-setup.exe"
    printf 'Desktop build of SnackPilot.\n' > "$root/notes/body.md"
    # The real receipt names the verified `.app` and comes from
    # package-macos.sh, not the packed zip: its checksum is the bundle-content
    # manifest, so the manifest must be built from the zip regardless.
    cat > "$root/desktop-macos/native-build-receipt.json" <<EOF
{
  "platform": "macos",
  "architecture": "aarch64",
  "version": "$version",
  "build": "43",
  "artifact_path": "$root/../macos/SnackPilot.app",
  "trust_state": "$trust",
  "checksum_scheme": "sha256-bundle-manifest-v1",
  "checksum": "0000000000000000000000000000000000000000000000000000000000000000"
}
EOF
    cat > "$root/desktop-windows/native-build-receipt.json" <<EOF
{
  "platform": "windows",
  "architecture": "x86_64",
  "version": "$version",
  "build": "43",
  "artifact_path": "$root/desktop-windows/SnackPilot-$version-windows-x64-setup.exe",
  "trust_state": "unsigned"
}
EOF
}

# The secret goes through the environment only: `env K=value` would show the
# whole key in `ps`.
run_script() { # root out [KEY=VALUE...]
    local root="$1" out="$2"
    shift 2
    (
        unset SNACKPILOT_UPDATE_SIGNING_KEY
        pair=""
        for pair in "$@"; do export "$pair"; done
        exec bash "$SCRIPT" "$version" "$root/desktop-macos" "$root/desktop-windows" \
            "$root/notes/body.md" "$out"
    )
}

field() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' "$1" "$2"; }
sha256() { python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"; }
size() { python3 -c 'import os,sys; print(os.path.getsize(sys.argv[1]))' "$1"; }

# --- the run the other cases build on --------------------------------------
root="$tmp/dist"; out="$tmp/update"
make_tree "$root"
if run_script "$root" "$out" SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" > "$tmp/run.log" 2>&1; then
    pass "the script signs a signed macOS build and the Windows installer"
else
    fail "the script signs a signed macOS build and the Windows installer"
    cat "$tmp/run.log"
fi
win="$out/SnackPilot-manifest-windows-x64.json"
mac="$out/SnackPilot-manifest-macos-arm64.json"

# @req REQ-UPD-18
manifest_fields() {
    local f ok=1
    for f in "$win" "$mac" "$win.sig" "$mac.sig"; do
        [ -f "$f" ] || { echo "missing $f"; ok=0; }
    done
    [ "$ok" = 1 ] || return 1

    local installer="$root/desktop-windows/SnackPilot-$version-windows-x64-setup.exe"
    local zip="$root/desktop-macos/SnackPilot-$version-macos-arm64.zip"
    [ "$(field "$win" size)" = "$(size "$installer")" ] || { echo "size"; return 1; }
    [ "$(field "$win" sha256)" = "$(sha256 "$installer")" ] || { echo "sha256"; return 1; }
    [ "$(field "$mac" size)" = "$(size "$zip")" ] || { echo "mac size"; return 1; }
    [ "$(field "$mac" sha256)" = "$(sha256 "$zip")" ] || { echo "mac sha256"; return 1; }
    [ "$(field "$win" schema)" = 1 ] || return 1
    [ "$(field "$win" platform)" = windows ] || return 1
    [ "$(field "$win" architecture)" = x64 ] || return 1
    [ "$(field "$win" version)" = "$version" ] || return 1
    [ "$(field "$win" build)" = 43 ] || return 1
    [ "$(field "$win" minimum_os)" = "10.0.22000" ] || return 1
    [ "$(field "$win" artifact)" = "SnackPilot-$version-windows-x64-setup.exe" ] || return 1
    [ "$(field "$win" tag)" = "desktop/v$version" ] || return 1
    [ "$(field "$win" notes_url)" = "https://github.com/radaiko/SnackPilot/releases/tag/desktop/v$version" ] || return 1
    [ "$(field "$win" notes)" = "Desktop build of SnackPilot." ] || return 1
    [ "$(field "$mac" artifact)" = "SnackPilot-$version-macos-arm64.zip" ] || return 1
    [ "$(field "$mac" platform)" = macos ] || return 1
    [ "$(field "$mac" architecture)" = arm64 ] || return 1
    [ "$(field "$mac" minimum_os)" = "26.0" ] || return 1
    [ "$(field "$mac" build)" = 43 ] || return 1
}
if manifest_fields; then pass "manifest_fields_match_artifact_and_receipt"; else fail "manifest_fields_match_artifact_and_receipt"; fi

# `openssl pkeyutl` wants the raw signature, the app wants hex: convert the
# as-published text back to bytes and hand those to OpenSSL.
as_raw_sig() { python3 -c 'import sys; open(sys.argv[2], "wb").write(bytes.fromhex(open(sys.argv[1]).read().strip()))' "$1" "$2"; }

# @req REQ-UPD-03
as_raw_sig "$win.sig" "$tmp/win.sig.bin"
as_raw_sig "$mac.sig" "$tmp/mac.sig.bin"
if openssl pkeyutl -verify -pubin -inkey "$THROWAWAY_PUB" -rawin -in "$win" -sigfile "$tmp/win.sig.bin" >/dev/null 2>&1 \
    && openssl pkeyutl -verify -pubin -inkey "$THROWAWAY_PUB" -rawin -in "$mac" -sigfile "$tmp/mac.sig.bin" >/dev/null 2>&1 \
    && python3 -c 'import sys
sig = [open(p).read() for p in sys.argv[1:]]
assert all(len(s.strip()) == 128 and s == s.lower() and s.strip() == s[:-1] for s in sig)' "$win.sig" "$mac.sig"; then
    pass "signature_verifies_with_openssl"
else
    fail "signature_verifies_with_openssl"
fi

if cp "$win" "$tmp/tampered.json" && printf ' ' >> "$tmp/tampered.json" \
    && ! openssl pkeyutl -verify -pubin -inkey "$THROWAWAY_PUB" -rawin -in "$tmp/tampered.json" -sigfile "$tmp/win.sig.bin" >/dev/null 2>&1; then
    pass "tampered_manifest_fails_verification"
else
    fail "tampered_manifest_fails_verification"
fi

# @req REQ-UPD-19
root2="$tmp/dist-nokey"; out2="$tmp/update-nokey"
make_tree "$root2"
if run_script "$root2" "$out2" SNACKPILOT_UPDATE_SIGNING_KEY= >/dev/null 2>&1; then
    fail "missing_key_fails_closed_and_writes_nothing"
elif [ -z "$(find "$out2" -type f 2>/dev/null)" ]; then
    pass "missing_key_fails_closed_and_writes_nothing"
else
    fail "missing_key_fails_closed_and_writes_nothing"
fi

# @req REQ-UPD-19
root3="$tmp/dist-unsigned"; out3="$tmp/update-unsigned"
make_tree "$root3" unsigned
if run_script "$root3" "$out3" SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" > "$tmp/unsigned.log" 2>&1 \
    && [ -f "$out3/SnackPilot-manifest-windows-x64.json" ] \
    && [ ! -e "$out3/SnackPilot-manifest-macos-arm64.json" ] \
    && [ ! -e "$out3/SnackPilot-manifest-macos-arm64.json.sig" ] \
    && grep -qi "macos" "$tmp/unsigned.log"; then
    pass "unsigned_macos_receipt_skips_macos_manifest"
else
    fail "unsigned_macos_receipt_skips_macos_manifest"
fi

# The macOS manifest hashes the published zip, not what the receipt names: the
# receipt's `SnackPilot.app` is never in the publish job's `dist/`, and a
# receipt-named artifact here is exactly what broke the first real release.
# @req REQ-UPD-18
root_zip="$tmp/dist-nozip"; out_zip="$tmp/update-nozip"
make_tree "$root_zip"
rm -f "$root_zip/desktop-macos/SnackPilot-$version-macos-arm64.zip"
if run_script "$root_zip" "$out_zip" SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" > "$tmp/nozip.log" 2>&1; then
    fail "missing_published_macos_zip_fails_closed"
elif [ -z "$(find "$out_zip" -type f 2>/dev/null)" ] && grep -qi "macos" "$tmp/nozip.log"; then
    pass "missing_published_macos_zip_fails_closed"
else
    fail "missing_published_macos_zip_fails_closed"
fi

# The Windows receipt names its installer by a Windows path, and the publish job
# runs on Linux: taking the name from that path must not need Windows.
# @req REQ-UPD-18
root_winpath="$tmp/dist-winpath"; out_winpath="$tmp/update-winpath"
make_tree "$root_winpath"
win_exe="SnackPilot-$version-windows-x64-setup.exe"
win_sha="$(sha256 "$root_winpath/desktop-windows/$win_exe")"
cat > "$root_winpath/desktop-windows/native-build-receipt.json" <<EOF
{"platform":"windows","architecture":"x86_64","version":"$version","build":"43",
 "artifact_path":"C:\\\\Users\\\\ci\\\\dist\\\\windows\\\\$win_exe","trust_state":"unsigned","checksum":"$win_sha"}
EOF
if run_script "$root_winpath" "$out_winpath" SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" > "$tmp/winpath.log" 2>&1 \
    && [ "$(field "$out_winpath/SnackPilot-manifest-windows-x64.json" artifact)" = "$win_exe" ]; then
    pass "windows_receipt_with_a_windows_path_is_read"
else
    fail "windows_receipt_with_a_windows_path_is_read"
    cat "$tmp/winpath.log"
fi

# @req REQ-UPD-18
root4="$tmp/dist-bom"; out4="$tmp/update-bom"
make_tree "$root4"
python3 - "$root4/desktop-windows/native-build-receipt.json" <<'PY'
import sys
path = sys.argv[1]
data = open(path, "rb").read()
open(path, "wb").write(b"\xef\xbb\xbf" + data.replace(b'"build": "43"', b'"build": "47"'))
PY
if run_script "$root4" "$out4" SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" >/dev/null 2>&1 \
    && [ "$(field "$out4/SnackPilot-manifest-windows-x64.json" build)" = 47 ]; then
    pass "bom_receipt_is_read"
else
    fail "bom_receipt_is_read"
fi

output_is_ascii_json() {
    python3 - "$@" <<'PY'
import sys
for path in sys.argv[1:]:
    data = open(path, "rb").read()
    assert data.endswith(b"\n") and not data.endswith(b"\n\n"), path
    assert data.isascii(), path
PY
}
if output_is_ascii_json "$win" "$mac"; then pass "output_is_ascii_json_with_trailing_newline"; else fail "output_is_ascii_json_with_trailing_newline"; fi

# @req REQ-UPD-19
key_probe="$(printf '%s\n' "$THROWAWAY_KEY" | sed -n 2p)"
if ! grep -q "set -x" "$SCRIPT" \
    && ! grep -qF "$key_probe" "$tmp/run.log" \
    && ! grep -qF "$key_probe" "$win" "$mac" "$win.sig" "$mac.sig" \
    && [ -z "$(find "$tmp" -name 'snackpilot-*' 2>/dev/null)" ]; then
    pass "no_key_material_in_output_or_script_trace"
else
    fail "no_key_material_in_output_or_script_trace"
fi

# --- the workflow steps -----------------------------------------------------
job_block() { awk -v job="  $1:" -v stop="$2" '$0 == job { f = 1 } f && $0 == stop { exit } f' "$WORKFLOW"; }

# The `run: |` body of a step (`$1`) in a job (`$2`), de-indented and written to
# `$3`. Fails when the step has no `run:` body.
step_run_body() { # name job out
    job_block "$2" 'jobs:' | awk -v step="      - name: $1" '
        $0 == step { found = 1; next }
        found && /^      - name:/ { exit }
        found && !body && /^        run: \|/ { body = 1; next }
        found && body && /^        / { sub(/^        /, ""); print }
    ' > "$3"
    grep -q . "$3" || { echo "no run body for step '$1'"; return 1; }
}

# Prints the literal scalar value of an env key (`$2`) of a step (`$1`).
step_env() { # step key
    job_block publish 'jobs:' | awk -v step="      - name: $1" -v key="          $2:" '
        $0 == step { found = 1; next }
        found && /^      - name:/ { exit }
        found && index($0, key) == 1 { sub(/^[^:]*: /, ""); print; exit }
    '
}

# Runs an extracted step the way the runner does: `bash -euo pipefail <file>`
# from the workspace root, with stub tools on PATH and a stub uploader. Every
# stub call is appended to `calls`; `published` is written only when the step
# reaches the uploader.
run_step() { # step-file work calls published [VAR=value ...]
    local step="$1" work="$2" calls="$3" published="$4"
    shift 4
    mkdir -p "$work/tools/ci" "$work/bin"
    cat > "$work/tools/ci/github_release.py" <<PY
import pathlib, sys
pathlib.Path("$published").write_text("\n".join(sys.argv[1:]) + "\n")
PY
    chmod +x "$work/tools/ci/github_release.py"
    # `bash` in the step must be the real one, not the function of this test.
    local real_bash; real_bash="$(type -P bash)"
    cat > "$work/bin/bash" <<PY
#!/bin/sh
printf 'bash %s\n' "\$*" >> "$calls"
exec "$real_bash" "\$@"
PY
    chmod +x "$work/bin/bash"
    # `env` is called by path: a function of this test must not shadow it.
    ( cd "$work" && "$(type -P env)" PATH="$work/bin:$PATH" "$@" \
        bash -euo pipefail "$step" > "$work/step.log" 2>&1 )
}

# @req REQ-UPD-19
workflow_signs() {
    local work="$tmp/wf" real_script="$ROOT/tools/devops/make-update-manifest.sh"
    mkdir -p "$work/tools/devops" "$work/dist/desktop-macos" "$work/dist/desktop-windows" "$work/dist/notes"
    printf 'notes\n' > "$work/dist/notes/body.md"
    : > "$work/dist/desktop-macos/SnackPilot-9.9.9-macos-arm64.zip"
    : > "$work/dist/desktop-windows/SnackPilot-9.9.9-windows-x64-setup.exe"
    # The real script creates its output directory; the stub must too, or the
    # step's `ls -l dist/update` fails for the wrong reason.
    printf '#!/bin/sh\nmkdir -p dist/update\n' > "$work/tools/devops/make-update-manifest.sh"
    chmod +x "$work/tools/devops/make-update-manifest.sh"

    # The step names the secret only through `secrets.` in its env, never in an
    # `if:` (runner secret handling is deliberately shell-only) and never with `set -x`.
    local sign; sign="$(job_block publish 'jobs:' | awk '/^      - name: Sign the update manifests/ { f = 1 } f && /^      - name:/ && !/Sign the update manifests/ { exit } f { print }')"
    [ "$(step_env "Sign the update manifests" SNACKPILOT_UPDATE_SIGNING_KEY)" \
        = '${{ secrets.DESKTOP_UPDATE_SIGNING_KEY }}' ] || { echo "secret env"; return 1; }
    # grep -c, not grep -q: a silent SIGPIPE on the writer would fail `!` under
    # pipefail.
    [ "$(echo "$sign" | grep -c 'set -x')" = 0 ] || { echo "set -x"; return 1; }
    [ "$(echo "$sign" | grep -cE '^ +if:.*secrets\.')" = 0 ] || { echo "secret in if"; return 1; }

    step_run_body "Sign the update manifests" publish "$work/sign.sh" || return 1
    grep -q 'make-update-manifest.sh' "$work/sign.sh" || { echo "the step does not call the script"; return 1; }

    # The runner's shell runs it, with the secret set: the script's arguments
    # are the five of design D8.
    : > "$work/calls"
    run_step "$work/sign.sh" "$work" "$work/calls" "$work/published" \
        SNACKPILOT_UPDATE_SIGNING_KEY=throwaway VERSION=9.9.9 || { echo "signing step failed: $(cat "$work/step.log")"; return 1; }
    grep -qF "bash tools/devops/make-update-manifest.sh 9.9.9 dist/desktop-macos dist/desktop-windows dist/notes/body.md dist/update" "$work/calls" \
        || { echo "script arguments: $(cat "$work/calls" 2>/dev/null)"; return 1; }

    # "Sign the update manifests" sits between the notes and the publish.
    local steps sign_at notes publish
    steps="$(job_block publish 'jobs:' | sed -n 's/^      - name: //p')"
    notes="$(grep -nx 'Write the release notes' <<< "$steps" | cut -d: -f1)"
    sign_at="$(grep -nx 'Sign the update manifests' <<< "$steps" | cut -d: -f1)"
    publish="$(grep -nx 'Publish on GitHub' <<< "$steps" | cut -d: -f1)"
    [ -n "$notes" ] && [ -n "$sign_at" ] && [ -n "$publish" ] \
        && [ "$notes" -lt "$sign_at" ] && [ "$sign_at" -lt "$publish" ] \
        || { echo "publish steps: $(tr '\n' '|' <<< "$steps")"; return 1; }
}

# @req REQ-UPD-18
workflow_publishes_manifests() {
    local work="$tmp/wf-publish" calls published
    calls="$work.calls"; published="$work.published"
    mkdir -p "$work/dist/desktop-macos" "$work/dist/desktop-windows" "$work/dist/update"
    step_run_body "Publish on GitHub" publish "$work/publish.sh" || return 1

    local zip="SnackPilot-9.9.9-macos-arm64.zip"
    local exe="SnackPilot-9.9.9-windows-x64-setup.exe"
    local common=(GITHUB_TOKEN=x GITHUB_REPO=radaiko/SnackPilot TAG=desktop/v9.9.9 TITLE=t BODY_FILE=b)
    local unsigned='{"trust_state":"unsigned"}'
    local notarized='{"trust_state":"developer-id-notarized-stapled"}'

    # 1. A Windows installer without its manifest: nothing is published.
    printf '%s\n' "$unsigned" > "$work/dist/desktop-macos/native-build-receipt.json"
    : > "$work/dist/desktop-macos/$zip"
    : > "$work/dist/desktop-windows/$exe"
    : > "$work/dist/desktop-windows/$exe.sha256"
    : > "$work/dist/update/SnackPilot-manifest-windows-x64.json.sig"
    rm -f "$published" "$calls"
    if run_step "$work/publish.sh" "$work" "$calls" "$published" "${common[@]}"; then
        echo "published without the Windows manifest"; return 1
    fi
    [ ! -e "$published" ] || { echo "uploader ran without the Windows manifest"; return 1; }
    grep -q "missing artifact: dist/update/SnackPilot-manifest-windows-x64.json" "$work/step.log" \
        || { echo "no missing-manifest message: $(cat "$work/step.log")"; return 1; }

    # 2. The same tree with the manifest: the Windows and macOS assets publish,
    #    the macOS manifest pair is not expected for an unsigned build.
    : > "$work/dist/update/SnackPilot-manifest-windows-x64.json"
    rm -f "$published" "$calls"
    run_step "$work/publish.sh" "$work" "$calls" "$published" "${common[@]}" \
        || { echo "publish failed: $(cat "$work/step.log")"; return 1; }
    for asset in "dist/desktop-macos/$zip" "dist/desktop-windows/$exe" \
                 "dist/desktop-windows/$exe.sha256" "dist/update/SnackPilot-manifest-windows-x64.json" \
                 "dist/update/SnackPilot-manifest-windows-x64.json.sig"; do
        grep -qxF "$asset" "$published" || { echo "not published: $asset"; return 1; }
    done
    ! grep -q 'macos-arm64.json' "$published" || { echo "a macOS manifest was required of an unsigned build"; return 1; }

    # 3. A notarized macOS build must ship its own manifest pair too.
    printf '%s\n' "$notarized" > "$work/dist/desktop-macos/native-build-receipt.json"
    rm -f "$published" "$calls"
    if run_step "$work/publish.sh" "$work" "$calls" "$published" "${common[@]}"; then
        echo "published a notarized build without the macOS manifest"; return 1
    fi
    [ ! -e "$published" ] || { echo "uploader ran without the macOS manifest"; return 1; }
    : > "$work/dist/update/SnackPilot-manifest-macos-arm64.json"
    : > "$work/dist/update/SnackPilot-manifest-macos-arm64.json.sig"
    rm -f "$published" "$calls"
    run_step "$work/publish.sh" "$work" "$calls" "$published" "${common[@]}" \
        || { echo "publish failed: $(cat "$work/step.log")"; return 1; }
    grep -qxF "dist/update/SnackPilot-manifest-macos-arm64.json" "$published" \
        || { echo "macOS manifest not published"; return 1; }
    grep -qxF "dist/update/SnackPilot-manifest-macos-arm64.json.sig" "$published" \
        || { echo "macOS signature not published"; return 1; }
}
if workflow_signs; then pass "workflow_names_the_secret_only_through_env"; else fail "workflow_names_the_secret_only_through_env"; fi
if workflow_publishes_manifests; then pass "workflow_signs_before_publishing_and_publishes_the_manifests"; else fail "workflow_signs_before_publishing_and_publishes_the_manifests"; fi

# --- the committed fixture trio is a real run of this script ----------------
fixture_is_real_output() {
    local fx="$tmp/fixture" out5="$tmp/update-fixture" pub="$tmp/fixture-pub.pem"
    mkdir -p "$fx/desktop-windows"
    printf 'SnackPilot fixture payload\n' > "$fx/desktop-windows/SnackPilot-2.3.0-windows-x64-setup.exe"
    printf 'Dieser Build ist nicht von Developer ID signiert.\n' > "$fx/body.md"
    cat > "$fx/desktop-windows/native-build-receipt.json" <<EOF
{"platform":"windows","architecture":"x86_64","version":"2.3.0","build":"42",
 "artifact_path":"$fx/desktop-windows/SnackPilot-2.3.0-windows-x64-setup.exe","trust_state":"unsigned"}
EOF
    SNACKPILOT_UPDATE_SIGNING_KEY="$THROWAWAY_KEY" bash "$SCRIPT" "2.3.0" \
        "$fx/desktop-macos" "$fx/desktop-windows" "$fx/body.md" "$out5" > "$tmp/fixture.log" 2>&1 \
        || { cat "$tmp/fixture.log"; return 1; }
    for f in openssl-manifest.json openssl-manifest.json.sig openssl-public-key.hex; do
        [ -s "$DOCS_FIXTURES/$f" ] && cmp -s "$DOCS_FIXTURES/$f" "$ROOT/src/core/tests/fixtures/update/$f" \
            || { echo "fixture $f missing or not mirrored"; return 1; }
    done
    # The committed manifest must be what a fresh run writes — byte for byte,
    # apart from the `sha256` of the payload this test makes up. The signature is
    # checked against its own committed key below.
    python3 - "$out5/SnackPilot-manifest-windows-x64.json" "$DOCS_FIXTURES/openssl-manifest.json" <<'PY'
import json, sys
fresh = json.load(open(sys.argv[1]))
committed = json.load(open(sys.argv[2]))
fresh.pop("sha256")
committed.pop("sha256")
assert fresh == committed, (fresh, committed)
PY
    [ "$(field "$out5/SnackPilot-manifest-windows-x64.json" tag)" = "desktop/v2.3.0" ] || return 1
    python3 - "$DOCS_FIXTURES/openssl-public-key.hex" "$pub" <<'PY'
import base64, sys
key = bytes.fromhex(open(sys.argv[1]).read().strip())
assert len(key) == 32
der = bytes.fromhex("302a300506032b6570032100") + key
pem = base64.b64encode(der).decode()
open(sys.argv[2], "w").write("-----BEGIN PUBLIC KEY-----\n%s\n-----END PUBLIC KEY-----\n" % pem)
PY
    as_raw_sig "$DOCS_FIXTURES/openssl-manifest.json.sig" "$tmp/fixture.sig.bin"
    openssl pkeyutl -verify -pubin -inkey "$pub" -rawin \
        -in "$DOCS_FIXTURES/openssl-manifest.json" \
        -sigfile "$tmp/fixture.sig.bin" >/dev/null 2>&1
}
if fixture_is_real_output; then pass "committed_fixture_trio_is_a_real_script_output"; else fail "committed_fixture_trio_is_a_real_script_output"; fi

echo
if [ "$fails" -eq 0 ]; then echo "all update-manifest tests passed"; else echo "$fails update-manifest test(s) failed"; exit 1; fi
