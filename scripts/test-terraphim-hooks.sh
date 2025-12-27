#!/bin/bash
#
# Test script for Terraphim hooks
# Validates both use cases:
#   1. npm install → bun install
#   2. Claude Code → Terraphim AI attribution
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

PASSED=0
FAILED=0

assert_replace() {
    local input="$1"
    local expected="$2"
    local description="$3"
    local actual

    actual=$("$PROJECT_DIR/target/release/terraphim-agent" replace --fail-open 2>/dev/null <<< "$input")

    if [ "$actual" = "$expected" ]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $description"
        echo "    Input:    '$input'"
        echo "    Expected: '$expected'"
        echo "    Got:      '$actual'"
        ((FAILED++))
    fi
}

assert_hook() {
    local json_input="$1"
    local expected_command="$2"
    local description="$3"
    local actual

    actual=$(echo "$json_input" | "$PROJECT_DIR/.claude/hooks/npm_to_bun_guard.sh" 2>/dev/null | jq -r '.tool_input.command // empty')

    if [ "$actual" = "$expected_command" ]; then
        echo -e "${GREEN}✓${NC} $description"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $description"
        echo "    Expected: '$expected_command'"
        echo "    Got:      '$actual'"
        ((FAILED++))
    fi
}

echo "Terraphim Hooks Test Suite"
echo "=========================="
echo ""

# Check prerequisites
if [ ! -x "$PROJECT_DIR/target/release/terraphim-agent" ]; then
    echo "Building terraphim-agent..."
    (cd "$PROJECT_DIR" && cargo build -p terraphim_agent --release 2>/dev/null) || {
        echo "Failed to build terraphim-agent"
        exit 1
    }
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required for hook tests"
    exit 1
fi

echo "Test 1: Package Manager Replacement (terraphim-agent replace)"
echo "--------------------------------------------------------------"
assert_replace "npm install" "bun_install" "npm install → bun_install"
assert_replace "yarn install" "bun_install" "yarn install → bun_install"
assert_replace "pnpm install" "bun_install" "pnpm install → bun_install"
assert_replace "npm test" "bun test" "npm test → bun test"
assert_replace "yarn test" "bun test" "yarn test → bun test"
assert_replace "npm install && yarn test" "bun_install && bun test" "compound command replacement"

echo ""
echo "Test 2: Attribution Replacement (terraphim-agent replace)"
echo "---------------------------------------------------------"
assert_replace "Generated with Claude Code" "generated_with_terraphim" "Claude Code attribution"
assert_replace "Co-Authored-By: Claude" "Co-Authored-By: Terraphim AI" "Claude co-author"

echo ""
echo "Test 3: PreToolUse Hook (npm_to_bun_guard.sh)"
echo "---------------------------------------------"
assert_hook '{"tool_name":"Bash","tool_input":{"command":"npm install"}}' "bun_install" "Hook: npm install"
assert_hook '{"tool_name":"Bash","tool_input":{"command":"yarn test"}}' "bun test" "Hook: yarn test"

# Test pass-through for non-package-manager commands
echo ""
echo "Test 4: Pass-Through (non-package-manager commands)"
echo "---------------------------------------------------"
PASSTHROUGH_INPUT='{"tool_name":"Bash","tool_input":{"command":"ls -la"}}'
PASSTHROUGH_OUTPUT=$(echo "$PASSTHROUGH_INPUT" | "$PROJECT_DIR/.claude/hooks/npm_to_bun_guard.sh" 2>/dev/null)
if [ -z "$PASSTHROUGH_OUTPUT" ]; then
    echo -e "${GREEN}✓${NC} Non-package-manager command passes through unchanged"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} Non-package-manager command should pass through"
    ((FAILED++))
fi

# Test non-Bash tool pass-through
NONBASH_INPUT='{"tool_name":"Read","tool_input":{"path":"/etc/passwd"}}'
NONBASH_OUTPUT=$(echo "$NONBASH_INPUT" | "$PROJECT_DIR/.claude/hooks/npm_to_bun_guard.sh" 2>/dev/null)
if [ -z "$NONBASH_OUTPUT" ]; then
    echo -e "${GREEN}✓${NC} Non-Bash tool passes through unchanged"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} Non-Bash tool should pass through"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Results: $PASSED passed, $FAILED failed"
echo "========================================"

[ $FAILED -eq 0 ] && exit 0 || exit 1
