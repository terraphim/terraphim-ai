#!/usr/bin/env bash
# P2-6: render-server-nfpm.sh must validate --version before interpolating it
# into the nFPM YAML descriptor (defense in depth; verify-nfpm.sh already
# enforces the same grammar on the verify side).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-render-nfpm-version.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

printf 'fake-binary' >"$TMP/terraphim_server"
chmod +x "$TMP/terraphim_server"
export SOURCE_DATE_EPOCH=1700000000

expect_rejected() {
    local version="$1"
    if "$RENDER" --format deb --version "$version" \
        --target x86_64-unknown-linux-musl \
        --binary "$TMP/terraphim_server" --output "$TMP/out.yaml" \
        >/dev/null 2>&1; then
        fail "version '$version' was accepted and interpolated into YAML"
    fi
}

# Injection and malformed versions fail closed before any YAML is rendered.
expect_rejected '1.2.3"; inject: "yes'
expect_rejected '1.2.3${IFS}evil'
expect_rejected '../etc/passwd'
expect_rejected '1.2'
expect_rejected 'latest'
expect_rejected ''
expect_rejected '1.2.3
malicious: true'

# Valid semver (with or without the v prefix) still renders verbatim.
for version in 1.21.3 v1.21.3 0.0.1; do
    "$RENDER" --format deb --version "$version" \
        --target x86_64-unknown-linux-musl \
        --binary "$TMP/terraphim_server" --output "$TMP/out-$version.yaml" \
        >/dev/null || fail "valid version '$version' was rejected"
    grep -Fqx "version: $version" "$TMP/out-$version.yaml" \
        || fail "rendered YAML does not contain the verbatim version for '$version'"
done

echo "render-server-nfpm version validation tests passed"
