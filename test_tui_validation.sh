#!/bin/bash

# TUI Validation Test Runner
# This script validates that the TUI implementation matches the server UI and desktop implementations

set -e

echo "🚀 Starting TUI Validation Tests"
echo "================================="

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TERRAPHIM_SERVER=${TERRAPHIM_SERVER:-"http://localhost:8000"}
SERVER_TIMEOUT=${SERVER_TIMEOUT:-30}
TEST_TIMEOUT=${TEST_TIMEOUT:-300}

echo -e "${BLUE}Server URL: $TERRAPHIM_SERVER${NC}"
echo -e "${BLUE}Test Timeout: ${TEST_TIMEOUT}s${NC}"

# Function to check if server is running
check_server() {
    echo -e "${YELLOW}Checking if server is running...${NC}"
    if curl -f -s --connect-timeout 5 "$TERRAPHIM_SERVER/health" > /dev/null; then
        echo -e "${GREEN}✅ Server is running${NC}"
        return 0
    else
        echo -e "${RED}❌ Server is not running at $TERRAPHIM_SERVER${NC}"
        return 1
    fi
}

# Function to start server if not running
start_server_if_needed() {
    if ! check_server; then
        echo -e "${YELLOW}🚧 Starting server...${NC}"

        # Check if we're in the right directory
        if [[ ! -f "Cargo.toml" ]] || [[ ! -d "terraphim_server" ]]; then
            echo -e "${RED}❌ Not in project root directory${NC}"
            echo "Please run this script from the terraphim-ai project root"
            exit 1
        fi

        # Build and start server in background
        echo -e "${BLUE}Building server...${NC}"
        cargo build --bin terraphim_server --release

        echo -e "${BLUE}Starting server in background...${NC}"
        RUST_LOG=info cargo run --bin terraphim_server --release -- --config terraphim_engineer_config.json &
        SERVER_PID=$!

        # Wait for server to be ready
        echo -e "${YELLOW}Waiting for server to be ready...${NC}"
        for i in $(seq 1 $SERVER_TIMEOUT); do
            if check_server; then
                echo -e "${GREEN}✅ Server is ready (${i}s)${NC}"
                break
            fi
            if [[ $i -eq $SERVER_TIMEOUT ]]; then
                echo -e "${RED}❌ Server failed to start within ${SERVER_TIMEOUT}s${NC}"
                kill $SERVER_PID 2>/dev/null || true
                exit 1
            fi
            sleep 1
        done
    else
        SERVER_PID=""
    fi
}

# Function to run tests with timeout
run_test() {
    local test_name="$1"
    local test_command="$2"
    local description="$3"

    echo -e "\n${BLUE}🧪 Running: $test_name${NC}"
    echo -e "${YELLOW}   $description${NC}"

    if eval "timeout ${TEST_TIMEOUT} $test_command"; then
        echo -e "${GREEN}✅ $test_name PASSED${NC}"
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            echo -e "${RED}⏰ $test_name TIMED OUT (${TEST_TIMEOUT}s)${NC}"
        else
            echo -e "${RED}❌ $test_name FAILED (exit code: $exit_code)${NC}"
        fi
        return 1
    fi
}

# Function to cleanup
cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        echo -e "\n${YELLOW}🧹 Cleaning up server (PID: $SERVER_PID)...${NC}"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
}

# Set up cleanup on exit
trap cleanup EXIT

# Start server if needed
start_server_if_needed

# Track test results
PASSED_TESTS=0
FAILED_TESTS=0
TOTAL_TESTS=0

# Test 1: TUI Unit Tests
echo -e "\n${BLUE}📋 Phase 1: Unit Tests${NC}"
if run_test "TUI Unit Tests" \
    "cargo test -p terraphim_tui unit_test" \
    "Testing serialization, deserialization, and basic client functionality"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Test 2: TUI Integration Tests
echo -e "\n${BLUE}📋 Phase 2: Integration Tests${NC}"
export TERRAPHIM_SERVER
if run_test "TUI Integration Tests" \
    "cargo test -p terraphim_tui integration_test" \
    "Testing TUI client against live server API"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Test 3: TUI Error Handling Tests
if run_test "TUI Error Handling Tests" \
    "cargo test -p terraphim_tui error_handling_test" \
    "Testing error scenarios, timeouts, and edge cases"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Test 4: TUI vs Desktop Parity Tests
echo -e "\n${BLUE}📋 Phase 3: Parity Validation${NC}"
if run_test "TUI-Desktop Parity Tests" \
    "cargo test -p terraphim_server tui_desktop_parity_test" \
    "Validating TUI and desktop produce identical results"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Test 5: CLI Command Tests
echo -e "\n${BLUE}📋 Phase 4: CLI Command Validation${NC}"
if run_test "TUI CLI Search Command" \
    "cargo run -p terraphim_tui -- search 'test query' --limit 3" \
    "Testing CLI search functionality"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

if run_test "TUI CLI Roles List" \
    "cargo run -p terraphim_tui -- roles list" \
    "Testing CLI roles listing"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

if run_test "TUI CLI Config Show" \
    "cargo run -p terraphim_tui -- config show" \
    "Testing CLI config display"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

if run_test "TUI CLI Graph Display" \
    "cargo run -p terraphim_tui -- graph --top-k 5" \
    "Testing CLI graph visualization"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Test 6: Desktop Integration Tests (if available)
if [[ -d "desktop" ]] && command -v yarn &> /dev/null; then
    echo -e "\n${BLUE}📋 Phase 5: Desktop Integration Tests${NC}"
    if run_test "Desktop Search Tests" \
        "cd desktop && yarn test Search.test.ts" \
        "Testing desktop search component"; then
        ((PASSED_TESTS++))
    else
        ((FAILED_TESTS++))
    fi
    ((TOTAL_TESTS++))
else
    echo -e "\n${YELLOW}⏭️  Skipping desktop tests (yarn or desktop directory not found)${NC}"
fi

# Test 7: Performance and Load Tests
echo -e "\n${BLUE}📋 Phase 6: Performance Validation${NC}"
if run_test "TUI Performance Test" \
    "cargo test -p terraphim_tui test_concurrent_request_handling" \
    "Testing concurrent request handling"; then
    ((PASSED_TESTS++))
else
    ((FAILED_TESTS++))
fi
((TOTAL_TESTS++))

# Final Results
echo -e "\n${BLUE}📊 Test Results Summary${NC}"
echo "=========================="
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"
echo -e "${BLUE}Total:  $TOTAL_TESTS${NC}"

if [[ $FAILED_TESTS -eq 0 ]]; then
    echo -e "\n${GREEN}🎉 All tests passed! TUI implementation is validated.${NC}"
    exit 0
else
    echo -e "\n${RED}💥 $FAILED_TESTS test(s) failed. TUI implementation needs fixes.${NC}"

    # Provide helpful suggestions
    echo -e "\n${YELLOW}💡 Troubleshooting suggestions:${NC}"
    echo "1. Ensure server is running and accessible"
    echo "2. Check server logs for errors"
    echo "3. Verify configuration files are valid"
    echo "4. Run individual test commands for detailed output"
    echo "5. Check network connectivity and firewall settings"

    exit 1
fi
