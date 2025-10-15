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

# Install system dependencies (same as CI)
echo -e "${BLUE}📦 Installing system dependencies...${NC}"
sudo apt-get update -qq
sudo apt-get install -yqq --no-install-recommends \
    build-essential \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libssl-dev

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
# Run clippy with same flags as CI
if cargo clippy --workspace --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}  ✅ cargo clippy check passed${NC}"
else
    echo -e "${RED}  ❌ cargo clippy check failed${NC}"
    echo -e "${YELLOW}  Fix clippy warnings and try again${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 All format checks passed!${NC}"
echo ""
echo "✅ Code is properly formatted"
echo "✅ No clippy warnings found"
echo ""
echo "Ready to commit!"