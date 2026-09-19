#!/usr/bin/env bash
# Regression tests for the pinned nFPM verifier.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
VERIFY="$ROOT/.github/scripts/nfpm/verify-nfpm.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-verify-nfpm-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

assert_contains() {
    local file="$1"
    local pattern="$2"
    grep -Fq -- "$pattern" "$file" || fail "expected '$pattern' in $file"
}

make_nfpm_fixture() {
    local path="$1"
    local version_text="$2"
    cat > "$path" <<EOF
#!/usr/bin/env sh
if [ "\${1:-}" = "--version" ]; then
  cat <<'VERSION_EOF'
${version_text}
VERSION_EOF
  exit 0
fi
exit 64
EOF
    chmod 0755 "$path"
}

test_accepts_multiline_version_and_leading_v_input() {
    local bin="$TMP/nfpm-good"
    make_nfpm_fixture "$bin" "nFPM 2.47.0
commit: fixture"
    local sha
    sha="$(sha256sum "$bin" | awk '{print $1}')"

    "$VERIFY" --binary "$bin" --version v2.47.0 --sha256 "$sha" >"$TMP/good.out"
    assert_contains "$TMP/good.out" "version=2.47.0"
}

test_rejects_wrong_binary_sha() {
    local bin="$TMP/nfpm-wrong-sha"
    make_nfpm_fixture "$bin" "nFPM 2.47.0"

    if "$VERIFY" --binary "$bin" --version 2.47.0 --sha256 "0000000000000000000000000000000000000000000000000000000000000000" >"$TMP/sha.log" 2>&1; then
        fail "verifier accepted the wrong binary SHA"
    fi
    assert_contains "$TMP/sha.log" "FAILED"
}

test_rejects_wrong_version() {
    local bin="$TMP/nfpm-wrong-version"
    make_nfpm_fixture "$bin" "nfpm version v2.48.0"
    local sha
    sha="$(sha256sum "$bin" | awk '{print $1}')"

    if "$VERIFY" --binary "$bin" --version 2.47.0 --sha256 "$sha" >"$TMP/version.out" 2>"$TMP/version.err"; then
        fail "verifier accepted the wrong nFPM version"
    fi
    assert_contains "$TMP/version.err" "version mismatch"
}

test_rejects_non_semver_input() {
    local bin="$TMP/nfpm-bad-input"
    make_nfpm_fixture "$bin" "nfpm version 2.47.0"
    local sha
    sha="$(sha256sum "$bin" | awk '{print $1}')"

    if "$VERIFY" --binary "$bin" --version 2.47 --sha256 "$sha" >"$TMP/input.out" 2>"$TMP/input.err"; then
        fail "verifier accepted a non-semver input version"
    fi
    assert_contains "$TMP/input.err" "must be semantic"
}

test_accepts_multiline_version_and_leading_v_input
test_rejects_wrong_binary_sha
test_rejects_wrong_version
test_rejects_non_semver_input

echo "nFPM verifier tests passed"
