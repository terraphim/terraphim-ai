#!/usr/bin/env bash
# Behavioral regression for the fail-closed static-MUSL lint policy in
# build-server-packages.sh.
#
# 1. Production probe: an actually fully-static ELF is packaged through the
#    production build-server-packages.sh pipeline (real lintian/rpmlint via
#    host tools or the pinned Docker images) and the DEB+RPM production
#    path must pass while emitting exactly the justified static diagnostic
#    for terraphim-server at usr/bin/terraphim_server, with the full raw
#    lint output preserved as evidence.
# 2. Mutation negatives: stub lintian/rpmlint injected through PATH (the
#    same production lint_deb/lint_rpm code path resolves host tools via
#    PATH) prove that extra or wrong error lines, wrong packages/paths,
#    malformed variants, duplicates, inconsistent exit statuses and
#    tool/install/transport failures all fail closed.
# 3. Dynamic fixture gates stay meaningful: a clean (zero error line) tool
#    result still passes, which is what the dynamically linked fixtures of
#    the native gate rely on.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
BUILD="$ROOT/.github/scripts/nfpm/build-server-packages.sh"
RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
NFPM_BIN="${NFPM_BIN:-nfpm}"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-server-nfpm-static-lint.XXXXXX")"
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

docker_available() {
    command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

# An actually fully-static ELF for the qualified MUSL payload, in the
# production --version contract.
make_static_binary() {
    local path="$1"
    local cc_bin=""
    local candidate
    for candidate in "${CC:-}" cc gcc clang; do
        if [[ -n "$candidate" ]] && command -v "$candidate" >/dev/null 2>&1; then
            cc_bin="$candidate"
            break
        fi
    done
    [[ -n "$cc_bin" ]] || return 1
    local src="$path.fixture.c"
    mkdir -p "$(dirname "$path")"
    cat > "$src" <<'EOF'
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "--version") == 0) {
        printf("terraphim_server %s\n", VERSION);
        return 0;
    }
    fprintf(stderr, "usage: terraphim_server --version\n");
    return 64;
}
EOF
    "$cc_bin" -static -O2 -s -DVERSION="\"9.8.7\"" -o "$path" "$src" || return 1
    rm -f "$src"
    chmod 0755 "$path"
    file "$path" | grep -q 'statically linked' || return 1
}

if ! make_static_binary "$TMP/qualified/terraphim_server"; then
    echo "SKIP: no static-linking C compiler available (CC/cc/gcc/clang)" >&2
    exit 0
fi

build_fixture_packages() {
    local bin="$TMP/qualified/terraphim_server"
    "$RENDER" --format deb --version 9.8.7 --target x86_64-unknown-linux-musl \
        --binary "$bin" --output "$TMP/server-deb.yaml" >/dev/null
    "$RENDER" --format rpm --version 9.8.7 --target x86_64-unknown-linux-musl \
        --binary "$bin" --output "$TMP/server-rpm.yaml" >/dev/null
    "$NFPM_BIN" pkg --packager deb --config "$TMP/server-deb.yaml" --target "$TMP" >/dev/null
    "$NFPM_BIN" pkg --packager rpm --config "$TMP/server-rpm.yaml" --target "$TMP" >/dev/null
    [[ -f "$TMP/terraphim-server_9.8.7-1_amd64.deb" ]] || fail "fixture DEB was not built"
    [[ -f "$TMP/terraphim-server-9.8.7-1.x86_64.rpm" ]] || fail "fixture RPM was not built"
}

# ---------------------------------------------------------------------------
# 1. Production probe: full pipeline with real lintian/rpmlint (host tools
#    or Docker). The exact justified diagnostics must be accepted and the
#    evidence preserved in the output.
# ---------------------------------------------------------------------------
test_production_static_elf_probe_passes_exact_justified_diagnostics() {
    if ! command -v lintian >/dev/null 2>&1 && ! docker_available; then
        echo "SKIP: production static probe needs host lintian or Docker (rpmlint likewise)" >&2
        return 0
    fi

    local out="$TMP/prod-out"
    local log="$TMP/prod-run.log"
    if "$BUILD" \
        --version 9.8.7 \
        --target x86_64-unknown-linux-musl \
        --binary "$TMP/qualified/terraphim_server" \
        --out-dir "$out" \
        --nfpm "$NFPM_BIN" >"$log" 2>&1; then
        :
    else
        fail "production build-server-packages.sh rejected a fully-static MUSL payload: $(tail -20 "$log")"
    fi

    # Evidence: the raw justified diagnostics must be present in the output.
    grep -Fq 'E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server]' "$log" ||
        fail "lintian evidence missing the justified static diagnostic: $(cat "$log")"
    grep -Fq 'terraphim-server.x86_64: E: statically-linked-binary /usr/bin/terraphim_server' "$log" ||
        fail "rpmlint evidence missing the justified static diagnostic: $(cat "$log")"
    grep -Fq 'lint policy satisfied: lintian accepted' "$log" ||
        fail "missing lintian policy verdict: $(cat "$log")"
    grep -Fq 'lint policy satisfied: rpmlint accepted' "$log" ||
        fail "missing rpmlint policy verdict: $(cat "$log")"
    grep -Fq 'package payload ok x86_64-unknown-linux-musl' "$log" ||
        fail "production probe did not complete the payload qualification"
    [[ -f "$out/terraphim-server_9.8.7-1_amd64.deb" ]] || fail "probe DEB missing"
    [[ -f "$out/terraphim-server-9.8.7-1.x86_64.rpm" ]] || fail "probe RPM missing"
    [[ -f "$out/terraphim-server-9.8.7-x86_64-unknown-linux-musl.package-sha256sums.txt" ]] ||
        fail "probe package checksum manifest missing"
}

# ---------------------------------------------------------------------------
# 2/3. Stub-driven mutation negatives and clean-dynamic acceptance through
# the production lint functions (PATH-injected stub tools are resolved by
# the same `command -v` the production host path uses).
# ---------------------------------------------------------------------------
STUB_BIN="$TMP/stub-bin"
mkdir -p "$STUB_BIN"

write_stub() {
    # write_stub <tool> <exit-code> <output-file>
    local tool="$1" rc="$2" out="$3"
    # shellcheck disable=SC2016
    printf '#!/usr/bin/env bash\ncat %q\nexit %s\n' "$out" "$rc" > "$STUB_BIN/$tool"
    chmod 0755 "$STUB_BIN/$tool"
}

# Run a production lint function in a sourced subshell with the stub tools
# on PATH; captures the exit status without tripping this test's set -e.
run_lint() {
    local fn="$1" pkg="$2"
    (
        export PATH="$STUB_BIN:$PATH"
        export WORK_DIR="$TMP/work"
        export RPM_ARCH=x86_64
        export DEB_ARCH=amd64
        TERRAPHIM_BUILD_SERVER_PACKAGES_SOURCED=1 source "$BUILD"
        "$fn" "$pkg"
    ) >"$TMP/lint.stdout" 2>"$TMP/lint.stderr"
}

expect_lint_pass() {
    local fn="$1" pkg="$2" scenario="$3"
    if ! run_lint "$fn" "$pkg"; then
        fail "lint scenario '$scenario' should pass: $(cat "$TMP/lint.stderr")"
    fi
    grep -Fq 'lint policy satisfied' "$TMP/lint.stdout" ||
        fail "lint scenario '$scenario' passed without a policy verdict: $(cat "$TMP/lint.stdout" "$TMP/lint.stderr")"
}

expect_lint_fail() {
    local fn="$1" pkg="$2" scenario="$3" diagnostic="$4"
    if run_lint "$fn" "$pkg"; then
        fail "lint scenario '$scenario' must fail closed, but it passed: $(cat "$TMP/lint.stdout")"
    fi
    grep -Fq "$diagnostic" "$TMP/lint.stdout" "$TMP/lint.stderr" ||
        fail "lint scenario '$scenario' missing diagnostic '$diagnostic': $(cat "$TMP/lint.stdout" "$TMP/lint.stderr")"
}

LINTIAN_JUSTIFIED='E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server]'
RPMLINT_JUSTIFIED='terraphim-server.x86_64: E: statically-linked-binary /usr/bin/terraphim_server'

make_lintian_output() { printf '%s\n' "$@" > "$TMP/lintian-out"; }
make_rpmlint_output() { printf '%s\n' "$@" > "$TMP/rpmlint-out"; }

DEB_PKG="$TMP/terraphim-server_9.8.7-1_amd64.deb"
RPM_PKG="$TMP/terraphim-server-9.8.7-1.x86_64.rpm"

build_fixture_packages
mkdir -p "$TMP/work"

# Canonical stub results mirroring the real tool observations pinned from
# the production probe (lintian 2.116 exits 2 on errors, rpmlint 2.8 exits
# 64; both print the exact justified line plus nonfatal warnings).
test_canonical_justified_static_passes() {
    make_lintian_output \
        'N: running with root privileges is not recommended!' \
        "$LINTIAN_JUSTIFIED" \
        'W: terraphim-server: initial-upload-closes-no-bugs [usr/share/doc/terraphim-server/changelog.Debian.gz:1]'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_pass lint_deb "$DEB_PKG" "lintian exact justified static diagnostic"
    grep -Fq 'with 1 justified static-MUSL diagnostic(s)' "$TMP/lint.stdout" ||
        fail "lintian verdict must count exactly one justified diagnostic"

    make_rpmlint_output \
        '============================ rpmlint session starts ============================' \
        "$RPMLINT_JUSTIFIED" \
        'terraphim-server.x86_64: W: position-independent-executable-suggested /usr/bin/terraphim_server' \
        ' 1 packages and 0 specfiles checked; 1 errors, 1 warnings, 0 filtered, 1 badness'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_pass lint_rpm "$RPM_PKG" "rpmlint exact justified static diagnostic"
    grep -Fq 'with 1 justified static-MUSL diagnostic(s)' "$TMP/lint.stdout" ||
        fail "rpmlint verdict must count exactly one justified diagnostic"
}

# Dynamic fixtures (the native gate) rely on a genuinely clean result: zero
# error lines and a clean exit must keep passing.
test_clean_dynamic_result_still_passes() {
    make_lintian_output \
        'W: terraphim-server: initial-upload-closes-no-bugs [usr/share/doc/terraphim-server/changelog.Debian.gz:1]'
    write_stub lintian 0 "$TMP/lintian-out"
    expect_lint_pass lint_deb "$DEB_PKG" "lintian clean dynamic result"
    grep -Fq 'with 0 justified static-MUSL diagnostic(s)' "$TMP/lint.stdout" ||
        fail "clean lintian verdict must count zero justified diagnostics"

    make_rpmlint_output \
        '============================ rpmlint session starts ============================' \
        'terraphim-server.x86_64: W: position-independent-executable-suggested /usr/bin/terraphim_server' \
        ' 1 packages and 0 specfiles checked; 0 errors, 1 warnings, 0 filtered, 0 badness'
    write_stub rpmlint 0 "$TMP/rpmlint-out"
    expect_lint_pass lint_rpm "$RPM_PKG" "rpmlint clean dynamic result"
}

test_injected_extra_error_fails() {
    make_lintian_output "$LINTIAN_JUSTIFIED" \
        'E: terraphim-server: another-real-error [usr/bin/terraphim_server]'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian extra injected error" \
        'unjustified error'

    make_rpmlint_output "$RPMLINT_JUSTIFIED" \
        'terraphim-server.x86_64: E: no-documentation'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint extra injected error" \
        'unjustified error'
}

test_wrong_path_variant_fails() {
    make_lintian_output \
        'E: terraphim-server: statically-linked-binary [usr/sbin/terraphim_server]'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian wrong path" 'unjustified error'

    make_rpmlint_output \
        'terraphim-server.x86_64: E: statically-linked-binary /usr/sbin/terraphim_server'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint wrong path" 'unjustified error'
}

test_wrong_package_variant_fails() {
    make_lintian_output \
        'E: terraphim-other: statically-linked-binary [usr/bin/terraphim_server]'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian wrong package" 'unjustified error'

    make_rpmlint_output \
        'terraphim-other.x86_64: E: statically-linked-binary /usr/bin/terraphim_server'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint wrong package" 'unjustified error'

    # Wrong arch in the rpmlint N-V-R.A prefix is a wrong package identity.
    make_rpmlint_output \
        'terraphim-server.aarch64: E: statically-linked-binary /usr/bin/terraphim_server'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint wrong arch identity" 'unjustified error'
}

test_malformed_variant_fails() {
    # Parentheses instead of brackets (a plausible lintian formatting
    # change) must not be allowlisted by accident.
    make_lintian_output \
        'E: terraphim-server: statically-linked-binary (usr/bin/terraphim_server)'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian malformed brackets" 'unjustified error'

    # Extra tag argument/spacing variants are different diagnostics.
    make_lintian_output \
        'E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server] extra'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian trailing junk" 'unjustified error'

    make_rpmlint_output \
        'terraphim-server.x86_64: E: statically-linked-binary  /usr/bin/terraphim_server'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint malformed spacing" 'unjustified error'
}

test_duplicate_justified_line_fails() {
    make_lintian_output "$LINTIAN_JUSTIFIED" "$LINTIAN_JUSTIFIED"
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian duplicate justified" \
        'justified static-MUSL diagnostic more than once'

    make_rpmlint_output "$RPMLINT_JUSTIFIED" "$RPMLINT_JUSTIFIED"
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint duplicate justified" \
        'justified static-MUSL diagnostic more than once'
}

test_tool_failure_exit_fails() {
    # lintian usage/internal failures exit 25; 0 and 2 are the only
    # legitimate policy exits.
    make_lintian_output 'lintian: internal error'
    write_stub lintian 25 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian tool failure exit" \
        'tool/install/transport failure'

    # rpmlint exits 2 on usage/config errors; 0/64/65 are the only
    # legitimate policy exits.
    make_rpmlint_output 'rpmlint: error: no such file'
    write_stub rpmlint 2 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint tool failure exit" \
        'tool/install/transport failure'
}

test_inconsistent_status_fails() {
    # errors-exit without any parseable error line (lintian also exits 2
    # for an unreadable package, so this is the transport-failure guard).
    make_lintian_output \
        'W: terraphim-server: initial-upload-closes-no-bugs [usr/share/doc/terraphim-server/changelog.Debian.gz:1]'
    write_stub lintian 2 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian errors-exit without error line" \
        'no error line could be parsed'

    make_rpmlint_output \
        '============================ rpmlint session starts ============================' \
        ' 1 packages and 0 specfiles checked; 0 errors, 0 warnings, 0 filtered, 0 badness'
    write_stub rpmlint 64 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint errors-exit without error line" \
        'no error line could be parsed'

    # clean-exit with an error line present is equally inconsistent.
    make_lintian_output "$LINTIAN_JUSTIFIED"
    write_stub lintian 0 "$TMP/lintian-out"
    expect_lint_fail lint_deb "$DEB_PKG" "lintian clean-exit with error line" \
        'contains error lines'

    make_rpmlint_output "$RPMLINT_JUSTIFIED"
    write_stub rpmlint 0 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint clean-exit with error line" \
        'contains error lines'
}

test_empty_rpmlint_output_fails() {
    : > "$TMP/rpmlint-out"
    write_stub rpmlint 0 "$TMP/rpmlint-out"
    expect_lint_fail lint_rpm "$RPM_PKG" "rpmlint empty output" \
        'empty lint output'
}

# Full production script with an injected hostile lintian on PATH must fail
# closed (verify_deb lint runs before any RPM verification).
test_production_run_with_injected_extra_error_fails() {
    make_lintian_output "$LINTIAN_JUSTIFIED" \
        'E: terraphim-server: injected-hostile-error [usr/bin/terraphim_server]'
    write_stub lintian 2 "$TMP/lintian-out"

    local out="$TMP/hostile-out"
    if (
        export PATH="$STUB_BIN:$PATH"
        "$BUILD" \
            --version 9.8.7 \
            --target x86_64-unknown-linux-musl \
            --binary "$TMP/qualified/terraphim_server" \
            --out-dir "$out" \
            --nfpm "$NFPM_BIN"
    ) >"$TMP/hostile.log" 2>&1; then
        fail "production pipeline accepted an injected extra lint error"
    fi
    grep -Fq 'unjustified error' "$TMP/hostile.log" ||
        fail "hostile run missing unjustified-error diagnostic: $(cat "$TMP/hostile.log")"
    [[ ! -f "$out/terraphim-server-9.8.7-x86_64-unknown-linux-musl.package-sha256sums.txt" ]] ||
        fail "hostile run must not publish package checksum manifests"
}

# Full production script with a hostile rpmlint on PATH must fail closed
# (requires host rpm tooling or Docker for the RPM payload verification
# that precedes the RPM lint stage).
test_production_run_with_injected_wrong_path_rpm_error_fails() {
    if ! command -v rpm2cpio >/dev/null 2>&1 || ! command -v rpm >/dev/null 2>&1 || ! command -v cpio >/dev/null 2>&1; then
        if ! docker_available; then
            echo "SKIP: hostile RPM production run needs host rpm tooling or Docker" >&2
            return 0
        fi
    fi

    make_lintian_output "$LINTIAN_JUSTIFIED"
    write_stub lintian 2 "$TMP/lintian-out"
    make_rpmlint_output \
        'terraphim-server.x86_64: E: statically-linked-binary /usr/sbin/terraphim_server'
    write_stub rpmlint 64 "$TMP/rpmlint-out"

    local out="$TMP/hostile-rpm-out"
    if (
        export PATH="$STUB_BIN:$PATH"
        "$BUILD" \
            --version 9.8.7 \
            --target x86_64-unknown-linux-musl \
            --binary "$TMP/qualified/terraphim_server" \
            --out-dir "$out" \
            --nfpm "$NFPM_BIN"
    ) >"$TMP/hostile-rpm.log" 2>&1; then
        fail "production pipeline accepted an injected wrong-path RPM lint error"
    fi
    grep -Fq 'unjustified error' "$TMP/hostile-rpm.log" ||
        fail "hostile RPM run missing unjustified-error diagnostic: $(cat "$TMP/hostile-rpm.log")"
}

test_production_static_elf_probe_passes_exact_justified_diagnostics
test_canonical_justified_static_passes
test_clean_dynamic_result_still_passes
test_injected_extra_error_fails
test_wrong_path_variant_fails
test_wrong_package_variant_fails
test_malformed_variant_fails
test_duplicate_justified_line_fails
test_tool_failure_exit_fails
test_inconsistent_status_fails
test_empty_rpmlint_output_fails
test_production_run_with_injected_extra_error_fails
test_production_run_with_injected_wrong_path_rpm_error_fails

echo "server nFPM static-MUSL lint policy tests passed"
