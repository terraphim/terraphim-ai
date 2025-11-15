#!/bin/bash
# test_tui_repl.sh - Comprehensive TUI REPL functionality test

set -euo pipefail

BINARY="./target/release/terraphim-agent"
TEST_LOG="tui_test_results_$(date +%Y%m%d_%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Terraphim TUI REPL Functional Test ===" | tee $TEST_LOG
echo "Started at: $(date)" | tee -a $TEST_LOG
echo "" | tee -a $TEST_LOG

# Function to test a command
test_command() {
    local cmd="$1"
    local expected="$2"
    local description="$3"

    echo -e "${YELLOW}Testing:${NC} $description" | tee -a $TEST_LOG
    echo "Command: $cmd" | tee -a $TEST_LOG

    # Execute command (macOS compatible - no timeout command)
    output=$(echo -e "$cmd\n/quit" | $BINARY repl 2>&1 | tail -20 || true)

    # Check if expected text is in output
    if echo "$output" | grep -qi "$expected"; then
        echo -e "${GREEN}✓ PASS${NC}" | tee -a $TEST_LOG
        ((PASS_COUNT++))
    else
        echo -e "${RED}✗ FAIL${NC}" | tee -a $TEST_LOG
        echo "Expected: $expected" | tee -a $TEST_LOG
        echo "Got: $output" | tee -a $TEST_LOG
        ((FAIL_COUNT++))
    fi
    echo "---" | tee -a $TEST_LOG
}

# Test all REPL commands
echo "=== Command Tests ===" | tee -a $TEST_LOG

test_command "/help" "Available commands" "Help command displays available commands"
test_command "/role list" "Available roles" "List all available roles"
test_command "/config show" "config" "Show current configuration"
test_command "/search test" "search" "Search for documents containing 'test'"
test_command "/graph" "graph" "Display knowledge graph information"
test_command "/thesaurus" "thesaurus" "Display thesaurus statistics"
test_command "/autocomplete terr" "suggestion" "Autocomplete for 'terr'"
test_command "/extract \"Find patterns\"" "extract" "Extract entities from text"
test_command "/find TODO" "find" "Find pattern in documents"
test_command "/role select Default" "role" "Select Default role"
test_command "/chat Hello" "chat" "Send chat message"
test_command "/quit" "quit\|exit\|bye" "Exit REPL cleanly"

# Test invalid commands for error handling
echo -e "\n=== Error Handling Tests ===" | tee -a $TEST_LOG

test_command "/invalid_command" "unknown\|error\|invalid" "Invalid command shows error"
test_command "/search" "usage\|help\|missing" "Missing parameter shows help"
test_command "/role select NonExistent" "error\|not found\|invalid" "Non-existent role shows error"

# Generate summary
echo -e "\n=== Test Summary ===" | tee -a $TEST_LOG
echo "Total Tests: $((PASS_COUNT + FAIL_COUNT))" | tee -a $TEST_LOG
echo -e "${GREEN}Passed: $PASS_COUNT${NC}" | tee -a $TEST_LOG
echo -e "${RED}Failed: $FAIL_COUNT${NC}" | tee -a $TEST_LOG
echo "Pass Rate: $(( PASS_COUNT * 100 / (PASS_COUNT + FAIL_COUNT) ))%" | tee -a $TEST_LOG
echo "Completed at: $(date)" | tee -a $TEST_LOG

# Exit with status
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}" | tee -a $TEST_LOG
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}" | tee -a $TEST_LOG
    exit 1
fi
