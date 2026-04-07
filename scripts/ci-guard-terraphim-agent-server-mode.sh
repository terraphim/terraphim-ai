#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

python3 - <<'PY'
from pathlib import Path
import sys

repo_root = Path.cwd()

files = [
    repo_root / "crates/terraphim_agent/tests/offline_mode_tests.rs",
    repo_root / "crates/terraphim_agent/tests/server_mode_tests.rs",
    repo_root / "crates/terraphim_agent/tests/integration_tests.rs",
    repo_root / "crates/terraphim_agent/tests/kg_ranking_integration_test.rs",
]

violations = []

for path in files:
    content = path.read_text(encoding="utf-8")
    lines = content.splitlines()
    total = len(lines)

    for idx, line in enumerate(lines):
        if '"--server"' not in line and "'--server'" not in line:
            continue

        start = max(0, idx - 30)
        end = min(total, idx + 31)
        window_lines = lines[start:end]
        window = "\n".join(window_lines)

        if 'Command::new("cargo")' not in window:
            continue
        if '"terraphim_agent"' not in window:
            continue

        has_server_features = '"--features"' in window and '"server"' in window
        if not has_server_features:
            violations.append((path, idx + 1))

if violations:
    print("ERROR: terraphim_agent server-mode subprocess contract violation(s) detected.")
    print("Every cargo run invocation that uses --server must include --features server.")
    for path, line in violations:
        print(f"  - {path}:{line}")
    sys.exit(1)

print("PASS: terraphim_agent server-mode subprocess contract verified.")
PY
