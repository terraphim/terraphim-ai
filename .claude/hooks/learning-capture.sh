#!/bin/bash
# PostToolUse Learning Capture Hook
# Automatically captures failed commands as learning documents
#
# This hook runs after tool completion to capture failed commands
# with their error output for later learning and correction.
#
# Usage: Called automatically by Claude Code as a PostToolUse hook
# Input: JSON from stdin with tool_name, tool_input, and tool_result
# Output: Original JSON (hook is transparent/pass-through)
#
# Features:
# - Captures only failed bash commands (non-zero exit codes)
# - Redacts secrets automatically before storage
# - Ignores test commands (cargo test, npm test, etc.)
# - Fail-open: continues even if capture fails

set -euo pipefail

# Configuration
DEBUG="${TERRAPHIM_LEARN_DEBUG:-false}"

# Read JSON input
INPUT=$(cat)

# Extract tool information
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')

# Only process Bash commands
if [ "$TOOL_NAME" != "Bash" ]; then
    echo "$INPUT"
    exit 0
fi

# Extract command and result
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
RESULT=$(echo "$INPUT" | jq -r '.tool_result // empty')

# Skip if no command
if [ -z "$COMMAND" ]; then
    echo "$INPUT"
    exit 0
fi

# Extract exit code and output from result
EXIT_CODE=$(echo "$RESULT" | jq -r '.exit_code // 0')
STDOUT=$(echo "$RESULT" | jq -r '.stdout // empty')
STDERR=$(echo "$RESULT" | jq -r '.stderr // empty')

# Only capture failed commands
if [ "$EXIT_CODE" -eq 0 ]; then
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
    if [ "$DEBUG" = "true" ]; then
        echo "terraphim-agent not found, skipping learning capture" >&2
    fi
    echo "$INPUT"
    exit 0
fi

# Combine stdout and stderr for error context
ERROR_OUTPUT=""
if [ -n "$STDOUT" ]; then
    ERROR_OUTPUT="$STDOUT"
fi
if [ -n "$STDERR" ]; then
    if [ -n "$ERROR_OUTPUT" ]; then
        ERROR_OUTPUT="$ERROR_OUTPUT
$STDERR"
    else
        ERROR_OUTPUT="$STDERR"
    fi
fi

# Skip if no error output
if [ -z "$ERROR_OUTPUT" ]; then
    ERROR_OUTPUT="Command failed with exit code $EXIT_CODE"
fi

# Capture the learning (fail-open)
if [ "$DEBUG" = "true" ]; then
    echo "Capturing learning: $COMMAND (exit: $EXIT_CODE)" >&2
fi

CAPTURE_RESULT=$("$AGENT" learn capture "$COMMAND" --error "$ERROR_OUTPUT" --exit-code "$EXIT_CODE" 2>&1) || {
    if [ "$DEBUG" = "true" ]; then
        echo "Learning capture failed: $CAPTURE_RESULT" >&2
    fi
    # Fail-open: continue even if capture fails
    echo "$INPUT"
    exit 0
}

if [ "$DEBUG" = "true" ]; then
    echo "$CAPTURE_RESULT" >&2
fi

# Pass through original input (hook is transparent)
echo "$INPUT"
