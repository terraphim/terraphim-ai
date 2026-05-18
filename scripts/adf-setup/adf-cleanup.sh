#!/bin/sh
# adf-cleanup.sh -- pre-start sweep of stale ADF worktrees.
#
# Invoked by systemd as `ExecStartPre=` for adf-orchestrator.service.
# Runs as root so it can reclaim worktree contents owned by
# sub-process container builds and other elevated agents.
#
# Cross-reference: WORKTREE_REVIEW_PREFIX in
# crates/terraphim_orchestrator/src/scope.rs. The literal "review-"
# below must stay in sync with that constant.

set -eu
umask 022

ADF_REPO_PATH="${ADF_REPO_PATH:-/data/projects/terraphim/terraphim-ai}"
ADF_WORKTREE_ROOT="${ADF_WORKTREE_ROOT:-${ADF_REPO_PATH}/.worktrees}"
ADF_AGENT_TMP_ROOT="${ADF_AGENT_TMP_ROOT:-/tmp/adf-worktrees}"

swept=0
failed=0

sweep_one() {
    target="$1"
    if [ ! -e "$target" ]; then
        return 0
    fi
    if git -C "$ADF_REPO_PATH" worktree remove --force "$target" >/dev/null 2>&1; then
        swept=$((swept + 1))
        return 0
    fi
    # Fallback: recursive removal of the worktree directory tree.
    if /bin/rm -rf -- "$target"; then
        swept=$((swept + 1))
        return 0
    fi
    failed=$((failed + 1))
    return 0
}

# 1. Compound review residue under ${ADF_WORKTREE_ROOT}/review-*.
if [ -d "$ADF_WORKTREE_ROOT" ]; then
    for entry in "$ADF_WORKTREE_ROOT"/review-*; do
        [ -e "$entry" ] || continue
        sweep_one "$entry"
    done
fi

# 2. Per-agent residue under /tmp/adf-worktrees/*.
if [ -d "$ADF_AGENT_TMP_ROOT" ]; then
    for entry in "$ADF_AGENT_TMP_ROOT"/*; do
        [ -e "$entry" ] || continue
        sweep_one "$entry"
    done
fi

# 3. Reconcile git's admin registry. Failure here is not fatal.
git -C "$ADF_REPO_PATH" worktree prune --verbose 2>&1 || true

printf 'adf-cleanup: swept=%d failed=%d repo=%s\n' \
    "$swept" "$failed" "$ADF_REPO_PATH"

exit 0
