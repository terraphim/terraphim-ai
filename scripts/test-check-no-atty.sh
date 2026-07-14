#!/usr/bin/env bash
#
# test-check-no-atty.sh — self-test for scripts/check-no-atty.sh
#
# Proves the guard behaves correctly on:
#   1. atty present  -> guard FAILS (exit 1) and lists the offending crate
#   2. atty absent   -> guard PASSES (exit 0)
#   3. real-repo regression: the guard script itself exists, is executable,
#      and is syntactically valid bash
#
# Uses synthetic payloads via the guard's --from-stdin mode (deterministic,
# no network, no cargo invocation) — no mocks of cargo internals, only
# fixture text modelling real `cargo tree -i atty` output.
#
# Run: ./scripts/test-check-no-atty.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="${SCRIPT_DIR}/check-no-atty.sh"

pass=0
fail=0
assertions=0

ok() {
    pass=$((pass + 1))
    assertions=$((assertions + 1))
    echo "  PASS: $1"
}

not_ok() {
    fail=$((fail + 1))
    assertions=$((assertions + 1))
    echo "  FAIL: $1"
}

echo "## test-check-no-atty.sh — self-test for the atty reintroduction guard"
echo

# ---------------------------------------------------------------------------
# Pre-flight: the guard must exist, be executable, and parse as valid bash.
# ---------------------------------------------------------------------------
echo "Test 0: guard exists, is executable, and is valid bash syntax"
if [[ -f "${GUARD}" && -x "${GUARD}" ]]; then
    ok "guard file exists and is executable"
else
    not_ok "guard missing or not executable (${GUARD})"
fi
if bash -n "${GUARD}" 2>/dev/null; then
    ok "guard passes 'bash -n' syntax check"
else
    not_ok "guard has bash syntax errors"
fi
echo

# ---------------------------------------------------------------------------
# Test 1: atty present -> guard FAILS (exit 1) and names the offending crate.
# This is the AC: "Given a PR that adds atty back ... Then the step fails."
# ---------------------------------------------------------------------------
echo "Test 1: atty present in dependency tree -> guard fails (exit 1)"
present_payload="$(cat <<'EOF'
atty v0.2.14
└── terraphim_rlm v1.21.0 (/repo/crates/terraphim_rlm)
EOF
)"
out="$(CARGO_TREE_OUTPUT="${present_payload}" "${GUARD}" --from-stdin 2>&1)" && rc=0 || rc=$?
if [[ "${rc}" -eq 1 ]]; then
    ok "guard exits 1 when atty is present"
else
    not_ok "expected exit 1, got ${rc}"
fi
if printf '%s' "${out}" | grep -Eq "atty.*present"; then
    ok "failure message states atty is present"
else
    not_ok "failure message does not mention atty presence"
fi
if printf '%s' "${out}" | grep -Eq "terraphim_rlm"; then
    ok "failure message lists the offending consumer (terraphim_rlm)"
else
    not_ok "failure message omits the offending consumer"
fi
echo

# ---------------------------------------------------------------------------
# Test 2: atty absent -> guard PASSES (exit 0).
# This is the AC: "when no atty is present, the step succeeds."
# ---------------------------------------------------------------------------
echo "Test 2: atty absent from dependency tree -> guard passes (exit 0)"
absent_payload=""
out="$(CARGO_TREE_OUTPUT="${absent_payload}" "${GUARD}" --from-stdin 2>&1)" && rc=0 || rc=$?
if [[ "${rc}" -eq 0 ]]; then
    ok "guard exits 0 when atty is absent"
else
    not_ok "expected exit 0, got ${rc}"
fi
if printf '%s' "${out}" | grep -Eq "OK: 'atty' is not present"; then
    ok "success message confirms atty is absent"
else
    not_ok "success message malformed (got: ${out})"
fi
echo

# ---------------------------------------------------------------------------
# Test 3: a payload that mentions atty only as a comment (not a real entry)
# must NOT trigger a false positive. Models robustness against noise.
# ---------------------------------------------------------------------------
echo "Test 3: noise payload (atty mentioned but not a real tree entry) -> passes"
noise_payload="$(cat <<'EOF'
# note: atty was removed in PR #3105; do not reintroduce.
(No packages match.)
EOF
)"
out="$(CARGO_TREE_OUTPUT="${noise_payload}" "${GUARD}" --from-stdin 2>&1)" && rc=0 || rc=$?
if [[ "${rc}" -eq 0 ]]; then
    ok "guard does not false-positive on comment-style noise"
else
    not_ok "guard false-positives on noise (got exit ${rc})"
fi
echo

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo "## Summary: ${pass} passed, ${fail} failed (${assertions} assertions)"
if [[ "${fail}" -gt 0 ]]; then
    exit 1
fi
exit 0
