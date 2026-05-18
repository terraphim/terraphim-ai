#!/bin/sh
# test_adf_cleanup.sh -- POSIX shell driver for adf-cleanup.sh.
#
# Seeds three review-* worktrees plus a keep-me/ directory in a
# disposable git repo, runs the sweep script, and asserts that
# review-* entries are removed while keep-me/ is preserved. A
# second run verifies idempotency.

set -eu

THIS_DIR="$(cd "$(dirname "$0")" && pwd)"
CLEANUP_SH="${THIS_DIR}/../adf-cleanup.sh"

TMP="$(mktemp -d)"
trap '/bin/rm -rf "$TMP"' EXIT

REPO="${TMP}/repo"
WT_ROOT="${REPO}/.worktrees"
mkdir -p "$REPO"
git -C "$REPO" -c init.defaultBranch=main init -q
git -C "$REPO" -c user.email=test@example.com -c user.name=Test \
    commit --allow-empty -m "seed" -q

mkdir -p "$WT_ROOT/keep-me"

for i in 1 2 3; do
    git -C "$REPO" worktree add -q "${WT_ROOT}/review-test-${i}" HEAD
done

[ -d "${WT_ROOT}/review-test-1" ] || { echo "setup failed"; exit 1; }

ADF_REPO_PATH="$REPO" \
ADF_WORKTREE_ROOT="$WT_ROOT" \
ADF_AGENT_TMP_ROOT="${TMP}/agent-tmp-absent" \
    "$CLEANUP_SH"

for i in 1 2 3; do
    if [ -e "${WT_ROOT}/review-test-${i}" ]; then
        echo "FAIL: review-test-${i} still present"
        exit 1
    fi
done

[ -d "${WT_ROOT}/keep-me" ] || { echo "FAIL: keep-me removed"; exit 1; }

# Idempotency: second run.
ADF_REPO_PATH="$REPO" \
ADF_WORKTREE_ROOT="$WT_ROOT" \
ADF_AGENT_TMP_ROOT="${TMP}/agent-tmp-absent" \
    "$CLEANUP_SH"

echo "PASS: test_adf_cleanup"
