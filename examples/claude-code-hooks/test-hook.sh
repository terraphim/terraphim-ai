#!/usr/bin/env bash
# Test script for the Terraphim package manager hook
# This script validates that the hook works correctly

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK_SCRIPT="$SCRIPT_DIR/terraphim-package-manager-hook.sh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local input="$2"
    local expected_pattern="$3"

    echo -n "Testing: $test_name... "

    # Run the hook with the input
    local output
    output=$(echo "$input" | "$HOOK_SCRIPT" 2>&1)

    # Check if output matches expected pattern
    if echo "$output" | grep -qiE "$expected_pattern"; then
        echo -e "${GREEN}PASSED${NC}"
        ((TESTS_PASSED++))
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "  Input: $input"
        echo "  Expected pattern: $expected_pattern"
        echo "  Got: $output"
        ((TESTS_FAILED++))
        return 1
    fi
}

echo "================================================"
echo "Terraphim Package Manager Hook - Test Suite"
echo "================================================"
echo ""

# Check if terraphim-tui is available
if ! command -v terraphim-tui &> /dev/null && [ ! -f "$(git rev-parse --show-toplevel 2>/dev/null)/target/release/terraphim-tui" ]; then
    echo -e "${YELLOW}Warning: terraphim-tui not found. Building...${NC}"
    cargo build --release -p terraphim_tui
fi

# Test 1: npm install should become bun install
run_test "npm install replacement" \
    "npm install dependencies" \
    "bun"

# Test 2: yarn build should become bun build
run_test "yarn build replacement" \
    "yarn build the project" \
    "bun"

# Test 3: pnpm test should become bun test
run_test "pnpm test replacement" \
    "pnpm test all cases" \
    "bun"

# Test 4: npm install with multiple commands
run_test "npm install with && chain" \
    "npm install && npm build" \
    "bun"

# Test 5: Case insensitive matching
run_test "case insensitive NPM INSTALL" \
    "NPM INSTALL packages" \
    "bun"

# Test 6: No package manager command (pass through)
run_test "pass through non-package-manager command" \
    "echo hello world" \
    "echo hello world"

# Test 7: Mixed commands
run_test "mixed package managers" \
    "npm install && yarn build && pnpm test" \
    "bun"

echo ""
echo "================================================"
echo "Test Results"
echo "================================================"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ "$TESTS_FAILED" -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
