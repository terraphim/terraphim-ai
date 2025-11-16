#!/usr/bin/env bash
# Terraphim Package Manager Hook for Claude Code
# This hook intercepts commands and replaces package manager commands with preferred alternatives
# Example: npm install -> bun install

set -euo pipefail

# Configuration
TERRAPHIM_TUI_BIN="${TERRAPHIM_TUI_BIN:-terraphim-tui}"
TERRAPHIM_ROLE="${TERRAPHIM_ROLE:-Terraphim Engineer}"
HOOK_MODE="${HOOK_MODE:-replace}"  # replace, suggest, or passive

# Check if terraphim-tui is available
if ! command -v "$TERRAPHIM_TUI_BIN" &> /dev/null; then
    # If terraphim-tui is not in PATH, try the local build
    if [ -f "$(git rev-parse --show-toplevel 2>/dev/null)/target/release/terraphim-tui" ]; then
        TERRAPHIM_TUI_BIN="$(git rev-parse --show-toplevel)/target/release/terraphim-tui"
    else
        echo "Warning: terraphim-tui not found. Install it or build with: cargo build --release -p terraphim_tui" >&2
        exit 0  # Don't fail the hook, just pass through
    fi
fi

# Read stdin (the user's prompt or command)
INPUT=$(cat)

# Check if the input contains package manager commands
if ! echo "$INPUT" | grep -qiE '(npm|yarn|pnpm)\s+(install|run|build|test|dev|start)'; then
    # No package manager commands found, pass through
    echo "$INPUT"
    exit 0
fi

# Use terraphim-tui to replace package manager commands
REPLACED=$("$TERRAPHIM_TUI_BIN" replace "$INPUT" --role "$TERRAPHIM_ROLE" 2>/dev/null || echo "$INPUT")

# Handle different modes
case "$HOOK_MODE" in
    replace)
        # Replace automatically
        echo "$REPLACED"
        if [ "$REPLACED" != "$INPUT" ]; then
            echo "[Terraphim Hook] Replaced package manager commands with bun" >&2
        fi
        ;;
    suggest)
        # Suggest replacement but keep original
        echo "$INPUT"
        if [ "$REPLACED" != "$INPUT" ]; then
            echo "[Terraphim Hook] Suggestion: $REPLACED" >&2
        fi
        ;;
    passive)
        # Just log, don't modify
        echo "$INPUT"
        if [ "$REPLACED" != "$INPUT" ]; then
            echo "[Terraphim Hook] Would replace with: $REPLACED" >&2
        fi
        ;;
    *)
        echo "$INPUT"
        ;;
esac
