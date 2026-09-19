#!/usr/bin/env bash
# Regression test for the per-target cargo-deb parity packaging path used by
# the release workflow:
#   stage exact qualified bytes at target/<triple>/release/<bin> (the asset
#   path target/release/<bin> from [package.metadata.deb] resolves against
#   the triple-specific dir when cargo-deb is given --target), then
#   cargo deb --no-build --no-strip --target <triple>
# must yield exactly one DEB per matrix target whose Architecture matches the
# triple and whose payload bytes are the exact staged input.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
FIXTURE="$ROOT/tests/fixtures/cargo-deb-parity/fixture-server-payload"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-cargo-deb-staging.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

command -v cargo-deb >/dev/null 2>&1 || {
    echo "SKIP: cargo-deb not installed" >&2
    exit 0
}
command -v dpkg-deb >/dev/null 2>&1 || {
    echo "SKIP: dpkg-deb not installed" >&2
    exit 0
}
command -v cargo >/dev/null 2>&1 || {
    echo "SKIP: cargo not installed" >&2
    exit 0
}

cp -a "$FIXTURE" "$TMP/crate"

# Build the "qualified" fixture bytes once (standalone crate, no deps).
(cd "$TMP/crate" && cargo build --release --offline >/dev/null)
STAGED="$TMP/crate/target/release/fixture_server_payload"
[[ -f "$STAGED" ]] || fail "fixture binary was not built"
INPUT_SHA="$(sha256sum "$STAGED" | awk '{print $1}')"

verify_target() {
    local triple="$1"
    local expected_arch="$2"
    local out_dir="$TMP/out-$triple"
    local extract="$TMP/extract-$triple"

    mkdir -p "$TMP/crate/target/$triple/release"
    cp "$STAGED" "$TMP/crate/target/$triple/release/fixture_server_payload"

    (cd "$TMP/crate" && cargo deb \
        --no-build --no-strip \
        --target "$triple" \
        --output "$out_dir" >/dev/null 2>&1)

    local debs=()
    mapfile -t debs < <(find "$out_dir" -maxdepth 1 -type f -name '*.deb' | sort)
    [[ ${#debs[@]} -eq 1 ]] || fail "expected exactly one cargo-deb package for $triple, found ${#debs[@]}"

    local arch
    arch="$(dpkg-deb --field "${debs[0]}" Architecture)"
    [[ "$arch" == "$expected_arch" ]] || fail "$triple cargo-deb Architecture mismatch: expected $expected_arch actual $arch"

    mkdir -p "$extract"
    dpkg-deb --extract "${debs[0]}" "$extract"
    local payload_sha
    payload_sha="$(sha256sum "$extract/usr/bin/fixture_server_payload" | awk '{print $1}')"
    [[ "$payload_sha" == "$INPUT_SHA" ]] || fail "$triple cargo-deb payload SHA mismatch: staged bytes were altered (strip?): expected $INPUT_SHA actual $payload_sha"

    echo "OK $triple: arch=$arch payload=exact"
}

verify_target x86_64-unknown-linux-musl amd64
verify_target aarch64-unknown-linux-musl arm64

echo "cargo-deb per-target staging tests passed"
