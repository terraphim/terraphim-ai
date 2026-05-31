#!/usr/bin/env bash
# Test: Verify Redis binds to 127.0.0.1
set -euo pipefail

COMPOSE_FILE="docker/docker-compose.yml"

echo "Testing Redis binding..."

# Check that Redis port binding uses 127.0.0.1
if grep -q '127.0.0.1:6379:6379' "$COMPOSE_FILE"; then
    echo "PASS: Redis is bound to 127.0.0.1"
    exit 0
else
    echo "FAIL: Redis is not bound to 127.0.0.1"
    echo "Current binding:"
    grep -A1 'redis:' "$COMPOSE_FILE" | grep '6379' || true
    exit 1
fi
