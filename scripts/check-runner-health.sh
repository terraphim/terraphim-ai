#!/usr/bin/env bash
# Check self-hosted runner health status
# Usage: ./scripts/check-runner-health.sh [runner-name]
# Returns: 0 if any suitable runner is online and not busy, 1 otherwise

set -euo pipefail

RUNNER_NAME="${1:-}"

# Get all runners
RUNNERS=$(gh api repos/terraphim/terraphim-ai/actions/runners --jq '.runners')

# Find runners matching our criteria (self-hosted, Linux, X64)
if [[ -n "$RUNNER_NAME" ]]; then
  # Check specific runner
  STATUS=$(echo "$RUNNERS" | jq -r --arg name "$RUNNER_NAME" \
    '.[] | select(.name == $name and any(.labels[].name; . == "self-hosted")) | .status')
  echo "$RUNNER_NAME: $STATUS"

  if [[ "$STATUS" == "online" ]]; then
    exit 0
  else
    exit 1
  fi
else
  # Check for any available runner with [self-hosted, Linux, X64] labels
  AVAILABLE=$(echo "$RUNNERS" | jq '
    [.[] |
      select(any(.labels[].name; . == "self-hosted")) |
      select(any(.labels[].name; . == "Linux")) |
      select(any(.labels[].name; . == "X64")) |
      select(.status == "online") |
      select(.busy == false)
    ] | length
  ')

  if [[ "$AVAILABLE" -gt 0 ]]; then
    echo "✅ Found $AVAILABLE available self-hosted runner(s)"
    exit 0
  else
    echo "❌ No available self-hosted runners"
    exit 1
  fi
fi
