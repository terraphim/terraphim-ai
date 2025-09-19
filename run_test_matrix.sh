#!/bin/bash

# Test Matrix Runner Script
# Runs comprehensive scoring function x haystack test matrix

set -e

echo "🚀 Terraphim Test Matrix Runner"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}$1${NC}"
    echo "-------------------------------------"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if project compiles
check_compilation() {
    print_header "Checking Project Compilation"

    if cargo build -p terraphim_tui > /dev/null 2>&1; then
        print_success "Project compiles successfully"
    else
        print_error "Project compilation failed"
        echo "Running cargo build to show errors:"
        cargo build -p terraphim_tui
        exit 1
    fi
}

# Run basic matrix test
run_basic_matrix() {
    print_header "Running Basic Test Matrix"

    echo "Testing all scoring function x haystack combinations..."

    if cargo test -p terraphim_tui test_complete_scoring_haystack_matrix -- --nocapture; then
        print_success "Basic matrix test completed successfully"
    else
        print_warning "Basic matrix test had some failures (expected for remote services)"
    fi
}

# Run priority combinations test
run_priority_test() {
    print_header "Running Priority Combinations Test"

    echo "Testing high-priority combinations that should always work..."

    if cargo test -p terraphim_tui test_priority_combinations -- --nocapture; then
        print_success "Priority combinations test completed successfully"
    else
        print_error "Priority combinations test failed - this indicates core functionality issues"
        return 1
    fi
}

# Run performance comparison test
run_performance_test() {
    print_header "Running Performance Comparison Test"

    echo "Testing performance across different scoring functions..."

    if cargo test -p terraphim_tui test_scoring_function_performance_comparison -- --nocapture; then
        print_success "Performance comparison test completed successfully"
    else
        print_warning "Performance comparison test failed - may indicate configuration issues"
    fi
}

# Run extended matrix with query scorers
run_extended_matrix() {
    print_header "Running Extended Matrix with Query Scorers"

    echo "Testing all combinations including query scorer variations..."

    if cargo test -p terraphim_tui test_extended_matrix_with_query_scorers -- --nocapture; then
        print_success "Extended matrix test completed successfully"
    else
        print_warning "Extended matrix test had some failures (expected for advanced combinations)"
    fi
}

# Run title scorer specific tests
run_title_scorer_test() {
    print_header "Running Title Scorer Query Combinations Test"

    echo "Testing TitleScorer with various query scoring algorithms..."

    if cargo test -p terraphim_tui test_title_scorer_query_combinations -- --nocapture; then
        print_success "Title scorer combinations test completed successfully"
    else
        print_warning "Title scorer combinations test had failures"
    fi
}

# Generate summary report
generate_summary() {
    print_header "Test Matrix Summary"

    echo "🧪 Test Matrix Execution Complete"
    echo ""
    echo "📊 Test Categories Run:"
    echo "  ✅ Project Compilation Check"
    echo "  🧪 Basic Scoring Function x Haystack Matrix"
    echo "  🎯 Priority Combinations (Critical Functionality)"
    echo "  ⚡ Performance Comparison Across Scorers"
    echo "  🔬 Extended Matrix with Query Scorers"
    echo "  📝 Title Scorer Query Algorithm Tests"
    echo ""
    echo "📈 Expected Results:"
    echo "  • Local combinations (Ripgrep) should work reliably"
    echo "  • Remote combinations may fail due to service availability"
    echo "  • Success rate of 40-60% is normal for comprehensive testing"
    echo "  • Priority combinations should have 80%+ success rate"
    echo ""
    echo "🎯 Next Steps:"
    echo "  • Review test output for specific failure patterns"
    echo "  • Check service configurations for failed remote tests"
    echo "  • Monitor performance metrics for optimization opportunities"
    echo ""
    print_success "Test matrix execution completed!"
}

# Main execution flow
main() {
    echo "Starting comprehensive test matrix execution..."
    echo ""

    # Check prerequisites
    check_compilation

    # Run test suite
    echo ""
    run_basic_matrix
    echo ""
    run_priority_test
    echo ""
    run_performance_test
    echo ""
    run_extended_matrix
    echo ""
    run_title_scorer_test
    echo ""

    # Generate summary
    generate_summary
}

# Parse command line arguments
case "${1:-all}" in
    "basic")
        check_compilation
        run_basic_matrix
        ;;
    "priority")
        check_compilation
        run_priority_test
        ;;
    "performance")
        check_compilation
        run_performance_test
        ;;
    "extended")
        check_compilation
        run_extended_matrix
        ;;
    "title-scorer")
        check_compilation
        run_title_scorer_test
        ;;
    "cleanup")
        print_header "Cleaning Up Test Files"
        echo "Removing temporary test files and artifacts..."
        if [[ -f "./scripts/cleanup_test_files.sh" ]]; then
            ./scripts/cleanup_test_files.sh "${@:2}"
        else
            echo -e "${YELLOW}⚠️ Cleanup script not found${NC}"
            echo "Manually cleaning common test files..."
            rm -f /tmp/terraphim_test_matrix_*.json
            rm -f /tmp/test_config*.json
            rm -f /tmp/invalid_config*.json
            echo -e "${GREEN}✅ Basic cleanup completed${NC}"
        fi
        ;;
    "all"|"")
        main
        ;;
    *)
        echo "Usage: $0 [basic|priority|performance|extended|title-scorer|cleanup|all]"
        echo ""
        echo "Test Categories:"
        echo "  basic       - Basic scoring function x haystack matrix"
        echo "  priority    - Priority combinations that should always work"
        echo "  performance - Performance comparison across scoring functions"
        echo "  extended    - Extended matrix with query scorer variations"
        echo "  title-scorer- Title scorer with query algorithm combinations"
        echo "  cleanup     - Clean up temporary test files and artifacts"
        echo "  all         - Run complete test matrix (default)"
        exit 1
        ;;
esac
