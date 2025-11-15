#!/bin/bash
# Terraphim CI Build Orchestration Script
# Replaces Earthly build pipeline with GitHub Actions + Docker Buildx

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/target"
ARTIFACTS_DIR="${PROJECT_ROOT}/artifacts"

# Default values
RUST_VERSION="1.85.0"
NODE_VERSION="20"
UBUNTU_VERSIONS=("20.04" "22.04" "24.04")
RUST_TARGETS=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "armv7-unknown-linux-gnueabihf")
BUILD_TYPE="release"
SKIP_TESTS="false"
SKIP_DOCKER="false"
SKIP_PACKAGES="false"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Terraphim CI Build Script - GitHub Actions + Docker Buildx

OPTIONS:
    -h, --help              Show this help message
    -t, --targets TARGET    Comma-separated list of Rust targets
    -u, --ubuntu VERSIONS   Comma-separated list of Ubuntu versions
    -b, --build-type TYPE   Build type: release or debug (default: release)
    --skip-tests            Skip running tests
    --skip-docker           Skip Docker image builds
    --skip-packages         Skip .deb package creation
    --rust-version VERSION  Rust toolchain version (default: $RUST_VERSION)
    --node-version VERSION  Node.js version (default: $NODE_VERSION)

EXAMPLES:
    $0                                      # Build with defaults
    $0 --targets x86_64-unknown-linux-gnu  # Build single target
    $0 --ubuntu 22.04,24.04                # Build specific Ubuntu versions
    $0 --skip-docker --skip-packages       # Build binaries only

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -t|--targets)
                IFS=',' read -ra RUST_TARGETS <<< "$2"
                shift 2
                ;;
            -u|--ubuntu)
                IFS=',' read -ra UBUNTU_VERSIONS <<< "$2"
                shift 2
                ;;
            -b|--build-type)
                BUILD_TYPE="$2"
                shift 2
                ;;
            --skip-tests)
                SKIP_TESTS="true"
                shift
                ;;
            --skip-docker)
                SKIP_DOCKER="true"
                shift
                ;;
            --skip-packages)
                SKIP_PACKAGES="true"
                shift
                ;;
            --rust-version)
                RUST_VERSION="$2"
                shift 2
                ;;
            --node-version)
                NODE_VERSION="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Environment setup
setup_environment() {
    log_info "Setting up build environment..."

    # Create directories
    mkdir -p "$BUILD_DIR" "$ARTIFACTS_DIR"

    # Install required tools if not present
    if ! command -v rustup &> /dev/null; then
        log_info "Installing Rust toolchain $RUST_VERSION..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$RUST_VERSION"
        source "$HOME/.cargo/env"
    fi

    # Install Node.js if needed for local builds
    if ! command -v node &> /dev/null; then
        log_warning "Node.js not found. Install Node.js $NODE_VERSION for frontend builds."
    fi

    # Install Docker if needed for local builds
    if ! command -v docker &> /dev/null && [[ "$SKIP_DOCKER" == "false" ]]; then
        log_warning "Docker not found. Install Docker for multi-arch builds."
    fi

    log_success "Environment setup complete"
}

# Build frontend
build_frontend() {
    log_info "Building frontend..."

    cd "$PROJECT_ROOT/desktop"

    if ! command -v yarn &> /dev/null; then
        log_info "Installing Yarn..."
        npm install -g yarn
    fi

    log_info "Installing frontend dependencies..."
    yarn install --frozen-lockfile

    log_info "Running frontend linting..."
    yarn run check

    if [[ "$SKIP_TESTS" == "false" ]]; then
        log_info "Running frontend tests..."
        yarn test
    fi

    log_info "Building frontend..."
    yarn run build

    # Copy dist to terraphim_server
    cp -r dist ../terraphim_server/

    cd "$PROJECT_ROOT"
    log_success "Frontend build complete"
}

# Build Rust project for specific target
build_rust_target() {
    local target="$1"
    local ubuntu_version="$2"

    log_info "Building Rust target: $target (Ubuntu $ubuntu_version)"

    # Add target if not present
    rustup target add "$target"

    # Set up cross-compilation environment
    case "$target" in
        "aarch64-unknown-linux-gnu")
            export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
            export CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++"
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"
            ;;
        "armv7-unknown-linux-gnueabihf")
            export CC_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-gcc"
            export CXX_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-g++"
            export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="arm-linux-gnueabihf-gcc"
            ;;
        "x86_64-unknown-linux-musl")
            export CC_x86_64_unknown_linux_musl="musl-gcc"
            ;;
    esac

    # Build command
    local build_flag=""
    if [[ "$BUILD_TYPE" == "release" ]]; then
        build_flag="--release"
    fi

    cargo build $build_flag --target "$target" \
        --package terraphim_server \
        --package terraphim_mcp_server \
        --package terraphim_agent

    # Test binaries
    local target_dir="target/$target/$BUILD_TYPE"
    "$target_dir/terraphim_server" --version
    "$target_dir/terraphim_mcp_server" --version
    "$target_dir/terraphim-agent" --version

    # Copy binaries to artifacts
    local artifact_dir="$ARTIFACTS_DIR/binaries/$target-ubuntu$ubuntu_version"
    mkdir -p "$artifact_dir"
    cp "$target_dir/terraphim_server" "$artifact_dir/"
    cp "$target_dir/terraphim_mcp_server" "$artifact_dir/"
    cp "$target_dir/terraphim-agent" "$artifact_dir/"

    log_success "Rust build complete for $target"
}

# Create .deb packages
create_deb_packages() {
    local target="$1"
    local ubuntu_version="$2"

    log_info "Creating .deb package for $target (Ubuntu $ubuntu_version)"

    # Skip musl targets for .deb packages
    if [[ "$target" == *"musl"* ]]; then
        log_warning "Skipping .deb package for musl target: $target"
        return 0
    fi

    # Install cargo-deb if not present
    if ! command -v cargo-deb &> /dev/null; then
        log_info "Installing cargo-deb..."
        cargo install cargo-deb
    fi

    # Create .deb package
    cargo deb --target "$target" --package terraphim_server --no-build

    # Find and rename the .deb file
    local deb_file
    deb_file=$(find "target/$target/debian" -name "*.deb" | head -1)

    if [[ -n "$deb_file" ]]; then
        local version
        version=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "terraphim_server") | .version')

        local arch
        arch=$(echo "$target" | cut -d'-' -f1)

        local new_name="terraphim-server_${version}_ubuntu${ubuntu_version}_${arch}.deb"
        local dest_path="$ARTIFACTS_DIR/packages/$new_name"

        mkdir -p "$ARTIFACTS_DIR/packages"
        mv "$deb_file" "$dest_path"

        log_success "Created .deb package: $new_name"
    else
        log_error "Failed to find .deb package for $target"
    fi
}

# Build Docker images
build_docker_images() {
    log_info "Building Docker images..."

    if [[ "$SKIP_DOCKER" == "true" ]]; then
        log_info "Skipping Docker builds"
        return 0
    fi

    # Set up Docker Buildx
    docker buildx create --name terraphim-builder --use --bootstrap || true

    for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
        log_info "Building Docker image for Ubuntu $ubuntu_version"

        local tag="terraphim-server:ubuntu$ubuntu_version"

        docker buildx build \
            --platform linux/amd64,linux/arm64,linux/arm/v7 \
            --build-arg UBUNTU_VERSION="$ubuntu_version" \
            --build-arg RUST_VERSION="$RUST_VERSION" \
            --build-arg NODE_VERSION="$NODE_VERSION" \
            --tag "$tag" \
            --file docker/Dockerfile.multiarch \
            --load \
            .

        log_success "Docker image built: $tag"
    done
}

# Run tests
run_tests() {
    if [[ "$SKIP_TESTS" == "true" ]]; then
        log_info "Skipping tests"
        return 0
    fi

    log_info "Running test suite..."

    # Unit tests
    cargo test --workspace --lib

    # Integration tests
    cargo test --workspace --test '*'

    # Doc tests
    cargo test --workspace --doc

    log_success "All tests passed"
}

# Generate build summary
generate_summary() {
    log_info "Generating build summary..."

    local summary_file="$ARTIFACTS_DIR/build-summary.txt"

    cat > "$summary_file" << EOF
Terraphim Build Summary
======================

Build Type: $BUILD_TYPE
Rust Version: $RUST_VERSION
Node.js Version: $NODE_VERSION

Ubuntu Versions:
$(printf "  - %s\n" "${UBUNTU_VERSIONS[@]}")

Rust Targets:
$(printf "  - %s\n" "${RUST_TARGETS[@]}")

Artifacts Generated:
$(find "$ARTIFACTS_DIR" -type f | sed 's|.*artifacts/||' | sort | sed 's/^/  - /')

Build completed at: $(date)
EOF

    log_info "Build summary saved to: $summary_file"
    cat "$summary_file"
}

# Main build function
main() {
    log_info "Starting Terraphim CI build..."
    log_info "Build type: $BUILD_TYPE"
    log_info "Ubuntu versions: ${UBUNTU_VERSIONS[*]}"
    log_info "Rust targets: ${RUST_TARGETS[*]}"

    setup_environment
    build_frontend

    # Build for each combination of target and Ubuntu version
    for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
        for target in "${RUST_TARGETS[@]}"; do
            build_rust_target "$target" "$ubuntu_version"

            if [[ "$SKIP_PACKAGES" == "false" ]]; then
                create_deb_packages "$target" "$ubuntu_version"
            fi
        done
    done

    run_tests
    build_docker_images
    generate_summary

    log_success "Build completed successfully!"
    log_info "Artifacts available in: $ARTIFACTS_DIR"
}

# Parse arguments and run main function
parse_args "$@"
main
