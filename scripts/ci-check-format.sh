#!/bin/bash

# CI Format Check Script
# Mirrors the lint-and-format job from ci-native.yml
# Usage: ./scripts/ci-check-format.sh

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

echo -e "${BLUE}🧪 CI Format Check${NC}"
echo "==================="
echo "Mirroring GitHub Actions lint-and-format job"
echo ""

# Build frontend assets (required by terraphim_server build.rs)
echo -e "${BLUE}🌐 Building frontend assets...${NC}"
if command -v node &> /dev/null && command -v yarn &> /dev/null; then
    cd "$PROJECT_ROOT/desktop"
    yarn install --frozen-lockfile 2>/dev/null || yarn install
    yarn build
    cd "$PROJECT_ROOT"
    echo -e "${GREEN}  ✅ Frontend assets built${NC}"
else
    echo -e "${YELLOW}  ⚠️  Node.js/yarn not found, creating placeholder dist...${NC}"
    mkdir -p "$PROJECT_ROOT/terraphim_server/dist"
    echo '<!DOCTYPE html><html><body>Terraphim Server (CI placeholder)</body></html>' > "$PROJECT_ROOT/terraphim_server/dist/index.html"
fi

# Install system dependencies (same as CI)
echo -e "${BLUE}📦 Installing system dependencies...${NC}"
sudo apt-get update -qq
sudo apt-get install -yqq --no-install-recommends \
    build-essential \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libsoup2.4-dev \
    librsvg2-dev || true
# Try webkit 4.1 first (Ubuntu 22.04+), then 4.0 (Ubuntu 20.04)
sudo apt-get install -yqq --no-install-recommends \
    libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev 2>/dev/null || \
sudo apt-get install -yqq --no-install-recommends \
    libwebkit2gtk-4.0-dev libjavascriptcoregtk-4.0-dev
# Try ayatana-appindicator (newer) or appindicator (older)
sudo apt-get install -yqq --no-install-recommends \
    libayatana-appindicator3-dev 2>/dev/null || \
sudo apt-get install -yqq --no-install-recommends \
    libappindicator3-dev || true

# Install Rust toolchain (same version as CI)
echo -e "${BLUE}🦀 Installing Rust toolchain...${NC}"
if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Ensure we're using the correct Rust version
RUST_VERSION="1.87.0"
echo "Setting Rust version to $RUST_VERSION"
rustup default "$RUST_VERSION"
rustup component add rustfmt clippy

# Verify Rust version
ACTUAL_RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo "Current Rust version: $ACTUAL_RUST_VERSION"

if [[ "$ACTUAL_RUST_VERSION" != "$RUST_VERSION"* ]]; then
    echo -e "${YELLOW}⚠️  Warning: Rust version mismatch. Expected: $RUST_VERSION, Got: $ACTUAL_RUST_VERSION${NC}"
fi

# Set environment variables (same as CI)
export CARGO_TERM_COLOR=always

echo -e "${BLUE}🔍 Running cargo fmt check...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}  ✅ cargo fmt check passed${NC}"
else
    echo -e "${RED}  ❌ cargo fmt check failed${NC}"
    echo -e "${YELLOW}  Fix with: cargo fmt${NC}"
    exit 1
fi

echo -e "${BLUE}🔍 Running cargo clippy...${NC}"
# Pre-build dependencies to avoid timeout
echo "Pre-building dependencies..."
if cargo build --workspace --all-targets --all-features; then
    echo -e "${GREEN}  ✅ Dependencies pre-built successfully${NC}"
else
    echo -e "${YELLOW}  ⚠️  Dependency pre-build had issues, continuing anyway${NC}"
fi

# Run clippy with optimized flags and extended timeout
# Note: -D clippy::all turns clippy warnings to errors
# Allow certain lints that are common in test code and scaffolding:
# - dead_code, unused: experimental/scaffolding code
# - bool_assert_comparison, assertions_on_constants: test assertion patterns
# - useless_vec, items_after_test_module, module_inception: test organization
# - bool_comparison, nonminimal_bool: test boolean expressions
# - redundant_clone: performance not critical in tests
if timeout 1200 cargo clippy --workspace --all-targets --all-features --message-format=short -- \
    -D clippy::all \
    -A clippy::nursery -A clippy::pedantic \
    -A dead_code -A unused \
    -A clippy::bool_assert_comparison -A clippy::assertions_on_constants \
    -A clippy::useless_vec -A clippy::items_after_test_module -A clippy::module_inception \
    -A clippy::bool_comparison -A clippy::nonminimal_bool \
    -A clippy::redundant_clone; then
    echo -e "${GREEN}  ✅ cargo clippy check passed${NC}"
else
    echo -e "${RED}  ❌ cargo clippy check failed or timed out${NC}"
    echo -e "${YELLOW}  Fix clippy warnings and try again${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 All format checks passed!${NC}"
echo ""
echo "✅ Code is properly formatted"
echo "✅ No clippy warnings found"
echo ""
echo "Ready to commit!"
