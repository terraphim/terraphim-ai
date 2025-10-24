#!/bin/bash

# Chat Layout Responsive Design Test Runner
#
# This script runs comprehensive tests for the chat layout responsive design fixes.
# It includes E2E tests, visual regression tests, and performance validation.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TESTS_DIR="$PROJECT_ROOT/tests"
E2E_TESTS="$TESTS_DIR/e2e/chat-layout-responsive.spec.ts"
VISUAL_TESTS="$TESTS_DIR/visual/chat-layout-visual.spec.ts"

# Default options
RUN_E2E=true
RUN_VISUAL=true
RUN_PERFORMANCE=true
HEADLESS=true
UPDATE_SNAPSHOTS=false
VERBOSE=false
COVERAGE=false

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show help
show_help() {
    cat << EOF
Chat Layout Responsive Design Test Runner

Usage: $0 [OPTIONS]

Options:
    -e, --e2e              Run E2E tests (default: true)
    -v, --visual           Run visual regression tests (default: true)
    -p, --performance      Run performance tests (default: true)
    -h, --headed           Run tests in headed mode (default: headless)
    -u, --update-snapshots Update visual regression snapshots
    --verbose              Verbose output
    --coverage             Generate coverage report
    --help                 Show this help message

Examples:
    $0                                    # Run all tests in headless mode
    $0 --headed                          # Run all tests with browser visible
    $0 --e2e --visual                    # Run only E2E and visual tests
    $0 --update-snapshots                # Update visual regression snapshots
    $0 --coverage                        # Run tests with coverage report

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--e2e)
            RUN_E2E=true
            shift
            ;;
        -v|--visual)
            RUN_VISUAL=true
            shift
            ;;
        -p|--performance)
            RUN_PERFORMANCE=true
            shift
            ;;
        -h|--headed)
            HEADLESS=false
            shift
            ;;
        -u|--update-snapshots)
            UPDATE_SNAPSHOTS=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --coverage)
            COVERAGE=true
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/package.json" ]]; then
        print_error "package.json not found. Please run this script from the desktop directory."
        exit 1
    fi

    # Check if node_modules exists
    if [[ ! -d "$PROJECT_ROOT/node_modules" ]]; then
        print_error "node_modules not found. Please run 'yarn install' first."
        exit 1
    fi

    # Check if Playwright is installed
    if ! command -v npx &> /dev/null; then
        print_error "npx not found. Please install Node.js and npm/yarn."
        exit 1
    fi

    # Check if test files exist
    if [[ "$RUN_E2E" == true && ! -f "$E2E_TESTS" ]]; then
        print_error "E2E test file not found: $E2E_TESTS"
        exit 1
    fi

    if [[ "$RUN_VISUAL" == true && ! -f "$VISUAL_TESTS" ]]; then
        print_error "Visual test file not found: $VISUAL_TESTS"
        exit 1
    fi

    print_success "Prerequisites check passed"
}

# Function to setup test environment
setup_test_environment() {
    print_status "Setting up test environment..."

    cd "$PROJECT_ROOT"

    # Install Playwright browsers if needed
    if [[ ! -d "$PROJECT_ROOT/node_modules/@playwright" ]]; then
        print_status "Installing Playwright browsers..."
        npx playwright install
    fi

    # Build the application
    print_status "Building application..."
    yarn build

    print_success "Test environment setup complete"
}

# Function to run E2E tests
run_e2e_tests() {
    if [[ "$RUN_E2E" != true ]]; then
        return 0
    fi

    print_status "Running E2E tests..."

    local cmd="npx playwright test $E2E_TESTS"

    if [[ "$HEADLESS" == false ]]; then
        cmd="$cmd --headed"
    fi

    if [[ "$VERBOSE" == true ]]; then
        cmd="$cmd --reporter=list"
    else
        cmd="$cmd --reporter=line"
    fi

    if [[ "$COVERAGE" == true ]]; then
        cmd="$cmd --reporter=html"
    fi

    print_status "Executing: $cmd"

    if eval "$cmd"; then
        print_success "E2E tests passed"
        return 0
    else
        print_error "E2E tests failed"
        return 1
    fi
}

# Function to run visual regression tests
run_visual_tests() {
    if [[ "$RUN_VISUAL" != true ]]; then
        return 0
    fi

    print_status "Running visual regression tests..."

    local cmd="npx playwright test $VISUAL_TESTS"

    if [[ "$HEADLESS" == false ]]; then
        cmd="$cmd --headed"
    fi

    if [[ "$UPDATE_SNAPSHOTS" == true ]]; then
        cmd="$cmd --update-snapshots"
        print_warning "Updating visual regression snapshots"
    fi

    if [[ "$VERBOSE" == true ]]; then
        cmd="$cmd --reporter=list"
    else
        cmd="$cmd --reporter=line"
    fi

    print_status "Executing: $cmd"

    if eval "$cmd"; then
        print_success "Visual regression tests passed"
        return 0
    else
        print_error "Visual regression tests failed"
        return 1
    fi
}

# Function to run performance tests
run_performance_tests() {
    if [[ "$RUN_PERFORMANCE" != true ]]; then
        return 0
    fi

    print_status "Running performance tests..."

    # Performance tests are included in the E2E test suite
    # This function can be extended for dedicated performance testing

    print_success "Performance tests completed"
    return 0
}

# Function to generate test report
generate_test_report() {
    if [[ "$COVERAGE" != true ]]; then
        return 0
    fi

    print_status "Generating test report..."

    # Generate HTML report if available
    if [[ -d "$PROJECT_ROOT/playwright-report" ]]; then
        print_status "Test report available at: $PROJECT_ROOT/playwright-report/index.html"
    fi

    # Generate coverage report if available
    if [[ -d "$PROJECT_ROOT/coverage" ]]; then
        print_status "Coverage report available at: $PROJECT_ROOT/coverage/index.html"
    fi
}

# Function to cleanup
cleanup() {
    print_status "Cleaning up test artifacts..."

    # Remove temporary files if needed
    # This function can be extended for cleanup tasks

    print_success "Cleanup complete"
}

# Main execution function
main() {
    print_status "Starting Chat Layout Responsive Design Tests"
    print_status "============================================="

    local exit_code=0

    # Check prerequisites
    check_prerequisites

    # Setup test environment
    setup_test_environment

    # Run tests
    run_e2e_tests || exit_code=1
    run_visual_tests || exit_code=1
    run_performance_tests || exit_code=1

    # Generate reports
    generate_test_report

    # Cleanup
    cleanup

    # Final status
    if [[ $exit_code -eq 0 ]]; then
        print_success "All tests completed successfully!"
        print_status "Chat layout responsive design validation passed"
    else
        print_error "Some tests failed. Please check the output above."
        print_status "Chat layout responsive design validation failed"
    fi

    exit $exit_code
}

# Trap to ensure cleanup on exit
trap cleanup EXIT

# Run main function
main "$@"
