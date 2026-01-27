#!/bin/bash
# Workspace Build Script with Optimized Settings
# Reduces disk usage while maintaining build performance

set -euo pipefail

# Configuration
PROFILE=${PROFILE:-"ci-release"}
TARGET=${TARGET:-"x86_64-unknown-linux-gnu"}
FEATURES=${FEATURES:-""}
CLEAN_FIRST=${CLEAN_FIRST:-"false"}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

show_help() {
    cat << EOF
Terraphim AI Workspace Build Script

Usage: $0 [OPTIONS]

OPTIONS:
    -p, --profile PROFILE   Build profile (dev, release, ci, ci-release, size-optimized)
                            Default: ci-release
    -t, --target TARGET     Target triple
                            Default: x86_64-unknown-linux-gnu
    -f, --features FEATURES Comma-separated list of features
    -c, --clean             Clean before building
    -h, --help              Show this help message

PROFILES:
    dev             Fast builds with debug info
    release         Optimized release build
    ci              CI-optimized dev build (faster, less disk)
    ci-release      CI-optimized release build (default)
    size-optimized  Maximum size reduction

EXAMPLES:
    # Build with default settings (ci-release profile)
    $0

    # Build with specific features
    $0 --features "sqlite,redis"

    # Clean build for release
    $0 --profile release --clean

    # Build for different target
    $0 --target aarch64-unknown-linux-gnu

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--profile)
            PROFILE="$2"
            shift 2
            ;;
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -f|--features)
            FEATURES="$2"
            shift 2
            ;;
        -c|--clean)
            CLEAN_FIRST="true"
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Validate profile
valid_profiles=("dev" "release" "ci" "ci-release" "size-optimized")
if [[ ! " ${valid_profiles[@]} " =~ " ${PROFILE} " ]]; then
    log_error "Invalid profile: $PROFILE"
    log_info "Valid profiles: ${valid_profiles[*]}"
    exit 1
fi

log_step "Building Terraphim AI Workspace"
log_info "Profile: $PROFILE"
log_info "Target: $TARGET"
[[ -n "$FEATURES" ]] && log_info "Features: $FEATURES"

# Clean if requested
if [[ "$CLEAN_FIRST" == "true" ]]; then
    log_step "Cleaning previous builds..."
    cargo clean
fi

# Set environment variables for optimal builds
export CARGO_INCREMENTAL=1
export CARGO_PROFILE_DEV_CODEGEN_UNITS=256
export CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=unpacked

# Build command construction
BUILD_CMD="cargo build --profile $PROFILE --target $TARGET"

# Add features if specified
if [[ -n "$FEATURES" ]]; then
    BUILD_CMD="$BUILD_CMD --features \"$FEATURES\""
fi

# Show disk usage before build
log_step "Disk usage before build:"
df -h | grep -E '(Filesystem|/dev/)' || true

# Build workspace libraries first (parallel friendly)
log_step "Building workspace libraries..."
$BUILD_CMD --workspace --lib

# Build specific binaries
log_step "Building terraphim_server..."
$BUILD_CMD --package terraphim_server

log_step "Building terraphim_mcp_server..."
$BUILD_CMD --package terraphim_mcp_server 2>/dev/null || log_warn "terraphim_mcp_server not found"

log_step "Building terraphim-agent..."
$BUILD_CMD --package terraphim-agent 2>/dev/null || log_warn "terraphim-agent not found"

# Show binary sizes
log_step "Build complete! Binary sizes:"
TARGET_DIR="target/$TARGET/$PROFILE"
if [[ -d "$TARGET_DIR" ]]; then
    for binary in "$TARGET_DIR"/terraphim*; do
        if [[ -f "$binary" && -x "$binary" ]]; then
            size=$(du -h "$binary" | cut -f1)
            name=$(basename "$binary")
            log_info "$name: $size"
        fi
    done
else
    log_warn "Target directory not found: $TARGET_DIR"
fi

# Show disk usage after build
log_step "Disk usage after build:"
df -h | grep -E '(Filesystem|/dev/)' || true

# Show target directory size
if [[ -d "target" ]]; then
    log_info "Target directory size: $(du -sh target 2>/dev/null | cut -f1)"
fi

log_step "Build completed successfully!"
