#!/bin/bash

# Atomic Server Haystack Integration Test Script
# This script sets up atomic server, populates it with test data, and runs comprehensive Playwright tests

set -e  # Exit on any error

# Configuration
ATOMIC_SERVER_PORT=9883
TERRAPHIM_SERVER_PORT=8000
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/desktop"
ATOMIC_SERVER_DIR="/tmp/atomic_test_$(date +%s)"

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

# Cleanup function
cleanup() {
    log_info "🧹 Cleaning up test environment..."
    
    # Stop atomic server
    if [ ! -z "$ATOMIC_SERVER_PID" ]; then
        log_info "Stopping atomic server (PID: $ATOMIC_SERVER_PID)"
        kill -TERM "$ATOMIC_SERVER_PID" 2>/dev/null || true
        sleep 2
        kill -KILL "$ATOMIC_SERVER_PID" 2>/dev/null || true
    fi
    
    # Stop Terraphim server
    if [ ! -z "$TERRAPHIM_SERVER_PID" ]; then
        log_info "Stopping Terraphim server (PID: $TERRAPHIM_SERVER_PID)"
        kill -TERM "$TERRAPHIM_SERVER_PID" 2>/dev/null || true
        sleep 2
        kill -KILL "$TERRAPHIM_SERVER_PID" 2>/dev/null || true
    fi
    
    # Clean up atomic server data directory
    if [ -d "$ATOMIC_SERVER_DIR" ]; then
        rm -rf "$ATOMIC_SERVER_DIR"
        log_info "Cleaned up atomic server data directory"
    fi
    
    # Clean up any remaining processes
    pkill -f "atomic-server.*$ATOMIC_SERVER_PORT" 2>/dev/null || true
    pkill -f "terraphim_server.*$TERRAPHIM_SERVER_PORT" 2>/dev/null || true
    
    log_success "Cleanup completed"
}

# Set up cleanup trap
trap cleanup EXIT INT TERM

# Check prerequisites
check_prerequisites() {
    log_info "🔍 Checking prerequisites..."
    
    # Check if atomic-server is available
    if ! command -v atomic-server &> /dev/null; then
        log_error "atomic-server command not found. Please install atomic-server."
        log_info "Install with: cargo install atomic-server"
        exit 1
    fi
    
    # Check if Terraphim server binary exists
    TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/release/terraphim_server"
    if [ ! -f "$TERRAPHIM_SERVER_BINARY" ]; then
        TERRAPHIM_SERVER_BINARY="$PROJECT_ROOT/target/debug/terraphim_server"
        if [ ! -f "$TERRAPHIM_SERVER_BINARY" ]; then
            log_error "Terraphim server binary not found. Please build the project first."
            log_info "Build with: cargo build --release"
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

# Start atomic server
start_atomic_server() {
    log_info "🚀 Starting atomic server on port $ATOMIC_SERVER_PORT..."
    
    # Create data directory
    mkdir -p "$ATOMIC_SERVER_DIR"
    
    # Start atomic server in background
    atomic-server \
        --port "$ATOMIC_SERVER_PORT" \
        --data-dir "$ATOMIC_SERVER_DIR" \
        --allow-origin "*" \
        --log-level info \
        > "$ATOMIC_SERVER_DIR/atomic-server.log" 2>&1 &
    
    ATOMIC_SERVER_PID=$!
    
    # Wait for atomic server to start
    log_info "⏳ Waiting for atomic server to start..."
    for i in {1..30}; do
        if curl -s "http://localhost:$ATOMIC_SERVER_PORT" > /dev/null 2>&1; then
            log_success "Atomic server started successfully (PID: $ATOMIC_SERVER_PID)"
            return 0
        fi
        sleep 1
        log_info "Attempt $i/30: Waiting for atomic server..."
    done
    
    log_error "Atomic server failed to start within 30 seconds"
    cat "$ATOMIC_SERVER_DIR/atomic-server.log"
    return 1
}

# Populate atomic server with test data
populate_atomic_server() {
    log_info "📄 Populating atomic server with test documents..."
    
    # Use our existing setup script to populate atomic server
    cd "$PROJECT_ROOT"
    
    if [ -f "scripts/setup_terraphim_full.sh" ]; then
        log_info "Using setup_terraphim_full.sh to populate atomic server..."
        
        # Run the setup script with atomic server parameters
        bash scripts/setup_terraphim_full.sh \
            "http://localhost:$ATOMIC_SERVER_PORT" \
            "test_agent" \
            "http://localhost:$TERRAPHIM_SERVER_PORT" \
            --atomic-only
    else
        # Fallback: use the atomic client directly
        log_info "Using atomic client to populate test documents..."
        
        # Create test documents using atomic client
        ./target/release/terraphim_atomic_client create \
            --server "http://localhost:$ATOMIC_SERVER_PORT" \
            --agent "test_agent" \
            --class "Article" \
            --name "ATOMIC: Terraphim User Guide" \
            --description "Comprehensive guide for using Terraphim with atomic server integration."
        
        ./target/release/terraphim_atomic_client create \
            --server "http://localhost:$ATOMIC_SERVER_PORT" \
            --agent "test_agent" \
            --class "Article" \
            --name "ATOMIC: Search Features" \
            --description "Advanced search capabilities in Terraphim using atomic server backend."
        
        ./target/release/terraphim_atomic_client create \
            --server "http://localhost:$ATOMIC_SERVER_PORT" \
            --agent "test_agent" \
            --class "Article" \
            --name "ATOMIC: Configuration & Roles" \
            --description "Configuration guide for atomic server integration in Terraphim roles."
    fi
    
    log_success "Test documents created in atomic server"
}

# Start Terraphim server
start_terraphim_server() {
    log_info "🚀 Starting Terraphim server on port $TERRAPHIM_SERVER_PORT..."
    
    cd "$PROJECT_ROOT/terraphim_server"
    
    # Start Terraphim server with test configuration
    RUST_LOG=info \
    TEST_MODE=true \
    "$TERRAPHIM_SERVER_BINARY" > /tmp/terraphim_test.log 2>&1 &
    
    TERRAPHIM_SERVER_PID=$!
    
    # Wait for Terraphim server to start
    log_info "⏳ Waiting for Terraphim server to start..."
    for i in {1..30}; do
        if curl -s "http://localhost:$TERRAPHIM_SERVER_PORT/health" > /dev/null 2>&1; then
            log_success "Terraphim server started successfully (PID: $TERRAPHIM_SERVER_PID)"
            return 0
        fi
        sleep 1
        log_info "Attempt $i/30: Waiting for Terraphim server..."
    done
    
    log_error "Terraphim server failed to start within 30 seconds"
    cat /tmp/terraphim_test.log
    return 1
}

# Run Playwright tests
run_tests() {
    log_info "🧪 Running atomic server haystack integration tests..."
    
    cd "$DESKTOP_DIR"
    
    # Set environment variables for tests
    export ATOMIC_SERVER_URL="http://localhost:$ATOMIC_SERVER_PORT"
    export TERRAPHIM_SERVER_URL="http://localhost:$TERRAPHIM_SERVER_PORT"
    export SERVER_BINARY_PATH="$TERRAPHIM_SERVER_BINARY"
    export ATOMIC_SERVER_PATH="$(which atomic-server)"
    
    # Run specific atomic server haystack tests
    if [ "$CI" = "true" ]; then
        log_info "Running tests in CI mode (headless, verbose)"
        yarn playwright test tests/e2e/atomic-server-haystack.spec.ts \
            --reporter=github,html,json \
            --output-dir=test-results \
            --workers=1 \
            --retries=2 \
            --timeout=120000
    else
        log_info "Running tests in development mode"
        yarn playwright test tests/e2e/atomic-server-haystack.spec.ts \
            --reporter=html,list \
            --output-dir=test-results \
            --timeout=60000
    fi
    
    local test_exit_code=$?
    
    if [ $test_exit_code -eq 0 ]; then
        log_success "All atomic server haystack tests passed! 🎉"
    else
        log_error "Some tests failed (exit code: $test_exit_code)"
        log_info "Check test-results/ directory for detailed reports"
    fi
    
    return $test_exit_code
}

# Main execution flow
main() {
    log_info "🎯 Starting Atomic Server Haystack Integration Tests"
    log_info "=================================================="
    
    # Step 1: Check prerequisites
    check_prerequisites
    
    # Step 2: Start atomic server
    start_atomic_server
    
    # Step 3: Populate atomic server with test data
    populate_atomic_server
    
    # Step 4: Start Terraphim server
    start_terraphim_server
    
    # Step 5: Run the tests
    run_tests
    
    local final_exit_code=$?
    
    if [ $final_exit_code -eq 0 ]; then
        log_success "🎉 Atomic Server Haystack Integration Tests completed successfully!"
    else
        log_error "❌ Tests failed. Check the logs above for details."
    fi
    
    return $final_exit_code
}

# Run main function
main "$@" 