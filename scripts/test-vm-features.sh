#!/bin/bash
# Test automation script for VM execution features
# Tests: Rust execution, DirectSessionAdapter, Hook system, E2E workflows

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo "========================================"
echo "VM Features Test Suite"
echo "========================================"
echo ""

run_unit_tests() {
    echo "==> Running Unit Tests"
    echo ""

    echo "[1/5] Hook system tests..."
    cargo test -p terraphim_multi_agent --lib vm_execution::hooks::tests --quiet
    echo "✓ Hook tests passed"
    echo ""

    echo "[2/5] Session adapter tests..."
    cargo test -p terraphim_multi_agent --lib vm_execution::session_adapter::tests --quiet
    echo "✓ Session adapter tests passed"
    echo ""

    echo "[3/5] Code extractor tests..."
    cargo test -p terraphim_multi_agent --lib vm_execution::code_extractor::tests --quiet
    echo "✓ Code extractor tests passed"
    echo ""

    echo "[4/5] Rust execution unit tests..."
    cargo test -p terraphim_multi_agent --test rust_execution_tests rust_basic_tests --quiet
    echo "✓ Rust execution tests passed"
    echo ""

    echo "[5/5] Direct session unit tests..."
    cargo test -p terraphim_multi_agent --test direct_session_integration_tests direct_session_unit_tests --quiet
    echo "✓ Direct session tests passed"
    echo ""
}

run_integration_tests() {
    echo "==> Running Integration Tests (requires fcctl-web @ localhost:8080)"
    echo ""

    if ! check_fcctl_web; then
        echo "⚠️  fcctl-web not running, skipping integration tests"
        echo "   Start it with: cd scratchpad/firecracker-rust && cargo run -p fcctl-web"
        return 0
    fi

    echo "[1/4] Rust execution integration tests..."
    cargo test -p terraphim_multi_agent --test rust_execution_tests rust_integration_tests -- --ignored --test-threads=1
    echo "✓ Rust integration tests passed"
    echo ""

    echo "[2/4] DirectSessionAdapter integration tests..."
    cargo test -p terraphim_multi_agent --test direct_session_integration_tests direct_session_integration_tests -- --ignored --test-threads=1
    echo "✓ DirectSessionAdapter integration tests passed"
    echo ""

    echo "[3/4] Hook integration tests..."
    cargo test -p terraphim_multi_agent --test hook_integration_tests vm_client_with_hooks_tests -- --ignored --test-threads=1
    echo "✓ Hook integration tests passed"
    echo ""

    echo "[4/4] FcctlBridge integration tests..."
    cargo test -p terraphim_multi_agent --test direct_session_integration_tests fcctl_bridge_integration_tests -- --ignored --test-threads=1
    echo "✓ FcctlBridge integration tests passed"
    echo ""
}

run_e2e_tests() {
    echo "==> Running End-to-End Tests (requires full stack)"
    echo ""

    if ! check_fcctl_web; then
        echo "⚠️  fcctl-web not running, skipping E2E tests"
        return 0
    fi

    echo "[1/4] Complete workflow tests..."
    cargo test --test vm_execution_e2e_tests complete_workflow_tests -- --ignored --test-threads=1
    echo "✓ Workflow tests passed"
    echo ""

    echo "[2/4] Multi-language tests..."
    cargo test --test vm_execution_e2e_tests multi_language_workflow_tests -- --ignored --test-threads=1
    echo "✓ Multi-language tests passed"
    echo ""

    echo "[3/4] Hook integration E2E..."
    cargo test --test vm_execution_e2e_tests hook_integration_e2e_tests -- --ignored --test-threads=1
    echo "✓ Hook E2E tests passed"
    echo ""

    echo "[4/4] Performance E2E tests..."
    cargo test --test vm_execution_e2e_tests performance_e2e_tests -- --ignored --test-threads=1
    echo "✓ Performance tests passed"
    echo ""
}

run_rust_specific_tests() {
    echo "==> Running Rust Language Test Suite"
    echo ""

    if ! check_fcctl_web; then
        echo "⚠️  fcctl-web not running, running unit tests only"
        cargo test -p terraphim_multi_agent --test rust_execution_tests rust_basic_tests --quiet
        return 0
    fi

    echo "[1/3] Rust security restrictions..."
    cargo test -p terraphim_multi_agent --test rust_execution_tests test_rust_security --quiet
    echo "✓ Security tests passed"
    echo ""

    echo "[2/3] Rust compilation and execution..."
    cargo test -p terraphim_multi_agent --test rust_execution_tests rust_integration_tests::test_rust_hello_world -- --ignored
    echo "✓ Basic execution passed"
    echo ""

    echo "[3/3] Rust complex programs..."
    cargo test -p terraphim_multi_agent --test rust_execution_tests rust_integration_tests::test_rust_complex_program -- --ignored
    echo "✓ Complex program tests passed"
    echo ""
}

check_fcctl_web() {
    if curl -s "http://localhost:8080/health" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

show_help() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  unit          Run unit tests only (no server required)"
    echo "  integration   Run integration tests (requires fcctl-web)"
    echo "  e2e           Run end-to-end tests (requires full stack)"
    echo "  rust          Run Rust-specific test suite"
    echo "  all           Run all tests (default)"
    echo "  help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 unit                # Quick unit tests"
    echo "  $0 integration         # Integration tests only"
    echo "  $0 rust                # Test Rust language support"
    echo "  $0                     # Run everything"
    echo ""
}

case "${1:-all}" in
    unit)
        run_unit_tests
        ;;
    integration)
        run_integration_tests
        ;;
    e2e)
        run_e2e_tests
        ;;
    rust)
        run_rust_specific_tests
        ;;
    all)
        echo "Running complete test suite..."
        echo ""
        run_unit_tests
        run_integration_tests
        run_e2e_tests
        echo ""
        echo "========================================"
        echo "✓ All tests passed!"
        echo "========================================"
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Error: Unknown command '$1'"
        echo ""
        show_help
        exit 1
        ;;
esac
