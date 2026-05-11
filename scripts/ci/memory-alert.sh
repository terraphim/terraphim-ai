#!/usr/bin/env bash
# Memory alerting for adf-orchestrator and other services
# Usage: memory-alert.sh [--threshold PCT] [--service SERVICE]
# Exits 0 if memory usage below threshold, non-zero with warning if exceeded.
# Designed for systemd timer or cron execution.

set -euo pipefail

THRESHOLD="${THRESHOLD:-80}"
SERVICE="${SERVICE:-adf-orchestrator}"
GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"
GITEA_TOKEN="${GITEA_TOKEN:-}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --threshold) THRESHOLD="$2"; shift 2 ;;
        --service) SERVICE="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done

get_service_memory_pct() {
    local svc="$1"
    local meminfo

    if ! systemctl is-active --quiet "$svc" 2>/dev/null; then
        echo "-1"
        return
    fi

    local cgroup_path
    cgroup_path=$(systemctl show "$svc" -p ControlGroup --value 2>/dev/null || echo "")

    if [[ -n "$cgroup_path" ]] && [[ -f "/sys/fs/cgroup${cgroup_path}/memory.current" ]]; then
        local current max
        current=$(cat "/sys/fs/cgroup${cgroup_path}/memory.current" 2>/dev/null || echo "0")
        max=$(cat "/sys/fs/cgroup${cgroup_path}/memory.max" 2>/dev/null || echo "0")

        if [[ "$max" -gt 0 ]] && [[ "$current" -gt 0 ]]; then
            echo $((current * 100 / max))
            return
        fi
    fi

    local pid rss total_mem
    pid=$(systemctl show "$svc" -p MainPID --value 2>/dev/null || echo "0")
    if [[ "$pid" == "0" ]] || [[ -z "$pid" ]]; then
        echo "-1"
        return
    fi

    rss=$(grep VmRSS /proc/"$pid"/status 2>/dev/null | awk '{print $2}' || echo "0")
    total_mem=$(grep MemTotal /proc/meminfo | awk '{print $2}')

    if [[ "$total_mem" -gt 0 ]] && [[ "$rss" -gt 0 ]]; then
        echo $((rss * 100 / total_mem))
    else
        echo "-1"
    fi
}

PCT=$(get_service_memory_pct "$SERVICE")

if [[ "$PCT" == "-1" ]]; then
    echo "INFO: Service '$SERVICE' not running or memory metrics unavailable"
    exit 0
fi

echo "$SERVICE memory: ${PCT}% (threshold: ${THRESHOLD}%)"

if [[ "$PCT" -ge "$THRESHOLD" ]]; then
    echo "WARN: $SERVICE at ${PCT}% memory usage (threshold: ${THRESHOLD}%)" >&2

    if [[ -n "${SENTINEL_ISSUE:-}" ]] && [[ -n "$GITEA_TOKEN" ]]; then
        if [[ -z "$GITEA_TOKEN" ]] && [[ -f ~/.profile ]]; then
            source ~/.profile
        fi
        curl -sf -X POST -H "Authorization: token ${GITEA_TOKEN}" \
            "$GITEA_URL/api/v1/repos/terraphim/terraphim-ai/issues/${SENTINEL_ISSUE}/comments" \
            -H "Content-Type: application/json" \
            -d "{\"body\": \"[$(date -Iseconds)] Memory alert: $SERVICE at ${PCT}% (threshold: ${THRESHOLD}%). Consider restarting.\"}" \
            2>/dev/null || true
    fi

    exit 1
fi

exit 0
