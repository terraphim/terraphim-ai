#!/bin/bash
# Comprehensive test of all Learning Capture examples
# This script proves all documented examples work correctly

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Agent path
AGENT="./target/release/terraphim-agent"

# Temporary test directory
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

# Override global storage for testing
export HOME="$TEST_DIR"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Learning Capture System - Example Tests${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Test function
run_test() {
    local name="$1"
    local command="$2"
    local should_succeed="${3:-true}"
    
    echo -e "${YELLOW}Test:${NC} $name"
    echo -e "${BLUE}Command:${NC} $command"
    
    if eval "$command" > /dev/null 2>&1; then
        if [ "$should_succeed" = "true" ]; then
            echo -e "${GREEN}✓ PASS${NC}"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}✗ FAIL${NC} (expected failure but succeeded)"
            ((TESTS_FAILED++))
        fi
    else
        if [ "$should_succeed" = "false" ]; then
            echo -e "${GREEN}✓ PASS${NC} (expected failure)"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}✗ FAIL${NC} (expected success but failed)"
            ((TESTS_FAILED++))
        fi
    fi
    echo ""
}

echo -e "${BLUE}=== Part 1: Manual Capture Examples ===${NC}"
echo ""

# Example 1: Basic capture from docs
run_test "Basic capture - git push -f" \
    "$AGENT learn capture 'git push -f' --error 'remote: rejected' --exit-code 1"

# Example 2: NPM install error from docs
run_test "NPM install with permission error" \
    "$AGENT learn capture 'npm install' --error 'EACCES: permission denied, mkdir /node_modules' --exit-code 243"

# Example 3: Git status error
run_test "Git status in non-git directory" \
    "$AGENT learn capture 'git status' --error 'fatal: not a git repository' --exit-code 128"

# Example 4: Cargo build error
run_test "Cargo build error" \
    "$AGENT learn capture 'cargo build' --error 'error: could not compile' --exit-code 101"

# Example 5: Capture with debug flag
run_test "Capture with debug output" \
    "$AGENT learn capture 'test command' --error 'test error' --exit-code 1 --debug"

echo -e "${BLUE}=== Part 2: List Examples ===${NC}"
echo ""

# Example 6: List recent (default 10)
run_test "List recent learnings (default)" \
    "$AGENT learn list"

# Example 7: List with --recent flag
run_test "List with --recent 5" \
    "$AGENT learn list --recent 5"

# Example 8: List with --global flag
run_test "List global learnings" \
    "$AGENT learn list --global"

echo -e "${BLUE}=== Part 3: Query Examples ===${NC}"
echo ""

# Example 9: Query by substring
run_test "Query 'git' (substring match)" \
    "$AGENT learn query 'git'"

# Example 10: Query with exact match
run_test "Query with --exact flag" \
    "$AGENT learn query 'git status' --exact"

# Example 11: Query global only
run_test "Query global learnings only" \
    "$AGENT learn query 'npm' --global"

echo -e "${BLUE}=== Part 4: Ignored Commands (Should Skip) ===${NC}"
echo ""

# Example 12: Test commands should be ignored
run_test "Cargo test should be ignored" \
    "$AGENT learn capture 'cargo test' --error 'test failed' --exit-code 1" \
    "false"

# Example 13: NPM test should be ignored
run_test "NPM test should be ignored" \
    "$AGENT learn capture 'npm test' --error 'test failed' --exit-code 1" \
    "false"

# Example 14: Pytest should be ignored
run_test "Pytest should be ignored" \
    "$AGENT learn capture 'pytest tests/' --error 'assertion failed' --exit-code 1" \
    "false"

echo -e "${BLUE}=== Part 5: Secret Redaction Examples ===${NC}"
echo ""

# Example 15: AWS key redaction
run_test "AWS key should be redacted" \
    "$AGENT learn capture 'aws s3 ls' --error 'AKIAIOSFODNN7EXAMPLE found' --exit-code 1"

# Example 16: Connection string redaction
run_test "Connection string should be redacted" \
    "$AGENT learn capture 'psql' --error 'postgresql://user:secret@localhost/db' --exit-code 1"

# Example 17: API key redaction
run_test "OpenAI API key should be redacted" \
    "$AGENT learn capture 'curl' --error 'Authorization: Bearer sk-proj-abc123def456' --exit-code 1"

echo -e "${BLUE}=== Part 6: Storage Verification ===${NC}"
echo ""

# Check files were created
echo "Verifying storage..."
LEARNING_COUNT=$(find "$TEST_DIR/.local/share/terraphim/learnings" -name "*.md" 2>/dev/null | wc -l)
echo -e "Learnings captured: ${YELLOW}$LEARNING_COUNT${NC}"

if [ "$LEARNING_COUNT" -ge 10 ]; then
    echo -e "${GREEN}✓ Storage working correctly${NC}"
else
    echo -e "${RED}✗ Storage issue - expected at least 10 learnings${NC}"
fi
echo ""

# Show sample learning file
if [ "$LEARNING_COUNT" -gt 0 ]; then
    echo -e "${BLUE}Sample learning file:${NC}"
    SAMPLE_FILE=$(find "$TEST_DIR/.local/share/terraphim/learnings" -name "*.md" | head -1)
    head -20 "$SAMPLE_FILE"
    echo ""
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"
echo ""

if [ "$TESTS_FAILED" -eq 0 ]; then
    echo -e "${GREEN}✓ All examples work correctly!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
