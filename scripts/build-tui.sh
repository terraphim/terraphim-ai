#!/bin/bash
# build-tui.sh - TUI-specific build script for development and production
# Part of TUI remediation Phase 1: Emergency Stabilization

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
BUILD_PROFILE=${BUILD_PROFILE:-"debug"}
FEATURES=${FEATURES:-"repl-full"}
TARGET=${TARGET:-""}

echo -e "${BLUE}=== TUI Build Script ===${NC}"
echo "Profile: $BUILD_PROFILE"
echo "Features: $FEATURES"
if [[ -n "$TARGET" ]]; then
    echo "Target: $TARGET"
fi
echo ""

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

TUI-specific build script for optimized development workflows.

OPTIONS:
    --help                  Show this help message
    --profile PROFILE       Build profile: debug|release|release-lto (default: debug)
    --features FEATURES     Feature flags (default: repl-full)
    --target TARGET         Target triple (default: native)
    --dev                   Development build (debug + repl-full)
    --optimized             Optimized build (release-lto + repl-full)
    --test                  Build for testing (debug + all features)
    --run                   Build and run TUI immediately

EXAMPLES:
    $0 --dev                # Development build
    $0 --optimized          # Optimized production build
    $0 --test               # Build with all features for testing
    $0 --run --dev          # Build and run development version

BUILD PROFILES:
    debug           - Fast compilation, no optimizations
    release         - Optimized, debug info included
    release-lto     - Maximum optimizations, LTO enabled (smallest binary)

FEATURES:
    repl-full       - Full REPL functionality (recommended)
    openrouter      - OpenRouter AI integration
    mcp-rust-sdk    - MCP SDK integration

EOF
}

# Function to build TUI
build_tui() {
    local profile_flag=""
    local target_flag=""

    # Set profile flag
    case "$BUILD_PROFILE" in
        "debug")
            # No additional flags needed for debug
            ;;
        "release")
            profile_flag="--release"
            ;;
        "release-lto")
            profile_flag="--profile release-lto"
            ;;
        *)
            echo -e "${RED}Unknown profile: $BUILD_PROFILE${NC}"
            exit 1
            ;;
    esac

    # Set target flag
    if [[ -n "$TARGET" ]]; then
        target_flag="--target $TARGET"
    fi

    echo -e "${BLUE}🔨 Building TUI...${NC}"
    echo "Command: cargo build -p terraphim_tui $profile_flag --features $FEATURES $target_flag"

    # Build the TUI
    if cargo build -p terraphim_tui $profile_flag --features "$FEATURES" $target_flag; then
        echo -e "${GREEN}✅ Build successful${NC}"

        # Show binary info
        local target_dir="target/${TARGET:-$(rustc -vV | grep host | cut -d' ' -f2)}/$BUILD_PROFILE"
        if [[ "$BUILD_PROFILE" == "release-lto" ]]; then
            target_dir="target/${TARGET:-$(rustc -vV | grep host | cut -d' ' -f2)}/release-lto"
        fi

        local binary_path="$target_dir/terraphim-agent"
        if [[ -f "$binary_path" ]]; then
            local size=$(stat -f%z "$binary_path" 2>/dev/null || stat -c%s "$binary_path" 2>/dev/null || echo "unknown")
            echo -e "${GREEN}📦 Binary: $binary_path (${size} bytes)${NC}"
        fi

        return 0
    else
        echo -e "${RED}❌ Build failed${NC}"
        return 1
    fi
}

# Function to run TUI
run_tui() {
    local target_dir="target/${TARGET:-$(rustc -vV | grep host | cut -d' ' -f2)}/$BUILD_PROFILE"
    if [[ "$BUILD_PROFILE" == "release-lto" ]]; then
        target_dir="target/${TARGET:-$(rustc -vV | grep host | cut -d' ' -f2)}/release-lto"
    fi

    local binary_path="$target_dir/terraphim-agent"

    if [[ -f "$binary_path" ]]; then
        echo -e "${BLUE}🚀 Running TUI...${NC}"
        "$binary_path" "$@"
    else
        echo -e "${RED}❌ Binary not found: $binary_path${NC}"
        echo "Build first with: $0"
        exit 1
    fi
}

# Main function
main() {
    local run_after_build=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_usage
                exit 0
                ;;
            --profile)
                BUILD_PROFILE="$2"
                shift 2
                ;;
            --features)
                FEATURES="$2"
                shift 2
                ;;
            --target)
                TARGET="$2"
                shift 2
                ;;
            --dev)
                BUILD_PROFILE="debug"
                FEATURES="repl-full"
                shift
                ;;
            --optimized)
                BUILD_PROFILE="release-lto"
                FEATURES="repl-full"
                shift
                ;;
            --test)
                BUILD_PROFILE="debug"
                FEATURES="repl-full,openrouter,mcp-rust-sdk"
                shift
                ;;
            --run)
                run_after_build=true
                shift
                ;;
            -*)
                echo -e "${RED}Unknown option: $1${NC}" >&2
                show_usage
                exit 1
                ;;
            *)
                echo -e "${RED}Unknown argument: $1${NC}" >&2
                show_usage
                exit 1
                ;;
        esac
    done

    # Setup
    cd "$PROJECT_ROOT"

    # Build TUI
    if build_tui; then
        if [[ "$run_after_build" == "true" ]]; then
            echo ""
            run_tui
        fi
    else
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
