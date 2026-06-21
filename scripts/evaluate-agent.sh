#!/usr/bin/env bash
# Terraphim codebase evaluation script.
#
# Usage:
#   evaluate-agent.sh --mode baseline  [--output-dir DIR]
#   evaluate-agent.sh --mode candidate [--output-dir DIR] [--baseline-dir DIR]
#
# Captures cargo test + clippy metrics and, in candidate mode, invokes the
# eval-check binary to produce a verdict JSON report.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --------------------------------------------------------------------------- #
# Defaults
# --------------------------------------------------------------------------- #
MODE=""
OUTPUT_DIR="${EVAL_OUTPUT_DIR:-${REPO_ROOT}/.eval}"
BASELINE_DIR=""

# --------------------------------------------------------------------------- #
# Argument parsing
# --------------------------------------------------------------------------- #
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)         MODE="$2";         shift 2 ;;
        --output-dir)   OUTPUT_DIR="$2";   shift 2 ;;
        --baseline-dir) BASELINE_DIR="$2"; shift 2 ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 --mode baseline|candidate [--output-dir DIR] [--baseline-dir DIR]" >&2
            exit 1
            ;;
    esac
done

if [[ -z "$MODE" ]]; then
    echo "Error: --mode is required (baseline|candidate)" >&2
    exit 1
fi

# --------------------------------------------------------------------------- #
# Helpers
# --------------------------------------------------------------------------- #
log() { echo "[eval] $*"; }

# Parse 'test result: ok. N passed; M failed' from cargo test output.
parse_test_results() {
    local output="$1"
    local -n _failures="$2"
    local -n _count="$3"

    _failures=0
    _count=0
    while IFS= read -r line; do
        if [[ "$line" =~ ^"test result:".*([0-9]+)" passed".*([0-9]+)" failed" ]]; then
            _count=$(( _count + ${BASH_REMATCH[1]} + ${BASH_REMATCH[2]} ))
            _failures=$(( _failures + ${BASH_REMATCH[2]} ))
        fi
    done <<< "$output"
}

# Count 'warning:' and 'error:' lines from clippy output.
parse_clippy() {
    local output="$1"
    local -n _warnings="$2"
    local -n _errors="$3"

    _warnings=$(echo "$output" | grep -c '^warning:' || true)
    _errors=$(echo "$output" | grep -c '^error\[' || true)
}

# --------------------------------------------------------------------------- #
# Run metrics
# --------------------------------------------------------------------------- #
run_metrics() {
    local out_file="$1"
    cd "$REPO_ROOT"

    log "Running cargo test --workspace …"
    test_output=$(cargo test --workspace 2>&1) || true

    log "Running cargo clippy --workspace …"
    clippy_output=$(cargo clippy --workspace --all-targets 2>&1) || true

    local test_failures=0 test_count=0 clippy_warnings=0 clippy_errors=0
    parse_test_results "$test_output" test_failures test_count
    parse_clippy "$clippy_output" clippy_warnings clippy_errors

    log "  test_count=${test_count}  test_failures=${test_failures}"
    log "  clippy_warnings=${clippy_warnings}  clippy_errors=${clippy_errors}"

    cat > "$out_file" <<JSON
{
  "test_failures": ${test_failures},
  "clippy_warnings": ${clippy_warnings},
  "clippy_errors": ${clippy_errors},
  "test_count": ${test_count}
}
JSON
    log "Metrics saved to ${out_file}"
}

# --------------------------------------------------------------------------- #
# Baseline mode
# --------------------------------------------------------------------------- #
baseline_mode() {
    mkdir -p "$OUTPUT_DIR"
    local metrics_file="${OUTPUT_DIR}/baseline-metrics.json"
    log "=== Baseline capture ==="
    run_metrics "$metrics_file"
    log "Baseline complete. Metrics at: ${metrics_file}"
}

# --------------------------------------------------------------------------- #
# Candidate mode
# --------------------------------------------------------------------------- #
candidate_mode() {
    mkdir -p "$OUTPUT_DIR"
    local candidate_file="${OUTPUT_DIR}/candidate-metrics.json"

    # Locate baseline
    if [[ -z "$BASELINE_DIR" ]]; then
        BASELINE_DIR="$OUTPUT_DIR"
    fi
    local baseline_file="${BASELINE_DIR}/baseline-metrics.json"

    if [[ ! -f "$baseline_file" ]]; then
        echo "Error: baseline metrics not found at ${baseline_file}" >&2
        echo "Run with --mode baseline first, or pass --baseline-dir pointing at the baseline output." >&2
        exit 1
    fi

    log "=== Candidate capture ==="
    run_metrics "$candidate_file"

    # Find the eval-check binary.
    local eval_bin
    if command -v eval-check &>/dev/null; then
        eval_bin="eval-check"
    elif [[ -f "${REPO_ROOT}/target/release/eval-check" ]]; then
        eval_bin="${REPO_ROOT}/target/release/eval-check"
    elif [[ -f "${REPO_ROOT}/target/debug/eval-check" ]]; then
        eval_bin="${REPO_ROOT}/target/debug/eval-check"
    else
        log "Building eval-check …"
        cargo build -p terraphim_eval_check --bin eval-check 2>&1
        eval_bin="${REPO_ROOT}/target/debug/eval-check"
    fi

    local report_file="${OUTPUT_DIR}/verdict-report.json"
    log "Running verdict engine …"
    "$eval_bin" \
        --baseline "$baseline_file" \
        --candidate "$candidate_file" \
        --format json \
        | tee "$report_file" \
        || true

    local verdict
    verdict=$(python3 -c "import json,sys; d=json.load(open('${report_file}')); print(d.get('verdict','Unknown'))" 2>/dev/null || echo "Unknown")
    log "Verdict: ${verdict}"

    if [[ "$verdict" == "Degraded" ]]; then
        log "FAIL — candidate degraded the codebase."
        exit 1
    fi

    log "PASS — verdict is ${verdict}."
}

# --------------------------------------------------------------------------- #
# Dispatch
# --------------------------------------------------------------------------- #
case "$MODE" in
    baseline)  baseline_mode ;;
    candidate) candidate_mode ;;
    *)
        echo "Error: --mode must be 'baseline' or 'candidate', got '${MODE}'" >&2
        exit 1
        ;;
esac
