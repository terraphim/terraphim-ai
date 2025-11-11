#!/bin/bash
# test_tui_simple.sh - Simplified TUI test that handles startup delays

set -euo pipefail

BINARY="./target/debug/terraphim-tui"
TEST_LOG="tui_simple_test_$(date +%Y%m%d_%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Simplified Terraphim TUI Test ===${NC}" | tee $TEST_LOG
echo "Started at: $(date)" | tee -a $TEST_LOG
echo "" | tee -a $TEST_LOG

# Create a command file that runs multiple tests
COMMAND_FILE="/tmp/tui_test_commands.txt"
cat > $COMMAND_FILE << 'EOF'
/help
/role list
/config show
/search test
/role select Default
/chat Hello test
/quit
EOF

echo -e "${YELLOW}Testing TUI with command batch...${NC}" | tee -a $TEST_LOG

# Run TUI with all commands at once
output=$(timeout 60 $BINARY repl < $COMMAND_FILE 2>&1 || echo "TIMEOUT_OR_ERROR")

echo "Full output captured:" | tee -a $TEST_LOG
echo "$output" | tee -a $TEST_LOG
echo "" | tee -a $TEST_LOG

# Test for expected outputs
declare -a patterns=(
    "Available commands:"
    "Available roles:"
    '"id": "Embedded"'
    "Found.*result(s)"
    "Switched to role"
    "No LLM configured"
)

declare -a descriptions=(
    "Help command works"
    "Role listing works"
    "Config display works"
    "Search functionality works"
    "Role selection works"
    "Chat functionality works"
)

for i in "${!patterns[@]}"; do
    pattern="${patterns[$i]}"
    description="${descriptions[$i]}"

    echo -e "${YELLOW}Testing:${NC} $description" | tee -a $TEST_LOG

    if echo "$output" | grep -q "$pattern"; then
        echo -e "${GREEN}✓ PASS${NC}" | tee -a $TEST_LOG
        ((PASS_COUNT++))
    else
        echo -e "${RED}✗ FAIL${NC}" | tee -a $TEST_LOG
        echo "Expected pattern: $pattern" | tee -a $TEST_LOG
        ((FAIL_COUNT++))
    fi
done

# Cleanup
rm -f $COMMAND_FILE

echo -e "\n=== Test Summary ===" | tee -a $TEST_LOG
echo "Total Tests: $((PASS_COUNT + FAIL_COUNT))" | tee -a $TEST_LOG
echo -e "${GREEN}Passed: $PASS_COUNT${NC}" | tee -a $TEST_LOG
echo -e "${RED}Failed: $FAIL_COUNT${NC}" | tee -a $TEST_LOG

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}All simplified tests passed!${NC}" | tee -a $TEST_LOG
    exit 0
else
    echo -e "${YELLOW}Some tests failed - TUI may have initialization issues${NC}" | tee -a $TEST_LOG
    exit 1
fi