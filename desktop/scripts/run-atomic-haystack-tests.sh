#!/bin/bash

# Atomic Server Haystack Integration Test Script
# This script runs comprehensive Playwright tests for atomic haystack integration

set -e  # Exit on any error

# Configuration
ATOMIC_SERVER_PORT=9883
TERRAPHIM_SERVER_PORT=8000
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/desktop"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    log_info "🔍 Checking prerequisites..."
    
    # Check if atomic server is running
    if ! curl -s "http://localhost:$ATOMIC_SERVER_PORT" > /dev/null 2>&1; then
        log_error "Atomic server not running on port $ATOMIC_SERVER_PORT"
        log_info "Please ensure atomic server is running: ../atomic-server/target/release/atomic-server --port $ATOMIC_SERVER_PORT"
        exit 1
    fi
    
    # Check environment variables
    if [ -z "$ATOMIC_SERVER_SECRET" ]; then
        log_warning "ATOMIC_SERVER_SECRET not set. Loading from .env file..."
        if [ -f "$PROJECT_ROOT/.env" ]; then
            export $(cat "$PROJECT_ROOT/.env" | grep -v '^#' | xargs)
            log_success "Loaded environment from .env file"
        else
            log_error "No .env file found and ATOMIC_SERVER_SECRET not set"
            exit 1
        fi
    fi
    
    # Check if yarn is available
    if ! command -v yarn &> /dev/null; then
        log_error "yarn command not found. Please install yarn."
        exit 1
    fi
    
    # Check if Playwright is set up
    cd "$DESKTOP_DIR"
    if [ ! -d "node_modules/@playwright" ]; then
        log_warning "Playwright not installed. Installing dependencies..."
        yarn install
    fi
    
    log_success "Prerequisites check completed"
}

# Build Terraphim server if needed
build_terraphim_server() {
    log_info "🔨 Checking Terraphim server build..."
    
    cd "$PROJECT_ROOT"
    
    TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/release/terraphim_server"
    if [ ! -f "$TERRAPHIM_SERVER_BINARY" ]; then
        TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/debug/terraphim_server"
        if [ ! -f "$TERRAPHIM_SERVER_BINARY" ]; then
            log_info "Building Terraphim server..."
            cargo build --release
            if [ $? -ne 0 ]; then
                log_warning "Release build failed, trying debug build..."
                cargo build
                TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/debug/terraphim_server"
            else
                TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/release/terraphim_server"
            fi
        fi
    fi
    
    if [ ! -f "$TERRAPHIM_SERVER_BINARY" ]; then
        log_error "Failed to build Terraphim server"
        exit 1
    fi
    
    log_success "Terraphim server binary ready: $TERRAPHIM_SERVER_BINARY"
}

# Run atomic haystack tests
run_tests() {
    log_info "🧪 Running atomic haystack integration tests..."
    
    cd "$DESKTOP_DIR"
    
    # Set CI environment for consistent behavior
    export CI=true
    
    # Parse command line arguments
    CI_MODE=false
    SPECIFIC_TEST=""
    
    for arg in "$@"; do
        case $arg in
            --ci)
                CI_MODE=true
                ;;
            --test=*)
                SPECIFIC_TEST="${arg#*=}"
                ;;
        esac
    done
    
    # Determine which tests to run
    if [ -n "$SPECIFIC_TEST" ]; then
        TEST_PATTERN="tests/e2e/$SPECIFIC_TEST"
        log_info "Running specific test: $SPECIFIC_TEST"
    else
        TEST_PATTERN="tests/e2e/atomic-server-haystack.spec.ts tests/e2e/atomic-connection.spec.ts tests/e2e/atomic-haystack-search-validation.spec.ts"
        log_info "Running all atomic haystack tests"
    fi
    
    # Configure test execution based on CI mode
    if [ "$CI_MODE" = true ]; then
        PLAYWRIGHT_ARGS="--reporter=github,html,json --workers=1 --retries=3 --timeout=120000"
        log_info "Running in CI mode with enhanced reporting"
    else
        PLAYWRIGHT_ARGS="--reporter=list,html --workers=1 --retries=1 --timeout=60000"
        log_info "Running in development mode"
    fi
    
    # Run the tests
    yarn playwright test $TEST_PATTERN $PLAYWRIGHT_ARGS
    
    TEST_EXIT_CODE=$?
    
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        log_success "All atomic haystack tests passed!"
    else
        log_error "Some atomic haystack tests failed (exit code: $TEST_EXIT_CODE)"
        
        # Show test results location
        if [ -f "test-results/results.json" ]; then
            log_info "Test results available at: $DESKTOP_DIR/test-results/"
        fi
        
        # Show playwright report location
        if [ -f "playwright-report/index.html" ]; then
            log_info "Playwright report: $DESKTOP_DIR/playwright-report/index.html"
        fi
    fi
    
    return $TEST_EXIT_CODE
}

# Display usage information
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --ci                 Run in CI mode with enhanced reporting"
    echo "  --test=<filename>    Run specific test file (e.g., --test=atomic-server-haystack.spec.ts)"
    echo "  --help               Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Run all atomic haystack tests"
    echo "  $0 --ci                              # Run in CI mode"
    echo "  $0 --test=atomic-connection.spec.ts  # Run specific test"
}

# Main execution
main() {
    # Handle help flag
    for arg in "$@"; do
        if [ "$arg" = "--help" ] || [ "$arg" = "-h" ]; then
            show_usage
            exit 0
        fi
    done
    
    log_info "🚀 Starting atomic haystack integration tests..."
    log_info "Project root: $PROJECT_ROOT"
    log_info "Desktop directory: $DESKTOP_DIR"
    
    # Execute test pipeline
    check_prerequisites
    build_terraphim_server
    run_tests "$@"
    
    EXIT_CODE=$?
    
    if [ $EXIT_CODE -eq 0 ]; then
        log_success "🎉 Atomic haystack integration tests completed successfully!"
    else
        log_error "💥 Atomic haystack integration tests failed!"
    fi
    
    exit $EXIT_CODE
}

# Run main function with all arguments
main "$@" 