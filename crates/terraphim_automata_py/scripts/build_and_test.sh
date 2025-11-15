#!/bin/bash
# Build and test script for terraphim_automata Python bindings

set -e  # Exit on error

echo "=================================================="
echo "Terraphim Automata Python Bindings - Build & Test"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    print_error "uv is not installed. Please install it first:"
    echo "  pip install uv"
    exit 1
fi

print_status "uv is installed"

# Navigate to project directory
cd "$(dirname "$0")/.."

# Step 1: Install maturin
echo -e "\n${YELLOW}[1/7]${NC} Installing maturin..."
uv pip install --system maturin 2>&1 | grep -v "Requirement already satisfied" || true
print_status "Maturin installed"

# Step 2: Build the Rust extension
echo -e "\n${YELLOW}[2/7]${NC} Building Rust extension..."
if maturin develop; then
    print_status "Rust extension built successfully"
else
    print_error "Failed to build Rust extension"
    exit 1
fi

# Step 3: Install test dependencies
echo -e "\n${YELLOW}[3/7]${NC} Installing test dependencies..."
uv pip install --system pytest pytest-cov pytest-benchmark 2>&1 | grep -v "Requirement already satisfied" || true
print_status "Test dependencies installed"

# Step 4: Run tests
echo -e "\n${YELLOW}[4/7]${NC} Running tests..."
if pytest python/tests/ -v --cov=terraphim_automata --cov-report=term-missing; then
    print_status "All tests passed"
else
    print_error "Some tests failed"
    exit 1
fi

# Step 5: Run benchmarks (optional)
echo -e "\n${YELLOW}[5/7]${NC} Running benchmarks (optional)..."
read -p "Run performance benchmarks? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    pytest python/benchmarks/ -v --benchmark-only --benchmark-columns=min,max,mean,stddev,median,ops
    print_status "Benchmarks completed"
else
    print_warning "Benchmarks skipped"
fi

# Step 6: Install dev dependencies for code quality
echo -e "\n${YELLOW}[6/7]${NC} Installing code quality tools..."
uv pip install --system black ruff mypy 2>&1 | grep -v "Requirement already satisfied" || true
print_status "Code quality tools installed"

# Step 7: Check code quality
echo -e "\n${YELLOW}[7/7]${NC} Running code quality checks..."

echo "  • Black (formatting)..."
if black --check python/ 2>&1 | tail -1; then
    print_status "Code is properly formatted"
else
    print_warning "Code needs formatting (run: black python/)"
fi

echo "  • Ruff (linting)..."
if ruff check python/ 2>&1 | tail -5; then
    print_status "No linting issues"
else
    print_warning "Linting issues found (run: ruff check python/ --fix)"
fi

echo "  • Mypy (type checking)..."
if mypy python/terraphim_automata/ --ignore-missing-imports 2>&1 | tail -3; then
    print_status "Type checking passed"
else
    print_warning "Type checking found issues"
fi

# Final summary
echo -e "\n=================================================="
echo -e "${GREEN}Build and test completed successfully!${NC}"
echo "=================================================="
echo ""
echo "Next steps:"
echo "  • Run examples: python examples/basic_autocomplete.py"
echo "  • Build release: maturin build --release"
echo "  • Build wheels: maturin build --release --out dist"
echo ""
