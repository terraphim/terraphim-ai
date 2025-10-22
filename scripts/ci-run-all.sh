#!/bin/bash

# CI Run All Script
# Runs all CI checks in sequence (like ci-native.yml)
# Usage: ./scripts/ci-run-all.sh

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

echo -e "${BLUE}🚀 CI Run All Checks${NC}"
echo "===================="
echo "Running all CI checks in sequence (mirroring ci-native.yml)"
echo ""

# Configuration
SKIP_DESKTOP_TESTS="${SKIP_DESKTOP_TESTS:-false}"
TARGET="${TARGET:-x86_64-unknown-linux-gnu}"

echo "Target: $TARGET"
echo "Skip Desktop Tests: $SKIP_DESKTOP_TESTS"
echo ""

# Test results tracking
SCRIPTS_PASSED=0
SCRIPTS_FAILED=0
FAILED_SCRIPTS=()
START_TIME=$(date +%s)

# Function to run script and track results
run_script() {
    local script_name="$1"
    local script_path="$2"
    local description="$3"

    echo -e "\n${BLUE}🔄 Running: ${script_name}${NC}"
    echo "Description: $description"
    echo "Script: $script_path"
    echo "Start time: $(date)"

    local script_start_time=$(date +%s)

    if bash "$script_path"; then
        local script_end_time=$(date +%s)
        local script_duration=$((script_end_time - script_start_time))
        echo -e "${GREEN}  ✅ PASSED (${script_duration}s)${NC}"
        SCRIPTS_PASSED=$((SCRIPTS_PASSED + 1))
        return 0
    else
        local script_end_time=$(date +%s)
        local script_duration=$((script_end_time - script_start_time))
        echo -e "${RED}  ❌ FAILED (${script_duration}s)${NC}"
        SCRIPTS_FAILED=$((SCRIPTS_FAILED + 1))
        FAILED_SCRIPTS+=("$script_name")
        return 1
    fi
}

# Check if all scripts exist
echo -e "${BLUE}🔍 Checking script availability...${NC}"
REQUIRED_SCRIPTS=(
    "ci-check-format.sh"
    "ci-check-frontend.sh"
    "ci-check-rust.sh"
    "ci-check-tests.sh"
)

if [[ "$SKIP_DESKTOP_TESTS" == "false" ]]; then
    REQUIRED_SCRIPTS+=("ci-check-desktop.sh")
fi

for script in "${REQUIRED_SCRIPTS[@]}"; do
    if [[ ! -f "$SCRIPT_DIR/$script" ]]; then
        echo -e "${RED}❌ Required script not found: $SCRIPT_DIR/$script${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✅ All required scripts found${NC}"

# Run scripts in CI order (same as ci-native.yml dependencies)
echo -e "\n${BLUE}📋 Running CI Scripts in Order${NC}"
echo "================================"

# 1. Format check (lint-and-format job)
run_script "Format Check" \
    "$SCRIPT_DIR/ci-check-format.sh" \
    "Mirrors lint-and-format job: cargo fmt + clippy"

# 2. Frontend check (build-frontend job)
run_script "Frontend Check" \
    "$SCRIPT_DIR/ci-check-frontend.sh" \
    "Mirrors build-frontend job: Node.js deps + build + tests"

# 3. Rust build check (build-rust job)
run_script "Rust Build Check" \
    "$SCRIPT_DIR/ci-check-rust.sh $TARGET" \
    "Mirrors build-rust job: Rust build + cross-compilation"

# 4. Test suite check (test-suite job)
run_script "Test Suite Check" \
    "$SCRIPT_DIR/ci-check-tests.sh" \
    "Mirrors test-suite job: unit + integration + doc tests"

# 5. Desktop test check (test-desktop job)
if [[ "$SKIP_DESKTOP_TESTS" == "false" ]]; then
    run_script "Desktop Test Check" \
        "$SCRIPT_DIR/ci-check-desktop.sh" \
        "Mirrors test-desktop job: frontend e2e tests"
else
    echo -e "\n${YELLOW}⏭️  Skipping Desktop Tests (SKIP_DESKTOP_TESTS=true)${NC}"
fi

# Calculate total time
END_TIME=$(date +%s)
TOTAL_DURATION=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_DURATION / 60))
TOTAL_SECONDS=$((TOTAL_DURATION % 60))

echo -e "\n${BLUE}📊 Final Results${NC}"
echo "================="
echo "Total time: ${TOTAL_MINUTES}m ${TOTAL_SECONDS}s"

TOTAL_SCRIPTS=$((SCRIPTS_PASSED + SCRIPTS_FAILED))
echo "Total scripts: $TOTAL_SCRIPTS"
echo -e "${GREEN}Passed: $SCRIPTS_PASSED${NC}"
echo -e "${RED}Failed: $SCRIPTS_FAILED${NC}"

if [ $SCRIPTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL CHECKS PASSED!${NC}"
    echo ""
    echo "✅ Code is properly formatted"
    echo "✅ Frontend builds successfully"
    echo "✅ Rust binaries built for $TARGET"
    echo "✅ All tests pass"
    if [[ "$SKIP_DESKTOP_TESTS" == "false" ]]; then
        echo "✅ Desktop tests pass"
    fi
    echo ""
    echo "🚀 Ready for commit and merge!"
    echo ""
    echo "Next steps:"
    echo "1. git add ."
    echo "2. git commit -m \"Your commit message\""
    echo "3. git push"
    exit 0
else
    echo -e "\n${RED}❌ Some checks failed:${NC}"
    for script in "${FAILED_SCRIPTS[@]}"; do
        echo -e "${RED}  - $script${NC}"
    done
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Fix the failing checks above"
    echo "2. Re-run this script to validate fixes"
    echo "3. You can also run individual scripts:"
    echo "   - $SCRIPT_DIR/ci-check-format.sh"
    echo "   - $SCRIPT_DIR/ci-check-frontend.sh"
    echo "   - $SCRIPT_DIR/ci-check-rust.sh $TARGET"
    echo "   - $SCRIPT_DIR/ci-check-tests.sh"
    if [[ "$SKIP_DESKTOP_TESTS" == "false" ]]; then
        echo "   - $SCRIPT_DIR/ci-check-desktop.sh"
    fi
    exit 1
fi
