#!/usr/bin/env bash
#
# check-no-atty.sh — supply-chain guard: fail if the unmaintained `atty` crate
# is (re)introduced into the workspace dependency tree.
#
# Why: `atty 0.2.14` carries RUSTSEC-2024-0375 (unmaintained) and RUSTSEC-2021-0145
# (unsound unaligned read). It was removed in PR #3105 in favour of
# `std::io::IsTerminal`. Once removed, nothing structurally prevents a future
# dependency update from silently reintroducing it. This guard makes the removal
# permanent and advisory-independent (issue #3072).
#
# Semantics note: `cargo tree -i atty` exits 0 when atty IS present and non-zero
# (101) when it is absent. This script INVERTS that so the guard follows the
# intuitive convention: atty present -> exit 1 (fail), atty absent -> exit 0.
#
# Usage:
#   ./scripts/check-no-atty.sh            # uses cargo (real repo)
#   CARGO_TREE_OUTPUT="<text>" ./scripts/check-no-atty.sh --from-stdin
#                                         # synthetic fixture mode for tests
#
# Exit codes:
#   0  atty is NOT in the dependency tree (PASS)
#   1  atty IS in the dependency tree (FAIL) — prints offending consumers
#   2  usage / environment error
#
# Refs: #3072, #3105

set -euo pipefail

CRATE="atty"
ADVISORY_REFS="RUSTSEC-2024-0375 (unmaintained), RUSTSEC-2021-0145 (unsound)"

usage() {
    cat >&2 <<EOF
Usage: $0 [--from-stdin]
  --from-stdin   Read 'cargo tree -i ${CRATE}' output from the
                 CARGO_TREE_OUTPUT env var instead of invoking cargo
                 (used by the self-test suite for deterministic fixtures).
EOF
}

from_stdin=0
if [[ $# -gt 1 ]]; then
    usage
    exit 2
fi
if [[ $# -eq 1 ]]; then
    case "$1" in
        --from-stdin) from_stdin=1 ;;
        -h|--help) usage; exit 0 ;;
        *) usage; exit 2 ;;
    esac
fi

# Acquire `cargo tree -i <crate>` output. We only need to know whether the crate
# is present; `cargo tree -i` lists it on the first line when present and errors
# ("did not match any packages") when absent.
if [[ "${from_stdin}" -eq 1 ]]; then
    tree_output="${CARGO_TREE_OUTPUT:-}"
    # In stdin mode, an empty payload models the "did not match" (absent) case.
    present=0
    if printf '%s' "${tree_output}" | grep -Eq "^${CRATE} v[0-9]"; then
        present=1
    fi
else
    if ! command -v cargo >/dev/null 2>&1; then
        echo "ERROR: cargo not found on PATH (required to inspect the dependency tree)." >&2
        exit 2
    fi
    # `cargo tree -i` returns 0 when the package exists, non-zero otherwise.
    # Capture stdout (the reverse-dependency graph) for the failure message.
    if tree_output="$(cargo tree -i "${CRATE}" --workspace --all-features 2>/dev/null)"; then
        present=1
    else
        present=0
    fi
fi

if [[ "${present}" -eq 1 ]]; then
    cat >&2 <<EOF
ERROR: the unmaintained crate '${CRATE}' is present in the workspace dependency tree.

This is a supply-chain regression. '${CRATE}' was intentionally removed (PR #3105)
in favour of std::io::IsTerminal. Reintroducing it restores ${ADVISORY_REFS}.

Reverse-dependency graph (who pulls '${CRATE}' in):
---
${tree_output}
---

Fix: replace any 'atty = ...' dependency with std::io::IsTerminal (stable since
Rust 1.70; workspace MSRV is 1.91), then regenerate Cargo.lock.

Refs: #3072 (this guard), #3105 (original removal)
EOF
    exit 1
fi

echo "OK: '${CRATE}' is not present in the workspace dependency tree."
exit 0
