#!/usr/bin/env bash
# CI per-test timeout assertion (Refs #2997)
#
# Asserts that the workspace nextest profile terminates runaway tests within
# the #2997 acceptance window (<=150s total = 60s slow-threshold + 90s grace).
#
# Why a dedicated script: .config/nextest.toml's [profile.default] is inherited
# by [profile.ci] (nextest merges profiles). The effective terminate time is
# `period * terminate-after`. A regression here silently restores the 180s+
# runaway window that blocked the #2934 workspace test gate. This guard fails
# fast (sub-second) before the slow CI job discovers it.
#
# Usage: ./scripts/ci-check-nextest-timeout.sh
# Exit: 0 = within window; 1 = runaway window or unparseable config.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG="$PROJECT_ROOT/.config/nextest.toml"

# 60s slow-threshold + 90s grace, per #2997 acceptance criteria.
MAX_TERMINATE_SECONDS=150

if [[ ! -f "$CONFIG" ]]; then
    echo "FAIL: nextest config not found at $CONFIG" >&2
    exit 1
fi

# Resolve the effective slow-timeout for [profile.default] (the profile that
# [profile.ci] inherits). nextest does not let `show-config` dump the merged
# profile, so parse the TOML directly with python3 stdlib (tomllib, no mocks).
read -r PERIOD TERMINATE_AFTER < <(python3 - "$CONFIG" <<'PY'
import sys, tomllib, re
path = sys.argv[1]
with open(path, "rb") as f:
    data = tomllib.load(f)
default = data.get("profile", {}).get("default", {})
st = default.get("slow-timeout")
if not isinstance(st, dict):
    print("MISSING MISSING")
    sys.exit(0)
period = st.get("period", "60s")
ta = st.get("terminate-after", 1)
def to_seconds(s):
    if isinstance(s, (int, float)):
        return float(s)
    m = re.fullmatch(r"\s*(\d+(?:\.\d+)?)\s*(ms|s|m|h)?\s*", str(s))
    if not m:
        return 0.0
    n = float(m.group(1)); unit = m.group(2) or "s"
    return n * {"ms":0.001,"s":1,"m":60,"h":3600}[unit]
print(to_seconds(period), ta)
PY
)

if [[ "$PERIOD" == "MISSING" ]]; then
    echo "FAIL: [profile.default].slow-timeout missing in $CONFIG" >&2
    exit 1
fi

# Bash arithmetic on integers (sub-second precision is irrelevant here).
EFFECTIVE=$(python3 -c "import math; print(math.ceil($PERIOD * $TERMINATE_AFTER))")

echo "nextest [profile.default] slow-timeout: period=${PERIOD}s terminate-after=${TERMINATE_AFTER} -> terminate@${EFFECTIVE}s (limit=${MAX_TERMINATE_SECONDS}s)"

if (( EFFECTIVE > MAX_TERMINATE_SECONDS )); then
    echo "FAIL: effective terminate (${EFFECTIVE}s) exceeds #2997 limit (${MAX_TERMINATE_SECONDS}s)" >&2
    echo "Fix: reduce terminate-after in .config/nextest.toml [profile.default]" >&2
    exit 1
fi

echo "PASS: runaway tests terminate within ${EFFECTIVE}s (<= ${MAX_TERMINATE_SECONDS}s)"
