#!/usr/bin/env bash
# test-ci-runner-health-check.sh -- contract tests for scripts/ci/runner-health-check.sh
#
# Asserts that the CI health check gates on the source-controlled host-only pool
# contract (Plan Task 4, Refs #3222) *before* it needs Gitea credentials, and
# that the pre-existing rustup toolchain permission repair (Refs #2463) still
# runs on the healthy path.
#
# Hermetic: no systemd, no Gitea, no network. `curl` is injected by prepending a
# stub to PATH; the pool gate is injected through RUNNER_POOL_HEALTH_SCRIPT.

set -uo pipefail

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
CI_SH="${THIS_DIR}/../ci/runner-health-check.sh"

[ -f "$CI_SH" ] || { echo "FAIL: missing $CI_SH" >&2; exit 1; }

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

BIN="${TMP}/bin"
mkdir -p "$BIN"

# curl stub: serves one online, freshly-seen runner for the admin API and
# swallows any issue-comment POST. Never touches the network.
cat > "${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
case "$*" in
    *"/api/v1/admin/runners"*)
        printf '[{"id":6,"name":"terraphim-native-1","status":"online","last_seen":%s}]\n' "$(date +%s)"
        exit 0
        ;;
esac
exit 0
EOF
chmod +x "${BIN}/curl"

PASS_GATE="${TMP}/pool-pass.sh"
FAIL_GATE="${TMP}/pool-fail.sh"
printf '#!/usr/bin/env bash\necho "OK: pool healthy"\nexit 0\n' > "$PASS_GATE"
printf '#!/usr/bin/env bash\necho "FAIL: terraphim-gitea-runner-3.service RUNNER_VM_MODE=firecracker" >&2\nexit 1\n' > "$FAIL_GATE"
chmod +x "$PASS_GATE" "$FAIL_GATE"

FAILURES=0
CASE=""
fail() { echo "FAIL [${CASE}]: $*" >&2; FAILURES=$((FAILURES + 1)); }

OUT=""
run_ci() {
    # `env` (not an assignment prefix) so the per-case VAR=value words in "$@"
    # are applied after expansion.
    OUT="$(env "PATH=${BIN}:${PATH}" "$@" bash "$CI_SH" 2>&1)"
}

# --- case 1: a failing host pool fails the CI check, before credentials ------
CASE="pool-failure-gates-before-token"
run_ci RUNNER_POOL_HEALTH_SCRIPT="$FAIL_GATE" HOME="$TMP" GITEA_TOKEN="" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "a firecracker pool member must fail the CI health check"
case "$OUT" in
    *firecracker*) ;;
    *) fail "the pool gate's diagnosis must be surfaced; got: $OUT" ;;
esac
case "$OUT" in
    *"GITEA_TOKEN not set"*) fail "the pool gate must run before the token requirement; got: $OUT" ;;
esac

# --- case 2: a healthy pool lets the check proceed to the Gitea stage --------
CASE="healthy-pool-proceeds"
run_ci RUNNER_POOL_HEALTH_SCRIPT="$PASS_GATE" HOME="$TMP" GITEA_TOKEN="" && rc=0 || rc=$?
[ "$rc" -ne 0 ] || fail "expected the missing-token failure once the pool gate passes"
case "$OUT" in
    *"GITEA_TOKEN not set"*) ;;
    *) fail "expected to reach the Gitea stage; got: $OUT" ;;
esac

# --- case 3: rustup permission repair is preserved on the healthy path -------
# Refs #2463: a toolchain bin/* without +x must be detected and repaired.
CASE="rustup-permission-repair-preserved"
RUSTUP="${TMP}/rustup"
mkdir -p "${RUSTUP}/toolchains/stable-x86_64-unknown-linux-gnu/bin"
BROKEN="${RUSTUP}/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo"
printf '#!/bin/sh\n' > "$BROKEN"
chmod 644 "$BROKEN"
FIXER="${TMP}/fix-perms.sh"
cat > "$FIXER" <<EOF
#!/usr/bin/env bash
chmod +x "$BROKEN"
echo "fixer ran"
EOF
chmod +x "$FIXER"

run_ci RUNNER_POOL_HEALTH_SCRIPT="$PASS_GATE" HOME="$TMP" GITEA_TOKEN="fake-token" \
    RUSTUP_HOME="$RUSTUP" FIX_RUST_PERMS_SCRIPT="$FIXER" && rc=0 || rc=$?
[ "$rc" -eq 0 ] || fail "healthy pool + online runner + repaired perms must exit 0; got $rc; output: $OUT"
[ -x "$BROKEN" ] || fail "the rustup permission repair did not run"
case "$OUT" in
    *"rust toolchain permissions repaired"*) ;;
    *) fail "expected the rustup repair message; got: $OUT" ;;
esac

# --- case 4: the pool gate can be skipped for off-host invocations -----------
CASE="pool-gate-skippable"
run_ci SKIP_POOL_CHECK=1 RUNNER_POOL_HEALTH_SCRIPT="$FAIL_GATE" HOME="$TMP" GITEA_TOKEN="" && rc=0 || rc=$?
case "$OUT" in
    *"GITEA_TOKEN not set"*) ;;
    *) fail "SKIP_POOL_CHECK=1 must bypass the pool gate; got: $OUT" ;;
esac

if [ "$FAILURES" -eq 0 ]; then
    echo "PASS: CI runner health check contract (4 cases)"
    exit 0
fi
echo "FAILURES: $FAILURES" >&2
exit 1
