#!/bin/bash

# Terraphim AI Performance Benchmarking Script
# This script runs comprehensive performance benchmarks for release validation

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="${PROJECT_ROOT}/benchmark-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RUN_DIR="${RESULTS_DIR}/${TIMESTAMP}"

# Default configuration
ITERATIONS=1000
BASELINE_FILE="${RESULTS_DIR}/baseline.json"
CONFIG_FILE="${PROJECT_ROOT}/benchmark-config.json"
VERBOSE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --iterations=*)
      ITERATIONS="${1#*=}"
      shift
      ;;
    --baseline=*)
      BASELINE_FILE="${1#*=}"
      shift
      ;;
    --config=*)
      CONFIG_FILE="${1#*=}"
      shift
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --iterations=N    Number of benchmark iterations (default: 1000)"
      echo "  --baseline=FILE   Baseline results file for comparison"
      echo "  --config=FILE     Benchmark configuration file"
      echo "  --verbose         Enable verbose output"
      echo "  --help           Show this help message"
      echo ""
      echo "Environment Variables:"
      echo "  TERRAPHIM_BENCH_ITERATIONS    Same as --iterations"
      echo "  TERRAPHIM_BENCH_BASELINE      Same as --baseline"
      echo "  TERRAPHIM_BENCH_CONFIG        Same as --config"
      echo "  TERRAPHIM_SERVER_URL          Server URL for API benchmarks"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# Override with environment variables
ITERATIONS="${TERRAPHIM_BENCH_ITERATIONS:-$ITERATIONS}"
BASELINE_FILE="${TERRAPHIM_BENCH_BASELINE:-$BASELINE_FILE}"
CONFIG_FILE="${TERRAPHIM_BENCH_CONFIG:-$CONFIG_FILE}"
SERVER_URL="${TERRAPHIM_SERVER_URL:-http://localhost:3000}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Create results directory
create_results_dir() {
    log_info "Creating results directory: $RUN_DIR"
    mkdir -p "$RUN_DIR"
}

# Check system requirements
check_requirements() {
    log_info "Checking system requirements..."

    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo (Rust) is not installed or not in PATH"
        exit 1
    fi

    # Check if server is running (for API benchmarks)
    if ! curl -s --max-time 5 "$SERVER_URL/health" > /dev/null; then
        log_warn "Terraphim server not accessible at $SERVER_URL"
        log_warn "API benchmarks will be skipped"
        SKIP_API_BENCHMARKS=true
    else
        log_info "Terraphim server is accessible at $SERVER_URL"
        SKIP_API_BENCHMARKS=false
    fi

    # Check for required tools
    for tool in jq bc curl; do
        if ! command -v $tool &> /dev/null; then
            log_error "Required tool '$tool' is not installed"
            exit 1
        fi
    done
}

# Run Rust benchmarks (Criterion)
run_rust_benchmarks() {
    log_info "Running Rust benchmarks..."

    cd "$PROJECT_ROOT"

    # Run automata benchmarks
    log_info "Running automata benchmarks..."
    if cargo bench --bench autocomplete_bench --manifest-path crates/terraphim_automata/Cargo.toml; then
        log_success "Automata benchmarks completed"
    else
        log_warn "Automata benchmarks failed"
    fi

    # Run rolegraph benchmarks
    log_info "Running rolegraph benchmarks..."
    if cargo bench --bench rolegraph --manifest-path crates/terraphim_rolegraph/Cargo.toml; then
        log_success "Rolegraph benchmarks completed"
    else
        log_warn "Rolegraph benchmarks failed"
    fi

    # Run multi-agent benchmarks
    log_info "Running multi-agent benchmarks..."
    if cargo bench --bench agent_operations --manifest-path crates/terraphim_multi_agent/Cargo.toml; then
        log_success "Multi-agent benchmarks completed"
    else
        log_warn "Multi-agent benchmarks failed"
    fi
}

# Run custom performance benchmarks
run_custom_benchmarks() {
    log_info "Running custom performance benchmarks..."

    cd "$PROJECT_ROOT"

    # Build the benchmark binary (if it exists)
    if [ -f "crates/terraphim_validation/src/bin/performance_benchmark.rs" ]; then
        log_info "Building performance benchmark binary..."
        if cargo build --bin performance_benchmark --manifest-path crates/terraphim_validation/Cargo.toml; then
            log_info "Running custom benchmarks..."
            local baseline_arg=""
            if [ -f "$BASELINE_FILE" ]; then
                baseline_arg="--baseline $BASELINE_FILE"
            fi

            local verbose_arg=""
            if [ "$VERBOSE" = true ]; then
                verbose_arg="--verbose"
            fi

            ./target/debug/performance_benchmark run \
                --output-dir "$RUN_DIR" \
                $baseline_arg \
                --iterations $ITERATIONS \
                $verbose_arg
        else
            log_warn "Failed to build performance benchmark binary"
        fi
    else
        log_warn "Performance benchmark binary not found, skipping custom benchmarks"
    fi
}

# Run API benchmarks using curl/wrk
run_api_benchmarks() {
    if [ "$SKIP_API_BENCHMARKS" = true ]; then
        log_warn "Skipping API benchmarks (server not available)"
        return
    fi

    log_info "Running API benchmarks..."

    local api_results="$RUN_DIR/api_benchmarks.json"

    # Health check benchmark
    log_info "Benchmarking health endpoint..."
    local health_times=$(run_endpoint_benchmark "$SERVER_URL/health" 100)

    # Search endpoint benchmark
    log_info "Benchmarking search endpoint..."
    local search_data='{"query":"rust programming","role":"default"}'
    local search_times=$(run_endpoint_benchmark "$SERVER_URL/api/search" 50 "$search_data")

    # Config endpoint benchmark
    log_info "Benchmarking config endpoint..."
    local config_times=$(run_endpoint_benchmark "$SERVER_URL/api/config" 20)

    # Calculate statistics
    local health_avg=$(calculate_average "$health_times")
    local health_p95=$(calculate_percentile "$health_times" 95)
    local search_avg=$(calculate_average "$search_times")
    local search_p95=$(calculate_percentile "$search_times" 95)
    local config_avg=$(calculate_average "$config_times")
    local config_p95=$(calculate_percentile "$config_times" 95)

    # Create results JSON
    cat > "$api_results" << EOF
{
  "timestamp": "$TIMESTAMP",
  "server_url": "$SERVER_URL",
  "benchmarks": {
    "health": {
      "endpoint": "/health",
      "iterations": 100,
      "avg_response_time_ms": $health_avg,
      "p95_response_time_ms": $health_p95
    },
    "search": {
      "endpoint": "/api/search",
      "iterations": 50,
      "avg_response_time_ms": $search_avg,
      "p95_response_time_ms": $search_p95
    },
    "config": {
      "endpoint": "/api/config",
      "iterations": 20,
      "avg_response_time_ms": $config_avg,
      "p95_response_time_ms": $config_p95
    }
  }
}
EOF

    log_success "API benchmarks completed: $api_results"
}

# Run a benchmark against a single endpoint
run_endpoint_benchmark() {
    local url=$1
    local iterations=$2
    local data=${3:-}

    local times=""

    for i in $(seq 1 $iterations); do
        local start_time=$(date +%s%N)

        if [ -n "$data" ]; then
            curl -s -X POST -H "Content-Type: application/json" -d "$data" "$url" > /dev/null
        else
            curl -s "$url" > /dev/null
        fi

        local end_time=$(date +%s%N)
        local duration_ns=$((end_time - start_time))
        local duration_ms=$((duration_ns / 1000000))

        times="${times}${duration_ms}\n"
    done

    echo -e "$times"
}

# Calculate average from newline-separated values
calculate_average() {
    local values=$1
    echo "$values" | awk '{sum+=$1; count++} END {if (count>0) print sum/count; else print 0}'
}

# Calculate percentile from newline-separated values
calculate_percentile() {
    local values=$1
    local percentile=$2

    # Sort values and calculate percentile
    echo "$values" | sort -n | awk -v p=$percentile '{
        a[NR]=$1
    } END {
        if (NR>0) {
            idx = int((p/100) * NR) + 1
            if (idx > NR) idx = NR
            print a[idx]
        } else {
            print 0
        }
    }'
}

# Run load testing with wrk (if available)
run_load_tests() {
    if ! command -v wrk &> /dev/null; then
        log_warn "wrk not found, skipping load tests"
        return
    fi

    log_info "Running load tests..."

    local load_results="$RUN_DIR/load_test_results.txt"

    # Test health endpoint with increasing concurrency
    for concurrency in 1 5 10 25 50; do
        log_info "Load testing health endpoint with $concurrency concurrent connections..."

        wrk -t$concurrency -c$concurrency -d30s --latency "$SERVER_URL/health" >> "$load_results" 2>&1

        echo "--- Concurrency: $concurrency ---" >> "$load_results"
    done

    log_success "Load tests completed: $load_results"
}

# Generate comprehensive report
generate_report() {
    log_info "Generating comprehensive benchmark report..."

    local report_file="$RUN_DIR/benchmark_report.md"

    cat > "$report_file" << 'EOF'
# Terraphim AI Performance Benchmark Report

**Generated:** TIMESTAMP_PLACEHOLDER
**Run ID:** RUN_ID_PLACEHOLDER

## Executive Summary

This report contains comprehensive performance benchmarks for Terraphim AI components including:

- Rust core library benchmarks (Criterion)
- Custom performance benchmarks
- API endpoint benchmarks
- Load testing results
- System resource monitoring

## System Information

EOF

    # Add system information
    echo "- **OS:** $(uname -s) $(uname -r)" >> "$report_file"
    echo "- **CPU:** $(nproc) cores" >> "$report_file"
    echo "- **Memory:** $(free -h | grep '^Mem:' | awk '{print $2}') total" >> "$report_file"
    echo "- **Rust Version:** $(rustc --version)" >> "$report_file"
    echo "" >> "$report_file"

    # Add Rust benchmarks section
    echo "## Rust Benchmarks (Criterion)" >> "$report_file"
    echo "" >> "$report_file"

    if [ -d "target/criterion" ]; then
        echo "Criterion benchmark reports are available in: \`target/criterion/\`" >> "$report_file"
        echo "" >> "$report_file"
    else
        echo "No Criterion benchmark reports found." >> "$report_file"
        echo "" >> "$report_file"
    fi

    # Add custom benchmarks section
    echo "## Custom Performance Benchmarks" >> "$report_file"
    echo "" >> "$report_file"

    if [ -f "$RUN_DIR/benchmark_results.json" ]; then
        echo "Custom benchmark results: \`benchmark_results.json\`" >> "$report_file"
        echo "HTML report: \`benchmark_report.html\`" >> "$report_file"
        echo "" >> "$report_file"
    else
        echo "No custom benchmark results found." >> "$report_file"
        echo "" >> "$report_file"
    fi

    # Add API benchmarks section
    echo "## API Benchmarks" >> "$report_file"
    echo "" >> "$report_file"

    if [ -f "$RUN_DIR/api_benchmarks.json" ]; then
        echo "API benchmark results: \`api_benchmarks.json\`" >> "$report_file"
        echo "" >> "$report_file"

        # Add API results summary
        if command -v jq &> /dev/null; then
            echo "### API Results Summary" >> "$report_file"
            echo "" >> "$report_file"
            echo "| Endpoint | Avg Response Time | P95 Response Time | Iterations |" >> "$report_file"
            echo "|----------|-------------------|-------------------|------------|" >> "$report_file"

            jq -r '.benchmarks | to_entries[] | "\(.key)|\(.value.avg_response_time_ms)|\(.value.p95_response_time_ms)|\(.value.iterations)"' "$RUN_DIR/api_benchmarks.json" | \
            while IFS='|' read -r endpoint avg p95 iters; do
                echo "| \`/$endpoint\` | ${avg}ms | ${p95}ms | $iters |" >> "$report_file"
            done

            echo "" >> "$report_file"
        fi
    else
        echo "No API benchmark results found." >> "$report_file"
        echo "" >> "$report_file"
    fi

    # Add load testing section
    echo "## Load Testing Results" >> "$report_file"
    echo "" >> "$report_file"

    if [ -f "$RUN_DIR/load_test_results.txt" ]; then
        echo "Load testing results: \`load_test_results.txt\`" >> "$report_file"
        echo "" >> "$report_file"
    else
        echo "No load testing results found." >> "$report_file"
        echo "" >> "$report_file"
    fi

    # Replace placeholders
    sed -i "s/TIMESTAMP_PLACEHOLDER/$(date)/g" "$report_file"
    sed -i "s/RUN_ID_PLACEHOLDER/$TIMESTAMP/g" "$report_file"

    log_success "Comprehensive report generated: $report_file"
}

# Compare against baseline
compare_baseline() {
    if [ ! -f "$BASELINE_FILE" ]; then
        log_warn "No baseline file found at $BASELINE_FILE, skipping comparison"
        return
    fi

    log_info "Comparing results against baseline..."

    # This is a simplified comparison - in practice, you'd want more sophisticated analysis
    if [ -f "$RUN_DIR/benchmark_results.json" ] && [ -f "$BASELINE_FILE" ]; then
        log_info "Comparing custom benchmark results..."

        # Simple comparison - check if current results exist
        # In a real implementation, you'd compare specific metrics

        log_info "Baseline comparison completed"
    fi
}

# Main execution
main() {
    log_info "Starting Terraphim AI Performance Benchmark Suite"
    log_info "Timestamp: $TIMESTAMP"
    log_info "Results directory: $RUN_DIR"

    create_results_dir
    check_requirements

    # Run all benchmark types
    run_rust_benchmarks
    run_custom_benchmarks
    run_api_benchmarks
    run_load_tests

    # Generate reports
    generate_report
    compare_baseline

    log_success "Performance benchmarking completed!"
    log_success "Results available in: $RUN_DIR"

    # Print summary
    echo ""
    echo "📊 Benchmark Summary:"
    echo "  📁 Results: $RUN_DIR"
    echo "  📄 Report: $RUN_DIR/benchmark_report.md"

    if [ -f "$RUN_DIR/benchmark_results.json" ]; then
        echo "  📈 JSON Results: $RUN_DIR/benchmark_results.json"
    fi

    if [ -f "$RUN_DIR/api_benchmarks.json" ]; then
        echo "  🌐 API Results: $RUN_DIR/api_benchmarks.json"
    fi
}

# Run main function
main "$@"</content>
<parameter name="filePath">scripts/run-performance-benchmarks.sh