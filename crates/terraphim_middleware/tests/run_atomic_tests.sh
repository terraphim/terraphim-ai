#!/bin/bash

# Terraphim Atomic Server Integration Test Runner
# This script helps run atomic server integration tests with proper setup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 Terraphim Atomic Server Integration Test Runner${NC}"
echo "=================================================="

# Check if atomic server is running
check_atomic_server() {
    echo -e "${BLUE}📡 Checking if Atomic Server is running...${NC}"
    
    if curl -s http://localhost:9883 > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Atomic Server is running at http://localhost:9883${NC}"
        return 0
    else
        echo -e "${RED}❌ Atomic Server is not running at http://localhost:9883${NC}"
        echo -e "${YELLOW}💡 Please start Atomic Server before running tests:${NC}"
        echo "   docker run -p 9883:9883 joepmeneer/atomic-server"
        echo "   or download from: https://github.com/atomicdata-dev/atomic-server"
        return 1
    fi
}

# Check environment variables
check_environment() {
    echo -e "${BLUE}🔧 Checking environment variables...${NC}"
    
    if [ -f ".env" ]; then
        echo -e "${GREEN}✅ Found .env file, loading...${NC}"
        source .env
    fi
    
    if [ -z "$ATOMIC_SERVER_URL" ]; then
        echo -e "${YELLOW}⚠️  ATOMIC_SERVER_URL not set, using default: http://localhost:9883${NC}"
        export ATOMIC_SERVER_URL="http://localhost:9883"
    else
        echo -e "${GREEN}✅ ATOMIC_SERVER_URL: $ATOMIC_SERVER_URL${NC}"
    fi
    
    if [ -z "$ATOMIC_SERVER_SECRET" ]; then
        echo -e "${YELLOW}⚠️  ATOMIC_SERVER_SECRET not set${NC}"
        echo -e "${YELLOW}💡 Tests will run with anonymous access (limited functionality)${NC}"
        echo -e "${YELLOW}💡 To get a secret, create an agent in your Atomic Server instance${NC}"
    else
        echo -e "${GREEN}✅ ATOMIC_SERVER_SECRET is set (${#ATOMIC_SERVER_SECRET} chars)${NC}"
    fi
}

# Run tests
run_tests() {
    echo -e "${BLUE}🧪 Running atomic server integration tests...${NC}"
    
    # Set logging level for tests
    export RUST_LOG="${RUST_LOG:-terraphim_middleware=debug,terraphim_atomic_client=debug}"
    export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
    
    echo -e "${BLUE}📋 Test 1: Atomic Haystack Config Integration${NC}"
    if cargo test --test atomic_haystack_config_integration test_atomic_haystack_with_terraphim_config -- --nocapture --test-threads=1; then
        echo -e "${GREEN}✅ Config integration test passed${NC}"
    else
        echo -e "${RED}❌ Config integration test failed${NC}"
        return 1
    fi
    
    echo -e "${BLUE}📋 Test 2: Configuration Validation${NC}"
    if cargo test --test atomic_haystack_config_integration test_atomic_haystack_config_validation -- --nocapture; then
        echo -e "${GREEN}✅ Config validation test passed${NC}"
    else
        echo -e "${RED}❌ Config validation test failed${NC}"
        return 1
    fi
    
    echo -e "${BLUE}📋 Test 3: Invalid Secret Handling${NC}"
    if cargo test --test atomic_haystack_config_integration test_atomic_haystack_invalid_secret -- --nocapture; then
        echo -e "${GREEN}✅ Invalid secret test passed${NC}"
    else
        echo -e "${RED}❌ Invalid secret test failed${NC}"
        return 1
    fi
    
    if [ -n "$ATOMIC_SERVER_SECRET" ]; then
        echo -e "${BLUE}📋 Test 4: Anonymous Access (requires running server)${NC}"
        if cargo test --test atomic_haystack_config_integration test_atomic_haystack_anonymous_access -- --nocapture --ignored; then
            echo -e "${GREEN}✅ Anonymous access test passed${NC}"
        else
            echo -e "${YELLOW}⚠️  Anonymous access test failed (may be expected)${NC}"
        fi
    fi
    
    echo -e "${BLUE}📋 Test 5: Document Import and Search (requires running server)${NC}"
    if cargo test --test atomic_document_import_test -- --nocapture --ignored; then
        echo -e "${GREEN}✅ Document import test passed${NC}"
    else
        echo -e "${YELLOW}⚠️  Document import test failed (requires server + auth)${NC}"
    fi
}

# Show usage examples
show_examples() {
    echo -e "${BLUE}📚 Usage Examples${NC}"
    echo "=================="
    echo
    echo -e "${YELLOW}1. Basic Setup:${NC}"
    echo "   export ATOMIC_SERVER_URL=\"http://localhost:9883\""
    echo "   export ATOMIC_SERVER_SECRET=\"your-base64-secret\""
    echo
    echo -e "${YELLOW}2. Run specific test:${NC}"
    echo "   cargo test --test atomic_haystack_config_integration test_atomic_haystack_with_terraphim_config -- --nocapture"
    echo
    echo -e "${YELLOW}3. Run example configuration:${NC}"
    echo "   cd ../terraphim_config && cargo run --example atomic_server_config"
    echo
    echo -e "${YELLOW}4. Generate atomic server secret:${NC}"
    echo "   # In your atomic server web interface, go to Settings > Agents > Create Agent"
    echo "   # Copy the base64-encoded secret and set it as ATOMIC_SERVER_SECRET"
    echo
}

# Main execution
main() {
    case "$1" in
        "check")
            check_atomic_server
            check_environment
            ;;
        "examples")
            show_examples
            ;;
        "test")
            if ! check_atomic_server; then
                exit 1
            fi
            check_environment
            run_tests
            ;;
        *)
            echo -e "${YELLOW}Usage: $0 {check|test|examples}${NC}"
            echo
            echo "Commands:"
            echo "  check     - Check if Atomic Server is running and environment is set up"
            echo "  test      - Run all atomic server integration tests"
            echo "  examples  - Show usage examples and setup instructions"
            echo
            echo "Environment Variables:"
            echo "  ATOMIC_SERVER_URL    - Atomic server URL (default: http://localhost:9883)"
            echo "  ATOMIC_SERVER_SECRET - Base64-encoded agent secret (required for full tests)"
            echo "  RUST_LOG            - Logging level (default: debug for terraphim modules)"
            exit 1
            ;;
    esac
}

main "$@" 