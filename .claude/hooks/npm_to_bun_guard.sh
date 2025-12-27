#!/bin/bash
#
# PreToolUse hook that uses terraphim-agent for knowledge graph-based replacement.
# Replaces npm/yarn/pnpm commands with bun using the KG definitions in docs/src/kg/
#
# Installation: Add to .claude/settings.local.json under hooks.PreToolUse
#

set -e

# Read JSON input from stdin
INPUT=$(cat)

# Extract tool name and command using jq
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Only process Bash commands
[ "$TOOL_NAME" != "Bash" ] && exit 0
[ -z "$COMMAND" ] && exit 0

# Skip if no package manager references
echo "$COMMAND" | grep -qE '\b(npm|yarn|pnpm|npx)\b' || exit 0

# Find terraphim-agent
AGENT=""
command -v terraphim-agent >/dev/null 2>&1 && AGENT="terraphim-agent"
[ -z "$AGENT" ] && [ -x "./target/release/terraphim-agent" ] && AGENT="./target/release/terraphim-agent"
[ -z "$AGENT" ] && [ -x "$HOME/.cargo/bin/terraphim-agent" ] && AGENT="$HOME/.cargo/bin/terraphim-agent"

# If no agent found, pass through unchanged
[ -z "$AGENT" ] && exit 0

# Use terraphim-agent replace with fail-open mode
REPLACED=$("$AGENT" replace --fail-open 2>/dev/null <<< "$COMMAND")

# If replacement changed something, output modified tool_input
if [ -n "$REPLACED" ] && [ "$REPLACED" != "$COMMAND" ]; then
    [ "${TERRAPHIM_VERBOSE:-0}" = "1" ] && echo "Terraphim: '$COMMAND' → '$REPLACED'" >&2

    # Output modified tool_input JSON
    echo "$INPUT" | jq --arg cmd "$REPLACED" '.tool_input.command = $cmd'
fi

# No output = allow original through
exit 0
