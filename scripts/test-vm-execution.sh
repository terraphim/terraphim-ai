#!/bin/bash

# VM Execution Test Runner Script
# Comprehensive testing suite for LLM-to-Firecracker VM execution system

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
FCCTL_WEB_URL="http://localhost:8080"
FCCTL_WEB_PID=""
TEST_TIMEOUT=300 # 5 minutes
PARALLEL_JOBS=4

# Logging
LOG_DIR="test-logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/vm-execution-tests-$(date +%Y%m%d-%H%M%S).log"

log() {
    echo -e "$1" | tee -a "$LOG_FILE"
}

log_info() {
    log "${BLUE}[INFO]${NC} $1"
}

log_success() {
    log "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    log "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    log "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
VM Execution Test Runner

USAGE:
    $0 [OPTIONS] [TEST_SUITE]

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -s, --server        Start fcctl-web server automatically
    -k, --keep-server   Keep server running after tests
    -t, --timeout SEC   Set test timeout (default: 300)
    -j, --jobs NUM      Number of parallel test jobs (default: 4)
    --no-cleanup        Don't cleanup test artifacts
    --coverage          Generate test coverage report

TEST_SUITES:
    unit                Run unit tests only
    integration         Run integration tests only
    websocket          Run WebSocket tests only
    e2e                Run end-to-end tests only
    security           Run security tests only
    performance        Run performance tests only
    all                Run all test suites (default)

EXAMPLES:
    $0                  # Run all tests with existing server
    $0 -s               # Start server and run all tests
    $0 unit             # Run only unit tests
    $0 -s -k e2e        # Start server, run e2e tests, keep server running
    $0 --coverage all   # Run all tests with coverage

ENVIRONMENT:
    FCCTL_WEB_URL      fcctl-web server URL (default: http://localhost:8080)
    RUST_LOG           Rust logging level (default: info)
    TEST_TIMEOUT       Test timeout in seconds
EOF
}

# Parse command line arguments
VERBOSE=false
START_SERVER=false
KEEP_SERVER=false
NO_CLEANUP=false
GENERATE_COVERAGE=false
TEST_SUITE="all"

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -s|--server)
            START_SERVER=true
            shift
            ;;
        -k|--keep-server)
            KEEP_SERVER=true
            shift
            ;;
        -t|--timeout)
            TEST_TIMEOUT="$2"
            shift 2
            ;;
        -j|--jobs)
            PARALLEL_JOBS="$2"
            shift 2
            ;;
        --no-cleanup)
            NO_CLEANUP=true
            shift
            ;;
        --coverage)
            GENERATE_COVERAGE=true
            shift
            ;;
        unit|integration|websocket|e2e|security|performance|all)
            TEST_SUITE="$1"
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Set verbose output
if [ "$VERBOSE" = true ]; then
    set -x
    export RUST_LOG="${RUST_LOG:-debug}"
else
    export RUST_LOG="${RUST_LOG:-info}"
fi

# Coverage setup
if [ "$GENERATE_COVERAGE" = true ]; then
    export CARGO_INCREMENTAL=0
    export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
    export RUSTDOCFLAGS="-Cpanic=abort"

    # Install grcov if not present
    if ! command -v grcov &> /dev/null; then
        log_info "Installing grcov for coverage reporting..."
        cargo install grcov
    fi
fi

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."

    if [ -n "$FCCTL_WEB_PID" ] && [ "$KEEP_SERVER" = false ]; then
        log_info "Stopping fcctl-web server (PID: $FCCTL_WEB_PID)..."
        kill "$FCCTL_WEB_PID" 2>/dev/null || true
        wait "$FCCTL_WEB_PID" 2>/dev/null || true
    fi

    if [ "$NO_CLEANUP" = false ]; then
        # Clean up test artifacts
        rm -rf test-vm-* test-agent-* /tmp/terraphim-test-* 2>/dev/null || true
    fi

    if [ "$GENERATE_COVERAGE" = true ]; then
        generate_coverage_report
    fi
}

# Set trap for cleanup
trap cleanup EXIT

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."

    local missing_deps=()

    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi

    if ! command -v rustc &> /dev/null; then
        missing_deps+=("rustc")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_error "Please install Rust toolchain: https://rustup.rs/"
        exit 1
    fi

    # Check for fcctl-web binary if we need to start server
    if [ "$START_SERVER" = true ]; then
        if [ ! -f "scratchpad/firecracker-rust/fcctl-web/target/debug/fcctl-web" ] &&
           [ ! -f "scratchpad/firecracker-rust/fcctl-web/target/release/fcctl-web" ]; then
            log_info "Building fcctl-web server..."
            cd scratchpad/firecracker-rust/fcctl-web
            cargo build --release
            cd - > /dev/null
        fi
    fi

    log_success "Dependencies check passed"
}

# Start fcctl-web server
start_server() {
    if [ "$START_SERVER" = false ]; then
        return
    fi

    log_info "Starting fcctl-web server..."

    cd scratchpad/firecracker-rust/fcctl-web

    # Try release build first, then debug
    if [ -f "target/release/fcctl-web" ]; then
        ./target/release/fcctl-web &
    elif [ -f "target/debug/fcctl-web" ]; then
        ./target/debug/fcctl-web &
    else
        log_error "fcctl-web binary not found. Please build it first."
        exit 1
    fi

    FCCTL_WEB_PID=$!
    cd - > /dev/null

    # Wait for server to start
    log_info "Waiting for server to start..."
    for i in {1..30}; do
        if curl -s "$FCCTL_WEB_URL/health" > /dev/null 2>&1; then
            log_success "fcctl-web server started (PID: $FCCTL_WEB_PID)"
            return
        fi
        sleep 1
    done

    log_error "Failed to start fcctl-web server"
    exit 1
}

# Check if server is running
check_server() {
    log_info "Checking fcctl-web server availability..."

    if curl -s "$FCCTL_WEB_URL/health" > /dev/null 2>&1; then
        log_success "fcctl-web server is available at $FCCTL_WEB_URL"
    else
        log_warning "fcctl-web server not available at $FCCTL_WEB_URL"
        if [ "$START_SERVER" = false ]; then
            log_error "Server not running. Use -s flag to start automatically or start manually"
            exit 1
        fi
    fi
}

# Run unit tests
run_unit_tests() {
    log_info "Running unit tests..."

    local test_args=()
    if [ "$VERBOSE" = true ]; then
        test_args+=("--" "--nocapture")
    fi

    if timeout "$TEST_TIMEOUT" cargo test -p terraphim_multi_agent vm_execution "${test_args[@]}" 2>&1 | tee -a "$LOG_FILE"; then
        log_success "Unit tests passed"
        return 0
    else
        log_error "Unit tests failed"
        return 1
    fi
}

# Run integration tests
run_integration_tests() {
    log_info "Running integration tests..."

    cd scratchpad/firecracker-rust/fcctl-web

    local test_args=()
    if [ "$VERBOSE" = true ]; then
        test_args+=("--" "--nocapture")
    fi

    local result=0
    if ! timeout "$TEST_TIMEOUT" cargo test llm_api_tests "${test_args[@]}" 2>&1 | tee -a "../../../$LOG_FILE"; then
        log_error "Integration tests failed"
        result=1
    else
        log_success "Integration tests passed"
    fi

    cd - > /dev/null
    return $result
}

# Run WebSocket tests
run_websocket_tests() {
    log_info "Running WebSocket tests..."

    cd scratchpad/firecracker-rust/fcctl-web

    local test_args=("--ignored")
    if [ "$VERBOSE" = true ]; then
        test_args+=("--" "--nocapture")
    fi

    local result=0
    if ! timeout "$TEST_TIMEOUT" cargo test websocket_tests "${test_args[@]}" 2>&1 | tee -a "../../../$LOG_FILE"; then
        log_error "WebSocket tests failed"
        result=1
    else
        log_success "WebSocket tests passed"
    fi

    cd - > /dev/null
    return $result
}

# Run end-to-end tests
run_e2e_tests() {
    log_info "Running end-to-end tests..."

    local test_args=("--ignored")
    if [ "$VERBOSE" = true ]; then
        test_args+=("--" "--nocapture")
    fi

    if timeout "$TEST_TIMEOUT" cargo test agent_vm_integration_tests "${test_args[@]}" 2>&1 | tee -a "$LOG_FILE"; then
        log_success "End-to-end tests passed"
        return 0
    else
        log_error "End-to-end tests failed"
        return 1
    fi
}

# Run security tests
run_security_tests() {
    log_info "Running security tests..."

    local result=0

    # Unit security tests
    if ! cargo test -p terraphim_multi_agent test_dangerous_code_validation test_code_injection_prevention 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Unit security tests failed"
        result=1
    fi

    # Integration security tests
    cd scratchpad/firecracker-rust/fcctl-web
    if ! cargo test security_tests 2>&1 | tee -a "../../../$LOG_FILE"; then
        log_error "Integration security tests failed"
        result=1
    fi
    cd - > /dev/null

    if [ $result -eq 0 ]; then
        log_success "Security tests passed"
    fi

    return $result
}

# Run performance tests
run_performance_tests() {
    log_info "Running performance tests..."

    local result=0

    # Unit performance tests
    if ! cargo test -p terraphim_multi_agent performance_tests --release 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Unit performance tests failed"
        result=1
    fi

    # Integration performance tests
    cd scratchpad/firecracker-rust/fcctl-web
    if ! cargo test websocket_performance_tests --ignored --release 2>&1 | tee -a "../../../$LOG_FILE"; then
        log_error "WebSocket performance tests failed"
        result=1
    fi
    cd - > /dev/null

    # Agent performance tests
    if ! cargo test agent_performance_tests --ignored --release 2>&1 | tee -a "$LOG_FILE"; then
        log_error "Agent performance tests failed"
        result=1
    fi

    if [ $result -eq 0 ]; then
        log_success "Performance tests passed"
    fi

    return $result
}

# Generate coverage report
generate_coverage_report() {
    if [ "$GENERATE_COVERAGE" = false ]; then
        return
    fi

    log_info "Generating test coverage report..."

    # Clean previous coverage data
    find . -name "*.profraw" -delete 2>/dev/null || true

    grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o target/coverage/

    log_success "Coverage report generated at target/coverage/index.html"
}

# Run test suite
run_test_suite() {
    local suite="$1"
    local failed_tests=()

    case "$suite" in
        "unit")
            run_unit_tests || failed_tests+=("unit")
            ;;
        "integration")
            run_integration_tests || failed_tests+=("integration")
            ;;
        "websocket")
            run_websocket_tests || failed_tests+=("websocket")
            ;;
        "e2e")
            run_e2e_tests || failed_tests+=("e2e")
            ;;
        "security")
            run_security_tests || failed_tests+=("security")
            ;;
        "performance")
            run_performance_tests || failed_tests+=("performance")
            ;;
        "all")
            run_unit_tests || failed_tests+=("unit")
            run_integration_tests || failed_tests+=("integration")
            run_websocket_tests || failed_tests+=("websocket")
            run_e2e_tests || failed_tests+=("e2e")
            run_security_tests || failed_tests+=("security")
            run_performance_tests || failed_tests+=("performance")
            ;;
        *)
            log_error "Unknown test suite: $suite"
            exit 1
            ;;
    esac

    return ${#failed_tests[@]}
}

# Main execution
main() {
    log_info "VM Execution Test Runner Started"
    log_info "Test suite: $TEST_SUITE"
    log_info "Log file: $LOG_FILE"

    check_dependencies

    if [ "$START_SERVER" = true ]; then
        start_server
    else
        check_server
    fi

    local start_time=$(date +%s)

    # Run the specified test suite
    if run_test_suite "$TEST_SUITE"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        log_success "All tests passed! Duration: ${duration}s"
        log_info "Test log available at: $LOG_FILE"

        if [ "$GENERATE_COVERAGE" = true ]; then
            log_info "Coverage report: target/coverage/index.html"
        fi

        exit 0
    else
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))

        log_error "Some tests failed! Duration: ${duration}s"
        log_error "Check test log for details: $LOG_FILE"
        exit 1
    fi
}

# Run main function
main "$@"
