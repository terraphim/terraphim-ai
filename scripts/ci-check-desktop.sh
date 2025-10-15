#!/bin/bash

# CI Desktop Test Check Script
# Mirrors the test-desktop job from ci-native.yml
# Usage: ./scripts/ci-check-desktop.sh

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

echo -e "${BLUE}🖥️  CI Desktop Test Check${NC}"
echo "========================"
echo "Mirroring GitHub Actions test-desktop job"
echo ""

# Configuration (same as CI)
NODE_VERSION="18"
DESKTOP_DIR="$PROJECT_ROOT/desktop"

if [[ ! -d "$DESKTOP_DIR" ]]; then
    echo -e "${RED}❌ Desktop directory not found: $DESKTOP_DIR${NC}"
    exit 1
fi

# Check Node.js version
if command -v node &> /dev/null; then
    CURRENT_NODE_VERSION=$(node --version | sed 's/v//')
    echo "Current Node.js version: $CURRENT_NODE_VERSION"

    if [[ "$CURRENT_NODE_VERSION" != "$NODE_VERSION"* ]]; then
        echo -e "${YELLOW}⚠️  Node.js version mismatch. Expected: $NODE_VERSION, Got: $CURRENT_NODE_VERSION${NC}"
    fi
else
    echo -e "${RED}❌ Node.js not installed${NC}"
    exit 1
fi

# Install system dependencies (same as CI)
echo -e "${BLUE}📦 Installing system dependencies...${NC}"
sudo apt-get update
sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libsoup2.4-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    pkg-config

# Create symlinks for webkit2gtk-sys and javascriptcore-rs-sys crates looking for 4.0
echo -e "${BLUE}🔗 Creating library symlinks...${NC}"
sudo ln -sf /usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk-4.1.pc /usr/lib/x86_64-linux-gnu/pkgconfig/webkit2gtk-4.0.pc
sudo ln -sf /usr/lib/x86_64-linux-gnu/pkgconfig/javascriptcoregtk-4.1.pc /usr/lib/x86_64-linux-gnu/pkgconfig/javascriptcoregtk-4.0.pc
sudo ln -sf /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.1.so /usr/lib/x86_64-linux-gnu/libwebkit2gtk-4.0.so
sudo ln -sf /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.1.so /usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-4.0.so

cd "$DESKTOP_DIR"

echo -e "${BLUE}📦 Installing frontend dependencies...${NC}"
if [[ -f yarn.lock ]]; then
    yarn install --frozen-lockfile
else
    yarn install
fi

echo -e "${BLUE}🎭 Installing Playwright browsers...${NC}"
npx playwright install --with-deps

echo -e "${BLUE}📂 Setting up frontend dist directory...${NC}"
# Create or copy frontend dist (same as CI)
mkdir -p dist
if [[ -d ../desktop/dist ]]; then
    echo "Copying frontend files..."
    cp -r ../desktop/dist/* dist/ || echo "No frontend files found to copy"
else
    echo "No frontend dist found, using existing or creating placeholder"
    if [[ ! -f dist/index.html ]]; then
        echo '<html><body><h1>Frontend Placeholder</h1></body></html>' > dist/index.html
    fi
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

echo -e "${BLUE}🧪 Running Frontend Unit Tests${NC}"
run_test "Frontend Unit Tests" \
    "yarn test --reporter=verbose --run"

echo -e "${BLUE}🧪 Running E2E Tests${NC}"
run_test "E2E Tests" \
    "CI=true yarn e2e --reporter=line --bail=1"

echo -e "${BLUE}🧪 Running Config Wizard Tests${NC}"
run_test "Config Wizard Tests" \
    "CI=true npx playwright test tests/e2e/config-wizard.spec.ts --reporter=line"

echo -e "${BLUE}📊 Desktop Test Results Summary${NC}"
echo "================================"

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED))
echo "Total test suites: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "${RED}Failed: $TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All desktop tests passed!${NC}"
    echo ""
    echo "✅ Frontend unit tests: PASSED"
    echo "✅ E2E tests: PASSED"
    echo "✅ Config wizard tests: PASSED"
    echo ""
    echo "Desktop application is ready for deployment!"
    exit 0
else
    echo -e "\n${RED}❌ Some desktop tests failed:${NC}"
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