#!/usr/bin/env bash
# Helper script for terraphim package manager replacement skill
# Usage: ./replace.sh "text to replace"

set -euo pipefail

# Find repository root and terraphim-tui binary
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "")"
TERRAPHIM_TUI_BIN=""

# Check common locations
if command -v terraphim-tui &> /dev/null; then
    TERRAPHIM_TUI_BIN="terraphim-tui"
elif [ -n "$REPO_ROOT" ] && [ -f "$REPO_ROOT/target/release/terraphim-tui" ]; then
    TERRAPHIM_TUI_BIN="$REPO_ROOT/target/release/terraphim-tui"
elif [ -f "../../../target/release/terraphim-tui" ]; then
    TERRAPHIM_TUI_BIN="../../../target/release/terraphim-tui"
fi

if [ -z "$TERRAPHIM_TUI_BIN" ] || [ ! -f "$TERRAPHIM_TUI_BIN" ]; then
    echo "Error: terraphim-tui not found" >&2
    echo "Build it with: cargo build --release -p terraphim_tui" >&2
    exit 1
fi

# Get input text
if [ $# -eq 0 ]; then
    # Read from stdin
    TEXT=$(cat)
else
    # Read from argument
    TEXT="$1"
fi

# Change to repository root so terraphim-tui can find docs/src/kg/
if [ -n "$REPO_ROOT" ]; then
    cd "$REPO_ROOT"
fi

# Perform replacement, suppressing stderr
"$TERRAPHIM_TUI_BIN" replace "$TEXT" 2>/dev/null
