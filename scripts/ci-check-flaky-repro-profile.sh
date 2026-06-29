#!/usr/bin/env bash
# Guard: verify the deterministic flaky-test reproduction profile is present and
# correctly configured in .config/nextest.toml.
#
# Issue #2999: the "flaky-repro" profile MUST set:
#   test-threads = 1   (serial execution is nextest's only determinism primitive)
#   retries      = 5   (AC: "reproduced within 5 attempts")
#
# Uses only python3 stdlib (tomllib) — no third-party deps, no mocks.
# Exit 0 = profile valid; Exit 1 = missing or misconfigured.
set -euo pipefail

CONFIG="${1:-.config/nextest.toml}"

if [[ ! -f "${CONFIG}" ]]; then
    echo "FAIL: nextest config not found at ${CONFIG}" >&2
    exit 1
fi

python3 - "${CONFIG}" <<'PYEOF'
import sys
import tomllib
from pathlib import Path

config_path = Path(sys.argv[1])
data = tomllib.loads(config_path.read_text())
profile = data.get("profile", {}).get("flaky-repro")

if profile is None:
    print("FAIL: [profile.flaky-repro] missing from nextest config")
    sys.exit(1)

threads = profile.get("test-threads")
retries = profile.get("retries")

errors = []
if threads != 1:
    errors.append(f"test-threads = {threads!r} (expected 1 for serial/deterministic order)")
if retries != 5:
    errors.append(f"retries = {retries!r} (expected 5 per AC)")

if errors:
    print("FAIL: [profile.flaky-repro] misconfigured:")
    for e in errors:
        print(f"  - {e}")
    sys.exit(1)

print(f"PASS: [profile.flaky-repro] test-threads={threads} retries={retries} (serial, <=5 attempts)")
PYEOF
