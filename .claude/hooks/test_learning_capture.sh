#!/bin/bash
# Integration test for learning-capture.sh hook
#
# Tests the hook's behavior with various inputs

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Path to hook
HOOK_PATH="${HOOK_PATH:-./.claude/hooks/learning-capture.sh}"

# Test function
test_case() {
    local name="$1"
    local input="$2"
    local expected_exit="${3:-0}"
    
    echo -n "Testing: $name... "
    
    if result=$(echo "$input" | "$HOOK_PATH" 2>/dev/null); then
        if [ "$expected_exit" -eq 0 ]; then
            echo -e "${GREEN}PASS${NC}"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}FAIL${NC} (expected failure but passed)"
            ((TESTS_FAILED++))
        fi
    else
        if [ "$expected_exit" -ne 0 ]; then
            echo -e "${GREEN}PASS${NC}"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}FAIL${NC} (expected pass but failed)"
            ((TESTS_FAILED++))
        fi
    fi
}

# Test 1: Non-Bash tool (should pass through)
test_case "Non-Bash tool passes through" '{
    "tool_name": "Write",
    "tool_input": {"file_path": "/tmp/test.txt", "content": "hello"},
    "tool_result": {"success": true}
}' 0

# Test 2: Successful Bash command (should pass through without capture)
test_case "Successful Bash command passes through" '{
    "tool_name": "Bash",
    "tool_input": {"command": "echo hello", "description": "Test"},
    "tool_result": {"exit_code": 0, "stdout": "hello", "stderr": ""}
}' 0

# Test 3: Failed Bash command (should attempt capture)
# Note: This will fail to capture if terraphim-agent is not found, but hook should still pass through
test_case "Failed Bash command passes through (fail-open)" '{
    "tool_name": "Bash",
    "tool_input": {"command": "false", "description": "Test failure"},
    "tool_result": {"exit_code": 1, "stdout": "", "stderr": "Command failed"}
}' 0

# Test 4: Empty command (should pass through)
test_case "Empty command passes through" '{
    "tool_name": "Bash",
    "tool_input": {},
    "tool_result": {"exit_code": 0}
}' 0

# Test 5: JSON with stderr only
test_case "Failed command with stderr only" '{
    "tool_name": "Bash",
    "tool_input": {"command": "invalid_command", "description": "Test"},
    "tool_result": {"exit_code": 127, "stdout": "", "stderr": "command not found"}
}' 0

# Test 6: JSON with both stdout and stderr
test_case "Failed command with both outputs" '{
    "tool_name": "Bash",
    "tool_input": {"command": "git push", "description": "Test"},
    "tool_result": {"exit_code": 1, "stdout": "Everything up-to-date", "stderr": "remote: rejected"}
}' 0

# Summary
echo ""
echo "========================================"
echo "Test Results:"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "========================================"

if [ "$TESTS_FAILED" -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
