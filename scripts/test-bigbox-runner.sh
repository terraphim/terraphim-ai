#!/bin/bash

# Bigbox Runner Validation Script
# Tests that the bigbox server has all required dependencies for CI
# Usage: ./scripts/test-bigbox-runner.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Bigbox Runner Validation${NC}"
echo "=============================="
echo ""

# Check system information
echo -e "${BLUE}📋 System Information${NC}"
echo "OS: $(uname -s) $(uname -r)"
echo "Architecture: $(uname -m)"
echo "Available memory: $(free -h | grep '^Mem:' | awk '{print $7}')"
echo "Available disk space: $(df -h . | tail -1 | awk '{print $4}')"
echo ""

# Check required system tools
echo -e "${BLUE}🔍 Checking Required System Tools${NC}"
REQUIRED_TOOLS=("git" "curl" "wget" "docker" "python3" "make" "g++")

for tool in "${REQUIRED_TOOLS[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} $tool"
    else
        echo -e "  ${RED}✗${NC} $tool (missing)"
        echo -e "${YELLOW}  Install with: sudo apt-get install $tool${NC}"
    fi
done

# Check Rust toolchain
echo ""
echo -e "${BLUE}🦀 Checking Rust Toolchain${NC}"

if command -v cargo &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo -e "  ${GREEN}✓${NC} Rust $RUST_VERSION"

    # Check required components
    if cargo fmt --version &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} rustfmt"
    else
        echo -e "  ${RED}✗${NC} rustfmt (missing)"
        echo -e "${YELLOW}  Install with: rustup component add rustfmt${NC}"
    fi

    if cargo clippy --version &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} clippy"
    else
        echo -e "  ${RED}✗${NC} clippy (missing)"
        echo -e "${YELLOW}  Install with: rustup component add clippy${NC}"
    fi
else
    echo -e "  ${RED}✗${NC} Rust not installed"
    echo -e "${YELLOW}  Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y${NC}"
fi

# Check Node.js
echo ""
echo -e "${BLUE}📦 Checking Node.js${NC}"

if command -v node &> /dev/null; then
    NODE_VERSION=$(node --version)
    echo -e "  ${GREEN}✓${NC} Node.js $NODE_VERSION"

    if command -v npm &> /dev/null; then
        NPM_VERSION=$(npm --version)
        echo -e "  ${GREEN}✓${NC} npm $NPM_VERSION"
    else
        echo -e "  ${RED}✗${NC} npm (missing)"
    fi
else
    echo -e "  ${RED}✗${NC} Node.js not installed"
    echo -e "${YELLOW}  Install with: curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && sudo apt-get install -y nodejs${NC}"
fi

# Check system dependencies for Rust compilation
echo ""
echo -e "${BLUE}📚 Checking Rust Build Dependencies${NC}"
BUILD_DEPS=("clang" "libclang-dev" "pkg-config" "libssl-dev" "build-essential")

for dep in "${BUILD_DEPS[@]}"; do
    if dpkg -l | grep -q "^ii.*$dep "; then
        echo -e "  ${GREEN}✓${NC} $dep"
    else
        echo -e "  ${RED}✗${NC} $dep (missing)"
        echo -e "${YELLOW}  Install with: sudo apt-get install $dep${NC}"
    fi
done

# Check system dependencies for Node.js modules
echo ""
echo -e "${BLUE}🎨 Checking Node.js Build Dependencies${NC}"
NODE_DEPS=("libcairo2-dev" "libpango1.0-dev" "libjpeg-dev" "libgif-dev" "librsvg2-dev")

for dep in "${NODE_DEPS[@]}"; do
    if dpkg -l | grep -q "^ii.*$dep "; then
        echo -e "  ${GREEN}✓${NC} $dep"
    else
        echo -e "  ${RED}✗${NC} $dep (missing)"
        echo -e "${YELLOW}  Install with: sudo apt-get install $dep${NC}"
    fi
done

# Test basic Rust project compilation
echo ""
echo -e "${BLUE}🧪 Testing Basic Rust Compilation${NC}"

if command -v cargo &> /dev/null; then
    echo "Creating test Rust project..."
    TEST_DIR=$(mktemp -d)
    cd "$TEST_DIR"

    cargo init --name test-project --bin > /dev/null 2>&1
    echo 'fn main() { println!("Hello from bigbox!"); }' > src/main.rs

    if cargo build --release > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} Basic Rust compilation successful"
        echo -e "  ${GREEN}✓${NC} Test binary: $(ls target/release/test-project)"
    else
        echo -e "  ${RED}✗${NC} Rust compilation failed"
    fi

    cd - > /dev/null
    rm -rf "$TEST_DIR"
else
    echo -e "  ${YELLOW}⚠${NC} Skipping Rust test (cargo not available)"
fi

# Test basic Node.js project
echo ""
echo -e "${BLUE}🧪 Testing Basic Node.js Project${NC}"

if command -v npm &> /dev/null; then
    echo "Creating test Node.js project..."
    TEST_DIR=$(mktemp -d)
    cd "$TEST_DIR"

    npm init -y > /dev/null 2>&1
    echo 'console.log("Hello from bigbox!");' > test.js

    if node test.js > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} Basic Node.js execution successful"
    else
        echo -e "  ${RED}✗${NC} Node.js execution failed"
    fi

    cd - > /dev/null
    rm -rf "$TEST_DIR"
else
    echo -e "  ${YELLOW}⚠${NC} Skipping Node.js test (npm not available)"
fi

# Check Docker
echo ""
echo -e "${BLUE}🐳 Checking Docker${NC}"

if command -v docker &> /dev/null; then
    if docker info > /dev/null 2>&1; then
        echo -e "  ${GREEN}✓${NC} Docker daemon running"
        DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | cut -d',' -f1)
        echo -e "  ${GREEN}✓${NC} Docker $DOCKER_VERSION"
    else
        echo -e "  ${RED}✗${NC} Docker daemon not running"
        echo -e "${YELLOW}  Start with: sudo systemctl start docker${NC}"
    fi
else
    echo -e "  ${RED}✗${NC} Docker not installed"
fi

# Check available disk space for caching
echo ""
echo -e "${BLUE}💾 Checking Cache Storage${NC}"

if [ -d "/home/runner" ]; then
    RUNNER_HOME="/home/runner"
elif [ -d "$HOME" ]; then
    RUNNER_HOME="$HOME"
else
    RUNNER_HOME="/tmp"
fi

CACHE_SPACE=$(df -h "$RUNNER_HOME" | tail -1 | awk '{print $4}')
echo -e "  ${GREEN}✓${NC} Available cache space: $CACHE_SPACE"

# Summary
echo ""
echo -e "${BLUE}📊 Validation Summary${NC}"
echo "=========================="

# Count results
TOTAL_CHECKS=0
PASSED_CHECKS=0

# Simple validation completion check
echo "Validation completed. Review the output above for any missing dependencies."
echo ""
echo -e "${GREEN}✅ Bigbox runner validation complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Install any missing dependencies identified above"
echo "2. Set up GitHub Actions runner with labels: self-hosted, linux, bigbox"
echo "3. Test runner with: ./run.sh --run"
echo "4. Verify CI workflows use the runner correctly"