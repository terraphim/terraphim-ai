#!/bin/bash

# CI Rust Build Check Script
# Mirrors the build-rust job from ci-native.yml
# Usage: ./scripts/ci-check-rust.sh [target]

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

echo -e "${BLUE}🦀 CI Rust Build Check${NC}"
echo "======================="
echo "Mirroring GitHub Actions build-rust job"
echo ""

# Configuration (same as CI)
TARGET="${1:-x86_64-unknown-linux-gnu}"
RUST_VERSION="1.87.0"
CARGO_TERM_COLOR="always"

echo "Target: $TARGET"
echo "Rust version: $RUST_VERSION"
echo ""

# Install system dependencies (same as CI)
echo -e "${BLUE}📦 Installing system dependencies...${NC}"
sudo apt-get update -qq
sudo apt-get install -yqq --no-install-recommends \
    build-essential \
    bison \
    flex \
    ca-certificates \
    openssl \
    libssl-dev \
    bc \
    wget \
    git \
    curl \
    cmake \
    pkg-config \
    musl-tools \
    musl-dev \
    software-properties-common \
    gpg-agent \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libsoup2.4-dev \
    libjavascriptcoregtk-4.0-dev \
    libappindicator3-dev \
    librsvg2-dev \
    clang \
    libclang-dev \
    llvm-dev \
    libc++-dev \
    libc++abi-dev

# Setup cross-compilation toolchain if needed
if [[ "$TARGET" != "x86_64-unknown-linux-gnu" ]]; then
    echo -e "${BLUE}🔧 Setting up cross-compilation toolchain for $TARGET...${NC}"
    case "$TARGET" in
        "aarch64-unknown-linux-gnu")
            sudo apt-get install -yqq gcc-aarch64-linux-gnu libc6-dev-arm64-cross
            export "CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc"
            export "CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++"
            export "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
            ;;
        "armv7-unknown-linux-musleabihf"|"armv7-unknown-linux-gnueabihf")
            sudo apt-get install -yqq gcc-arm-linux-gnueabihf libc6-dev-armhf-cross
            export "CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc"
            export "CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++"
            export "CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc"
            ;;
        "x86_64-unknown-linux-musl")
            export "CC_x86_64_unknown_linux_musl=musl-gcc"
            ;;
        *)
            echo -e "${YELLOW}⚠️  Unknown target $TARGET, skipping cross-compilation setup${NC}"
            ;;
    esac
fi

# Install Rust toolchain (same version as CI)
echo -e "${BLUE}🦀 Installing Rust toolchain...${NC}"
if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$RUST_VERSION"
    source "$HOME/.cargo/env"
else
    rustup default "$RUST_VERSION"
fi

# Add target and components
echo -e "${BLUE}🎯 Adding Rust target and components...${NC}"
rustup target add "$TARGET"
rustup component add clippy rustfmt

# Verify Rust version
ACTUAL_RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo "Current Rust version: $ACTUAL_RUST_VERSION"

if [[ "$ACTUAL_RUST_VERSION" != "$RUST_VERSION"* ]]; then
    echo -e "${YELLOW}⚠️  Warning: Rust version mismatch. Expected: $RUST_VERSION, Got: $ACTUAL_RUST_VERSION${NC}"
fi

# Set environment variables
export CARGO_TERM_COLOR="$CARGO_TERM_COLOR"
export CARGO_HOME="$HOME/.cargo"

# Create frontend dist directory if it doesn't exist (for embedding)
echo -e "${BLUE}📂 Setting up frontend dist directory...${NC}"
mkdir -p terraphim_server/dist
if [[ -d desktop/dist ]]; then
    echo "Copying frontend dist to terraphim_server..."
    cp -r desktop/dist/* terraphim_server/dist/ || echo "No frontend files found to copy"
else
    echo "No desktop/dist found, creating placeholder"
    echo '<html><body><h1>No Frontend</h1></body></html>' > terraphim_server/dist/index.html
fi

echo -e "${BLUE}🏗️  Building Rust project...${NC}"
echo "Building main binaries for target $TARGET..."

# Build all main binaries (same as CI)
BUILD_PACKAGES=(
    "terraphim_server"
    "terraphim_mcp_server"
    "terraphim_tui"
)

BUILD_SUCCESS=true
for package in "${BUILD_PACKAGES[@]}"; do
    echo "Building $package..."
    if cargo build --release --target "$TARGET" --package "$package"; then
        echo -e "${GREEN}  ✅ $package built successfully${NC}"
    else
        echo -e "${RED}  ❌ $package build failed${NC}"
        BUILD_SUCCESS=false
    fi
done

if [[ "$BUILD_SUCCESS" == "true" ]]; then
    echo -e "${BLUE}🧪 Testing built binaries...${NC}"

    # Test binaries exist and can run version command (same as CI)
    BINARY_PATH="target/$TARGET/release"
    for binary in "terraphim_server" "terraphim_mcp_server" "terraphim-tui"; do
        if [[ -f "$BINARY_PATH/$binary" ]]; then
            echo "Testing $binary --version"
            if "$BINARY_PATH/$binary" --version; then
                echo -e "${GREEN}  ✅ $binary runs successfully${NC}"
            else
                echo -e "${RED}  ❌ $binary failed to run${NC}"
                BUILD_SUCCESS=false
            fi
        else
            echo -e "${RED}  ❌ $binary not found at $BINARY_PATH/$binary${NC}"
            BUILD_SUCCESS=false
        fi
    done
fi

if [[ "$BUILD_SUCCESS" == "true" ]]; then
    echo -e "${GREEN}🎉 Rust build check completed successfully!${NC}"
    echo ""
    echo "✅ All binaries built successfully for $TARGET"
    echo "✅ Binaries are executable"
    echo "✅ Build artifacts available in target/$TARGET/release/"
    echo ""
    echo "Built binaries:"
    ls -la target/$TARGET/release/terraphim*
else
    echo -e "${RED}❌ Rust build check failed!${NC}"
    exit 1
fi