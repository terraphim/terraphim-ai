#!/bin/bash
# Docker Buildx Multi-Architecture Build Script for Terraphim
# Builds Docker images for multiple Ubuntu versions and architectures

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default values
PLATFORMS="linux/amd64,linux/arm64,linux/arm/v7"
UBUNTU_VERSIONS=("20.04" "22.04" "24.04")
RUST_VERSION="1.85.0"
NODE_VERSION="20"
PUSH="false"
TAG="latest"
REGISTRY="ghcr.io"
IMAGE_NAME="terraphim-ai/terraphim-server"
BUILD_ARGS=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Docker Buildx Multi-Architecture Build Script

OPTIONS:
    -h, --help                    Show this help message
    -p, --platforms PLATFORMS    Target platforms (default: $PLATFORMS)
    -u, --ubuntu-versions VERS   Comma-separated Ubuntu versions (default: ${UBUNTU_VERSIONS[*]})
    -t, --tag TAG                Docker image tag (default: $TAG)
    -r, --registry REGISTRY      Container registry (default: $REGISTRY)
    -i, --image IMAGE            Image name (default: $IMAGE_NAME)
    --push                       Push images to registry
    --rust-version VERSION       Rust version (default: $RUST_VERSION)
    --node-version VERSION       Node.js version (default: $NODE_VERSION)
    --build-arg ARG=VALUE        Additional build arguments
    --no-cache                   Build without cache
    --ubuntu18                   Include Ubuntu 18.04 in build

EXAMPLES:
    $0                                          # Build all versions locally
    $0 --push --tag v1.0.0                    # Build and push with tag
    $0 --ubuntu-versions 22.04,24.04          # Build specific Ubuntu versions
    $0 --platforms linux/amd64                # Build single architecture
    $0 --build-arg FEATURES=openrouter        # Pass additional build args

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
            -p|--platforms)
                PLATFORMS="$2"
                shift 2
                ;;
            -u|--ubuntu-versions)
                IFS=',' read -ra UBUNTU_VERSIONS <<< "$2"
                shift 2
                ;;
            -t|--tag)
                TAG="$2"
                shift 2
                ;;
            -r|--registry)
                REGISTRY="$2"
                shift 2
                ;;
            -i|--image)
                IMAGE_NAME="$2"
                shift 2
                ;;
            --push)
                PUSH="true"
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
            --build-arg)
                BUILD_ARGS="$BUILD_ARGS --build-arg $2"
                shift 2
                ;;
            --no-cache)
                BUILD_ARGS="$BUILD_ARGS --no-cache"
                shift
                ;;
            --ubuntu18)
                UBUNTU_VERSIONS=("18.04" "${UBUNTU_VERSIONS[@]}")
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Setup Docker Buildx
setup_buildx() {
    log_info "Setting up Docker Buildx..."

    # Check if Docker is available
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    # Enable experimental features
    export DOCKER_CLI_EXPERIMENTAL=enabled

    # Check if buildx is available
    if ! docker buildx version &> /dev/null; then
        log_error "Docker Buildx is not available"
        log_info "Please update Docker to a version that includes Buildx"
        exit 1
    fi

    # Create or use existing builder
    local builder_name="terraphim-builder"

    if ! docker buildx inspect "$builder_name" &> /dev/null; then
        log_info "Creating new Buildx builder: $builder_name"
        docker buildx create \
            --name "$builder_name" \
            --driver docker-container \
            --bootstrap
    fi

    # Use the builder
    docker buildx use "$builder_name"

    # Inspect builder capabilities
    log_info "Builder capabilities:"
    docker buildx inspect --bootstrap

    log_success "Docker Buildx setup complete"
}

# Build single Ubuntu version
build_ubuntu_version() {
    local ubuntu_version="$1"
    local full_image_name="$REGISTRY/$IMAGE_NAME"

    log_info "Building Docker image for Ubuntu $ubuntu_version..."
    log_info "Platforms: $PLATFORMS"
    log_info "Image: $full_image_name:$TAG-ubuntu$ubuntu_version"

    # Build command
    local build_cmd=(
        docker buildx build
        --platform "$PLATFORMS"
        --file "$PROJECT_ROOT/docker/Dockerfile.multiarch"
        --build-arg "UBUNTU_VERSION=$ubuntu_version"
        --build-arg "RUST_VERSION=$RUST_VERSION"
        --build-arg "NODE_VERSION=$NODE_VERSION"
        --tag "$full_image_name:$TAG-ubuntu$ubuntu_version"
        --tag "$full_image_name:latest-ubuntu$ubuntu_version"
    )

    # Add custom build args
    if [[ -n "$BUILD_ARGS" ]]; then
        read -ra EXTRA_ARGS <<< "$BUILD_ARGS"
        build_cmd+=("${EXTRA_ARGS[@]}")
    fi

    # Add cache settings
    build_cmd+=(
        --cache-from "type=local,src=/tmp/.buildx-cache-$ubuntu_version"
        --cache-to "type=local,dest=/tmp/.buildx-cache-$ubuntu_version,mode=max"
    )

    # Add provenance and SBOM settings (disable for smaller images)
    build_cmd+=(
        --provenance=false
        --sbom=false
    )

    # Push or load based on configuration
    if [[ "$PUSH" == "true" ]]; then
        build_cmd+=(--push)
        log_info "Images will be pushed to registry"
    else
        # For local builds, we can't load multi-arch images
        # We'll build without loading for testing purposes
        log_warning "Multi-arch images cannot be loaded locally. Building for registry push only."
        log_info "Use --push to push to registry, or specify single platform with --platforms"
    fi

    # Add build context
    build_cmd+=("$PROJECT_ROOT")

    # Execute build
    log_info "Executing: ${build_cmd[*]}"
    "${build_cmd[@]}"

    log_success "Build completed for Ubuntu $ubuntu_version"
}

# Verify built images
verify_images() {
    if [[ "$PUSH" != "true" ]]; then
        log_info "Skipping image verification (images not pushed)"
        return 0
    fi

    log_info "Verifying built images..."

    for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
        local full_image_name="$REGISTRY/$IMAGE_NAME:$TAG-ubuntu$ubuntu_version"

        log_info "Verifying image: $full_image_name"

        # Inspect the multi-arch manifest
        if docker buildx imagetools inspect "$full_image_name" > /tmp/inspect_output 2>&1; then
            log_success "Image verified: $full_image_name"

            # Show architecture information
            log_info "Available architectures:"
            grep -A 10 "Manifests:" /tmp/inspect_output | grep "Platform:" | sed 's/^/  /'
        else
            log_error "Failed to verify image: $full_image_name"
            cat /tmp/inspect_output
        fi
    done
}

# Test image functionality
test_images() {
    if [[ "$PUSH" != "true" ]]; then
        log_info "Skipping image testing (images not pushed)"
        return 0
    fi

    log_info "Testing image functionality..."

    # Test on current platform if supported
    local current_platform
    current_platform="linux/$(docker version --format '{{.Server.Arch}}')"

    if echo "$PLATFORMS" | grep -q "$current_platform"; then
        local ubuntu_version="${UBUNTU_VERSIONS[0]}"  # Test first Ubuntu version
        local test_image="$REGISTRY/$IMAGE_NAME:$TAG-ubuntu$ubuntu_version"

        log_info "Testing image on $current_platform: $test_image"

        # Test basic functionality
        if docker run --rm --platform="$current_platform" "$test_image" --version; then
            log_success "Image test passed"
        else
            log_error "Image test failed"
        fi
    else
        log_info "Current platform $current_platform not in build targets, skipping functional test"
    fi
}

# Generate build summary
generate_summary() {
    log_info "Build Summary"
    log_info "============="
    log_info "Ubuntu Versions: ${UBUNTU_VERSIONS[*]}"
    log_info "Platforms: $PLATFORMS"
    log_info "Registry: $REGISTRY"
    log_info "Image: $IMAGE_NAME"
    log_info "Tag: $TAG"
    log_info "Pushed: $PUSH"
    log_info "Rust Version: $RUST_VERSION"
    log_info "Node.js Version: $NODE_VERSION"

    if [[ "$PUSH" == "true" ]]; then
        log_info ""
        log_info "Available Images:"
        for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
            log_info "  docker pull $REGISTRY/$IMAGE_NAME:$TAG-ubuntu$ubuntu_version"
        done
    fi
}

# Cleanup function
cleanup() {
    log_info "Cleaning up build cache..."

    # Clean up temporary cache directories
    for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
        if [[ -d "/tmp/.buildx-cache-$ubuntu_version" ]]; then
            rm -rf "/tmp/.buildx-cache-$ubuntu_version"
        fi
    done
}

# Main function
main() {
    log_info "Starting Docker multi-architecture build..."

    cd "$PROJECT_ROOT"

    setup_buildx

    # Build each Ubuntu version
    for ubuntu_version in "${UBUNTU_VERSIONS[@]}"; do
        build_ubuntu_version "$ubuntu_version"
    done

    verify_images
    test_images
    generate_summary

    log_success "All builds completed successfully!"
}

# Trap cleanup on exit
trap cleanup EXIT

# Parse arguments and run main function
parse_args "$@"
main
