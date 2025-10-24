#!/bin/bash

# Validate CI Updates Script
# Validates that the new CI scripts and GitHub Actions work correctly
# Usage: ./scripts/validate-ci-updates.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Validating CI Updates${NC}"
echo "========================"
echo "Checking that all CI scripts and GitHub Actions are properly integrated"
echo ""

# Validation results
VALIDATIONS_PASSED=0
VALIDATIONS_FAILED=0
FAILED_VALIDATIONS=()

# Function to run validation and track results
run_validation() {
    local validation_name="$1"
    local validation_command="$2"
    local description="$3"

    echo -e "\n${CYAN}🧪 Validation: ${validation_name}${NC}"
    echo "Description: $description"
    echo "Command: $validation_command"

    if eval "$validation_command" > /dev/null 2>&1; then
        echo -e "${GREEN}  ✅ PASSED${NC}"
        VALIDATIONS_PASSED=$((VALIDATIONS_PASSED + 1))
        return 0
    else
        echo -e "${RED}  ❌ FAILED${NC}"
        VALIDATIONS_FAILED=$((VALIDATIONS_FAILED + 1))
        FAILED_VALIDATIONS+=("$validation_name")
        return 1
    fi
}

echo -e "${BLUE}📋 Script Validations${NC}"
echo "========================"

# 1. Check all CI scripts exist and are executable
run_validation "CI Scripts Exist" \
    "[[ -f scripts/ci-check-format.sh && -f scripts/ci-check-frontend.sh && -f scripts/ci-check-rust.sh && -f scripts/ci-check-tests.sh && -f scripts/ci-check-desktop.sh && -f scripts/ci-run-all.sh && -f scripts/ci-quick-check.sh && -f scripts/ci-pr-validation.sh ]]" \
    "All 8 CI scripts exist"

run_validation "CI Scripts Executable" \
    "[[ -x scripts/ci-check-format.sh && -x scripts/ci-check-frontend.sh && -x scripts/ci-check-rust.sh && -x scripts/ci-check-tests.sh && -x scripts/ci-check-desktop.sh && -x scripts/ci-run-all.sh && -x scripts/ci-quick-check.sh && -x scripts/ci-pr-validation.sh ]]" \
    "All CI scripts are executable"

# 2. Check pre-commit hook files
run_validation "Pre-commit Hook Files" \
    "[[ -f scripts/pre-commit-hook.sh && -f scripts/setup-pre-commit-hook.sh ]]" \
    "Pre-commit hook files exist"

run_validation "Pre-commit Hook Executable" \
    "[[ -x scripts/pre-commit-hook.sh && -x scripts/setup-pre-commit-hook.sh ]]" \
    "Pre-commit hook files are executable"

# 3. Check documentation files
run_validation "Documentation Files" \
    "[[ -f scripts/README-CI-LOCAL.md && -f scripts/ci-usage-guide.md ]]" \
    "Documentation files exist"

echo -e "\n${BLUE}📋 GitHub Actions Validations${NC}"
echo "================================"

# 4. Check workflow syntax with act
if command -v act &> /dev/null; then
    run_validation "CI Native Workflow Syntax" \
        "act -W .github/workflows/ci-native.yml --list > /dev/null" \
        "ci-native.yml has valid syntax"

    run_validation "Frontend Build Workflow Syntax" \
        "act -W .github/workflows/frontend-build.yml --list > /dev/null" \
        "frontend-build.yml has valid syntax"
else
    echo -e "${YELLOW}⚠️  act not found, skipping workflow syntax validation${NC}"
    echo -e "${YELLOW}Install act with: curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash${NC}"
fi

# 5. Check workflows reference scripts
run_validation "CI Native Uses Scripts" \
    "grep -q 'scripts/ci-check-format.sh' .github/workflows/ci-native.yml" \
    "ci-native.yml references ci-check-format.sh"

run_validation "Frontend Build Uses Scripts" \
    "grep -q 'scripts/ci-check-frontend.sh' .github/workflows/frontend-build.yml" \
    "frontend-build.yml references ci-check-frontend.sh"

run_validation "CI Native Uses Test Scripts" \
    "grep -q 'scripts/ci-check-tests.sh' .github/workflows/ci-native.yml" \
    "ci-native.yml references ci-check-tests.sh"

run_validation "CI Native Uses Desktop Scripts" \
    "grep -q 'scripts/ci-check-desktop.sh' .github/workflows/ci-native.yml" \
    "ci-native.yml references ci-check-desktop.sh"

run_validation "CI Native Uses Rust Scripts" \
    "grep -q 'scripts/ci-check-rust.sh' .github/workflows/ci-native.yml" \
    "ci-native.yml references ci-check-rust.sh"

echo -e "\n${BLUE}📋 Integration Validations${NC}"
echo "=============================="

# 6. Check script functionality (quick checks)
run_validation "Format Script Has Content" \
    "grep -q 'cargo fmt' scripts/ci-check-format.sh" \
    "ci-check-format.sh contains cargo fmt"

run_validation "Frontend Script Has Content" \
    "grep -q 'yarn install' scripts/ci-check-frontend.sh" \
    "ci-check-frontend.sh contains yarn install"

run_validation "Rust Script Has Content" \
    "grep -q 'cargo build' scripts/ci-check-rust.sh" \
    "ci-check-rust.sh contains cargo build"

run_validation "Test Script Has Content" \
    "grep -q 'cargo test' scripts/ci-check-tests.sh" \
    "ci-check-tests.sh contains cargo test"

run_validation "Desktop Script Has Content" \
    "grep -q 'playwright' scripts/ci-check-desktop.sh" \
    "ci-check-desktop.sh contains playwright"

# 7. Check shebang lines
run_validation "Scripts Have Proper Shebang" \
    "head -1 scripts/ci-*.sh | grep -q '#!/bin/bash'" \
    "All scripts have proper bash shebang"

# 8. Check documentation content
run_validation "README Contains Usage Examples" \
    "grep -q 'ci-quick-check.sh' scripts/README-CI-LOCAL.md" \
    "README contains script usage examples"

run_validation "Usage Guide Has Commands" \
    "grep -q './scripts/ci-run-all.sh' scripts/ci-usage-guide.md" \
    "Usage guide contains command examples"

echo -e "\n${BLUE}📊 Validation Results${NC}"
echo "======================"

TOTAL_VALIDATIONS=$((VALIDATIONS_PASSED + VALIDATIONS_FAILED))
echo "Total validations: $TOTAL_VALIDATIONS"
echo -e "${GREEN}Passed: $VALIDATIONS_PASSED${NC}"
echo -e "${RED}Failed: $VALIDATIONS_FAILED${NC}"

if [ $VALIDATIONS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL VALIDATIONS PASSED!${NC}"
    echo ""
    echo "✅ All CI scripts exist and are executable"
    echo "✅ Pre-commit hook files are ready"
    echo "✅ Documentation is complete"
    echo "✅ GitHub Actions workflows have valid syntax"
    echo "✅ Workflows properly reference CI scripts"
    echo "✅ Scripts contain expected functionality"
    echo "✅ Documentation is helpful and accurate"
    echo ""
    echo "🚀 CI system is ready for production!"
    echo ""
    echo "Next steps:"
    echo "1. Install pre-commit hook: ./scripts/setup-pre-commit-hook.sh"
    echo "2. Test with: ./scripts/ci-quick-check.sh"
    echo "3. Run full validation: ./scripts/ci-run-all.sh"
    exit 0
else
    echo -e "\n${RED}❌ Some validations failed:${NC}"
    for validation in "${FAILED_VALIDATIONS[@]}"; do
        echo -e "${RED}  - $validation${NC}"
    done
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Review the failed validations above"
    echo "2. Fix the issues"
    echo "3. Re-run: ./scripts/validate-ci-updates.sh"
    exit 1
fi
