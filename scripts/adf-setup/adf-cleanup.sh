#!/bin/sh
# adf-cleanup.sh -- pre-start sweep of stale ADF worktrees.
#
# Invoked by systemd as `ExecStartPre=` for adf-orchestrator.service.
# Runs as root so it can reclaim worktree contents owned by
# sub-process container builds and other elevated agents.
#
# SAFETY: Only deletes directories that contain a valid
# .adf-worktree-manifest.json file matching this repository.
# Unknown directories are preserved regardless of naming convention.
#
# Cross-reference: WORKTREE_REVIEW_PREFIX and WORKTREE_MANIFEST_FILENAME in
# crates/terraphim_orchestrator/src/scope.rs.

set -eu
umask 022

ADF_REPO_PATH="${ADF_REPO_PATH:-/data/projects/terraphim/terraphim-ai}"
ADF_WORKTREE_ROOT="${ADF_WORKTREE_ROOT:-${ADF_REPO_PATH}/.worktrees}"
ADF_AGENT_TMP_ROOT="${ADF_AGENT_TMP_ROOT:-/tmp/adf-worktrees}"

swept=0
failed=0
skipped=0

MANIFEST=".adf-worktree-manifest.json"

# Check that a directory carries a valid ADF worktree manifest.
# Validates repo_path and worktree_path fields against reality.
valid_manifest() {
    dir="$1"
    mf="${dir}/${MANIFEST}"
    [ -f "$mf" ] || return 1
    # Extract repo_path and worktree_path from the JSON manifest.
    # POSIX grep + sed fallback; jq is not guaranteed on minimal hosts.
    mf_repo=$(grep -o '"repo_path"[[:space:]]*:[[:space:]]*"[^"]*"' "$mf" 2>/dev/null | sed 's/.*"\([^"]*\)"$/\1/' | head -1)
    mf_path=$(grep -o '"worktree_path"[[:space:]]*:[[:space:]]*"[^"]*"' "$mf" 2>/dev/null | sed 's/.*"\([^"]*\)"$/\1/' | head -1)
    [ "$mf_repo" = "$ADF_REPO_PATH" ] || return 1
    [ "$mf_path" = "$dir" ] || return 1
    return 0
}

sweep_one() {
    target="$1"
    if [ ! -e "$target" ]; then
        return 0
    fi
    if ! valid_manifest "$target"; then
        printf 'adf-cleanup: skipping %s (no valid ADF manifest)\n' "$target" >&2
        skipped=$((skipped + 1))
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

printf 'adf-cleanup: swept=%d failed=%d skipped=%d repo=%s\n' \
    "$swept" "$failed" "$skipped" "$ADF_REPO_PATH"

exit 0
