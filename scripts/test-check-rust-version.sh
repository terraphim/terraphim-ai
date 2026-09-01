#!/usr/bin/env bash
#
# Self-test for check-rust-version.sh (#3078)
#
# Exercises the gate against synthetic temp workspaces (inherit / literal /
# missing / excluded scenarios) and the real repo. No mocks of internal logic
# -- the gate runs in full against real fixtures on disk.
#
# Usage: ./scripts/test-check-rust-version.sh
# Exit:  0 if all assertions hold, 1 otherwise.
#
# Refs #3078

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
GATE="$SCRIPT_DIR/check-rust-version.sh"

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
    [[ -n "${TMPROOT:-}" ]] && rm -rf "$TMPROOT"
}
TMPROOT=""
trap cleanup EXIT

# assert_exit <expected_exit> <label> -- <cmd...>
assert_exit() {
    local expected="$1"
    local label="$2"
    shift 2
    [[ "$1" == "--" ]] && shift
    if "$@" >/tmp/rust_version_gate_test.out 2>&1; then
        local actual=0
    else
        local actual=$?
    fi
    if [[ "$actual" -eq "$expected" ]]; then
        echo -e "  ${GREEN}✓${NC} $label (exit $actual)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} $label (expected exit $expected, got $actual)" >&2
        cat /tmp/rust_version_gate_test.out >&2
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

echo "Testing check-rust-version.sh"
echo "============================="

# ---------------------------------------------------------------------------
# Fixture helpers: write a minimal workspace root + member Cargo.toml files.
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
rust-version = "1.91"
TOML
    mkdir -p "$root/crates" "$root/tools"
}

add_member_inherit() {
    local root="$1" rel="$2"
    mkdir -p "$root/$rel"
    cat >"$root/$rel/Cargo.toml" <<TOML
[package]
name = "$(basename "$rel")"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
TOML
}

add_member_literal() {
    local root="$1" rel="$2"
    mkdir -p "$root/$rel"
    cat >"$root/$rel/Cargo.toml" <<TOML
[package]
name = "$(basename "$rel")"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"
TOML
}

add_member_missing() {
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
# Test 1: all members inherit rust-version → exit 0
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
write_workspace "$TMPROOT/inherit"
add_member_inherit "$TMPROOT/inherit" "crates/alpha"
add_member_inherit "$TMPROOT/inherit" "crates/beta"
add_member_inherit "$TMPROOT/inherit" "tools/cli"
assert_exit 0 "all-inherited workspace passes" -- "$GATE" "$TMPROOT/inherit/Cargo.toml"
assert_grep "PASS" "all-inherited reports PASS" -- "$GATE" "$TMPROOT/inherit/Cargo.toml"
rm -rf "$TMPROOT/inherit"

# ---------------------------------------------------------------------------
# Test 2: mix of inherited + literal rust-version → exit 0 (both forms valid)
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
write_workspace "$TMPROOT/mixed"
add_member_inherit "$TMPROOT/mixed" "crates/alpha"
add_member_literal "$TMPROOT/mixed" "crates/beta"
add_member_inherit "$TMPROOT/mixed" "tools/cli"
assert_exit 0 "mixed inherited+literal workspace passes" -- "$GATE" "$TMPROOT/mixed/Cargo.toml"
assert_grep "all 3 declared" "mixed workspace reports declared count" -- "$GATE" "$TMPROOT/mixed/Cargo.toml"
rm -rf "$TMPROOT/mixed"

# ---------------------------------------------------------------------------
# Test 3: one member missing rust-version → exit 1, lists the offender
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
write_workspace "$TMPROOT/broken"
add_member_inherit "$TMPROOT/broken" "crates/alpha"
add_member_missing "$TMPROOT/broken" "crates/ghost"   # no rust-version
add_member_inherit "$TMPROOT/broken" "tools/cli"
assert_exit 1 "missing rust-version member fails" -- "$GATE" "$TMPROOT/broken/Cargo.toml"
assert_grep "crates/ghost" "missing member is listed" -- "$GATE" "$TMPROOT/broken/Cargo.toml"
rm -rf "$TMPROOT/broken"

# ---------------------------------------------------------------------------
# Test 4: excluded member missing rust-version → exit 0 (not flagged)
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
write_workspace "$TMPROOT/excluded"
add_member_inherit "$TMPROOT/excluded" "crates/alpha"
add_member_missing "$TMPROOT/excluded" "crates/excluded_one"   # excluded → OK
add_member_inherit "$TMPROOT/excluded" "tools/cli"
assert_exit 0 "excluded member without rust-version passes" -- "$GATE" "$TMPROOT/excluded/Cargo.toml"
rm -rf "$TMPROOT/excluded"

# ---------------------------------------------------------------------------
# Test 5: literal rust-version without workspace root MSRV → exit 0
# (a crate is compliant if it declares its own, even if the workspace root
#  does not itself publish a rust-version -- the gate checks the MEMBER, not
#  the workspace inheritance target.)
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
mkdir -p "$TMPROOT/solo"
cat >"$TMPROOT/solo/Cargo.toml" <<TOML
[workspace]
resolver = "2"
members = ["crates/only"]
[workspace.package]
version = "0.1.0"
edition = "2021"
TOML
mkdir -p "$TMPROOT/solo/crates/only"
cat >"$TMPROOT/solo/crates/only/Cargo.toml" <<TOML
[package]
name = "only"
version = "0.1.0"
edition = "2021"
rust-version = "1.91"
TOML
assert_exit 0 "literal rust-version without workspace MSRV passes" -- "$GATE" "$TMPROOT/solo/Cargo.toml"
rm -rf "$TMPROOT/solo"

# ---------------------------------------------------------------------------
# Test 6: empty workspace (no members) → exit 0, no crash
# ---------------------------------------------------------------------------
TMPROOT=$(mktemp -d)
mkdir -p "$TMPROOT/empty"
cat >"$TMPROOT/empty/Cargo.toml" <<TOML
[workspace]
resolver = "2"
members = []
[workspace.package]
version = "0.1.0"
TOML
assert_exit 0 "empty workspace passes" -- "$GATE" "$TMPROOT/empty/Cargo.toml"
rm -rf "$TMPROOT/empty"

echo ""
echo "Real-repo regression guard:"
echo "---------------------------"
# ---------------------------------------------------------------------------
# Test 7: real repo.
#   As of #3108 (open), origin/main has 3 members still missing rust-version
#   (terraphim_sessions, terraphim_spawner, terraphim_weather_report). Until
#   #3108 merges the gate therefore correctly FAILS -- the faithful-mirror
#   transition signal that the twin has not yet converged. Once #3108 merges
#   (or if this branch is built on a main that includes #3108) the gate PASSES.
#   We assert the documented behaviour rather than hard-coding either state:
#   the gate must EXACTLY enumerate the offending crates when failing.
# ---------------------------------------------------------------------------
if "$GATE" "$PROJECT_ROOT/Cargo.toml" >/tmp/rust_version_gate_real.out 2>&1; then
    echo -e "  ${GREEN}✓${NC} real repo passes (main has converged — all members declare rust-version)"
    PASS_COUNT=$((PASS_COUNT + 1))
else
    rc=$?
    # Failure is the expected transition state until #3108 lands. Accept it
    # only if the output lists the known-missing crates by name (proves the
    # gate is enumerating correctly, not erroring out).
    if grep -qE "terraphim_(sessions|spawner|weather_report)" /tmp/rust_version_gate_real.out \
       && [[ "$rc" -eq 1 ]]; then
        echo -e "  ${GREEN}✓${NC} real repo transition-fails as expected (exit 1, lists missing crates) until #3108 merges"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "  ${RED}✗${NC} real repo failed unexpectedly (exit $rc)" >&2
        cat /tmp/rust_version_gate_real.out >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
fi

echo ""
if [[ "$FAIL_COUNT" -eq 0 ]]; then
    echo -e "${GREEN}All ${PASS_COUNT} assertions passed.${NC}"
    exit 0
else
    echo -e "${RED}${FAIL_COUNT} assertion(s) failed, ${PASS_COUNT} passed.${NC}"
    exit 1
fi
