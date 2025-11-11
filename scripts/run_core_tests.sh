#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the project root directory
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${BLUE}🧪 Terraphim AI Core Test Suite${NC}"
echo -e "${BLUE}==============================${NC}"
echo ""

# Function to run a test with error handling
run_test() {
    local test_name="$1"
    local test_command="$2"
    local timeout_duration="${3:-300}"  # Default 5 minutes

    echo -e "${BLUE}▶️ $test_name${NC}"
    echo "Command: $test_command"
    echo ""

    # Run with timeout
    if timeout "$timeout_duration" bash -c "$test_command"; then
        echo -e "${GREEN}✅ $test_name - PASSED${NC}"
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}❌ $test_name - TIMEOUT (${timeout_duration}s)${NC}"
        else
            echo -e "${RED}❌ $test_name - FAILED${NC}"
        fi
        return 1
    fi
}

# Parse command line arguments
VERBOSE=false
PARALLEL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --verbose)
            VERBOSE=true
            shift
            ;;
        --parallel)
            PARALLEL=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --verbose    Show detailed test output"
            echo "  --parallel   Run tests in parallel where possible"
            echo "  --help, -h   Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Verbose flag for cargo
CARGO_VERBOSE=""
if [ "$VERBOSE" = "true" ]; then
    CARGO_VERBOSE="-- --nocapture"
fi

echo -e "${BLUE}📋 Core Test Configuration:${NC}"
echo "• Verbose Output: $([ "$VERBOSE" = "true" ] && echo "✅" || echo "❌")"
echo "• Parallel Execution: $([ "$PARALLEL" = "true" ] && echo "✅" || echo "❌")"
echo ""

echo -e "${BLUE}1️⃣ CORE LIBRARY TESTS${NC}"
echo -e "${BLUE}====================${NC}"
echo ""

# Core library tests (fast, no external dependencies)
TOTAL_TESTS=$((TOTAL_TESTS + 3))

if run_test "Terraphim Types Unit Tests" "cargo test -p terraphim_types --lib $CARGO_VERBOSE" 120; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
echo ""

if run_test "Terraphim Automata Unit Tests" "cargo test -p terraphim_automata --lib $CARGO_VERBOSE" 180; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
echo ""

if run_test "Terraphim Persistence Unit Tests" "cargo test -p terraphim_persistence --lib $CARGO_VERBOSE" 300; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
echo ""

echo -e "${BLUE}2️⃣ ADDITIONAL CRATE TESTS${NC}"
echo -e "${BLUE}========================${NC}"
echo ""

# Additional core crates (if they exist and have tests)
ADDITIONAL_CRATES=(
    "terraphim_agent_messaging"
    "terraphim-markdown-parser"
    "haystack_atlassian"
    "haystack_jmap"
)

for crate in "${ADDITIONAL_CRATES[@]}"; do
    if [ -d "crates/$crate" ] && [ -f "crates/$crate/Cargo.toml" ]; then
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        if run_test "$crate Unit Tests" "cargo test -p $crate --lib $CARGO_VERBOSE" 180; then
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            FAILED_TESTS=$((FAILED_TESTS + 1))
        fi
        echo ""
    fi
done

echo -e "${BLUE}3️⃣ BASIC COMPILATION CHECKS${NC}"
echo -e "${BLUE}==========================${NC}"
echo ""

# Basic compilation checks for all workspace crates
TOTAL_TESTS=$((TOTAL_TESTS + 1))

if run_test "Workspace Compilation Check" "cargo check --workspace --all-targets" 300; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi
echo ""

# Test Results Summary
echo -e "${BLUE}📊 CORE TEST RESULTS SUMMARY${NC}"
echo -e "${BLUE}=============================${NC}"
echo ""

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}🎉 ALL CORE TESTS PASSED!${NC}"
else
    echo -e "${RED}⚠️ SOME CORE TESTS FAILED${NC}"
fi

echo ""
echo "📈 Results:"
echo -e "  • Total Tests: $TOTAL_TESTS"
echo -e "  • ${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "  • ${RED}Failed: $FAILED_TESTS${NC}"
echo -e "  • Success Rate: $(( (PASSED_TESTS * 100) / TOTAL_TESTS ))%"

echo ""
echo -e "${BLUE}📝 Additional Information:${NC}"
echo "• Core tests focus on unit tests without external dependencies"
echo "• Integration tests available via: ./scripts/run_integration_tests.sh"
echo "• MCP tests available via: ./scripts/run_mcp_tests.sh"
echo "• Full test suite available via: ./scripts/run_all_tests.sh --category core"

echo ""
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ Core test suite completed successfully!${NC}"
    exit 0
else
    echo -e "${RED}❌ Core test suite completed with failures.${NC}"
    exit 1
fi