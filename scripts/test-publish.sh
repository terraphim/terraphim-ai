#!/usr/bin/env bash
set -euo pipefail

################################################################################
# test-publish.sh
#
# Test publishing scripts locally
#
# Usage:
#   ./scripts/test-publish.sh [TARGET]
#
# Arguments:
#   TARGET    Target to test: crates, pypi, npm, or all (default: all)
#
# Examples:
#   # Test all publishing scripts
#   ./scripts/test-publish.sh
#
#   # Test only crates publishing
#   ./scripts/test-publish.sh crates
#
#   # Test in dry-run mode
#   ./scripts/test-publish.sh all --dry-run
#
################################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

DRY_RUN="${DRY_RUN:-false}"
TARGET="${1:-all}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Testing Terraphim Publishing Scripts${NC}"
echo "======================================"
echo ""

# Test individual script
test_script() {
    local script_name="$1"
    local help_arg="${2:---help}"
    local script_path="$SCRIPT_DIR/$script_name"

    echo -e "${BLUE}Testing: $script_name${NC}"

    if [[ ! -f "$script_path" ]]; then
        echo -e "${RED}✗ Script not found: $script_name${NC}"
        return 1
    fi

    if [[ ! -x "$script_path" ]]; then
        echo -e "${YELLOW}⚠ Making script executable: $script_name${NC}"
        chmod +x "$script_path"
    fi

    # Test help output
    echo -n "  Help output: "
    if "$script_path" "$help_arg" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi

    # Test script syntax
    echo -n "  Syntax check: "
    if bash -n "$script_path" 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi

    echo ""
    return 0
}

# Test crates publishing
test_crates() {
    echo -e "${BLUE}Testing: Crates Publishing${NC}"

    # Check if in project root
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        echo -e "${RED}✗ Not in project root${NC}"
        return 1
    fi

    # Check if crates exist
    if [[ ! -d "$PROJECT_ROOT/crates/terraphim_types" ]]; then
        echo -e "${RED}✗ Crates directory not found${NC}"
        return 1
    fi

    echo -e "${GREEN}✓${NC} Project structure valid"

    # Try dry-run (if no token set, this will still validate the script)
    if [[ "$DRY_RUN" == "true" ]]; then
        echo -n "  Dry-run test: "
        if "$SCRIPT_DIR/publish-crates.sh" --version 0.0.0-test --dry-run > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
        else
            echo -e "${YELLOW}⚠ (may need valid token)${NC}"
        fi
    fi

    echo ""
}

# Test PyPI publishing
test_pypi() {
    echo -e "${BLUE}Testing: PyPI Publishing${NC}"

    # Check package directory
    if [[ ! -f "$PROJECT_ROOT/crates/terraphim_automata_py/pyproject.toml" ]]; then
        echo -e "${RED}✗ Python package not found${NC}"
        return 1
    fi

    echo -e "${GREEN}✓${NC} Python package found"

    # Check required tools
    echo -n "  Python3: "
    if command -v python3 &> /dev/null; then
        echo -e "${GREEN}✓${NC} ($(python3 --version))"
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi

    echo -n "  pip: "
    if python3 -m pip --version &> /dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi

    echo -n "  twine: "
    if python3 -m twine --version &> /dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Not installed${NC}"
    fi

    echo -n "  maturin: "
    if python3 -m maturin --version &> /dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${YELLOW}⚠ Not installed${NC}"
    fi

    echo ""
}

# Test npm publishing
test_npm() {
    echo -e "${BLUE}Testing: npm Publishing${NC}"

    # Check package directory
    if [[ ! -f "$PROJECT_ROOT/terraphim_ai_nodejs/package.json" ]]; then
        echo -e "${RED}✗ Node.js package not found${NC}"
        return 1
    fi

    echo -e "${GREEN}✓${NC} Node.js package found"

    # Check required tools
    echo -n "  Node.js: "
    if command -v node &> /dev/null; then
        echo -e "${GREEN}✓${NC} ($(node --version))"
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi

    echo -n "  npm: "
    if command -v npm &> /dev/null; then
        echo -e "${GREEN}✓${NC} ($(npm --version))"
    else
        echo -e "${RED}✗${NC}"
    fi

    echo -n "  yarn: "
    if command -v yarn &> /dev/null; then
        echo -e "${GREEN}✓${NC} ($(yarn --version))"
    else
        echo -e "${YELLOW}⚠ Not installed${NC}"
    fi

    echo ""
}

# Summary
show_summary() {
    echo ""
    echo "======================================"
    echo -e "${GREEN}Testing Complete!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Set up tokens (if not already set):"
    echo "     - CARGO_REGISTRY_TOKEN for crates.io"
    echo "     - PYPI_API_TOKEN for PyPI"
    echo "     - NPM_TOKEN for npm"
    echo ""
    echo "  2. Test dry-run publishing:"
    echo "     ./scripts/publish-crates.sh -v 1.0.0 -d"
    echo "     ./scripts/publish-pypi.sh -v 1.0.0 -d"
    echo "     ./scripts/publish-npm.sh -v 1.0.0 -d"
    echo ""
    echo "  3. For real publishing (double-check version!):"
    echo "     ./scripts/publish-crates.sh -v 1.0.1"
    echo "     ./scripts/publish-pypi.sh -v 1.0.1"
    echo "     ./scripts/publish-npm.sh -v 1.0.1"
    echo ""
}

# Parse arguments
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
    esac
done

# Run tests
FAILED=0

# Test scripts
test_script "publish-crates.sh" || FAILED=1
test_script "publish-pypi.sh" || FAILED=1
test_script "publish-npm.sh" || FAILED=1

# Test targets
case "$TARGET" in
    crates)
        test_crates || FAILED=1
        ;;
    pypi)
        test_pypi || FAILED=1
        ;;
    npm)
        test_npm || FAILED=1
        ;;
    all)
        test_crates || FAILED=1
        test_pypi || FAILED=1
        test_npm || FAILED=1
        ;;
    *)
        echo -e "${RED}Unknown target: $TARGET${NC}"
        echo "Usage: $0 [crates|pypi|npm|all]"
        exit 1
        ;;
esac

# Summary
show_summary

if [[ $FAILED -eq 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
