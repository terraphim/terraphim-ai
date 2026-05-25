#!/bin/bash
# bigbox-sync.sh -- idempotent ff-only sync of bigbox to current Gitea main,
# rebuild release binary, install, restart orchestrator.
#
# Usage (on bigbox):
#   bash scripts/bigbox-sync.sh [--no-restart]
#
# Memory rule: NEVER use git reset --hard on bigbox. Working trees may
# contain in-flight ADF agent edits.

set -euo pipefail

REPO="${BIGBOX_REPO:-/data/projects/terraphim/terraphim-ai-fresh}"
NO_RESTART=0
for arg in "$@"; do
    case "$arg" in
        --no-restart) NO_RESTART=1 ;;
        *) echo "Unknown arg: $arg" >&2; exit 2 ;;
    esac
done

cd "$REPO"

echo "[bigbox-sync] repo: $REPO"
echo "[bigbox-sync] current HEAD: $(git log --oneline -1)"

# Refuse to proceed if the working tree has uncommitted changes.
# The memory rule is: no -X theirs, no reset --hard, no destructive ops.
if [[ -n "$(git status --short)" ]]; then
    echo "[bigbox-sync] ERROR: working tree is dirty. Refusing to sync." >&2
    git status --short >&2
    echo "[bigbox-sync] Inspect and either commit or stash before re-running." >&2
    exit 3
fi

echo "[bigbox-sync] git fetch origin --quiet"
git fetch origin --quiet

CURRENT=$(git rev-parse HEAD)
TARGET=$(git rev-parse origin/main)

if [[ "$CURRENT" == "$TARGET" ]]; then
    echo "[bigbox-sync] already at origin/main ($TARGET). Nothing to do."
    exit 0
fi

echo "[bigbox-sync] git checkout main"
git checkout main --quiet

echo "[bigbox-sync] git merge --ff-only origin/main"
git merge --ff-only origin/main

echo "[bigbox-sync] new HEAD: $(git log --oneline -1)"

echo "[bigbox-sync] cargo build --release -p terraphim_orchestrator"
cargo build --release -p terraphim_orchestrator

if [[ "$NO_RESTART" == "1" ]]; then
    echo "[bigbox-sync] --no-restart specified; skipping install + restart."
    exit 0
fi

echo "[bigbox-sync] sudo install target/release/adf /usr/local/bin/adf"
sudo install -m 755 target/release/adf /usr/local/bin/adf

echo "[bigbox-sync] sudo systemctl restart adf-orchestrator"
sudo systemctl restart adf-orchestrator

sleep 3
echo "[bigbox-sync] orchestrator status:"
sudo systemctl is-active adf-orchestrator
sudo systemctl status adf-orchestrator --no-pager | head -10

echo "[bigbox-sync] complete."
