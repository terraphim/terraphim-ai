#!/usr/bin/env bash
# Wrong-architecture regression tests for build-server-packages.sh.
#
# Builds tampered-arch nFPM fixture packages (correct payload bytes and
# receipts, wrong Architecture/ARCH metadata) and asserts the production
# verifiers fail closed on them by sourcing the producer with
# TERRAPHIM_BUILD_SERVER_PACKAGES_SOURCED=1.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
BUILD="$ROOT/.github/scripts/nfpm/build-server-packages.sh"
RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
NFPM_BIN="${NFPM_BIN:-nfpm}"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-server-nfpm-arch.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
export SOURCE_DATE_EPOCH=1700000000

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

command -v "$NFPM_BIN" >/dev/null 2>&1 || {
    echo "SKIP: nFPM not available ($NFPM_BIN)" >&2
    exit 0
}
command -v dpkg-deb >/dev/null 2>&1 || {
    echo "SKIP: dpkg-deb not installed" >&2
    exit 0
}

make_fixture_binary() {
    local path="$1"
    mkdir -p "$(dirname "$path")"
    printf '#!/usr/bin/env sh\nprintf "terraphim_server 9.8.7\\n"\n' > "$path"
    chmod 0755 "$path"
}

# Render a descriptor for the requested target, tamper the package arch, and
# pack it with nFPM under an explicit output name. Payload bytes and receipts
# stay correct; only the package architecture metadata is wrong.
build_tampered_package() {
    local format="$1"
    local target="$2"
    local real_arch="$3"
    local wrong_arch="$4"
    local out="$5"

    local cfg="$TMP/$format-$wrong_arch.yaml"
    "$RENDER" \
        --format "$format" \
        --version 9.8.7 \
        --target "$target" \
        --binary "$BINARY" \
        --output "$cfg" >/dev/null
    sed -i "s/^arch: ${real_arch}\$/arch: ${wrong_arch}/" "$cfg"
    grep -q "^arch: ${wrong_arch}\$" "$cfg" || fail "failed to tamper $format arch ($real_arch -> $wrong_arch)"
    "$NFPM_BIN" pkg --packager "$format" --config "$cfg" --target "$out" >/dev/null
    [[ -f "$out" ]] || fail "nFPM did not produce tampered $format at $out"
}

# Run a verifier function from the sourced producer in a subshell with the
# fixture globals set; the subshell exit code is the verifier outcome.
run_sourced_verifier() {
    local verifier="$1"
    local pkg="$2"

    (
        export VERSION=9.8.7
        export TARGET=x86_64-unknown-linux-musl
        export DEB_ARCH=amd64
        export RPM_ARCH=x86_64
        export WORK_DIR="$TMP/work"
        export EXPECTED_SHA
        TERRAPHIM_BUILD_SERVER_PACKAGES_SOURCED=1 source "$BUILD"
        "$verifier" "$pkg"
    ) >"$TMP/verifier.stdout" 2>"$TMP/verifier.stderr"
}

BINARY="$TMP/qualified/terraphim_server"
make_fixture_binary "$BINARY"
mkdir -p "$TMP/work"
EXPECTED_SHA="$(sha256sum "$BINARY" | awk '{print $1}')"

# DEB: a package declaring arm64 while the target demanded amd64 must fail
# closed before any release artifact is accepted.
test_wrong_arch_deb_fails_closed() {
    local wrong_deb="$TMP/terraphim-server_9.8.7-1_arm64.deb"
    build_tampered_package deb x86_64-unknown-linux-musl amd64 arm64 "$wrong_deb"

    if run_sourced_verifier verify_deb "$wrong_deb"; then
        fail "verify_deb accepted a wrong-architecture DEB"
    fi
    grep -Fq "DEB arch mismatch expected=amd64 actual=arm64" "$TMP/verifier.stderr" ||
        fail "missing DEB arch mismatch diagnostics: $(cat "$TMP/verifier.stderr")"
}

# RPM: a package carrying aarch64 while the target demanded x86_64 must fail
# closed, consuming the arch from the Docker RPM metadata or a host query.
test_wrong_arch_rpm_fails_closed() {
    local wrong_rpm="$TMP/terraphim-server-9.8.7-1.aarch64.rpm"

    if ! command -v rpm >/dev/null 2>&1 && ! docker info >/dev/null 2>&1; then
        echo "SKIP: RPM arch regression needs host rpm or Docker" >&2
        return 0
    fi
    build_tampered_package rpm x86_64-unknown-linux-musl x86_64 aarch64 "$wrong_rpm"

    if run_sourced_verifier verify_rpm "$wrong_rpm"; then
        fail "verify_rpm accepted a wrong-architecture RPM"
    fi
    grep -Fq "RPM arch mismatch expected=x86_64 actual=aarch64" "$TMP/verifier.stderr" ||
        fail "missing RPM arch mismatch diagnostics: $(cat "$TMP/verifier.stderr")"
}

# cargo-deb parity: a parity package with the wrong Architecture is rejected.
test_wrong_arch_cargo_deb_parity_fails_closed() {
    local parity_dir="$TMP/cargo-deb-parity"
    local wrong_deb="$parity_dir/terraphim-server_9.8.7-1_arm64.deb"
    mkdir -p "$parity_dir"
    build_tampered_package deb x86_64-unknown-linux-musl amd64 arm64 "$wrong_deb"

    if run_sourced_verifier verify_cargo_deb_parity "$parity_dir"; then
        fail "verify_cargo_deb_parity accepted a wrong-architecture parity package"
    fi
    grep -Fq "cargo-deb arch mismatch expected=amd64 actual=arm64" "$TMP/verifier.stderr" ||
        fail "missing cargo-deb arch mismatch diagnostics: $(cat "$TMP/verifier.stderr")"
}

test_wrong_arch_deb_fails_closed
test_wrong_arch_rpm_fails_closed
test_wrong_arch_cargo_deb_parity_fails_closed

echo "server nFPM wrong-arch regression tests passed"
