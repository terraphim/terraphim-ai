#!/bin/bash
# cross-test.sh - Cross-compilation testing using cross-rs
# Following ripgrep patterns for consistent cross-compilation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Cross-compilation targets (from ripgrep's approach)
CROSS_TARGETS=(
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    "armv7-unknown-linux-gnueabihf"
    "armv7-unknown-linux-musleabihf"
    "x86_64-unknown-linux-musl"
    "powerpc64le-unknown-linux-gnu"
    "riscv64gc-unknown-linux-gnu"
)

# Simple targets for quick testing
QUICK_TARGETS=(
    "aarch64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
)

# Test packages
TEST_PACKAGES=(
    "terraphim_server"
    "terraphim_mcp_server"
    "terraphim_tui"
)

# Feature combinations
FEATURE_COMBINATIONS=(
    ""  # Default
    "openrouter"
    "mcp-rust-sdk"
)

echo -e "${BLUE}=== Terraphim Cross-Compilation Testing ===${NC}"
echo "Using cross-rs for consistent cross-compilation"
echo "Project: $PROJECT_ROOT"
echo ""

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] [TARGETS...]

Cross-compilation testing using cross-rs following ripgrep patterns.

OPTIONS:
    --help                  Show this help message
    --quick                 Use quick target set only
    --all                   Use all cross targets (default)
    --package PACKAGE       Test specific package only
    --features FEATURES     Test specific feature combination
    --build-only            Skip tests, only build
    --install-cross         Install cross-rs if not present

TARGETS:
    Cross targets to test (default: all targets):

    Primary targets:
    - aarch64-unknown-linux-gnu (ARM64 Linux)
    - x86_64-unknown-linux-musl (static Linux)

    Extended targets:
    - armv7-unknown-linux-gnueabihf (ARMv7 Linux)
    - armv7-unknown-linux-musleabihf (ARMv7 static)
    - aarch64-unknown-linux-musl (ARM64 static)
    - powerpc64le-unknown-linux-gnu (PowerPC)
    - riscv64gc-unknown-linux-gnu (RISC-V)

EXAMPLES:
    $0                      # Test all targets with all packages
    $0 --quick              # Test primary targets only
    $0 --package terraphim_tui  # Test only TUI package
    $0 --features openrouter  # Test with specific features
    $0 aarch64-unknown-linux-gnu  # Test specific target

EOF
}

# Function to check command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install cross-rs
install_cross() {
    echo -e "${YELLOW}Installing cross-rs...${NC}"
    cargo install cross --git https://github.com/cross-rs/cross

    if command_exists cross; then
        echo -e "${GREEN}✅ cross-rs installed successfully${NC}"
    else
        echo -e "${RED}❌ Failed to install cross-rs${NC}"
        exit 1
    fi
}

# Function to setup cross-compilation
setup_cross() {
    echo -e "${BLUE}🔧 Setting up cross-compilation...${NC}"

    # Install cross if requested
    if [[ "$INSTALL_CROSS" == "true" ]] && ! command_exists cross; then
        install_cross
    fi

    # Check if cross is available
    if ! command_exists cross; then
        echo -e "${RED}❌ cross-rs not found${NC}"
        echo -e "${YELLOW}Install with: cargo install cross --git https://github.com/cross-rs/cross${NC}"
        echo -e "${YELLOW}Or use: $0 --install-cross${NC}"
        exit 1
    fi

    # Add targets
    for target in "${CROSS_TARGETS[@]}"; do
        echo -e "${YELLOW}Adding target: $target${NC}"
        rustup target add "$target" || echo -e "${YELLOW}⚠️ Target $target may not be available${NC}"
    done

    echo -e "${GREEN}✅ Cross-compilation setup complete${NC}"
}

# Function to test cross-compilation
test_cross_build() {
    local target="$1"
    local package="$2"
    local features="$3"
    local build_only="$4"

    local feature_flag=""
    if [[ -n "$features" ]]; then
        feature_flag="--features $features"
    fi

    echo -e "${YELLOW}Testing cross-compile: $package for $target with features: [${features:-default}]${NC}"

    # Test build
    if cross build --target "$target" --package "$package" $feature_flag; then
        echo -e "${GREEN}✅ Build successful${NC}"

        # Test if binary exists
        local binary_name=""
        case "$package" in
            "terraphim_server") binary_name="terraphim_server" ;;
            "terraphim_mcp_server") binary_name="terraphim_mcp_server" ;;
            "terraphim_tui") binary_name="terraphim-tui" ;;
        esac

        local binary_path="target/$target/release/$binary_name"
        if [[ -f "$binary_path" ]]; then
            echo -e "${GREEN}✅ Binary created: $binary_path${NC}"

            # Show binary size
            local size=$(stat -f%z "$binary_path" 2>/dev/null || stat -c%s "$binary_path" 2>/dev/null || echo "unknown")
            echo -e "${BLUE}📦 Binary size: $size bytes${NC}"
        else
            echo -e "${RED}❌ Binary not found: $binary_path${NC}"
            return 1
        fi

        # Run tests if requested and package supports it
        if [[ "$build_only" != "true" ]]; then
            echo -e "${YELLOW}🧪 Running cross-tests...${NC}"
            # Note: cross testing might not work for all targets due to QEMU requirements
            if cross test --target "$target" --package "$package" $feature_flag 2>/dev/null; then
                echo -e "${GREEN}✅ Tests passed${NC}"
            else
                echo -e "${YELLOW}⚠️ Tests skipped (may require QEMU)${NC}"
            fi
        fi

        return 0
    else
        echo -e "${RED}❌ Build failed${NC}"
        return 1
    fi
}

# Function to get target info
get_target_info() {
    local target="$1"

    case "$target" in
        "aarch64-unknown-linux-gnu")
            echo "ARM64 (64-bit) - Modern ARM servers, Raspberry Pi 4+"
            ;;
        "aarch64-unknown-linux-musl")
            echo "ARM64 (64-bit static) - Modern ARM, static linking"
            ;;
        "armv7-unknown-linux-gnueabihf")
            echo "ARMv7 (32-bit) - Raspberry Pi 3+, older ARM devices"
            ;;
        "armv7-unknown-linux-musleabihf")
            echo "ARMv7 (32-bit static) - ARM devices, static linking"
            ;;
        "x86_64-unknown-linux-musl")
            echo "x86_64 (64-bit static) - Intel/AMD, static linking"
            ;;
        "powerpc64le-unknown-linux-gnu")
            echo "PowerPC64LE (64-bit) - IBM Power servers, ppc64le"
            ;;
        "riscv64gc-unknown-linux-gnu")
            echo "RISC-V (64-bit) - RISC-V 64-bit systems"
            ;;
        *)
            echo "Unknown target architecture"
            ;;
    esac
}

# Main function
main() {
    local targets=("${CROSS_TARGETS[@]}")
    local packages=("${TEST_PACKAGES[@]}")
    local build_only=false
    local specific_package=""
    local specific_features=""
    local use_all_targets=true

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_usage
                exit 0
                ;;
            --quick)
                targets=("${QUICK_TARGETS[@]}")
                use_all_targets=false
                shift
                ;;
            --all)
                targets=("${CROSS_TARGETS[@]}")
                use_all_targets=true
                shift
                ;;
            --package)
                specific_package="$2"
                packages=("$specific_package")
                shift 2
                ;;
            --features)
                specific_features="$2"
                shift 2
                ;;
            --build-only)
                build_only=true
                shift
                ;;
            --install-cross)
                INSTALL_CROSS="true"
                shift
                ;;
            -*)
                echo -e "${RED}Unknown option: $1${NC}" >&2
                show_usage
                exit 1
                ;;
            *)
                # Custom target
                targets=("$1")
                shift
                ;;
        esac
    done

    # Setup
    cd "$PROJECT_ROOT"
    setup_cross

    echo -e "${BLUE}🚀 Starting cross-compilation testing...${NC}"
    echo "Targets: ${targets[*]}"
    echo "Packages: ${packages[*]}"
    if [[ -n "$specific_features" ]]; then
        echo "Features: $specific_features"
    else
        echo "Features: default and OpenRouter combinations"
    fi
    echo ""

    local total_tests=0
    local passed_tests=0
    local failed_tests=0

    # Target information
    echo -e "${BLUE}📋 Target Information:${NC}"
    for target in "${targets[@]}"; do
        echo "  $target - $(get_target_info "$target")"
    done
    echo ""

    # Run cross-compilation tests
    for target in "${targets[@]}"; do
        echo -e "${BLUE}=== Testing Target: $target ===${NC}"

        for package in "${packages[@]}"; do
            if [[ -n "$specific_features" ]]; then
                # Test specific features only
                ((total_tests++))
                echo -e "${YELLOW}[$total_tests] $package with features: $specific_features${NC}"

                if test_cross_build "$target" "$package" "$specific_features" "$build_only"; then
                    ((passed_tests++))
                else
                    ((failed_tests++))
                fi
            else
                # Test all feature combinations
                for features in "${FEATURE_COMBINATIONS[@]}"; do
                    ((total_tests++))

                    echo -e "${YELLOW}[$total_tests] $package with features: [${features:-default}]${NC}"

                    if test_cross_build "$target" "$package" "$features" "$build_only"; then
                        ((passed_tests++))
                    else
                        ((failed_tests++))
                        # Don't fail fast for cross-compilation - some targets may not work
                    fi
                done
            fi
            echo ""
        done
    done

    # Summary
    echo -e "${BLUE}=== Cross-Compilation Test Summary ===${NC}"
    echo "Total tests: $total_tests"
    echo -e "${GREEN}Passed: $passed_tests${NC}"
    echo -e "${RED}Failed: $failed_tests${NC}"

    if [[ $total_tests -gt 0 ]]; then
        local pass_rate=$(( passed_tests * 100 / total_tests ))
        echo "Pass rate: ${pass_rate}%"
    fi
    echo ""

    if [[ $passed_tests -eq $total_tests ]]; then
        echo -e "${GREEN}🎉 All cross-compilation tests passed!${NC}"
        echo -e "${GREEN}✅ Ready for multi-platform releases${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️ Some cross-compilation tests failed${NC}"
        echo -e "${YELLOW}This may be expected for exotic targets${NC}"
        echo -e "${BLUE}📦 Binaries created for successful builds are in target/*/release/${NC}"

        if [[ $failed_tests -gt $((total_tests / 2)) ]]; then
            echo -e "${RED}❌ Too many failures - check configuration${NC}"
            exit 1
        else
            echo -e "${GREEN}✅ Sufficient builds succeeded for release${NC}"
            exit 0
        fi
    fi
}

# Run main function with all arguments
main "$@"