#!/usr/bin/env bash
# Unit tests for the pure helpers in lib.sh. Run: bash tools/devops/test-lib.sh
set -uo pipefail
source "$(dirname "$0")/lib.sh"

fails=0
assert()      { if eval "$1"; then echo "ok: $2"; else echo "FAIL: $2"; fails=$((fails+1)); fi; }
assert_not()  { if eval "$1"; then echo "FAIL: $2"; fails=$((fails+1)); else echo "ok: $2"; fi; }

# validate_semver
assert     'validate_semver 2.0.0'      'accepts 2.0.0'
assert     'validate_semver 10.20.30'   'accepts 10.20.30'
assert_not 'validate_semver 2.0'        'rejects 2.0'
assert_not 'validate_semver v2.0.0'     'rejects v2.0.0'
assert_not 'validate_semver 2.0.0-rc1'  'rejects 2.0.0-rc1'

# version_gt
assert     'version_gt 2.0.1 2.0.0'  '2.0.1 > 2.0.0'
assert     'version_gt 2.1.0 2.0.9'  '2.1.0 > 2.0.9'
assert_not 'version_gt 2.0.0 2.0.0'  '2.0.0 !> 2.0.0'
assert_not 'version_gt 2.0.0 2.0.1'  '2.0.0 !> 2.0.1'

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT

# set_cargo_package_version — only the [package] version, not a dep's
cat > "$tmp/Cargo.toml" <<'EOF'
[package]
name = "snackpilot-core"
version = "2.0.0"

[dependencies]
serde = { version = "1.0" }
EOF
set_cargo_package_version "$tmp/Cargo.toml" 2.1.0
assert 'grep -q '"'"'^version = "2.1.0"'"'"' "'"$tmp"'/Cargo.toml"' 'cargo package version bumped'
assert 'grep -q '"'"'version = "1.0"'"'"' "'"$tmp"'/Cargo.toml"'      'cargo dep version untouched'

# set_yaml_key
cat > "$tmp/project.yml" <<'EOF'
    MARKETING_VERSION: "2.0.0"
    CURRENT_PROJECT_VERSION: "1"
EOF
set_yaml_key "$tmp/project.yml" MARKETING_VERSION 2.1.0
set_yaml_key "$tmp/project.yml" CURRENT_PROJECT_VERSION 42
assert 'grep -q '"'"'MARKETING_VERSION: "2.1.0"'"'"' "'"$tmp"'/project.yml"'      'marketing version bumped'
assert 'grep -q '"'"'CURRENT_PROJECT_VERSION: "42"'"'"' "'"$tmp"'/project.yml"'   'project version bumped'

# set_gradle_string / set_gradle_int
cat > "$tmp/build.gradle.kts" <<'EOF'
        versionCode = 1
        versionName = "2.0.0"
EOF
set_gradle_string "$tmp/build.gradle.kts" versionName 2.1.0
set_gradle_int "$tmp/build.gradle.kts" versionCode 42
assert 'grep -q '"'"'versionName = "2.1.0"'"'"' "'"$tmp"'/build.gradle.kts"'  'gradle versionName bumped'
assert 'grep -q '"'"'versionCode = 42'"'"' "'"$tmp"'/build.gradle.kts"'        'gradle versionCode bumped'

echo "---"; [[ $fails -eq 0 ]] && echo "ALL PASS" || { echo "$fails FAILED"; exit 1; }
