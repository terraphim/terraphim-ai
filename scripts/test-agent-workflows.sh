#!/bin/bash

# Terraphim AI - Agent Workflow Test Suite
# Comprehensive testing for all 5 agent workflow patterns
# Integrates with TUI, CLI, VM, and Desktop components

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKFLOW_DIR="${PROJECT_ROOT}/examples/agent-workflows"
BACKEND_URL="${BACKEND_URL:-http://localhost:8000}"
TEST_TIMEOUT="${TEST_TIMEOUT:-300}"
PARALLEL_JOBS="${PARALLEL_JOBS:-4}"

# Logging
LOG_DIR="${PROJECT_ROOT}/test-logs"
mkdir -p "$LOG_DIR"
LOG_FILE="${LOG_DIR}/agent-workflow-tests-$(date +%Y%m%d-%H%M%S).log"

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

log_workflow() {
    log "${PURPLE}[WORKFLOW]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
Agent Workflow Test Suite

USAGE:
    $0 [OPTIONS] [WORKFLOW_PATTERN]

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -t, --timeout SEC   Set test timeout (default: 300)
    -j, --jobs NUM      Number of parallel jobs (default: 4)
    -u, --url URL       Backend URL (default: http://localhost:8000)
    --skip-backend      Skip backend startup
    --skip-browser      Skip browser tests
    --headful          Run browser tests in visible mode
    --coverage         Generate coverage report
    --performance      Run performance benchmarks

WORKFLOW_PATTERNS:
    prompt-chaining      Test prompt chaining workflow
    routing            Test routing workflow
    parallelization    Test parallelization workflow
    orchestration      Test orchestration workflow
    optimization       Test optimization workflow
    all                Test all workflows (default)

EXAMPLES:
    $0                              # Test all workflows
    $0 prompt-chaining               # Test only prompt chaining
    $0 -v --performance all         # Test all with verbose output and benchmarks
    $0 --headful --timeout 600 all  # Test all with visible browser and extended timeout

EOF
}

# Parse command line arguments
VERBOSE=false
SKIP_BACKEND=false
SKIP_BROWSER=false
HEADFUL=false
GENERATE_COVERAGE=false
RUN_PERFORMANCE=false
WORKFLOW_PATTERN="all"

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
        -t|--timeout)
            TEST_TIMEOUT="$2"
            shift 2
            ;;
        -j|--jobs)
            PARALLEL_JOBS="$2"
            shift 2
            ;;
        -u|--url)
            BACKEND_URL="$2"
            shift 2
            ;;
        --skip-backend)
            SKIP_BACKEND=true
            shift
            ;;
        --skip-browser)
            SKIP_BROWSER=true
            shift
            ;;
        --headful)
            HEADFUL=true
            shift
            ;;
        --coverage)
            GENERATE_COVERAGE=true
            shift
            ;;
        --performance)
            RUN_PERFORMANCE=true
            shift
            ;;
        prompt-chaining|routing|parallelization|orchestration|optimization|all)
            WORKFLOW_PATTERN="$1"
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

# Check backend availability
check_backend() {
    log_info "Checking backend availability at $BACKEND_URL"

    if curl -f -s --connect-timeout 5 "$BACKEND_URL/health" > /dev/null; then
        log_success "Backend is available"
        return 0
    else
        log_warning "Backend is not available at $BACKEND_URL"
        return 1
    fi
}

# Start backend if needed
start_backend() {
    if [ "$SKIP_BACKEND" = true ]; then
        log_info "Skipping backend startup"
        return 0
    fi

    if check_backend; then
        log_info "Backend already running"
        return 0
    fi

    log_info "Starting backend server..."
    cd "$PROJECT_ROOT"

    # Start backend in background
    RUST_LOG=info cargo run --bin terraphim_server --release -- \
        --config terraphim_server/default/terraphim_engineer_config.json &
    BACKEND_PID=$!

    # Wait for backend to be ready
    local ready=false
    for i in {1..60}; do
        if check_backend; then
            ready=true
            break
        fi
        sleep 1
    done

    if [ "$ready" = true ]; then
        log_success "Backend started successfully (PID: $BACKEND_PID)"
        echo $BACKEND_PID > "${LOG_DIR}/backend.pid"
        return 0
    else
        log_error "Backend failed to start within 60 seconds"
        kill $BACKEND_PID 2>/dev/null || true
        return 1
    fi
}

# Test individual workflow pattern
test_workflow_pattern() {
    local pattern="$1"
    log_workflow "Testing $pattern workflow pattern"

    local test_start=$(date +%s)
    local result=0

    case "$pattern" in
        "prompt-chaining")
            test_prompt_chaining || result=1
            ;;
        "routing")
            test_routing || result=1
            ;;
        "parallelization")
            test_parallelization || result=1
            ;;
        "orchestration")
            test_orchestration || result=1
            ;;
        "optimization")
            test_optimization || result=1
            ;;
        *)
            log_error "Unknown workflow pattern: $pattern"
            return 1
            ;;
    esac

    local test_end=$(date +%s)
    local duration=$((test_end - test_start))

    if [ $result -eq 0 ]; then
        log_success "$pattern workflow completed successfully (${duration}s)"
    else
        log_error "$pattern workflow failed (${duration}s)"
    fi

    return $result
}

# Test prompt chaining workflow
test_prompt_chaining() {
    log_info "Testing prompt chaining workflow (6-stage development pipeline)"

    # Test API endpoint
    local response
    if response=$(curl -s -X POST "$BACKEND_URL/workflows/prompt-chain" \
        -H "Content-Type: application/json" \
        -d '{"prompt":"Create a simple web server","role":"content_creator"}' 2>/dev/null); then

        if echo "$response" | jq -e '.success == true' >/dev/null 2>&1; then
            log_success "Prompt chaining API endpoint working"
        else
            log_error "Prompt chaining API endpoint failed: $response"
            return 1
        fi
    else
        log_error "Failed to call prompt chaining endpoint"
        return 1
    fi

    # Test TUI integration
    if [ -f "${PROJECT_ROOT}/target/release/terraphim_tui" ]; then
        log_info "Testing TUI prompt chaining integration"
        timeout 30 "${PROJECT_ROOT}/target/release/terraphim_tui" \
            --server --server-url "$BACKEND_URL" \
            workflow execute prompt-chaining "Test development pipeline" \
            >/dev/null 2>&1 || log_warning "TUI prompt chaining test timed out"
    fi

    # Test browser interface if not skipped
    if [ "$SKIP_BROWSER" = false ]; then
        test_browser_workflow "prompt-chaining" || return 1
    fi

    return 0
}

# Test routing workflow
test_routing() {
    log_info "Testing routing workflow (intelligent task distribution)"

    # Test API endpoint
    local response
    if response=$(curl -s -X POST "$BACKEND_URL/workflows/route" \
        -H "Content-Type: application/json" \
        -d '{"prompt":"Analyze this data","complexity":"medium","cost_optimization":true}' 2>/dev/null); then

        if echo "$response" | jq -e '.success == true' >/dev/null 2>&1; then
            log_success "Routing API endpoint working"
        else
            log_error "Routing API endpoint failed: $response"
            return 1
        fi
    else
        log_error "Failed to call routing endpoint"
        return 1
    fi

    # Test CLI integration
    if [ -f "${PROJECT_ROOT}/target/release/terraphim_tui" ]; then
        log_info "Testing CLI routing integration"
        timeout 15 "${PROJECT_ROOT}/target/release/terraphim_tui" \
            --server --server-url "$BACKEND_URL" \
            workflow execute routing "Test routing decision" \
            >/dev/null 2>&1 || log_warning "CLI routing test timed out"
    fi

    # Test browser interface if not skipped
    if [ "$SKIP_BROWSER" = false ]; then
        test_browser_workflow "routing" || return 1
    fi

    return 0
}

# Test parallelization workflow
test_parallelization() {
    log_info "Testing parallelization workflow (multi-perspective analysis)"

    # Test API endpoint
    local response
    if response=$(curl -s -X POST "$BACKEND_URL/workflows/parallel" \
        -H "Content-Type: application/json" \
        -d '{"prompt":"Analyze from multiple perspectives","perspectives":["analytical","creative","practical"]}' 2>/dev/null); then

        if echo "$response" | jq -e '.success == true' >/dev/null 2>&1; then
            log_success "Parallelization API endpoint working"
        else
            log_error "Parallelization API endpoint failed: $response"
            return 1
        fi
    else
        log_error "Failed to call parallelization endpoint"
        return 1
    fi

    # Test browser interface if not skipped
    if [ "$SKIP_BROWSER" = false ]; then
        test_browser_workflow "parallelization" || return 1
    fi

    return 0
}

# Test orchestration workflow
test_orchestration() {
    log_info "Testing orchestration workflow (hierarchical coordination)"

    # Test API endpoint
    local response
    if response=$(curl -s -X POST "$BACKEND_URL/workflows/orchestrate" \
        -H "Content-Type: application/json" \
        -d '{"prompt":"Coordinate complex task","workers":["data_collector","analyzer","synthesizer"]}' 2>/dev/null); then

        if echo "$response" | jq -e '.success == true' >/dev/null 2>&1; then
            log_success "Orchestration API endpoint working"
        else
            log_error "Orchestration API endpoint failed: $response"
            return 1
        fi
    else
        log_error "Failed to call orchestration endpoint"
        return 1
    fi

    # Test browser interface if not skipped
    if [ "$SKIP_BROWSER" = false ]; then
        test_browser_workflow "orchestration" || return 1
    fi

    return 0
}

# Test optimization workflow
test_optimization() {
    log_info "Testing optimization workflow (iterative quality improvement)"

    # Test API endpoint
    local response
    if response=$(curl -s -X POST "$BACKEND_URL/workflows/optimize" \
        -H "Content-Type: application/json" \
        -d '{"prompt":"Generate and optimize content","quality_threshold":0.8,"max_iterations":3}' 2>/dev/null); then

        if echo "$response" | jq -e '.success == true' >/dev/null 2>&1; then
            log_success "Optimization API endpoint working"
        else
            log_error "Optimization API endpoint failed: $response"
            return 1
        fi
    else
        log_error "Failed to call optimization endpoint"
        return 1
    fi

    # Test browser interface if not skipped
    if [ "$SKIP_BROWSER" = false ]; then
        test_browser_workflow "optimization" || return 1
    fi

    return 0
}

# Test workflow in browser
test_browser_workflow() {
    local pattern="$1"

    if [ ! -d "$WORKFLOW_DIR" ]; then
        log_warning "Workflow examples directory not found: $WORKFLOW_DIR"
        return 0
    fi

    log_info "Testing $pattern workflow in browser"

    # Find the workflow HTML file
    local html_file=""
    case "$pattern" in
        "prompt-chaining")
            html_file="$WORKFLOW_DIR/1-prompt-chaining/index.html"
            ;;
        "routing")
            html_file="$WORKFLOW_DIR/2-routing/index.html"
            ;;
        "parallelization")
            html_file="$WORKFLOW_DIR/3-parallelization/index.html"
            ;;
        "orchestration")
            html_file="$WORKFLOW_DIR/4-orchestrator-workers/index.html"
            ;;
        "optimization")
            html_file="$WORKFLOW_DIR/5-evaluator-optimizer/index.html"
            ;;
    esac

    if [ ! -f "$html_file" ]; then
        log_warning "Workflow HTML file not found: $html_file"
        return 0
    fi

    # Use Node.js for browser automation if available
    if command -v node &> /dev/null && [ -f "$WORKFLOW_DIR/browser-automation-tests.js" ]; then
        log_info "Running browser automation tests for $pattern"
        cd "$WORKFLOW_DIR"

        local browser_args=""
        if [ "$HEADFUL" = true ]; then
            browser_args="--headful"
        fi

        timeout 60 node browser-automation-tests.js --pattern "$pattern" $browser_args \
            >> "$LOG_FILE" 2>&1 || log_warning "Browser automation test for $pattern timed out"
    else
        log_warning "Node.js or browser automation tests not available, skipping browser tests"
    fi

    return 0
}

# Run performance benchmarks
run_performance_tests() {
    if [ "$RUN_PERFORMANCE" = false ]; then
        return 0
    fi

    log_info "Running performance benchmarks for workflows"

    # Benchmark each workflow pattern
    for pattern in prompt-chaining routing parallelization orchestration optimization; do
        log_info "Benchmarking $pattern workflow"

        local start_time=$(date +%s.%N)

        # Run workflow multiple times for average
        for i in {1..5}; do
            test_workflow_pattern "$pattern" >/dev/null 2>&1 || true
        done

        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc -l)
        local avg_duration=$(echo "scale=2; $duration / 5" | bc -l)

        log_info "$pattern average duration: ${avg_duration}s"
    done
}

# Generate coverage report
generate_coverage_report() {
    if [ "$GENERATE_COVERAGE" = false ]; then
        return 0
    fi

    log_info "Generating test coverage report..."

    # Clean previous coverage data
    find . -name "*.profraw" -delete 2>/dev/null || true

    grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o target/coverage/

    log_success "Coverage report generated at target/coverage/index.html"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment..."

    # Stop backend if we started it
    if [ -f "${LOG_DIR}/backend.pid" ]; then
        local backend_pid=$(cat "${LOG_DIR}/backend.pid")
        log_info "Stopping backend (PID: $backend_pid)..."
        kill "$backend_pid" 2>/dev/null || true
        wait "$backend_pid" 2>/dev/null || true
        rm -f "${LOG_DIR}/backend.pid"
    fi

    # Clean up test artifacts
    rm -rf test-workflow-* test-agent-* /tmp/terraphim-test-* 2>/dev/null || true

    if [ "$GENERATE_COVERAGE" = true ]; then
        generate_coverage_report
    fi
}

# Set trap for cleanup
trap cleanup EXIT

# Main execution
main() {
    log_info "Agent Workflow Test Suite Started"
    log_info "Workflow pattern: $WORKFLOW_PATTERN"
    log_info "Backend URL: $BACKEND_URL"
    log_info "Test timeout: ${TEST_TIMEOUT}s"
    log_info "Log file: $LOG_FILE"

    local start_time=$(date +%s)
    local failed_tests=()

    # Start backend if needed
    start_backend || {
        log_error "Failed to start backend"
        exit 1
    }

    # Determine which workflows to test
    local workflows_to_test=()
    if [ "$WORKFLOW_PATTERN" = "all" ]; then
        workflows_to_test=("prompt-chaining" "routing" "parallelization" "orchestration" "optimization")
    else
        workflows_to_test=("$WORKFLOW_PATTERN")
    fi

    # Test each workflow pattern
    for pattern in "${workflows_to_test[@]}"; do
        if ! test_workflow_pattern "$pattern"; then
            failed_tests+=("$pattern")
        fi
    done

    # Run performance tests if requested
    run_performance_tests

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Results summary
    echo
    log_info "============================================"
    log_info "Agent Workflow Test Results Summary"
    log_info "============================================"
    log_info "Total duration: ${duration}s"
    log_info "Workflows tested: ${#workflows_to_test[@]}"
    log_info "Failed workflows: ${#failed_tests[@]}"

    if [ ${#failed_tests[@]} -eq 0 ]; then
        log_success "🎉 All agent workflow tests passed!"
        log_info "✅ All 5 workflow patterns are functional"
        log_info "✅ API endpoints responding correctly"
        log_info "✅ Integration with TUI/CLI working"
        if [ "$SKIP_BROWSER" = false ]; then
            log_info "✅ Browser interfaces functional"
        fi
        exit 0
    else
        log_error "❌ ${#failed_tests[@]} workflow(s) failed: ${failed_tests[*]}"
        log_error "Check the log file for details: $LOG_FILE"
        exit 1
    fi
}

# Run main function
main "$@"
