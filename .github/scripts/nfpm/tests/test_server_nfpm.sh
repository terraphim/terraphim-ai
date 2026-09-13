#!/usr/bin/env bash
# Hermetic tests for the terraphim_server managed-package producer.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
BUILD="$ROOT/.github/scripts/nfpm/build-server-packages.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-server-nfpm-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
export SOURCE_DATE_EPOCH=1700000000

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

assert_contains() {
    local file="$1"
    local pattern="$2"
    grep -Fq -- "$pattern" "$file" || fail "expected '$pattern' in $file"
}

assert_not_contains() {
    local file="$1"
    local pattern="$2"
    ! grep -Fq -- "$pattern" "$file" || fail "did not expect '$pattern' in $file"
}

make_fixture_binary() {
    local path="$1"
    mkdir -p "$(dirname "$path")"
    printf '#!/usr/bin/env sh\nprintf "terraphim_server 9.8.7\\n"\n' > "$path"
    chmod 0755 "$path"
}

test_render_deb_descriptor() {
    local bin="$TMP/target/x86_64-unknown-linux-musl/release/terraphim_server"
    local yaml="$TMP/server-deb.yaml"
    make_fixture_binary "$bin"

    "$RENDER" --format deb --version 9.8.7 --target x86_64-unknown-linux-musl --binary "$bin" --output "$yaml" >/dev/null

    assert_contains "$yaml" "name: terraphim-server"
    assert_contains "$yaml" "arch: amd64"
    assert_contains "$yaml" "section: utils"
    assert_contains "$yaml" "dst: /usr/bin/terraphim_server"
    assert_contains "$yaml" "dst: /usr/share/terraphim/package-manager.d/terraphim_server"
    assert_contains "$yaml" "dst: /usr/share/doc/terraphim-server/copyright"
    assert_contains "$yaml" "dst: /usr/share/doc/terraphim-server/changelog.Debian.gz"
    assert_contains "$yaml" "dst: /usr/share/man/man1/terraphim_server.1.gz"
    assert_contains "$yaml" "src: $yaml.terraphim_server.receipt"
    grep -qx 'dpkg' "$yaml.terraphim_server.receipt"
    [[ "$(stat -c '%a' "$yaml.terraphim_server.receipt")" == "644" ]] || fail "receipt mode is not 0644"
    assert_contains "$yaml" "depends: []"

    # Machine-readable Debian copyright must reference the common license
    # instead of embedding the full Apache-2.0 text.
    grep -Fq '/usr/share/common-licenses/Apache-2.0' "$yaml.copyright" ||
        fail "copyright does not reference /usr/share/common-licenses/Apache-2.0"
    ! grep -Fq 'TERMS AND CONDITIONS FOR USE' "$yaml.copyright" ||
        fail "copyright embeds the full Apache-2.0 license text"
    assert_contains "$RENDER" "gzip -9n"
    # The hand-rendered changelog.Debian.gz stays DEB-only; nFPM changelog
    # metadata is RPM-only.
    assert_not_contains "$yaml" "changelog:"
}

test_render_rpm_descriptor() {
    local bin="$TMP/target/aarch64-unknown-linux-musl/release/terraphim_server"
    local yaml="$TMP/server-rpm.yaml"
    make_fixture_binary "$bin"

    "$RENDER" --format rpm --version 9.8.7 --target aarch64-unknown-linux-musl --binary "$bin" --output "$yaml" >/dev/null

    assert_contains "$yaml" "arch: aarch64"
    assert_contains "$yaml" "src: $yaml.terraphim_server.receipt"
    grep -qx 'rpm' "$yaml.terraphim_server.receipt"
    assert_contains "$yaml" "dst: /usr/bin/terraphim_server"
    assert_not_contains "$yaml" "digest:"

    # Native RPM metadata: changelog tags, capitalized concise summary, and
    # RPM-only content types for license/readme.
    assert_contains "$yaml" "changelog: $yaml.changelog.yaml"
    assert_contains "$yaml" "summary: Privacy-first semantic search server"
    assert_contains "$yaml" "dst: /usr/share/licenses/terraphim-server/LICENSE-Apache-2.0"
    assert_contains "$yaml" "type: license"
    assert_contains "$yaml" "type: doc"
    assert_not_contains "$yaml" "provides:"
    assert_not_contains "$yaml" "replaces:"
    assert_not_contains "$yaml" "conflicts:"
    grep -q 'semver: 9.8.7' "$yaml.changelog.yaml" || fail "changelog.yaml missing semver entry"
}

test_render_rejects_gnu_target() {
    local bin="$TMP/target/x86_64-unknown-linux-gnu/release/terraphim_server"
    local yaml="$TMP/server-gnu.yaml"
    make_fixture_binary "$bin"

    if "$RENDER" --format deb --version 9.8.7 --target x86_64-unknown-linux-gnu --binary "$bin" --output "$yaml" 2>"$TMP/reject.err"; then
        fail "renderer accepted a GNU target"
    fi
    assert_contains "$TMP/reject.err" "only qualified MUSL targets are accepted"
}

test_build_reports_missing_nfpm_without_fallback_claim() {
    local bin="$TMP/target/x86_64-unknown-linux-musl/release/terraphim_server"
    make_fixture_binary "$bin"

    if "$BUILD" --version 9.8.7 --target x86_64-unknown-linux-musl --binary "$bin" --out-dir "$TMP/out" --nfpm "$TMP/missing-nfpm" 2>"$TMP/missing.err"; then
        fail "build succeeded without nFPM"
    fi
    assert_contains "$TMP/missing.err" "nFPM is required"
    assert_contains "$TMP/missing.err" "cargo-deb parity path"
}

test_deb_payload_fixture_matches_input_binary() {
    command -v dpkg-deb >/dev/null 2>&1 || {
        echo "SKIP: dpkg-deb not installed"
        return 0
    }

    local bin="$TMP/payload/terraphim_server"
    local pkgroot="$TMP/deb-root"
    local deb="$TMP/terraphim-server_9.8.7_amd64.deb"
    local extract="$TMP/deb-extract"
    make_fixture_binary "$bin"

    mkdir -p "$pkgroot/DEBIAN" "$pkgroot/usr/bin" "$pkgroot/usr/share/terraphim/package-manager.d"
    cp "$bin" "$pkgroot/usr/bin/terraphim_server"
    chmod 0755 "$pkgroot/usr/bin/terraphim_server"
    printf 'dpkg\n' > "$pkgroot/usr/share/terraphim/package-manager.d/terraphim_server"
    cat > "$pkgroot/DEBIAN/control" <<'EOF'
Package: terraphim-server
Version: 9.8.7
Section: utility
Priority: optional
Architecture: amd64
Maintainer: Terraphim Contributors <team@terraphim.ai>
Description: Terraphim AI server test package
EOF

    dpkg-deb --build --root-owner-group "$pkgroot" "$deb" >/dev/null
    dpkg-deb --extract "$deb" "$extract"

    local expected actual
    expected="$(sha256sum "$bin" | awk '{print $1}')"
    actual="$(sha256sum "$extract/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$actual" == "$expected" ]] || fail "DEB payload SHA mismatch"
    grep -qx 'dpkg' "$extract/usr/share/terraphim/package-manager.d/terraphim_server"
}

test_build_script_expects_nfpm_deb_filename() {
    assert_contains "$BUILD" 'terraphim-server_${VERSION}-1_${DEB_ARCH}.deb'
    assert_not_contains "$BUILD" 'terraphim-server_${VERSION}_${DEB_ARCH}.deb'
}

test_build_script_fails_closed_without_source_date_epoch_fallback() {
    assert_contains "$BUILD" 'SOURCE_DATE_EPOCH is required outside a git worktree'
    assert_not_contains "$BUILD" 'SOURCE_DATE_EPOCH=0'
}

test_build_script_has_cargo_deb_parity_oracle() {
    assert_contains "$BUILD" '--cargo-deb-dir'
    assert_contains "$BUILD" "expected exactly one cargo-deb terraphim-server package"
    assert_contains "$BUILD" "cargo-deb payload SHA mismatch"
    assert_contains "$BUILD" "cargo-deb/nFPM payload SHA mismatch"
}

test_build_script_fails_closed_on_package_architecture() {
    assert_contains "$BUILD" 'dpkg-deb --field "$pkg" Architecture'
    assert_contains "$BUILD" "DEB arch mismatch expected=\$DEB_ARCH actual=\$pkg_arch"
    assert_contains "$BUILD" "sed -n 's/^arch=//p' \"\$metadata\""
    assert_contains "$BUILD" "rpm -qp --qf '%{ARCH}' \"\$pkg\""
    assert_contains "$BUILD" "RPM arch mismatch expected=\$RPM_ARCH actual=\$pkg_arch"
    assert_contains "$BUILD" "cargo-deb arch mismatch expected=\$DEB_ARCH actual=\$cargo_arch"
}

test_native_gate_require_install_policy_and_receipt_ownership() {
    local native="$ROOT/.github/scripts/nfpm/tests/test_server_nfpm_native.sh"

    assert_contains "$native" "REQUIRE_INSTALL=1 requires the DEB install/upgrade/remove gate"
    assert_contains "$native" "REQUIRE_INSTALL=1 requires the RPM install/upgrade/remove gate"
    assert_contains "$native" "QUALIFIED: \$TARGET DEB byte/metadata/lint checks passed"
    assert_contains "$native" "QUALIFIED: \$TARGET RPM byte/metadata/lint checks passed"
    # Receipt ownership: the package-manager.d receipt must be owned by the
    # package manager database, not only the binary.
    assert_contains "$native" "dpkg-query -S /usr/share/terraphim/package-manager.d/terraphim_server"
    assert_contains "$native" "rpm -qf /usr/share/terraphim/package-manager.d/terraphim_server"
}

test_native_gate_builds_cross_arch_elf_fixture_for_lint() {
    local native="$ROOT/.github/scripts/nfpm/tests/test_server_nfpm_native.sh"

    # Cross targets package a deterministic arch-correct ELF (correct
    # e_machine, PT_INTERP + PT_DYNAMIC/DT_NEEDED) so lintian/rpmlint see a
    # dynamically linked foreign-arch binary; cross fixtures are never
    # executed (install lifecycle is native-only).
    assert_contains "$native" "write_cross_elf_fixture"
    assert_contains "$native" "if ! is_native_target; then"
    assert_contains "$native" 'write_cross_elf_fixture "$path"'
}

test_workflow_builds_per_target_cargo_deb_parity_from_qualified_musl() {
    local workflow="$ROOT/.github/workflows/release-comprehensive.yml"

    assert_contains "$workflow" "NFPM_ARCHIVE_SHA256: 0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783"
    assert_contains "$workflow" "NFPM_BINARY_SHA256: 17133a2467ffb7cec851c2d7bae0c6098d09d7ed7d3d101a9605f6a473323936"
    assert_contains "$workflow" ".github/scripts/nfpm/verify-nfpm.sh"
    assert_contains "$workflow" "test_server_nfpm_native.sh"
    # Parity is built per matrix target from the exact qualified MUSL bytes
    # staged into the cargo-deb assets path; never from a host-native build.
    assert_contains "$workflow" 'cp "$BIN" "target/${TARGET}/release/terraphim_server"'
    assert_contains "$workflow" "--no-build --no-strip"
    assert_contains "$workflow" '--output "cargo-deb-parity/${TARGET}"'
    assert_contains "$workflow" '--cargo-deb-dir "cargo-deb-parity/${TARGET}"'
    assert_not_contains "$workflow" "cargo-deb-artifact"
    # REQUIRE_INSTALL=1 only where the target triple matches the runner arch.
    assert_contains "$workflow" "runner.arch == 'X64' && matrix.target == 'x86_64-unknown-linux-musl'"
    assert_contains "$workflow" "runner.arch == 'ARM64' && matrix.target == 'aarch64-unknown-linux-musl'"
}

test_workflow_assembles_managed_release_inventory() {
    local workflow="$ROOT/.github/workflows/release-comprehensive.yml"

    assert_contains "$workflow" "assemble-release-inventory.sh"
    assert_contains "$workflow" "pattern: server-managed-packages-*"
    assert_contains "$workflow" "--managed-target x86_64-unknown-linux-musl"
    assert_contains "$workflow" "--managed-target aarch64-unknown-linux-musl"
    # The authoritative release paths are managed-only: the legacy
    # cargo-deb DEB shares its canonical basename with the managed x86_64
    # DEB, so merging both stages can only fail on duplicate rejection.
    assert_not_contains "$workflow" "path: legacy-deb"
    assert_not_contains "$workflow" "--legacy legacy-deb"
}

test_build_script_has_docker_closed_fallbacks_and_lint() {
    assert_contains "$BUILD" "docker_rpm_tool"
    assert_contains "$BUILD" "require_docker_or_fail"
    assert_contains "$BUILD" "lintian --fail-on error"
    assert_not_contains "$BUILD" "--fail-on error,warning"
    assert_contains "$BUILD" "rpmlint"
    assert_contains "$BUILD" "RPM payload and metadata verification"
}

test_build_script_has_fail_closed_static_musl_lint_policy() {
    # The only allowlisted lint error is the exact justified static-MUSL
    # diagnostic for terraphim-server at usr/bin/terraphim_server; the
    # fail-closed parser is shared by the host and Docker lint paths.
    assert_contains "$BUILD" "LINTIAN_JUSTIFIED_STATIC_E='E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server]'"
    assert_contains "$BUILD" 'statically-linked-binary /usr/bin/terraphim_server$'
    assert_contains "$BUILD" "enforce_lint_policy lintian"
    assert_contains "$BUILD" "enforce_lint_policy rpmlint"
    assert_contains "$BUILD" "unjustified error"
    assert_contains "$BUILD" "tool/install/transport failure"
    assert_contains "$BUILD" "no error line could be parsed"
    assert_contains "$BUILD" "contains error lines"
    assert_contains "$BUILD" "justified static-MUSL diagnostic more than once"
    assert_contains "$BUILD" "empty lint output"
    assert_contains "$BUILD" "--tag-display-limit 0"
    # Broad tag suppression is a forbidden policy escape hatch.
    assert_not_contains "$BUILD" "--suppress-tags"
    assert_not_contains "$BUILD" "--suppress-tags-from-file"
}

test_workflow_installs_hash_pinned_nfpm() {
    local workflow="$ROOT/.github/workflows/release-comprehensive.yml"

    assert_contains "$workflow" "NFPM_ARCHIVE_SHA256: 0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783"
    assert_contains "$workflow" "NFPM_BINARY_SHA256: 17133a2467ffb7cec851c2d7bae0c6098d09d7ed7d3d101a9605f6a473323936"
    assert_contains "$workflow" ".github/scripts/nfpm/verify-nfpm.sh"
    assert_contains "$workflow" "name: debian-packages"
    assert_contains "$workflow" "test_server_nfpm_native.sh"
}

test_render_deb_descriptor
test_render_rpm_descriptor
test_render_rejects_gnu_target
test_build_reports_missing_nfpm_without_fallback_claim
test_deb_payload_fixture_matches_input_binary
test_build_script_expects_nfpm_deb_filename
test_build_script_fails_closed_without_source_date_epoch_fallback
test_build_script_has_cargo_deb_parity_oracle
test_build_script_has_docker_closed_fallbacks_and_lint
test_build_script_has_fail_closed_static_musl_lint_policy
test_build_script_fails_closed_on_package_architecture
test_native_gate_require_install_policy_and_receipt_ownership
test_native_gate_builds_cross_arch_elf_fixture_for_lint
test_workflow_builds_per_target_cargo_deb_parity_from_qualified_musl
test_workflow_assembles_managed_release_inventory
test_workflow_installs_hash_pinned_nfpm

echo "server nFPM tests passed"
