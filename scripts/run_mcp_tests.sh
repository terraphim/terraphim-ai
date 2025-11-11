#!/bin/bash

# Terraphim AI MCP Test Runner
# Dedicated script for MCP-specific testing

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RESULTS_DIR="${PROJECT_ROOT}/test-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
REPORT_FILE="${RESULTS_DIR}/mcp_test_report_${TIMESTAMP}.md"

# Create results directory
mkdir -p "${RESULTS_DIR}"

# Function to print colored output
print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to run MCP tests
run_mcp_tests() {
    print_status "$BLUE" "🔧 Running MCP Server Tests"
    echo "=================================="
    
    local mcp_success=0
    local mcp_total=0
    
    # Test MCP middleware compilation (correct package)
    print_status "$YELLOW" "Testing MCP middleware compilation..."
    if cargo check -p terraphim_middleware --features mcp-rust-sdk; then
        ((mcp_success++))
        print_status "$GREEN" "✅ MCP middleware compilation successful"
    else
        print_status "$RED" "❌ MCP middleware compilation failed"
    fi
    ((mcp_total++))
    
    # Test MCP middleware unit tests
    print_status "$YELLOW" "Testing MCP middleware unit tests..."
    if cargo test -p terraphim_middleware --features mcp-rust-sdk --lib; then
        ((mcp_success++))
        print_status "$GREEN" "✅ MCP middleware unit tests passed"
    else
        print_status "$RED" "❌ MCP middleware unit tests failed"
    fi
    ((mcp_total++))
    
    # Test MCP server compilation (without feature flag)
    print_status "$YELLOW" "Testing MCP server compilation..."
    if cargo check -p terraphim_server; then
        ((mcp_success++))
        print_status "$GREEN" "✅ MCP server compilation successful"
    else
        print_status "$RED" "❌ MCP server compilation failed"
    fi
    ((mcp_total++))
    
    # Test MCP server unit tests
    print_status "$YELLOW" "Testing MCP server unit tests..."
    if cargo test -p terraphim_server --lib; then
        ((mcp_success++))
        print_status "$GREEN" "✅ MCP server unit tests passed"
    else
        print_status "$RED" "❌ MCP server unit tests failed"
    fi
    ((mcp_total++))
    
    # Test MCP integration tests if they exist
    if [ -d "${PROJECT_ROOT}/crates/terraphim_server/tests/mcp" ]; then
        print_status "$YELLOW" "Testing MCP integration tests..."
        if cargo test -p terraphim_server --test '*' -- mcp; then
            ((mcp_success++))
            print_status "$GREEN" "✅ MCP integration tests passed"
        else
            print_status "$RED" "❌ MCP integration tests failed"
        fi
        ((mcp_total++))
    else
        print_status "$YELLOW" "⚠️  MCP integration tests not found"
    fi
    
    return $((mcp_total - mcp_success))
}

# Function to test MCP examples
test_mcp_examples() {
    print_status "$BLUE" "📚 Testing MCP Examples"
    echo "==========================="
    
    local examples_success=0
    local examples_total=0
    
    # Find MCP examples
    local mcp_examples=($(find "${PROJECT_ROOT}/examples" -name "*mcp*" -type f 2>/dev/null || true))
    
    if [ ${#mcp_examples[@]} -eq 0 ]; then
        print_status "$YELLOW" "⚠️  No MCP examples found"
        return 0
    fi
    
    for example in "${mcp_examples[@]}"; do
        print_status "$YELLOW" "Testing example: $(basename "$example")"
        ((examples_total++))
        
        case "$example" in
            *.py)
                if python3 "$example" --help 2>/dev/null; then
                    ((examples_success++))
                    print_status "$GREEN" "✅ Python example works"
                else
                    print_status "$RED" "❌ Python example failed"
                fi
                ;;
            *.js)
                if node "$example" --help 2>/dev/null; then
                    ((examples_success++))
                    print_status "$GREEN" "✅ JavaScript example works"
                else
                    print_status "$RED" "❌ JavaScript example failed"
                fi
                ;;
            *.sh)
                if bash "$example" --help 2>/dev/null || bash "$example" 2>/dev/null; then
                    ((examples_success++))
                    print_status "$GREEN" "✅ Shell example works"
                else
                    print_status "$RED" "❌ Shell example failed"
                fi
                ;;
        esac
    done
    
    return $((examples_total - examples_success))
}

# Function to generate report
generate_report() {
    local mcp_failures=$1
    local examples_failures=$2
    
    cat > "${REPORT_FILE}" << EOF
# MCP Test Report

**Timestamp:** $(date)
**Test Results Directory:** ${RESULTS_DIR}

## Summary

- **MCP Tests:** $([ $mcp_failures -eq 0 ] && echo "PASSED" || echo "FAILED")
- **Examples:** $([ $examples_failures -eq 0 ] && echo "PASSED" || echo "FAILED")

## Test Details

### MCP Server Tests
- Middleware Compilation: $(cargo check -p terraphim_middleware --features mcp-rust-sdk >/dev/null 2>&1 && echo "✅ PASSED" || echo "❌ FAILED")
- Middleware Unit Tests: $(cargo test -p terraphim_middleware --features mcp-rust-sdk --lib >/dev/null 2>&1 && echo "✅ PASSED" || echo "❌ FAILED")
- Server Compilation: $(cargo check -p terraphim_server >/dev/null 2>&1 && echo "✅ PASSED" || echo "❌ FAILED")
- Server Unit Tests: $(cargo test -p terraphim_server --lib >/dev/null 2>&1 && echo "✅ PASSED" || echo "❌ FAILED")
- Integration Tests: $([ -d "${PROJECT_ROOT}/crates/terraphim_server/tests/mcp" ] && echo "Available" || echo "Not Available")

### MCP Examples
- Total Examples Found: $(find "${PROJECT_ROOT}/examples" -name "*mcp*" -type f 2>/dev/null | wc -l)
- Examples Passed: $([ $examples_failures -eq 0 ] && echo "All" || echo "Some")

## Recommendations

EOF

    if [ $mcp_failures -eq 0 ] && [ $examples_failures -eq 0 ]; then
        cat >> "${REPORT_FILE}" << EOF
✅ **All MCP tests passed!** The MCP integration is working correctly.

### Next Steps
- Add more comprehensive MCP integration tests
- Add performance benchmarks for MCP operations
- Consider adding MCP protocol compliance tests
EOF
    else
        cat >> "${REPORT_FILE}" << EOF
⚠️ **Some MCP tests failed.** Please review the failures above.

### Required Actions
- Fix MCP server compilation issues
- Add missing MCP integration tests
- Update MCP examples to work with current API
- Ensure all MCP dependencies are properly configured
EOF
    fi
    
    print_status "$GREEN" "📄 Report generated: ${REPORT_FILE}"
}

# Main execution
main() {
    print_status "$BLUE" "🚀 Starting Terraphim AI MCP Test Suite"
    print_status "$BLUE" "======================================"
    print_status "$BLUE" "Timestamp: ${TIMESTAMP}"
    print_status "$BLUE" "Results will be saved to: ${RESULTS_DIR}"
    echo
    
    # Run MCP tests
    local mcp_failures=0
    if ! run_mcp_tests; then
        mcp_failures=$?
    fi
    
    echo
    
    # Test MCP examples
    local examples_failures=0
    if ! test_mcp_examples; then
        examples_failures=$?
    fi
    
    echo
    
    # Generate report
    generate_report $mcp_failures $examples_failures
    
    # Final status
    local total_failures=$((mcp_failures + examples_failures))
    if [ $total_failures -eq 0 ]; then
        print_status "$GREEN" "🎉 All MCP tests passed!"
        exit 0
    else
        print_status "$RED" "❌ ${total_failures} MCP test categories failed"
        exit 1
    fi
}

# Check if required tools are available
check_dependencies() {
    local missing_deps=()
    
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_status "$RED" "❌ Missing dependencies: ${missing_deps[*]}"
        exit 1
    fi
}

# Run dependency check
check_dependencies

# Execute main function
main "$@"
