#!/usr/bin/env bash
# Behavioral contract tests for assemble-release-inventory.sh.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
ASSEMBLE="$ROOT/.github/scripts/release/assemble-release-inventory.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-assemble-inventory.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

expect_ok() {
    "$ASSEMBLE" "$@" >/dev/null || fail "expected success: $*"
}

expect_fail() {
    local message="$1"
    shift
    if "$ASSEMBLE" "$@" >"$TMP/stdout" 2>"$TMP/stderr"; then
        fail "expected failure ($message): $*"
    fi
    grep -Fq "$message" "$TMP/stderr" || fail "expected '$message' in stderr, got: $(cat "$TMP/stderr")"
}

make_stage() {
    local dir="$1"
    shift
    mkdir -p "$dir"
    local name
    for name in "$@"; do
        printf 'fixture\n' > "$dir/$name"
    done
}

# A complete managed matrix merges DEB/RPM/checksum outputs into the inventory.
# The legacy stage here carries a non-colliding basename; the amd64 legacy vs
# managed collision is a separate, dedicated test below.
test_complete_managed_matrix_is_merged() {
    local out="$TMP/out1" legacy="$TMP/legacy1" staging="$TMP/staging1"
    mkdir -p "$out"
    : > "$out/terraphim_server-x86_64-apple-darwin"
    make_stage "$legacy" "terraphim-legacy-docs_1.0.0-1_all.deb"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_ok --output "$out" --legacy "$legacy" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    local merged
    for merged in \
        "terraphim-legacy-docs_1.0.0-1_all.deb" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"; do
        [[ -f "$out/$merged" ]] || fail "expected merged asset $out/$merged"
    done
}

# Managed DEB colliding by basename with the legacy host-native cargo-deb
# package must fail closed instead of silently clobbering.
test_managed_legacy_basename_conflict_fails() {
    local out="$TMP/out2" legacy="$TMP/legacy2" staging="$TMP/staging2"
    mkdir -p "$out"
    : > "$out/terraphim_server-universal-apple-darwin"
    make_stage "$legacy" "terraphim-server_1.0.0-1_amd64.deb"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "duplicate release asset basename: terraphim-server_1.0.0-1_amd64.deb" \
        --output "$out" --legacy "$legacy" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ ! -e "$out/terraphim-server_1.0.0-1_amd64.deb" ]] ||
        fail "conflicting asset was merged despite duplicate rejection"
}

# A partial managed matrix (one target present, one missing) is all-or-nothing.
test_partial_managed_matrix_fails() {
    local out="$TMP/out3" staging="$TMP/staging3"
    mkdir -p "$out"
    : > "$out/terraphim_server-x86_64-apple-darwin"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "managed package matrix incomplete; missing targets: aarch64-unknown-linux-musl (all-or-nothing)" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ ! -e "$out/terraphim-server_1.0.0-1_amd64.deb" ]] ||
        fail "partial managed matrix must not be merged"
}

# An absent managed stage (job skipped) leaves the inventory untouched.
test_absent_managed_stage_is_tolerated() {
    local out="$TMP/out4"
    mkdir -p "$out"
    : > "$out/terraphim_server-universal-apple-darwin"
    expect_ok --output "$out" --managed-staging "$TMP/staging-absent" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl
    [[ "$(find "$out" -maxdepth 1 -type f | wc -l)" -eq 1 ]] ||
        fail "absent managed stage must not add inventory entries"
}

# A present managed artifact directory lacking the RPM output is rejected.
test_incomplete_managed_target_dir_fails() {
    local out="$TMP/out5" staging="$TMP/staging5"
    mkdir -p "$out"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "managed artifact directory missing RPM output" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl
}

# Binary artifacts colliding by basename with legacy packages are also rejected.
test_binary_legacy_basename_conflict_fails() {
    local out="$TMP/out6" legacy="$TMP/legacy6"
    mkdir -p "$out"
    : > "$out/terraphim-server_1.0.0-1_amd64.deb"
    make_stage "$legacy" "terraphim-server_1.0.0-1_amd64.deb"

    expect_fail "duplicate release asset basename: terraphim-server_1.0.0-1_amd64.deb" \
        --output "$out" --legacy "$legacy"
}

# The workflow's authoritative release paths are managed-only: a complete
# managed matrix with no legacy stage at all must assemble cleanly (this is
# the call shape used by create-release and upload-recovered-release-assets).
test_managed_only_inventory_succeeds() {
    local out="$TMP/out7" staging="$TMP/staging7"
    mkdir -p "$out"
    : > "$out/terraphim_server-universal-apple-darwin"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_ok --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    local merged
    for merged in \
        "terraphim_server-universal-apple-darwin" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"; do
        [[ -f "$out/$merged" ]] || fail "expected managed-only asset $out/$merged"
    done
}

# A managed target directory is an exact producer/consumer boundary. An
# unrelated fourth file must reject the entire matrix before any asset moves.
test_unexpected_managed_inventory_fails_before_merge() {
    local out="$TMP/out8" staging="$TMP/staging8"
    mkdir -p "$out"
    printf 'binary\n' > "$out/terraphim_server-universal-apple-darwin"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt" \
        "unexpected.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "unexpected managed artifact" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ ! -e "$out/terraphim-server_1.0.0-1_amd64.deb" ]] ||
        fail "unexpected managed inventory was partially merged"
}

# Every format in a target directory must describe the same version. A stale
# DEB beside current RPM/checksum outputs must fail before authoritative merge.
test_stale_version_managed_inventory_fails_before_merge() {
    local out="$TMP/out9" staging="$TMP/staging9"
    mkdir -p "$out"
    printf 'binary\n' > "$out/terraphim_server-universal-apple-darwin"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_0.9.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "unexpected managed artifact" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ ! -e "$out/terraphim-server-1.0.0-1.x86_64.rpm" ]] ||
        fail "stale managed inventory was partially merged"
}

test_unsafe_managed_inputs_fail_before_merge() {
    local out="$TMP/out10" staging="$TMP/staging10" linked="$TMP/linked-rpm"
    mkdir -p "$out"
    printf 'linked package\n' > "$linked"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    ln -s "$linked" "$staging/server-managed-packages-x86_64-unknown-linux-musl/terraphim-server-1.0.0-1.x86_64.rpm"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "managed artifact must be a regular non-symlink file" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ "$(find "$out" -mindepth 1 -maxdepth 1 -type f | wc -l)" -eq 0 ]] ||
        fail "unsafe managed inventory was partially merged"
}

test_zero_length_managed_input_fails_before_merge() {
    local out="$TMP/out11" staging="$TMP/staging11"
    mkdir -p "$out"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    : > "$staging/server-managed-packages-x86_64-unknown-linux-musl/terraphim-server_1.0.0-1_amd64.deb"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"

    expect_fail "managed artifact must not be zero-length" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl
}

# The staging root is itself an exact producer/consumer boundary. A directory
# for any target outside the release matrix must not be mistaken for an absent
# managed stage.
test_wrong_target_managed_staging_dir_fails() {
    local out="$TMP/out12" staging="$TMP/staging12"
    mkdir -p "$out"
    make_stage "$staging/server-managed-packages-riscv64gc-unknown-linux-gnu" \
        "terraphim-server_1.0.0-1_riscv64.deb"

    expect_fail "unexpected managed staging entry" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl
}

# Unexpected root entries, including hidden files, reject an otherwise
# complete matrix before any managed package is merged.
test_unexpected_managed_staging_entry_fails_before_merge() {
    local out="$TMP/out13" staging="$TMP/staging13"
    mkdir -p "$out"
    make_stage "$staging/server-managed-packages-x86_64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_amd64.deb" \
        "terraphim-server-1.0.0-1.x86_64.rpm" \
        "terraphim-server-1.0.0-x86_64-unknown-linux-musl.package-sha256sums.txt"
    make_stage "$staging/server-managed-packages-aarch64-unknown-linux-musl" \
        "terraphim-server_1.0.0-1_arm64.deb" \
        "terraphim-server-1.0.0-1.aarch64.rpm" \
        "terraphim-server-1.0.0-aarch64-unknown-linux-musl.package-sha256sums.txt"
    printf 'unexpected\n' > "$staging/.unexpected-root-entry"

    expect_fail "unexpected managed staging entry" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl

    [[ ! -e "$out/terraphim-server_1.0.0-1_amd64.deb" ]] ||
        fail "managed matrix with an unexpected root entry was partially merged"
}

# A symlink at the staging root must not be followed or ignored, regardless of
# whether it resolves to a directory.
test_managed_staging_root_symlink_entry_fails() {
    local out="$TMP/out14" staging="$TMP/staging14" linked="$TMP/linked-root-dir"
    mkdir -p "$out" "$staging" "$linked"
    ln -s "$linked" "$staging/server-managed-packages-extra"

    expect_fail "unexpected managed staging entry" \
        --output "$out" --managed-staging "$staging" \
        --managed-target x86_64-unknown-linux-musl \
        --managed-target aarch64-unknown-linux-musl
}

test_complete_managed_matrix_is_merged
test_managed_legacy_basename_conflict_fails
test_partial_managed_matrix_fails
test_absent_managed_stage_is_tolerated
test_incomplete_managed_target_dir_fails
test_binary_legacy_basename_conflict_fails
test_managed_only_inventory_succeeds
test_unexpected_managed_inventory_fails_before_merge
test_stale_version_managed_inventory_fails_before_merge
test_unsafe_managed_inputs_fail_before_merge
test_zero_length_managed_input_fails_before_merge
test_wrong_target_managed_staging_dir_fails
test_unexpected_managed_staging_entry_fails_before_merge
test_managed_staging_root_symlink_entry_fails

echo "assemble-release-inventory tests passed"
