#!/bin/sh
# test_adf_cleanup.sh -- POSIX shell driver for adf-cleanup.sh.
#
# Seeds three review-* worktrees (with ADF manifests) plus a keep-me/
# directory (without manifest) in a disposable git repo, runs the
# sweep script, and asserts that review-* entries are removed while
# keep-me/ is preserved. A second run verifies idempotency.

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

# Worktree manifest helper: writes a valid .adf-worktree-manifest.json
# into the target directory.
write_manifest() {
    dir="$1"
    cat > "${dir}/.adf-worktree-manifest.json" <<MANIFEST_EOF
{
  "version": 1,
  "repo_path": "${REPO}",
  "worktree_path": "${dir}",
  "creator": "test",
  "session_id": "test-session",
  "pid": $$,
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
MANIFEST_EOF
}

mkdir -p "$WT_ROOT/keep-me"

for i in 1 2 3; do
    git -C "$REPO" worktree add -q "${WT_ROOT}/review-test-${i}" HEAD
    write_manifest "${WT_ROOT}/review-test-${i}"
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
