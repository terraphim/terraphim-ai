#!/bin/bash
# Pre-LLM Validation Hook
# Validates input before LLM calls using knowledge graph connectivity
#
# This hook intercepts tool calls and validates the content for semantic
# coherence before allowing them to proceed.
#
# Usage: Called automatically by Claude Code as a PreToolUse hook
# Input: JSON from stdin with tool_name and tool_input
# Output: Original JSON (pass-through) or modified JSON with validation warnings

set -euo pipefail

# Read JSON input
INPUT=$(cat)

# Extract tool name
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Only validate certain tools that involve LLM context
case "$TOOL_NAME" in
    "Task"|"WebSearch"|"WebFetch")
        # These tools might benefit from pre-validation
        ;;
    *)
        # Pass through other tools unchanged
        echo "$INPUT"
        exit 0
        ;;
esac

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
    # No agent found, pass through
    echo "$INPUT"
    exit 0
fi

# Extract prompt/query from tool input
PROMPT=$(echo "$INPUT" | jq -r '.tool_input.prompt // .tool_input.query // .tool_input.description // empty')

if [ -z "$PROMPT" ]; then
    # No prompt to validate
    echo "$INPUT"
    exit 0
fi

# Validate connectivity (advisory mode - always pass through)
VALIDATION=$("$AGENT" validate --connectivity --json "$PROMPT" 2>/dev/null || echo '{"connected":true}')
CONNECTED=$(echo "$VALIDATION" | jq -r '.connected // true')

if [ "$CONNECTED" = "false" ]; then
    # Add validation warning to the input but still allow it
    MATCHED=$(echo "$VALIDATION" | jq -r '.matched_terms | join(", ") // "none"')

    # Log warning (visible in Claude Code logs)
    echo "Pre-LLM validation warning: Input spans unrelated concepts" >&2
    echo "Matched terms: $MATCHED" >&2
fi

# Always pass through (advisory mode)
echo "$INPUT"
