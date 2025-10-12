#!/usr/bin/env bash

# Terraphim AI 1Password Integration Validation Script
# This script validates the complete 1Password integration implementation

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}🔍 Terraphim AI 1Password Integration Validation${NC}"
echo "=================================================="

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

run_test() {
    local test_name="$1"
    local test_command="$2"

    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    echo -n "Testing $test_name... "

    if eval "$test_command" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAIL${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

run_test_verbose() {
    local test_name="$1"
    local test_command="$2"

    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    echo "Testing $test_name..."

    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASS: $test_name${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAIL: $test_name${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    echo
}

echo -e "${YELLOW}📋 Component Validation${NC}"
echo "========================"

# Test 1: Rust compilation
run_test "Rust workspace compilation" "cd '$PROJECT_ROOT' && cargo check --workspace"

# Test 2: 1Password CLI crate
run_test "1Password CLI crate compilation" "cd '$PROJECT_ROOT' && cargo check -p terraphim_onepassword_cli"

# Test 3: Settings crate with 1Password feature
run_test "Settings crate with 1Password feature" "cd '$PROJECT_ROOT' && cargo check -p terraphim_settings --features onepassword"

# Test 4: Desktop application with 1Password
run_test "Desktop application compilation" "cd '$PROJECT_ROOT' && cargo check -p terraphim-ai-desktop"

echo -e "${YELLOW}📁 File Structure Validation${NC}"
echo "============================="

# Test 5: Required files exist
run_test "1Password CLI crate exists" "test -f '$PROJECT_ROOT/crates/terraphim_onepassword_cli/src/lib.rs'"
run_test "Setup script exists" "test -f '$PROJECT_ROOT/scripts/setup-1password-terraphim.sh'"
run_test "Setup script is executable" "test -x '$PROJECT_ROOT/scripts/setup-1password-terraphim.sh'"
run_test "README documentation exists" "test -f '$PROJECT_ROOT/README_1PASSWORD_INTEGRATION.md'"

echo -e "${YELLOW}📝 Template Validation${NC}"
echo "======================"

# Test 6: Configuration templates
run_test "Environment template exists" "test -f '$PROJECT_ROOT/templates/env.terraphim.template'"
run_test "Settings template exists" "test -f '$PROJECT_ROOT/templates/settings.toml.template'"
run_test "Server config template exists" "test -f '$PROJECT_ROOT/templates/server_config.json.template'"
run_test "Tauri config template exists" "test -f '$PROJECT_ROOT/templates/tauri.conf.json.template'"

# Test 7: Template content validation
run_test "Environment template has op:// references" "grep -q 'op://' '$PROJECT_ROOT/templates/env.terraphim.template'"
run_test "Settings template has op:// references" "grep -q 'op://' '$PROJECT_ROOT/templates/settings.toml.template'"
run_test "No hardcoded secrets in templates" "! find '$PROJECT_ROOT/templates' -name '*.template' -exec grep -H -E '(password|secret|key|token)' {} \\; | grep -v 'op://' | grep -q ."

echo -e "${YELLOW}🔧 Workflow Validation${NC}"
echo "======================"

# Test 8: CI/CD integration
run_test "CI workflow template exists" "test -f '$PROJECT_ROOT/.github/workflows/ci-1password.yml.template'"
run_test "CI workflow has 1Password integration" "grep -q '1password/install-cli-action' '$PROJECT_ROOT/.github/workflows/ci-1password.yml.template'"

echo -e "${YELLOW}🏗️ Build System Validation${NC}"
echo "==========================="

# Test 9: Cargo.toml updates
run_test "Desktop Cargo.toml includes 1Password dependency" "grep -q 'terraphim_onepassword_cli' '$PROJECT_ROOT/desktop/src-tauri/Cargo.toml'"
run_test "Desktop Cargo.toml includes onepassword feature" "grep -q 'features.*onepassword' '$PROJECT_ROOT/desktop/src-tauri/Cargo.toml'"

echo -e "${YELLOW}🔍 Code Structure Validation${NC}"
echo "============================"

# Test 10: Tauri commands
run_test "Tauri commands include 1Password functions" "grep -q 'onepassword_status' '$PROJECT_ROOT/desktop/src-tauri/src/cmd.rs'"
run_test "Tauri main.rs registers 1Password commands" "grep -q 'onepassword_status' '$PROJECT_ROOT/desktop/src-tauri/src/main.rs'"

# Test 11: Settings crate integration
run_test "Settings crate has 1Password feature" "grep -q 'onepassword.*=' '$PROJECT_ROOT/crates/terraphim_settings/Cargo.toml'"
run_test "Settings crate has load_with_onepassword function" "grep -q 'load_with_onepassword' '$PROJECT_ROOT/crates/terraphim_settings/src/lib.rs'"

echo -e "${YELLOW}📚 Documentation Validation${NC}"
echo "==========================="

# Test 12: Documentation completeness
run_test "README has setup instructions" "grep -q 'Setup Instructions' '$PROJECT_ROOT/README_1PASSWORD_INTEGRATION.md'"
run_test "README has usage examples" "grep -q 'Usage' '$PROJECT_ROOT/README_1PASSWORD_INTEGRATION.md'"
run_test "README has troubleshooting section" "grep -q 'Troubleshooting' '$PROJECT_ROOT/README_1PASSWORD_INTEGRATION.md'"
run_test "README has security best practices" "grep -q 'Security Best Practices' '$PROJECT_ROOT/README_1PASSWORD_INTEGRATION.md'"

echo
echo "=================================================="
echo -e "${BLUE}📊 Test Results Summary${NC}"
echo "=================================================="
echo -e "Total tests run: ${BLUE}$TESTS_TOTAL${NC}"
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo
    echo -e "${GREEN}🎉 All tests passed! 1Password integration is ready for use.${NC}"
    echo
    echo "Next steps:"
    echo "1. Run the setup script: ./scripts/setup-1password-terraphim.sh dev"
    echo "2. Populate secrets in 1Password vaults"
    echo "3. Test configuration generation: op inject -i templates/env.terraphim.template"
    echo "4. Deploy with 1Password-enhanced CI/CD pipeline"
    exit 0
else
    echo
    echo -e "${RED}❌ Some tests failed. Please review the errors above.${NC}"
    echo
    echo "Common issues:"
    echo "- Missing dependencies: Run 'cargo check --workspace'"
    echo "- File permissions: Ensure scripts are executable"
    echo "- Template syntax: Validate op:// reference format"
    exit 1
fi
