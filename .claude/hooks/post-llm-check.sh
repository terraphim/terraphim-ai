#!/bin/bash
# Post-LLM Checklist Validation Hook
# Validates LLM outputs against domain checklists
#
# This hook runs after tool completion to validate outputs meet
# required standards.
#
# Usage: Called automatically by Claude Code as a PostToolUse hook
# Input: JSON from stdin with tool_name and tool_result
# Output: Original JSON with validation annotations

set -euo pipefail

# Read JSON input
INPUT=$(cat)

# Extract tool name and result
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
TOOL_RESULT=$(echo "$INPUT" | jq -r '.tool_result // empty')

# Only validate certain tools
case "$TOOL_NAME" in
    "Write"|"Edit"|"MultiEdit")
        # Code-related tools - use code_review checklist
        CHECKLIST="code_review"
        ;;
    *)
        # Pass through other tools
        echo "$INPUT"
        exit 0
        ;;
esac

if [ -z "$TOOL_RESULT" ]; then
    echo "$INPUT"
    exit 0
fi

# Find terraphim-agent
AGENT=""
for path in \
    "./target/release/terraphim-agent" \
    "./target/debug/terraphim-agent" \
    "$(which terraphim-agent 2>/dev/null || true)"; do
    if [ -x "$path" ]; then
        AGENT="$path"
        break
    fi
done

if [ -z "$AGENT" ]; then
    echo "$INPUT"
    exit 0
fi

# Validate against checklist (advisory mode)
VALIDATION=$("$AGENT" validate --checklist "$CHECKLIST" --json "$TOOL_RESULT" 2>/dev/null || echo '{"passed":true}')
PASSED=$(echo "$VALIDATION" | jq -r '.passed // true')

if [ "$PASSED" = "false" ]; then
    # Log validation failure (advisory)
    MISSING=$(echo "$VALIDATION" | jq -r '.missing | join(", ") // "none"')
    SATISFIED=$(echo "$VALIDATION" | jq -r '.satisfied | join(", ") // "none"')

    echo "Post-LLM checklist validation ($CHECKLIST):" >&2
    echo "  Satisfied: $SATISFIED" >&2
    echo "  Missing: $MISSING" >&2
fi

# Always pass through (advisory mode)
echo "$INPUT"
