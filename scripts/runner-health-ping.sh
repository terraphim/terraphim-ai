#!/usr/bin/env bash
#
# runner-health-ping.sh -- host-side external liveness checks for the
# terraphim-gitea-runner.  Run by a systemd user timer every 15 min.
#
# Replaces the ping-healthchecks and runner-registration steps that were
# removed from .gitea/workflows/runner-health.yml because they used
# sandbox-denied commands (curl, python3, if).
#
# Environment:
#   RUNNER_HEALTH_PING_URL  healthchecks.io (or similar) ping endpoint
#   GITEA_URL               Gitea base URL (default https://git.terraphim.cloud)
#   GITEA_TOKEN             API token with org:read on terraphim
#
set -euo pipefail

GITEA_URL="${GITEA_URL:-https://git.terraphim.cloud}"

# --- 1. External healthchecks.io ping ---
if [ -n "${RUNNER_HEALTH_PING_URL:-}" ]; then
    curl -fsS --retry 3 --retry-delay 2 "$RUNNER_HEALTH_PING_URL" \
        && echo "ping: OK" \
        || echo "ping: FAILED (non-fatal)"
else
    echo "ping: RUNNER_HEALTH_PING_URL not set, skipping"
fi

# --- 2. Gitea online-runner count ---
if [ -n "${GITEA_TOKEN:-}" ]; then
    response=$(curl -fsS -H "Authorization: token $GITEA_TOKEN" \
        "$GITEA_URL/api/v1/orgs/terraphim/actions/runners" 2>/dev/null || echo '{"runners":[]}')

    online=$(echo "$response" | python3 -c \
        "import sys,json; d=json.load(sys.stdin); print(sum(1 for r in d.get('runners',[]) if r.get('status')=='online'))")

    echo "Online runners: $online"

    if [ "$online" -eq 0 ]; then
        echo "ALERT: no online runners registered for org terraphim"
        exit 1
    fi
else
    echo "runner-count: GITEA_TOKEN not set, skipping"
fi
