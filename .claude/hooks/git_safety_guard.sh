#!/bin/bash
#
# PreToolUse hook that blocks destructive git and filesystem commands.
# Uses terraphim-agent guard command for pattern matching.
#
# Blocks: git checkout --, git reset --hard, rm -rf, git push --force, etc.
# Allows: git checkout -b, rm -rf /tmp/, git push --force-with-lease, etc.
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

# Discover terraphim-agent
AGENT=""
if command -v terraphim-agent >/dev/null 2>&1; then
    AGENT="terraphim-agent"
elif [ -x "./target/release/terraphim-agent" ]; then
    AGENT="./target/release/terraphim-agent"
elif [ -x "./target/debug/terraphim-agent" ]; then
    AGENT="./target/debug/terraphim-agent"
elif [ -x "$HOME/.cargo/bin/terraphim-agent" ]; then
    AGENT="$HOME/.cargo/bin/terraphim-agent"
fi

# If no agent found, pass through unchanged (fail-open)
[ -z "$AGENT" ] && exit 0

# Check command against guard patterns
RESULT=$("$AGENT" guard --json <<< "$COMMAND" 2>/dev/null) || exit 0

# Parse decision
DECISION=$(echo "$RESULT" | jq -r '.decision // empty')

# If blocked, output deny decision for Claude Code
if [ "$DECISION" = "block" ]; then
    REASON=$(echo "$RESULT" | jq -r '.reason // "Command blocked by git_safety_guard"')

    cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "BLOCKED by git_safety_guard\n\nReason: $REASON\n\nCommand: $COMMAND\n\nIf this operation is truly needed, ask the user for explicit permission and have them run the command manually."
  }
}
EOF
fi

# If allowed, no output (Claude Code proceeds normally)
exit 0
