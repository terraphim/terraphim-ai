#!/usr/bin/env bash
#
# Self-test for check-crate-manifests.sh (#3062)
#
# Exercises the gate against synthetic temp workspaces (pass / missing /
# excluded scenarios) and the real repo. No mocks of internal logic — the
# gate runs in full against real fixtures on disk.
#
# Usage: ./scripts/test-check-crate-manifests.sh
# Exit:  0 if all assertions hold, 1 otherwise.
#
# Refs #3062

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GATE="$SCRIPT_DIR/check-crate-manifests.sh"

if [[ ! -x "$GATE" ]]; then
    chmod +x "$GATE" 2>/dev/null || true
fi
if [[ ! -r "$GATE" ]]; then
    echo "ERROR: gate script not found: $GATE" >&2
    exit 2
fi

if [[ -t 1 ]]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    NC='\033[0m'
else
    GREEN=''; RED=''; NC=''
fi

PASS_COUNT=0
FAIL_COUNT=0

cleanup() {
    [[ -n "${TMPDIR:-}" ]] && rm -rf "$TMPDIR"
}
TMPDIR=""
trap cleanup EXIT

# assert_exit <expected_exit> <label> -- <cmd...>
assert_exit() {
    local expected="$1"
    local label="$2"
    shift 2
    [[ "$1" == "--" ]] && shift
    if "$@" >/tmp/manifest_gate_test.out 2>&1; then
        local actual=0
    else
        local actual=$?
    fi
    if [[ "$actual" -eq "$expected" ]]; then
        echo -e "  ${GREEN}✓${NC} $label (exit $actual)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} $label (expected exit $expected, got $actual)" >&2
        cat /tmp/manifest_gate_test.out >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

# assert_grep <pattern> <label> -- run the gate and grep output for pattern.
assert_grep() {
    local pattern="$1"
    local label="$2"
    shift 2
    [[ "$1" == "--" ]] && shift
    local out
    if out=$("$@" 2>&1); then :; fi
    if grep -qE "$pattern" <<<"$out"; then
        echo -e "  ${GREEN}✓${NC} $label"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} $label (pattern not found: $pattern)" >&2
        echo "--- output ---" >&2
        echo "$out" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

echo "Testing check-crate-manifests.sh"
echo "================================"

# ---------------------------------------------------------------------------
# Fixture helper: write a minimal workspace root + member dirs.
# ---------------------------------------------------------------------------
write_workspace() {
    local root="$1"
    mkdir -p "$root"
    cat >"$root/Cargo.toml" <<TOML
[workspace]
resolver = "2"
members = [
    "crates/*",
    "tools/cli",
]
exclude = [
    "crates/excluded_one",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
TOML
    mkdir -p "$root/crates" "$root/tools"
}

add_member() {
    local root="$1" rel="$2"
    mkdir -p "$root/$rel"
    cat >"$root/$rel/Cargo.toml" <<TOML
[package]
name = "$(basename "$rel")"
version = "0.1.0"
edition = "2021"
TOML
}

# ---------------------------------------------------------------------------
# Test 1: valid workspace → exit 0
# ---------------------------------------------------------------------------
TMPDIR=$(mktemp -d)
write_workspace "$TMPDIR/valid"
add_member "$TMPDIR/valid" "crates/alpha"
add_member "$TMPDIR/valid" "crates/beta"
add_member "$TMPDIR/valid" "tools/cli"
assert_exit 0 "valid workspace passes" -- "$GATE" "$TMPDIR/valid/Cargo.toml"
assert_grep "all 3 effective" "valid workspace reports count" -- "$GATE" "$TMPDIR/valid/Cargo.toml"

# ---------------------------------------------------------------------------
# Test 2: member dir without Cargo.toml → exit 1, lists the path
# ---------------------------------------------------------------------------
TMPDIR2=$(mktemp -d)
write_workspace "$TMPDIR2/broken"
add_member "$TMPDIR2/broken" "crates/alpha"
mkdir -p "$TMPDIR2/broken/crates/ghost" # matches glob but no Cargo.toml
assert_exit 1 "missing-manifest member fails" -- "$GATE" "$TMPDIR2/broken/Cargo.toml"
assert_grep "crates/ghost" "missing path is listed" -- "$GATE" "$TMPDIR2/broken/Cargo.toml"
rm -rf "$TMPDIR2"

# ---------------------------------------------------------------------------
# Test 3: excluded dir without Cargo.toml → exit 0 (not flagged)
# ---------------------------------------------------------------------------
TMPDIR3=$(mktemp -d)
write_workspace "$TMPDIR3/excluded"
add_member "$TMPDIR3/excluded" "crates/alpha"
mkdir -p "$TMPDIR3/excluded/crates/excluded_one" # excluded → no Cargo.toml OK
assert_exit 0 "excluded dir without manifest passes" -- "$GATE" "$TMPDIR3/excluded/Cargo.toml"
rm -rf "$TMPDIR3"

# ---------------------------------------------------------------------------
# Test 4: real repo → exit 0 (regression guard against #3030 recurrence)
# ---------------------------------------------------------------------------
assert_exit 0 "real repo workspace passes" -- "$GATE" "$PROJECT_ROOT/Cargo.toml"

echo ""
if [[ "$FAIL_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}All ${PASS_COUNT} assertions passed.${NC}"
    exit 0
else
    echo -e "${RED}${FAIL_COUNT} assertion(s) failed, ${PASS_COUNT} passed.${NC}"
    exit 1
fi
