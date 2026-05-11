#!/usr/bin/env bash
# Gitea act-runner health check
# Usage: runner-health-check.sh [--gitea-url URL] [--stale-minutes N]
# Exits 0 if runner is active and not stale, non-zero otherwise.
# Designed for systemd timer or cron execution.

set -euo pipefail

GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"
GITEA_TOKEN="${GITEA_TOKEN:-}"
STALE_MINUTES="${STALE_MINUTES:-30}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --gitea-url) GITEA_URL="$2"; shift 2 ;;
        --stale-minutes) STALE_MINUTES="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$GITEA_TOKEN" ]]; then
    if [[ -f ~/.profile ]]; then
        source ~/.profile
    fi
    GITEA_TOKEN="${GITEA_TOKEN:-}"
fi

if [[ -z "$GITEA_TOKEN" ]]; then
    echo "ERROR: GITEA_TOKEN not set" >&2
    exit 1
fi

RUNNERS_JSON=$(curl -sf -H "Authorization: token $GITEA_TOKEN" \
    "$GITEA_URL/api/v1/admin/runners" 2>/dev/null) || {
    echo "WARN: Failed to query Gitea runners API" >&2

    if pgrep -x "act_runner" >/dev/null 2>&1; then
        echo "WARN: act_runner process is running but API unreachable" >&2
        exit 0
    else
        echo "ERROR: act_runner process not found" >&2
        exit 2
    fi
}

TOTAL=$(echo "$RUNNERS_JSON" | jq '.length // 0')
ONLINE=$(echo "$RUNNERS_JSON" | jq '[.[] | select(.status == "online")] | length')
OFFLINE=$(echo "$RUNNERS_JSON" | jq '[.[] | select(.status != "online")] | length')

echo "Runners: $TOTAL total, $ONLINE online, $OFFLINE offline"

if [[ "$ONLINE" -eq 0 ]]; then
    echo "ERROR: No online runners detected" >&2
    if [[ -n "${SENTINEL_ISSUE:-}" ]]; then
        curl -sf -X POST -H "Authorization: token $GITEA_TOKEN" \
            "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/issues/${SENTINEL_ISSUE}/comments" \
            -H "Content-Type: application/json" \
            -d "{\"body\": \"[$(date -Iseconds)] CI runner health check failed: 0/$TOTAL runners online\"}" \
            2>/dev/null || true
    fi
    exit 1
fi

STALE_THRESHOLD=$((STALE_MINUTES * 60))
NOW=$(date +%s)
STALE_COUNT=0

for runner_id in $(echo "$RUNNERS_JSON" | jq -r '.[] | select(.status == "online") | .id'); do
    LAST_SEEN=$(echo "$RUNNERS_JSON" | jq -r ".[] | select(.id == $runner_id) | .last_seen // 0")
    if [[ "$LAST_SEEN" != "null" ]] && [[ "$LAST_SEEN" -gt 0 ]]; then
        SEEN_AGO=$((NOW - LAST_SEEN))
        if [[ "$SEEN_AGO" -gt "$STALE_THRESHOLD" ]]; then
            NAME=$(echo "$RUNNERS_JSON" | jq -r ".[] | select(.id == $runner_id) | .name")
            echo "WARN: Runner '$NAME' last seen ${SEEN_AGO}s ago (threshold: ${STALE_THRESHOLD}s)" >&2
            STALE_COUNT=$((STALE_COUNT + 1))
        fi
    fi
done

if [[ "$STALE_COUNT" -gt 0 ]] && [[ "$STALE_COUNT" -eq "$ONLINE" ]]; then
    echo "ERROR: All $ONLINE online runners are stale (>${STALE_MINUTES}min)" >&2
    exit 1
fi

echo "OK: $ONLINE runner(s) online, $STALE_COUNT stale"
exit 0
