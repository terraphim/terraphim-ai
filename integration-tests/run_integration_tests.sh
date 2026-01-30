#!/bin/bash

# Terraphim AI Release Validation Integration Testing Framework
# Comprehensive integration testing for release validation

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FRAMEWORK_DIR="${SCRIPT_DIR}/framework"
SCENARIOS_DIR="${SCRIPT_DIR}/scenarios"
MATRIX_DIR="${SCRIPT_DIR}/matrix"
PERFORMANCE_DIR="${SCRIPT_DIR}/performance"
SECURITY_DIR="${SCRIPT_DIR}/security"
DOCKER_DIR="${SCRIPT_DIR}/docker"
CI_DIR="${SCRIPT_DIR}/ci"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_header() {
    echo -e "${PURPLE}========================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}========================================${NC}"
}

# Test results tracking
TEST_RESULTS_FILE="/tmp/terraphim_integration_results_$(date +%Y%m%d_%H%M%S).json"
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Initialize test results
init_test_results() {
    cat > "$TEST_RESULTS_FILE" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "framework_version": "1.0.0",
  "results": {
    "multi_component": [],
    "data_flow": [],
    "cross_platform": [],
    "error_handling": [],
    "performance": [],
    "security": []
  },
  "summary": {
    "total": 0,
    "passed": 0,
    "failed": 0,
    "skipped": 0,
    "coverage": 0.0
  }
}
EOF
}

# Update test results
update_test_result() {
    local category="$1"
    local test_name="$2"
    local status="$3"
    local duration="$4"
    local details="$5"

    # Update counters
    ((TOTAL_TESTS++))
    case "$status" in
        "passed") ((PASSED_TESTS++)) ;;
        "failed") ((FAILED_TESTS++)) ;;
        "skipped") ((SKIPPED_TESTS++)) ;;
    esac

    # Update JSON results
    jq --arg category "$category" \
       --arg test_name "$test_name" \
       --arg status "$status" \
       --arg duration "$duration" \
       --arg details "$details" \
       --arg timestamp "$(date -Iseconds)" \
       ".results.$category += [{\"name\": \$test_name, \"status\": \$status, \"duration\": \$duration, \"details\": \$details, \"timestamp\": \$timestamp}]" \
       "$TEST_RESULTS_FILE" > "${TEST_RESULTS_FILE}.tmp" && mv "${TEST_RESULTS_FILE}.tmp" "$TEST_RESULTS_FILE"
}

# Finalize test results
finalize_test_results() {
    local coverage_percentage=$(( PASSED_TESTS * 100 / TOTAL_TESTS ))

    jq --arg total "$TOTAL_TESTS" \
       --arg passed "$PASSED_TESTS" \
       --arg failed "$FAILED_TESTS" \
       --arg skipped "$SKIPPED_TESTS" \
       --arg coverage "$coverage_percentage" \
       '.summary = {"total": $total|tonumber, "passed": $passed|tonumber, "failed": $failed|tonumber, "skipped": $skipped|tonumber, "coverage": $coverage|tonumber}' \
       "$TEST_RESULTS_FILE" > "${TEST_RESULTS_FILE}.tmp" && mv "${TEST_RESULTS_FILE}.tmp" "$TEST_RESULTS_FILE"

    log_header "INTEGRATION TEST SUMMARY"
    echo "Total Tests: $TOTAL_TESTS"
    echo "Passed: $PASSED_TESTS"
    echo "Failed: $FAILED_TESTS"
    echo "Skipped: $SKIPPED_TESTS"
    echo "Coverage: ${coverage_percentage}%"
    echo ""
    echo "Detailed results saved to: $TEST_RESULTS_FILE"
}

# Health check functions
check_dependencies() {
    log_info "Checking dependencies..."

    # Check required tools
    local required_tools=("docker" "docker-compose" "cargo" "node" "npm" "curl" "jq")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool not found: $tool"
            exit 1
        fi
    done

    # Check Rust version
    if ! cargo --version | grep -q "cargo 1\."; then
        log_error "Rust/Cargo version 1.x required"
        exit 1
    fi

    # Check Node version
    if ! node --version | grep -q "v18\|v19\|v20"; then
        log_warning "Node.js version 18+ recommended (current: $(node --version))"
    fi

    log_success "All dependencies satisfied"
}

# Environment setup
setup_test_environment() {
    log_info "Setting up test environment..."

    # Create test directories
    mkdir -p /tmp/terraphim_integration_test_{server,client,shared}

    # Set environment variables
    export TERRAPHIM_TEST_MODE=true
    export RUST_LOG=debug
    export RUST_BACKTRACE=1
    export NODE_ENV=test

    # Clean up any existing containers
    docker-compose -f "${DOCKER_DIR}/docker-compose.test.yml" down -v 2>/dev/null || true

    log_success "Test environment ready"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."

    # Stop all test services
    docker-compose -f "${DOCKER_DIR}/docker-compose.test.yml" down -v 2>/dev/null || true

    # Remove test directories
    rm -rf /tmp/terraphim_integration_test_*

    # Kill any remaining processes
    pkill -f "terraphim.*test" || true
    pkill -f "integration.*test" || true

    log_success "Cleanup completed"
}

# Trap cleanup on exit
trap cleanup EXIT

# Main execution
main() {
    local start_time=$(date +%s)

    log_header "TERRAPHIM AI INTEGRATION TESTING FRAMEWORK"
    echo "Framework Directory: $FRAMEWORK_DIR"
    echo "Scenarios Directory: $SCENARIOS_DIR"
    echo "Results File: $TEST_RESULTS_FILE"
    echo ""

    # Initialize
    init_test_results
    check_dependencies
    setup_test_environment

    # Run test phases
    log_header "PHASE 1: MULTI-COMPONENT INTEGRATION TESTING"
    if [[ "${SKIP_MULTI_COMPONENT:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/multi_component_tests.sh"
    else
        log_info "Skipping multi-component tests (--skip-multi-component)"
    fi

    log_header "PHASE 2: DATA FLOW VALIDATION"
    if [[ "${SKIP_DATA_FLOW:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/data_flow_tests.sh"
    else
        log_info "Skipping data flow tests (--skip-data-flow)"
    fi

    log_header "PHASE 3: CROSS-PLATFORM INTEGRATION"
    if [[ "${SKIP_CROSS_PLATFORM:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/cross_platform_tests.sh"
    else
        log_info "Skipping cross-platform tests (--skip-cross-platform)"
    fi

    log_header "PHASE 4: ERROR HANDLING AND RECOVERY"
    if [[ "${SKIP_ERROR_HANDLING:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/error_handling_tests.sh"
    else
        log_info "Skipping error handling tests (--skip-error-handling)"
    fi

    log_header "PHASE 5: PERFORMANCE AND SCALABILITY"
    if [[ "${SKIP_PERFORMANCE:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/performance_tests.sh"
    else
        log_info "Skipping performance tests (--skip-performance)"
    fi

    log_header "PHASE 6: SECURITY INTEGRATION TESTING"
    if [[ "${SKIP_SECURITY:-false}" != "true" ]]; then
        bash "${SCENARIOS_DIR}/security_tests.sh"
    else
        log_info "Skipping security tests (--skip-security)"
    fi

    # Generate final report
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    finalize_test_results

    log_header "FRAMEWORK EXECUTION COMPLETE"
    echo "Total execution time: ${duration}s"

    # Exit with appropriate code
    if [[ $FAILED_TESTS -gt 0 ]]; then
        log_error "Integration tests failed: $FAILED_TESTS tests failed"
        exit 1
    elif [[ $coverage_percentage -lt 95 ]]; then
        log_warning "Integration coverage below 95%: ${coverage_percentage}%"
        exit 1
    else
        log_success "All integration tests passed with ${coverage_percentage}% coverage"
        exit 0
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-multi-component) SKIP_MULTI_COMPONENT=true ;;
            --skip-data-flow) SKIP_DATA_FLOW=true ;;
            --skip-cross-platform) SKIP_CROSS_PLATFORM=true ;;
            --skip-error-handling) SKIP_ERROR_HANDLING=true ;;
            --skip-performance) SKIP_PERFORMANCE=true ;;
            --skip-security) SKIP_SECURITY=true ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --skip-multi-component    Skip multi-component integration tests"
                echo "  --skip-data-flow         Skip data flow validation tests"
                echo "  --skip-cross-platform    Skip cross-platform integration tests"
                echo "  --skip-error-handling    Skip error handling and recovery tests"
                echo "  --skip-performance       Skip performance and scalability tests"
                echo "  --skip-security          Skip security integration tests"
                echo "  --help, -h              Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
        shift
    done
}

# Entry point
parse_args "$@"
main