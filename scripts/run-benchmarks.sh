#!/bin/bash

# Agent Performance Benchmark Runner
# This script runs comprehensive performance benchmarks for TerraphimAgent operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BENCHMARK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESULTS_DIR="${BENCHMARK_DIR}/benchmark-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

echo -e "${BLUE}🚀 Starting TerraphimAgent Performance Benchmark Suite${NC}"
echo -e "${BLUE}===============================================${NC}"
echo "Timestamp: ${TIMESTAMP}"
echo "Results will be saved to: ${RESULTS_DIR}/${TIMESTAMP}"
echo ""

# Create results directory
mkdir -p "${RESULTS_DIR}/${TIMESTAMP}"

# Function to run Rust benchmarks
run_rust_benchmarks() {
    echo -e "${YELLOW}📊 Running Rust Agent Operation Benchmarks${NC}"
    echo "============================================"

    cd "${BENCHMARK_DIR}/crates/terraphim_multi_agent"

    # Check if criterion dependency is available
    if ! grep -q "criterion" Cargo.toml; then
        echo -e "${RED}❌ Criterion not found in Cargo.toml. Adding dependency...${NC}"
        echo 'criterion = { version = "0.5", features = ["html_reports"] }' >> Cargo.toml
    fi

    # Run benchmarks with detailed output
    echo "Running cargo bench with criterion..."
    CARGO_BENCH_OUTPUT="${RESULTS_DIR}/${TIMESTAMP}/rust_benchmarks.txt"

    if cargo bench --features test-utils --bench agent_operations > "${CARGO_BENCH_OUTPUT}" 2>&1; then
        echo -e "${GREEN}✅ Rust benchmarks completed successfully${NC}"

        # Copy HTML reports if available
        if [ -d "target/criterion" ]; then
            cp -r target/criterion "${RESULTS_DIR}/${TIMESTAMP}/rust_criterion_reports"
            echo -e "${GREEN}📁 Criterion HTML reports saved${NC}"
        fi

        # Extract key metrics
        echo -e "${BLUE}📈 Key Rust Benchmark Results:${NC}"
        grep -E "(time:|slope:|R\^2:)" "${CARGO_BENCH_OUTPUT}" | head -20 || echo "Detailed metrics in ${CARGO_BENCH_OUTPUT}"
    else
        echo -e "${RED}❌ Rust benchmarks failed. Check ${CARGO_BENCH_OUTPUT} for details${NC}"
        return 1
    fi

    cd "${BENCHMARK_DIR}"
}

# Function to run JavaScript/Node.js benchmarks
run_js_benchmarks() {
    echo -e "${YELLOW}🌐 Running JavaScript WebSocket Performance Benchmarks${NC}"
    echo "================================================"

    cd "${BENCHMARK_DIR}/desktop"

    # Check if vitest is available
    if ! command -v npx vitest >/dev/null 2>&1; then
        echo -e "${RED}❌ Vitest not found. Installing dependencies...${NC}"
        yarn install
    fi

    # Run JavaScript benchmarks
    echo "Running Vitest benchmarks..."
    JS_BENCH_OUTPUT="${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks.json"

    if yarn run vitest --config vitest.benchmark.config.ts --reporter=json --outputFile="${JS_BENCH_OUTPUT}" --run; then
        echo -e "${GREEN}✅ JavaScript benchmarks completed successfully${NC}"

        # Extract key metrics from JSON report
        if [ -f "${JS_BENCH_OUTPUT}" ]; then
            echo -e "${BLUE}📈 Key JavaScript Benchmark Results:${NC}"
            # Use jq if available, otherwise just show file location
            if command -v jq >/dev/null 2>&1; then
                jq '.testResults[] | select(.status == "passed") | {name: .name, duration: .duration}' "${JS_BENCH_OUTPUT}" | head -10
            else
                echo "Detailed metrics in ${JS_BENCH_OUTPUT}"
            fi
        fi
    else
        echo -e "${RED}❌ JavaScript benchmarks failed${NC}"
        # Try alternative benchmark run
        echo "Attempting alternative benchmark execution..."
        if npx vitest run tests/benchmarks/ --reporter=verbose > "${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks_alt.txt" 2>&1; then
            echo -e "${GREEN}✅ Alternative JavaScript benchmark completed${NC}"
        else
            echo -e "${RED}❌ All JavaScript benchmark attempts failed${NC}"
            return 1
        fi
    fi

    cd "${BENCHMARK_DIR}"
}

# Function to generate comprehensive report
generate_report() {
    echo -e "${YELLOW}📋 Generating Comprehensive Performance Report${NC}"
    echo "============================================="

    REPORT_FILE="${RESULTS_DIR}/${TIMESTAMP}/performance_report.md"

    cat > "${REPORT_FILE}" << EOF
# TerraphimAgent Performance Benchmark Report

**Generated:** $(date)
**Timestamp:** ${TIMESTAMP}

## Executive Summary

This report contains performance benchmarks for TerraphimAgent operations including:

- Agent creation and initialization
- Command processing across different types
- Memory and context operations
- Knowledge graph queries
- WebSocket communication performance
- Concurrent operation handling

## Benchmark Categories

### 1. Rust Core Agent Operations

**Location:** \`crates/terraphim_multi_agent/benches/agent_operations.rs\`

Key operations benchmarked:
- Agent creation time
- Agent initialization
- Command processing (Generate, Answer, Analyze, Create, Review)
- Registry operations
- Memory operations
- Batch operations
- Concurrent operations
- Knowledge graph operations
- Automata operations
- LLM operations
- Tracking operations

### 2. JavaScript WebSocket Performance

**Location:** \`desktop/tests/benchmarks/agent-performance.benchmark.js\`

Key operations benchmarked:
- WebSocket connection establishment
- Message processing throughput
- Workflow start latency
- Command processing end-to-end
- Concurrent workflow execution
- Memory operation efficiency
- Error handling performance
- Throughput under load

## Performance Thresholds

The benchmarks include automated threshold checking for:

- WebSocket Connection: avg < 500ms, p95 < 1000ms
- Message Processing: avg < 100ms, p95 < 200ms
- Workflow Start: avg < 2000ms, p95 < 5000ms
- Command Processing: avg < 3000ms, p95 < 10000ms
- Memory Operations: avg < 50ms, p95 < 100ms

## Raw Results

### Rust Benchmarks
EOF

    # Append Rust results if available
    if [ -f "${RESULTS_DIR}/${TIMESTAMP}/rust_benchmarks.txt" ]; then
        echo "" >> "${REPORT_FILE}"
        echo "\`\`\`" >> "${REPORT_FILE}"
        cat "${RESULTS_DIR}/${TIMESTAMP}/rust_benchmarks.txt" >> "${REPORT_FILE}"
        echo "\`\`\`" >> "${REPORT_FILE}"
    fi

    cat >> "${REPORT_FILE}" << EOF

### JavaScript Benchmarks
EOF

    # Append JavaScript results if available
    if [ -f "${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks.json" ]; then
        echo "" >> "${REPORT_FILE}"
        echo "\`\`\`json" >> "${REPORT_FILE}"
        cat "${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks.json" >> "${REPORT_FILE}"
        echo "\`\`\`" >> "${REPORT_FILE}"
    elif [ -f "${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks_alt.txt" ]; then
        echo "" >> "${REPORT_FILE}"
        echo "\`\`\`" >> "${REPORT_FILE}"
        cat "${RESULTS_DIR}/${TIMESTAMP}/js_benchmarks_alt.txt" >> "${REPORT_FILE}"
        echo "\`\`\`" >> "${REPORT_FILE}"
    fi

    cat >> "${REPORT_FILE}" << EOF

## Recommendations

Based on benchmark results:

1. **Performance Hotspots:** Identify operations that consistently exceed thresholds
2. **Scaling Limits:** Note concurrency levels where performance degrades
3. **Memory Efficiency:** Monitor memory operations for optimization opportunities
4. **Network Performance:** Evaluate WebSocket communication patterns

## Files Generated

- Performance Report: \`performance_report.md\`
- Rust Benchmarks: \`rust_benchmarks.txt\`
- JavaScript Benchmarks: \`js_benchmarks.json\` or \`js_benchmarks_alt.txt\`
- Criterion Reports: \`rust_criterion_reports/\` (if available)

## Running Benchmarks

To reproduce these benchmarks:

\`\`\`bash
# Run all benchmarks
./scripts/run-benchmarks.sh

# Run only Rust benchmarks
cd crates/terraphim_multi_agent && cargo bench

# Run only JavaScript benchmarks
cd desktop && yarn run vitest --config vitest.benchmark.config.ts
\`\`\`
EOF

    echo -e "${GREEN}✅ Comprehensive report generated: ${REPORT_FILE}${NC}"
}

# Function to check system readiness
check_system_readiness() {
    echo -e "${YELLOW}🔍 Checking System Readiness${NC}"
    echo "============================="

    # Check for required tools
    local missing_tools=()

    if ! command -v cargo >/dev/null 2>&1; then
        missing_tools+=("cargo")
    fi

    if ! command -v node >/dev/null 2>&1; then
        missing_tools+=("node")
    fi

    if ! command -v yarn >/dev/null 2>&1 && ! command -v npm >/dev/null 2>&1; then
        missing_tools+=("yarn or npm")
    fi

    if [ ${#missing_tools[@]} -ne 0 ]; then
        echo -e "${RED}❌ Missing required tools: ${missing_tools[*]}${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ System ready for benchmarking${NC}"
    echo ""
}

# Main execution
main() {
    local run_rust=true
    local run_js=true
    local exit_code=0

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --rust-only)
                run_js=false
                shift
                ;;
            --js-only)
                run_rust=false
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [--rust-only|--js-only]"
                echo ""
                echo "Options:"
                echo "  --rust-only    Run only Rust benchmarks"
                echo "  --js-only      Run only JavaScript benchmarks"
                echo "  --help, -h     Show this help message"
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Execute benchmark suite
    check_system_readiness

    if [ "$run_rust" = true ]; then
        if ! run_rust_benchmarks; then
            exit_code=1
        fi
        echo ""
    fi

    if [ "$run_js" = true ]; then
        if ! run_js_benchmarks; then
            exit_code=1
        fi
        echo ""
    fi

    generate_report

    echo ""
    echo -e "${BLUE}🎯 Benchmark Summary${NC}"
    echo "==================="
    echo -e "Results saved to: ${GREEN}${RESULTS_DIR}/${TIMESTAMP}${NC}"
    echo -e "Report available: ${GREEN}${RESULTS_DIR}/${TIMESTAMP}/performance_report.md${NC}"

    if [ -d "${RESULTS_DIR}/${TIMESTAMP}/rust_criterion_reports" ]; then
        echo -e "Criterion HTML: ${GREEN}${RESULTS_DIR}/${TIMESTAMP}/rust_criterion_reports/index.html${NC}"
    fi

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}✅ All benchmarks completed successfully${NC}"
    else
        echo -e "${YELLOW}⚠️  Some benchmarks failed, but results were generated${NC}"
    fi

    exit $exit_code
}

# Execute main function with all arguments
main "$@"
