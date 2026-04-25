#!/bin/bash
# Heavier smoke: each job touches a different crate so they don't share
# cargo lock/target state. Forces real compile work above the
# min_local_time_ms threshold so rch dispatches to the queue.

set -u
RCH=/home/alex/.local/bin/rch
WS=${WS:-/data/projects/terraphim-ai}
cd "$WS"

PKGS=(terraphim_types terraphim_automata terraphim_settings terraphim_persistence \
      terraphim_config terraphim_rolegraph terraphim_atomic_client \
      terraphim_onepassword_cli)

START=$(date +%s)
echo "=== baseline ==="
$RCH status 2>&1 | grep -E "Workers|Builds"

echo "=== firing ${#PKGS[@]} concurrent rch exec -- cargo check ==="
for pkg in "${PKGS[@]}"; do
    ($RCH exec -- cargo check --package "$pkg" > /tmp/rch-h-$pkg.log 2>&1; echo "  $pkg exit=$?") &
done

# Check queue snapshots while they run
sleep 3
echo "--- t+3s ---"
$RCH queue 2>&1 | head -10
$RCH status 2>&1 | grep -E "Workers|Builds"

sleep 5
echo "--- t+8s ---"
$RCH queue 2>&1 | head -10
$RCH status 2>&1 | grep -E "Workers|Builds"

wait
END=$(date +%s)
echo "=== complete in $((END-START))s ==="
$RCH status 2>&1 | head -15
