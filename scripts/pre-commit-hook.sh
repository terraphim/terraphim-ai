#!/bin/bash

# Pre-commit hook for Terraphim AI
# Runs quick validation checks before allowing commits
# Usage: Install as .git/hooks/pre-commit

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔒 Pre-commit Checks${NC}"
echo "===================="
echo "Running quick validation before commit..."
echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Check if quick-check script exists
QUICK_CHECK_SCRIPT="$SCRIPT_DIR/ci-quick-check.sh"
if [[ ! -f "$QUICK_CHECK_SCRIPT" ]]; then
    echo -e "${RED}❌ Quick check script not found: $QUICK_CHECK_SCRIPT${NC}"
    echo -e "${YELLOW}Please ensure the CI scripts are properly installed.${NC}"
    exit 1
fi

# Run quick check with timeout
TIMEOUT=300  # 5 minutes
echo -e "${BLUE}⚡ Running quick checks (timeout: ${TIMEOUT}s)...${NC}"

if timeout "$TIMEOUT" bash "$QUICK_CHECK_SCRIPT"; then
    echo -e "\n${GREEN}✅ Pre-commit checks passed!${NC}"
    echo ""
    echo "🚀 Ready to commit!"
    exit 0
else
    EXIT_CODE=$?

    if [ $EXIT_CODE -eq 124 ]; then
        echo -e "\n${YELLOW}⏰ Pre-commit checks timed out after ${TIMEOUT}s${NC}"
        echo -e "${YELLOW}This might happen on first run or after major changes.${NC}"
    else
        echo -e "\n${RED}❌ Pre-commit checks failed!${NC}"
    fi

    echo ""
    echo -e "${YELLOW}Please fix the issues above before committing.${NC}"
    echo ""
    echo "Quick fixes:"
    echo "  1. Run: cargo fmt"
    echo "  2. Run: cargo clippy --fix --allow-dirty --allow-staged"
    echo "  3. Run: $QUICK_CHECK_SCRIPT"
    echo ""
    echo "To bypass these checks (not recommended):"
    echo "  git commit --no-verify"
    echo ""
    echo "To disable pre-commit checks permanently:"
    echo "  rm .git/hooks/pre-commit"

    exit 1
fi