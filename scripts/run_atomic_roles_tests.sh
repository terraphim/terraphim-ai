#!/bin/bash

# Script to run atomic roles end-to-end tests
# This script runs comprehensive tests for the new atomic server roles

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 Running Atomic Roles End-to-End Tests...${NC}"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Please run this script from the project root directory${NC}"
    exit 1
fi

# Check if Atomic Server is running
ATOMIC_SERVER_URL="${ATOMIC_SERVER_URL:-http://localhost:9883}"
echo -e "${BLUE}🔍 Checking if Atomic Server is running at ${ATOMIC_SERVER_URL}...${NC}"

if ! curl -s "${ATOMIC_SERVER_URL}" > /dev/null; then
    echo -e "${RED}❌ Atomic Server is not running at ${ATOMIC_SERVER_URL}${NC}"
    echo -e "${YELLOW}💡 Please start Atomic Server first:${NC}"
    echo -e "${YELLOW}   atomic-server start${NC}"
    echo -e "${YELLOW}   Then populate it with test data:${NC}"
    echo -e "${YELLOW}   ./scripts/populate_atomic_server.sh${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Atomic Server is running${NC}"

# Set environment variables for tests
export RUST_LOG="${RUST_LOG:-terraphim_middleware=debug,terraphim_atomic_client=debug}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

# Function to run a test
run_test() {
    local test_name="$1"
    local test_function="$2"
    local description="$3"

    echo -e "${BLUE}📋 Running: ${description}${NC}"

    if (cd crates/terraphim_middleware && cargo test --test atomic_roles_e2e_test "$test_function" -- --nocapture --test-threads=1); then
        echo -e "${GREEN}✅ ${test_name} passed${NC}"
        return 0
    else
        echo -e "${RED}❌ ${test_name} failed${NC}"
        return 1
    fi
}

# Track test results
passed_tests=0
failed_tests=0

# Run configuration validation test (doesn't require server)
echo -e "${BLUE}📋 Test 1: Configuration Validation${NC}"
if (cd crates/terraphim_middleware && cargo test --test atomic_roles_e2e_test test_atomic_roles_config_validation -- --nocapture); then
    echo -e "${GREEN}✅ Configuration validation test passed${NC}"
    ((passed_tests++))
else
    echo -e "${RED}❌ Configuration validation test failed${NC}"
    ((failed_tests++))
fi

# Check if we have the required environment for integration tests
if [ -z "$ATOMIC_SERVER_SECRET" ]; then
    echo -e "${YELLOW}⚠️ ATOMIC_SERVER_SECRET not set, skipping integration tests${NC}"
    echo -e "${YELLOW}💡 To run integration tests, set ATOMIC_SERVER_SECRET environment variable${NC}"
    echo -e "${YELLOW}   export ATOMIC_SERVER_SECRET=your-secret-here${NC}"
else
    echo -e "${GREEN}✅ ATOMIC_SERVER_SECRET is set, running integration tests${NC}"

    # Run Title Scorer role test
    if run_test "Title Scorer Role" "test_atomic_haystack_title_scorer_role" "Atomic Server with Title Scorer Role"; then
        ((passed_tests++))
    else
        ((failed_tests++))
    fi

    # Run Graph Embeddings role test
    if run_test "Graph Embeddings Role" "test_atomic_haystack_graph_embeddings_role" "Atomic Server with Graph Embeddings Role"; then
        ((passed_tests++))
    else
        ((failed_tests++))
    fi

    # Run role comparison test
    if run_test "Role Comparison" "test_atomic_haystack_role_comparison" "Comparing Title Scorer vs Graph Embeddings Roles"; then
        ((passed_tests++))
    else
        ((failed_tests++))
    fi
fi

# Summary
echo -e "${BLUE}📊 Test Summary:${NC}"
echo -e "${GREEN}✅ Passed: ${passed_tests} tests${NC}"
if [ $failed_tests -gt 0 ]; then
    echo -e "${RED}❌ Failed: ${failed_tests} tests${NC}"
    exit 1
else
    echo -e "${GREEN}🎉 All tests passed!${NC}"
fi

echo -e "${BLUE}🔧 Configuration files created:${NC}"
echo -e "${BLUE}   - atomic_title_scorer_config.json${NC}"
echo -e "${BLUE}   - atomic_graph_embeddings_config.json${NC}"

echo -e "${BLUE}📋 Test files created:${NC}"
echo -e "${BLUE}   - crates/terraphim_middleware/tests/atomic_roles_e2e_test.rs${NC}"

echo -e "${BLUE}🚀 Ready to use the new atomic server roles!${NC}"
