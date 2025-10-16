#!/bin/bash

# CI Quick Check Script
# Fast subset for pre-commit validation
# Usage: ./scripts/ci-quick-check.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}⚡ CI Quick Check${NC}"
echo "================"
echo "Fast subset for pre-commit validation"
echo ""

# Configuration
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"
SKIP_BUILD="${SKIP_BUILD:-false}"
SKIP_TESTS="${SKIP_TESTS:-false}"

echo "Target: $TARGET"
echo "Skip Build: $SKIP_BUILD"
echo "Skip Tests: $SKIP_TESTS"
echo ""

# Test results tracking
CHECKS_PASSED=0
CHECKS_FAILED=0
FAILED_CHECKS=()
START_TIME=$(date +%s)

# Function to run check and track results
run_check() {
    local check_name="$1"
    local check_command="$2"
    local description="$3"

    echo -e "\n${BLUE}⚡ Running: ${check_name}${NC}"
    echo "Description: $description"

    local check_start_time=$(date +%s)

    if eval "$check_command"; then
        local check_end_time=$(date +%s)
        local check_duration=$((check_end_time - check_start_time))
        echo -e "${GREEN}  ✅ PASSED (${check_duration}s)${NC}"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
        return 0
    else
        local check_end_time=$(date +%s)
        local check_duration=$((check_end_time - check_start_time))
        echo -e "${RED}  ❌ FAILED (${check_duration}s)${NC}"
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
        FAILED_CHECKS+=("$check_name")
        return 1
    fi
}

# Fast checks first (formatting, basic compilation)
echo -e "${BLUE}📋 Running Quick Checks${NC}"
echo "========================"

# 1. Quick format check (cargo fmt)
run_check "Quick Format Check" \
    "cargo fmt --all -- --check" \
    "Check if code is properly formatted"

# 2. Quick clippy check (cargo clippy with common warnings)
run_check "Quick Clippy Check" \
    "cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic" \
    "Check for common clippy warnings"

# 3. Cargo check (fast compilation check)
run_check "Cargo Check" \
    "cargo check --workspace --all-targets" \
    "Fast compilation check without building"

# 4. Rust build for default target (if not skipped)
if [[ "$SKIP_BUILD" == "false" ]]; then
    run_check "Quick Rust Build" \
        "cargo build --package terraphim_server --package terraphim_mcp_server --package terraphim_tui" \
        "Build main packages for default target"
else
    echo -e "\n${YELLOW}⏭️  Skipping Build (SKIP_BUILD=true)${NC}"
fi

# 5. Unit tests only (if not skipped)
if [[ "$SKIP_TESTS" == "false" ]]; then
    run_check "Quick Unit Tests" \
        "cargo test --workspace --lib" \
        "Run unit tests only (skip integration tests)"
else
    echo -e "\n${YELLOW}⏭️  Skipping Tests (SKIP_TESTS=true)${NC}"
fi

# 6. Frontend dependency check (quick check)
run_check "Frontend Dependency Check" \
    "cd desktop && yarn check --verify-tree" \
    "Check if frontend dependencies are consistent"

# 7. Frontend type check (if TypeScript is used)
if [[ -f "desktop/tsconfig.json" ]]; then
    run_check "Frontend Type Check" \
        "cd desktop && npx tsc --noEmit" \
        "Check TypeScript types"
else
    echo -e "\n${YELLOW}⏭️  Skipping TypeScript check (tsconfig.json not found)${NC}"
fi

# Calculate total time
END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_DURATION / 60))
TOTAL_SECONDS=$((TOTAL_DURATION % 60))

echo -e "\n${BLUE}📊 Quick Check Results${NC}"
echo "======================"
echo "Total time: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"

TOTAL_CHECKS=$((CHECKS_PASSED + CHECKS_FAILED))
echo "Total checks: $TOTAL_CHECKS"
echo -e "${GREEN}Passed: $CHECKS_PASSED${NC}"
echo -e "${RED}Failed: $CHECKS_FAILED${NC}"

if [ $CHECKS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}⚡ QUICK CHECK PASSED!${NC}"
    echo ""
    echo "✅ Code is properly formatted"
    echo "✅ No clippy warnings"
    echo "✅ Cargo check passes"
    if [[ "$SKIP_BUILD" == "false" ]]; then
        echo "✅ Rust build successful"
    fi
    if [[ "$SKIP_TESTS" == "false" ]]; then
        echo "✅ Unit tests pass"
    fi
    echo "✅ Frontend dependencies are consistent"
    echo ""
    echo "🚀 Ready for commit!"
    echo ""
    echo "For full CI validation, run:"
    echo "  $SCRIPT_DIR/ci-run-all.sh"
    exit 0
else
    echo -e "\n${RED}❌ Some quick checks failed:${NC}"
    for check in "${FAILED_CHECKS[@]}"; do
        echo -e "${RED}  - $check${NC}"
    done
    echo ""
    echo -e "${YELLOW}Quick fixes:${NC}"
    echo "1. Run: cargo fmt"
    echo "2. Run: cargo clippy --fix --allow-dirty --allow-staged"
    echo "3. Check the specific failures above"
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Fix the failed checks"
    echo "2. Re-run: $0"
    echo "3. For full validation: $SCRIPT_DIR/ci-run-all.sh"
    exit 1
fi