#!/usr/bin/env bash
# Terraphim user-prompt-submit hook for Claude Code
# This hook captures user corrections like "use X instead of Y" from prompts
# and stores them as ToolPreference learnings.
#
# Install: add this script to your Claude Code hooks directory and reference
# it in your claude-settings.json under the userPromptSubmit hook.

set -euo pipefail

# Configuration
TERRAPHIM_AGENT="${TERRAPHIM_AGENT:-terraphim-agent}"

# Check if terraphim-agent is available
if ! command -v "$TERRAPHIM_AGENT" &> /dev/null; then
    # Fallback: try local build from repo root
    if [ -f "$(git rev-parse --show-toplevel 2>/dev/null || echo ".")/target/release/terraphim-agent" ]; then
        TERRAPHIM_AGENT="$(git rev-parse --show-toplevel)/target/release/terraphim-agent"
    elif [ -f "$(git rev-parse --show-toplevel 2>/dev/null || echo ".")/target/debug/terraphim-agent" ]; then
        TERRAPHIM_AGENT="$(git rev-parse --show-toplevel)/target/debug/terraphim-agent"
    else
        echo "Warning: terraphim-agent not found. Install or build with: cargo build --release -p terraphim_agent" >&2
        # Pass through unchanged (fail-open)
        cat
        exit 0
    fi
fi

# Read stdin (the user's prompt as JSON from Claude Code)
INPUT=$(cat)

# Pipe the prompt JSON into terraphim-agent for user-prompt-submit processing.
# The agent expects {"user_prompt":"..."} and writes a correction file if a
# pattern like "use X instead of Y" is detected.
echo "$INPUT" | "$TERRAPHIM_AGENT" learn hook \
    --learn-hook-type user-prompt-submit \
    2>/dev/null || true

# Always pass the original input through unchanged (fail-open)
echo "$INPUT"
