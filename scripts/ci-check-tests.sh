#!/bin/bash

# CI Test Suite Check Script
# Mirrors the test-suite job from ci-native.yml
# Usage: ./scripts/ci-check-tests.sh

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

echo -e "${BLUE}🧪 CI Test Suite Check${NC}"
echo "======================"
echo "Mirroring GitHub Actions test-suite job"
echo ""

# Configuration (same as CI)
RUST_VERSION="1.87.0"
CARGO_TERM_COLOR="always"

# Install system dependencies (same as CI)
echo -e "${BLUE}📦 Installing system dependencies...${NC}"
sudo apt-get update
sudo apt-get install -y \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libsoup2.4-dev \
    pkg-config \
    build-essential

# Create symlinks for webkit2gtk-sys and javascriptcore-rs-sys crates looking for 4.0
echo -e "${BLUE}🔗 Creating library symlinks...${NC}"
sudo ln -sf /usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk-4.1.pc /usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk-4.0.pc
sudo ln -sf /usr/lib/x86_64-linux-gnu/pkgconfig/javascriptcoregtk-4.1.pc /usr/lib/x86_64-linux-gnu/pkgconfig/javascriptcoregtk-4.0.pc
sudo ln -sf /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.0.so
sudo ln -sf /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.1.so /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.0.so

# Install Rust toolchain (same version as CI)
echo -e "${BLUE}🦀 Installing Rust toolchain...${NC}"
if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    rustup default "$RUST_VERSION"
fi

# Verify Rust version
ACTUAL_RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo "Current Rust version: $ACTUAL_RUST_VERSION"

if [[ "$ACTUAL_RUST_VERSION" != "$RUST_VERSION"* ]]; then
    echo -e "${YELLOW}⚠️  Warning: Rust version mismatch. Expected: $RUST_VERSION, Got: $ACTUAL_RUST_VERSION${NC}"
fi

# Set environment variables
export CARGO_TERM_COLOR="$CARGO_TERM_COLOR"

# Create frontend dist directory if it doesn't exist (for integration tests)
echo -e "${BLUE}📂 Setting up frontend dist directory...${NC}"
mkdir -p terraphim_server/dist
if [[ -d desktop/dist ]]; then
    echo "Copying frontend dist to terraphim_server..."
    cp -r desktop/dist/* terraphim_server/dist/ || echo "No frontend files found to copy"
else
    echo "No desktop/dist found, creating placeholder"
    echo '<html><body><h1>No Frontend</h1></body></html>' > terraphim_server/dist/index.html
fi

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
FAILED_TESTS=()

# Function to run test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -e "\n${BLUE}🧪 Running: ${test_name}${NC}"
    echo "Command: $test_command"

    if eval "$test_command"; then
        echo -e "${GREEN}  ✅ PASSED${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}  ❌ FAILED${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        FAILED_TESTS+=("$test_name")
        return 1
    fi
}

echo -e "${BLUE}🧪 Running Unit Tests${NC}"
run_test "Unit Tests" \
    "cargo test --workspace --lib --features test-utils"

echo -e "${BLUE}🧪 Running Integration Tests${NC}"
run_test "Integration Tests" \
    "cargo test --workspace --test '*' --features test-utils"

echo -e "${BLUE}🧪 Running Documentation Tests${NC}"
run_test "Documentation Tests" \
    "cargo test --workspace --doc"

echo -e "${BLUE}📊 Test Results Summary${NC}"
echo "========================="

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
echo "Total test suites: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed!${NC}"
    echo ""
    echo "✅ Unit tests: PASSED"
    echo "✅ Integration tests: PASSED"
    echo "✅ Documentation tests: PASSED"
    echo ""
    echo "Code is ready for deployment!"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "${RED}  - $test${NC}"
    done
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo "1. Check the test output above for details"
    echo "2. Fix failing tests"
    echo "3. Re-run this script to validate fixes"
    exit 1
fi
