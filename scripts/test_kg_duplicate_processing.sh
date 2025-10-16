#!/bin/bash

# Test script for KG duplicate processing fix validation
# This script runs the comprehensive test suite to validate that:
# 1. Documents are processed exactly once per role
# 2. Full KG processing occurs on first attempt
# 3. Duplicate processing is prevented and optimized
# 4. Cache management works correctly

set -e

echo "🧪 Running KG Duplicate Processing Test Suite"
echo "=============================================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to run a test and capture output
run_test() {
    local test_name="$1"
    local test_function="$2"
    echo -e "\n${BLUE}🔍 Running: $test_name${NC}"
    echo "----------------------------------------"
    if cargo test --package terraphim_service --test "$test_function" -- --nocapture; then
        echo -e "${GREEN}✅ PASSED: $test_name${NC}"
        return 0
    else
        echo -e "${RED}❌ FAILED: $test_name${NC}"
        return 1
    fi
}

# Track test results
TOTAL_TESTS=0
PASSED_TESTS=0

# Test 1: Simplified unit tests for duplicate processing prevention
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "KG Duplicate Processing Tests (Basic)" "kg_duplicate_processing_simple_test"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
fi

# Test 2: Comprehensive tests covering edge cases, performance, and real-world scenarios
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "KG Duplicate Processing Tests (Comprehensive)" "kg_duplicate_processing_comprehensive_test"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
fi

# Summary
echo ""
echo "=============================================="
echo -e "${BLUE}📊 TEST SUMMARY${NC}"
echo "=============================================="
echo "Total Tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$((TOTAL_TESTS - PASSED_TESTS))${NC}"

if [ $PASSED_TESTS -eq $TOTAL_TESTS ]; then
    echo ""
    echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
    echo -e "${GREEN}✅ KG duplicate processing fix is working correctly${NC}"
    echo -e "${GREEN}✅ Documents are processed exactly once per role${NC}"
    echo -e "${GREEN}✅ Full processing occurs on first attempt${NC}"
    echo -e "${GREEN}✅ Performance optimizations are effective${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo -e "${YELLOW}Please review the test output above for details${NC}"
    exit 1
fi
