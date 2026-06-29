#!/usr/bin/env bash
# ADF Meta-Coordinator health checks
# Usage: ./scripts/adf-meta-coordinator-health.sh [--dry-run]
#
# Checks:
#   1. reconcile_tick SLOW stalls in adf-orchestrator journal
#   2. non-success agent exits
#   3. agents killed for exceeding max_cpu_seconds
#   4. worktree disk usage
#   5. adf binary version
#   6. worktree count
#   7. Gitea API health

set -euo pipefail

DRY_RUN="${1:-}"
OWNER="terraphim"
REPO="terraphim-ai"
LABEL="adf-health-alert"
EXPECTED_VERSION="adf 1.20.2"
WORKTREE_DIR="/data/projects/terraphim/terraphim-ai/.worktrees"
SINCE="4 hours ago"
WARN_EXIT=0

if [[ "$DRY_RUN" == "--dry-run" ]]; then
    echo "INFO: dry-run mode; no issues will be created"
fi

if [[ -z "${GITEA_TOKEN:-}" ]]; then
    echo "ERROR: GITEA_TOKEN is not set" >&2
    exit 1
fi

log_warn() { echo "WARNING: $*"; }
log_info() { echo "INFO: $*"; }

timestamp() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

# Search for an already-open issue with this exact title prefix.
# Prevents duplicate health-alert spam on every cron invocation.
search_open_issue() {
    local title_prefix="$1"
    curl -s -H "Authorization: token $GITEA_TOKEN" \
        "$GITEA_URL/api/v1/repos/$OWNER/$REPO/issues?state=open&label=$LABEL&limit=50" \
        | jq -r --arg prefix "$title_prefix" '.[] | select(.title | startswith($prefix)) | .number' \
        | head -n1
}

create_issue() {
    local title="$1"
    local body="$2"
    if [[ "$DRY_RUN" == "--dry-run" ]]; then
        log_info "[dry-run] would create issue: $title"
        return
    fi

    local existing
    existing=$(search_open_issue "$title" || true)
    if [[ -n "$existing" ]]; then
        log_info "Issue #$existing already open for '$title'; skipping"
        return
    fi

    gtr create-issue \
        --owner "$OWNER" \
        --repo "$REPO" \
        --title "$title" \
        --body "$body" \
        --labels "$LABEL"
}

# --- 1. reconcile_tick stalls ---
stall_count=$(sudo journalctl -u adf-orchestrator --since "$SINCE" 2>/dev/null | grep -c 'reconcile_tick SLOW' || true)
if [[ "$stall_count" -gt 0 ]]; then
    log_warn "$stall_count tick stalls detected in last 4h"
    create_issue "[ADF] Tick-stall detected: $stall_count in 4h" \
"$(timestamp): $stall_count reconcile_tick SLOW events detected.

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "0 tick stalls in last 4h"
fi

# --- 2. non-success agent exits ---
failures=$(sudo journalctl -u adf-orchestrator --since "$SINCE" 2>/dev/null \
    | grep 'exit classified' | grep -v 'success' | grep -v 'empty_success' || true)
failure_count=$(echo "$failures" | grep -c 'exit_class=' || true)
if [[ "$failure_count" -gt 3 ]]; then
    log_warn "$failure_count non-success agent exits in last 4h"
    create_issue "[ADF] $failure_count agent failures in 4h" \
"$(timestamp): $failure_count non-success agent exits in last 4h:

$(echo "$failures" | tail -20)

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "$failure_count non-success agent exits in last 4h (threshold 3)"
fi

# --- 3. max_cpu_seconds timeouts ---
timeouts=$(sudo journalctl -u adf-orchestrator --since "$SINCE" 2>/dev/null | grep 'AGENT EXCEEDED max_cpu_seconds' || true)
timeout_count=$(echo "$timeouts" | grep -c 'AGENT EXCEEDED' || true)
if [[ "$timeout_count" -gt 0 ]]; then
    log_warn "$timeout_count agents exceeded max_cpu_seconds in last 4h"
    create_issue "[ADF] $timeout_count agents exceeded max_cpu_seconds" \
"$(timestamp): Agents killed after exceeding their configured max_cpu_seconds:

$(echo "$timeouts" | tail -10)

Consider increasing max_cpu_seconds or investigating why these agents are running long.

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "0 max_cpu_seconds timeouts in last 4h"
fi

# --- 4. worktree disk usage ---
worktree_size=$(du -sm "$WORKTREE_DIR" 2>/dev/null | cut -f1 || echo 0)
if [[ "$worktree_size" -gt 10240 ]]; then
    log_warn "worktrees using ${worktree_size}MB (>10GB)"
    create_issue "[ADF] Worktree disk usage: ${worktree_size} MB" \
"$(timestamp): Worktree directory exceeds 10GB threshold ($WORKTREE_DIR).

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "worktree disk usage: ${worktree_size}MB"
fi

# --- 5. adf binary version ---
adf_version=$(/usr/local/bin/adf --version 2>/dev/null || echo 'unknown')
if [[ "$adf_version" != "$EXPECTED_VERSION" ]]; then
    log_warn "adf binary version mismatch: $adf_version (expected $EXPECTED_VERSION)"
    create_issue "[ADF] Binary version mismatch: $adf_version" \
"$(timestamp): Expected: $EXPECTED_VERSION
Actual: $adf_version

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "adf binary version: $adf_version"
fi

# --- 6. worktree count (informational) ---
# Count directories under .worktrees, not raw `ls` output.
worktree_count=$(find "$WORKTREE_DIR" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
if [[ "$worktree_count" -gt 10 ]]; then
    log_info "$worktree_count worktrees present"
else
    log_info "$worktree_count worktrees present (threshold 10)"
fi

# --- 7. Gitea API health ---
# Use the repo endpoint: the service token has write:repository but not read:user,
# so /api/v1/user returns 403 even when the API is healthy.
gitea_status=$(curl -s -o /dev/null -w '%{http_code}' \
    -H "Authorization: token $GITEA_TOKEN" \
    "$GITEA_URL/api/v1/repos/$OWNER/$REPO" || echo 000)
if [[ "$gitea_status" != '200' ]]; then
    log_warn "Gitea API returned HTTP $gitea_status"
    create_issue "[ADF] Gitea API unhealthy: HTTP $gitea_status" \
"$(timestamp): Gitea API health check failed (HTTP $gitea_status).

Theme-ID: adf-health-alert"
    WARN_EXIT=1
else
    log_info "Gitea API healthy (HTTP 200)"
fi

echo "=== Meta-coordinator health check complete ==="
exit "$WARN_EXIT"
