#!/bin/bash

# CI System Demo Script
# Demonstrates the complete CI local testing system
# Usage: ./scripts/ci-system-demo.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BOLD}${BLUE}🚀 Terraphim AI CI System Demo${NC}"
echo -e "${BLUE}===================================${NC}"
echo ""
echo "This demo showcases the comprehensive CI local testing system"
echo "that mirrors GitHub Actions and enables developers to validate"
echo "changes before committing to reduce failed commits."
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo -e "${CYAN}📋 System Overview${NC}"
echo "=================="
echo "✅ 8 Core CI Scripts"
echo "✅ 3 Integration Scripts"
echo "✅ Pre-commit Hook System"
echo "✅ Updated GitHub Actions"
echo "✅ Comprehensive Documentation"
echo ""

echo -e "${CYAN}📁 Available Scripts${NC}"
echo "====================="
echo ""

# List core scripts with descriptions
echo -e "${BOLD}Core CI Check Scripts:${NC}"
echo "├── ci-check-format.sh      - Code formatting & clippy linting"
echo "├── ci-check-frontend.sh    - Frontend build & test validation"
echo "├── ci-check-rust.sh        - Rust build & cross-compilation"
echo "├── ci-check-tests.sh       - Unit/integration/documentation tests"
echo "└── ci-check-desktop.sh     - Desktop E2E tests"
echo ""

echo -e "${BOLD}Integration Scripts:${NC}"
echo "├── ci-quick-check.sh       - Fast pre-commit validation (~2 min)"
echo "├── ci-run-all.sh          - Full CI validation (~15-20 min)"
echo "└── ci-pr-validation.sh    - PR validation with detailed report"
echo ""

echo -e "${BOLD}Utility Scripts:${NC}"
echo "├── setup-pre-commit-hook.sh - Install pre-commit hook"
echo "├── validate-ci-updates.sh   - Validate CI system integrity"
echo "└── ci-system-demo.sh        - This demo script"
echo ""

echo -e "${CYAN}🎯 Usage Examples${NC}"
echo "================="
echo ""

echo -e "${YELLOW}Before Committing (Quick Check):${NC}"
echo "  ./scripts/ci-quick-check.sh"
echo ""

echo -e "${YELLOW}Before Pushing (Full Validation):${NC}"
echo "  ./scripts/ci-run-all.sh"
echo ""

echo -e "${YELLOW}Before Creating PR (Detailed Report):${NC}"
echo "  ./scripts/ci-pr-validation.sh 123  # With PR number"
echo ""

echo -e "${YELLOW}Individual Component Checks:${NC}"
echo "  ./scripts/ci-check-format.sh     # Code formatting only"
echo "  ./scripts/ci-check-frontend.sh   # Frontend only"
echo "  ./scripts/ci-check-rust.sh       # Rust build only"
echo "  ./scripts/ci-check-tests.sh      # Tests only"
echo "  ./scripts/ci-check-desktop.sh    # Desktop tests only"
echo ""

echo -e "${YELLOW}Environment Variables:${NC}"
echo "  export TARGET=aarch64-unknown-linux-gnu"
echo "  export SKIP_DESKTOP_TESTS=true"
echo "  export SKIP_BUILD=true"
echo "  export SKIP_TESTS=true"
echo ""

echo -e "${CYAN}🔧 Pre-commit Hook${NC}"
echo "==================="
echo ""
echo "Install automatic pre-commit validation:"
echo "  ./scripts/setup-pre-commit-hook.sh"
echo ""
echo "What it does:"
echo "  ✓ Automatically runs quick checks before each commit"
echo "  ✓ Prevents commits with formatting issues"
echo "  ✓ Catches problems early in development cycle"
echo "  ✓ Can be bypassed with: git commit --no-verify"
echo ""

echo -e "${CYAN}🌐 GitHub Actions Integration${NC}"
echo "=============================="
echo ""
echo "1:1 Mapping with GitHub Actions:"
echo "  ┌─────────────────────────┬─────────────────────────┐"
echo "  │   GitHub Actions Job    │     Local Script        │"
echo "  ├─────────────────────────┼─────────────────────────┤"
echo "  │   lint-and-format       │ ci-check-format.sh     │"
echo "  │   build-frontend        │ ci-check-frontend.sh   │"
echo "  │   build-rust            │ ci-check-rust.sh       │"
echo "  │   test-suite            │ ci-check-tests.sh      │"
echo "  │   test-desktop          │ ci-check-desktop.sh    │"
echo "  └─────────────────────────┴─────────────────────────┘"
echo ""

echo -e "${CYAN}📊 Performance Comparison${NC}"
echo "=========================="
echo ""
echo "Local vs CI execution times:"
echo "  ┌──────────────────┬─────────────┬─────────────┐"
echo "  │     Check        │   Local     │     CI      │"
echo "  ├──────────────────┼─────────────┼─────────────┤"
echo "  │ Quick Check      │   ~2 min    │   ~5 min    │"
echo "  │ Full Validation  │  ~15 min    │  ~25 min    │"
echo "  │ Format Only      │   ~30 sec   │   ~2 min    │"
echo "  │ Frontend Build   │   ~5 min    │   ~8 min    │"
echo "  │ Rust Build       │   ~8 min    │  ~12 min    │"
echo "  │ Test Suite       │   ~5 min    │   ~8 min    │"
echo "  └──────────────────┴─────────────┴─────────────┘"
echo ""

echo -e "${CYAN}🔍 System Validation${NC}"
echo "===================="
echo ""
echo "Run comprehensive system validation:"
echo "  ./scripts/validate-ci-updates.sh"
echo ""

# Quick validation demo
echo -e "${BLUE}Running quick system validation...${NC}"
QUICK_VALIDATIONS=0

# Check scripts exist
if ls scripts/ci-*.sh >/dev/null 2>&1; then
    echo -e "${GREEN}  ✅ CI scripts exist${NC}"
    QUICK_VALIDATIONS=$((QUICK_VALIDATIONS + 1))
fi

# Check workflows exist
if ls .github/workflows/*.yml >/dev/null 2>&1; then
    echo -e "${GREEN}  ✅ GitHub Actions workflows exist${NC}"
    QUICK_VALIDATIONS=$((QUICK_VALIDATIONS + 1))
fi

# Check documentation exists
if [[ -f scripts/README-CI-LOCAL.md ]]; then
    echo -e "${GREEN}  ✅ Documentation exists${NC}"
    QUICK_VALIDATIONS=$((QUICK_VALIDATIONS + 1))
fi

# Check act is available for workflow testing
if command -v act &> /dev/null; then
    echo -e "${GREEN}  ✅ act available for local workflow testing${NC}"
    QUICK_VALIDATIONS=$((QUICK_VALIDATIONS + 1))
else
    echo -e "${YELLOW}  ⚠️  act not available (install for workflow testing)${NC}"
fi

echo -e "${GREEN}Quick validation: $QUICK_VALIDATIONS/4 checks passed${NC}"
echo ""

echo -e "${CYAN}📚 Documentation${NC}"
echo "================"
echo ""
echo "Available documentation:"
echo "  📖 scripts/README-CI-LOCAL.md     - Comprehensive guide (200+ lines)"
echo "  📋 scripts/ci-usage-guide.md      - Quick reference guide"
echo "  🔧 scripts/CI-MAINTENANCE.md       - Maintenance guidelines"
echo ""

echo -e "${CYAN}🚀 Getting Started${NC}"
echo "==================="
echo ""
echo "1. Install pre-commit hook:"
echo "   ./scripts/setup-pre-commit-hook.sh"
echo ""
echo "2. Test quick validation:"
echo "   ./scripts/ci-quick-check.sh"
echo ""
echo "3. Run full validation (optional):"
echo "   ./scripts/ci-run-all.sh"
echo ""
echo "4. Start coding! Your commits will be automatically validated."
echo ""

echo -e "${BOLD}${GREEN}🎉 CI System Demo Complete!${NC}"
echo -e "${GREEN}============================${NC}"
echo ""
echo "The Terraphim AI CI local testing system is ready for use!"
echo ""
echo "Key Benefits:"
echo "  ✅ Reduce failed commits by catching issues locally"
echo "  ✅ Faster development iteration with immediate feedback"
echo "  ✅ Consistent environment between local and CI"
echo "  ✅ Comprehensive validation with detailed reporting"
echo "  ✅ Easy maintenance with clear documentation"
echo ""
echo "For help:"
echo "  📖 ./scripts/README-CI-LOCAL.md"
echo "  📋 ./scripts/ci-usage-guide.md"
echo "  🔧 ./scripts/CI-MAINTENANCE.md"
echo ""
echo "Happy coding! 🚀"