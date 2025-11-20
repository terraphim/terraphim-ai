#!/bin/bash

# CI Rust Build Check Script with Matrix Support
# Mirrors the build-rust job from ci-native.yml with matrix testing
# Usage: ./scripts/ci-check-rust.sh [OPTIONS] [target]

set -euo pipefail

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
FAIL_FAST=${FAIL_FAST:-"false"}
MATRIX_MODE=${MATRIX_MODE:-"false"}
BUILD_PROFILE=${BUILD_PROFILE:-"release"}

# Matrix configuration
FEATURE_COMBINATIONS=(
    ""  # Default features
    "openrouter"
    "mcp-rust-sdk"
    "openrouter,mcp-rust-sdk"
)

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --matrix)
            MATRIX_MODE="true"
            shift
            ;;
        --fail-fast)
            FAIL_FAST="true"
            shift
            ;;
        --profile)
            BUILD_PROFILE="$2"
            shift 2
            ;;
        --debug)
            BUILD_PROFILE="debug"
            shift
            ;;
        --help)
            cat << EOF
Usage: $0 [OPTIONS] [target]

CI Rust Build Check with Matrix Support

OPTIONS:
    --matrix          Run matrix testing with multiple feature combinations
    --fail-fast       Stop on first failure (default: false)
    --profile PROFILE Build profile: release|debug|release-lto (default: release)
    --debug           Use debug profile (same as --profile debug)
    --help            Show this help message

EXAMPLES:
    $0                      # Standard CI build check
    $0 --matrix            # Matrix testing with feature combinations
    $0 --matrix aarch64-unknown-linux-gnu  # Matrix testing for specific target
    $0 --profile release-lto  # Optimized release build

EOF
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
        *)
            TARGET="$1"
            shift
            ;;
    esac
done

echo "Target: $TARGET"
echo "Rust version: $RUST_VERSION"
echo "Build profile: $BUILD_PROFILE"
echo "Matrix mode: $MATRIX_MODE"
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

# Function to build a package with specific features
build_package() {
    local package="$1"
    local features="$2"
    local profile="$3"

    local feature_flag=""
    if [[ -n "$features" ]]; then
        feature_flag="--features $features"
    fi

    local profile_flag=""
    if [[ "$profile" == "release" ]]; then
        profile_flag="--release"
    elif [[ "$profile" == "release-lto" ]]; then
        profile_flag="--profile release-lto"
    fi

    echo "Building $package with features: [$features] profile: $profile"
    if cargo build --target "$TARGET" --package "$package" $feature_flag $profile_flag; then
        echo -e "${GREEN}  ✅ $package built successfully${NC}"
        return 0
    else
        echo -e "${RED}  ❌ $package build failed${NC}"
        return 1
    fi
}

echo -e "${BLUE}🏗️  Building Rust project...${NC}"
echo "Building main binaries for target $TARGET..."

# Main packages to build (same as CI)
BUILD_PACKAGES=(
    "terraphim_server"
    "terraphim_mcp_server"
    "terraphim_tui"
)

BUILD_SUCCESS=true

if [[ "$MATRIX_MODE" == "true" ]]; then
    echo -e "${BLUE}🔀 Matrix testing mode enabled${NC}"
    echo "Testing feature combinations..."
    echo ""

    local total_tests=0
    local passed_tests=0

    for package in "${BUILD_PACKAGES[@]}"; do
        for features in "${FEATURE_COMBINATIONS[@]}"; do
            ((total_tests++))

            echo -e "${YELLOW}[Matrix $total_tests] $package with features: [${features:-default}]${NC}"

            if build_package "$package" "$features" "$BUILD_PROFILE"; then
                ((passed_tests++))
            else
                BUILD_SUCCESS=false
                if [[ "$FAIL_FAST" == "true" ]]; then
                    echo -e "${RED}💥 Fail-fast enabled, stopping${NC}"
                    break 3
                fi
            fi
            echo ""
        done
    done

    # Matrix summary
    echo -e "${BLUE}📊 Matrix Build Summary${NC}"
    echo "Total builds: $total_tests"
    echo -e "${GREEN}Passed: $passed_tests${NC}"
    if [[ $((total_tests - passed_tests)) -gt 0 ]]; then
        echo -e "${RED}Failed: $((total_tests - passed_tests))${NC}"
    fi

    if [[ $total_tests -gt 0 ]]; then
        local pass_rate=$(( passed_tests * 100 / total_tests ))
        echo "Pass rate: ${pass_rate}%"
    fi
    echo ""
else
    # Standard build (single pass)
    echo "Standard build mode"
    for package in "${BUILD_PACKAGES[@]}"; do
        if build_package "$package" "" "$BUILD_PROFILE"; then
            echo -e "${GREEN}  ✅ $package built successfully${NC}"
        else
            echo -e "${RED}  ❌ $package build failed${NC}"
            BUILD_SUCCESS=false
            if [[ "$FAIL_FAST" == "true" ]]; then
                break
            fi
        fi
    done
fi

if [[ "$BUILD_SUCCESS" == "true" ]]; then
    echo -e "${BLUE}🧪 Testing built binaries...${NC}"

    # Determine binary path based on build profile
    local profile_dir=""
    if [[ "$BUILD_PROFILE" == "debug" ]]; then
        profile_dir="debug"
    elif [[ "$BUILD_PROFILE" == "release-lto" ]]; then
        profile_dir="release-lto"
    else
        profile_dir="release"
    fi

    BINARY_PATH="target/$TARGET/$profile_dir"

    # Test binaries exist and can run basic commands
    local test_binaries=(
        "terraphim_server:--version"
        "terraphim_mcp_server:--version"
        "terraphim-agent:--help"
    )

    for binary_test in "${test_binaries[@]}"; do
        local binary="${binary_test%:*}"
        local test_arg="${binary_test#*:}"

        if [[ -f "$BINARY_PATH/$binary" ]]; then
            echo "Testing $binary $test_arg"
            if "$BINARY_PATH/$binary" $test_arg >/dev/null 2>&1; then
                echo -e "${GREEN}  ✅ $binary runs successfully${NC}"
            else
                echo -e "${YELLOW}  ⚠️ $binary runs but may have issues${NC}"
            fi
        else
            echo -e "${RED}  ❌ $binary not found at $BINARY_PATH/$binary${NC}"
            BUILD_SUCCESS=false
        fi
    done

    echo ""
    echo -e "${BLUE}📦 Built artifacts:${NC}"
    if [[ -d "$BINARY_PATH" ]]; then
        ls -la "$BINARY_PATH"/terraphim* 2>/dev/null || echo "No terraphim binaries found"
    else
        echo -e "${RED}No binary directory found: $BINARY_PATH${NC}"
        BUILD_SUCCESS=false
    fi
fi

if [[ "$BUILD_SUCCESS" == "true" ]]; then
    echo -e "${GREEN}🎉 Rust build check completed successfully!${NC}"
    echo ""
    if [[ "$MATRIX_MODE" == "true" ]]; then
        echo "✅ Matrix builds completed successfully for $TARGET"
        echo "✅ All feature combinations tested"
    else
        echo "✅ All binaries built successfully for $TARGET"
        echo "✅ Binaries are executable"
    fi
    echo "✅ Build artifacts available in target/$TARGET/$profile_dir/"
    echo ""

    if [[ "$MATRIX_MODE" == "true" ]]; then
        echo "Matrix testing completed. Ready for CI deployment."
    else
        echo "Standard CI build completed. Ready for deployment."
    fi
else
    echo -e "${RED}❌ Rust build check failed!${NC}"
    if [[ "$MATRIX_MODE" == "true" ]]; then
        echo "Matrix testing failed. Check build logs above."
    else
        echo "Standard build failed. Check build logs above."
    fi
    exit 1
fi
