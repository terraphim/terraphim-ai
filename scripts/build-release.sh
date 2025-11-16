#!/bin/bash
# build-release.sh - Production release builds following ripgrep patterns
# Creates optimized release artifacts with proper checksums and packaging

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RUST_VERSION=${RUST_VERSION:-"1.87.0}
BUILD_PROFILE=${BUILD_PROFILE:-"release-lto"}
OUTPUT_DIR=${OUTPUT_DIR:-"$PROJECT_ROOT/release-artifacts"}
VERSION=${VERSION:-"$(date +%Y%m%d-%H%M%S)"}
CREATE_DEB=${CREATE_DEB:-"true"}

# Release targets (matching ripgrep's approach)
RELEASE_TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "armv7-unknown-linux-gnueabihf"
)

# Feature combinations for releases
RELEASE_FEATURES=(
    ""  # Default
    "openrouter"
    "mcp-rust-sdk"
    "openrouter,mcp-rust-sdk"
)

# TUI-specific releases
TUI_FEATURES=(
    "repl-full"
    "repl-full,openrouter"
)

echo -e "${BLUE}=== Terraphim Release Build Script ===${NC}"
echo "Following ripgrep/jiff patterns for production builds"
echo "Project: $PROJECT_ROOT"
echo "Version: $VERSION"
echo "Build profile: $BUILD_PROFILE"
echo "Output directory: $OUTPUT_DIR"
echo ""

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Production release build script following ripgrep patterns.

OPTIONS:
    --help                  Show this help message
    --version VERSION       Set version string (default: YYYYMMDD-HHMMSS)
    --profile PROFILE       Build profile: release|release-lto (default: release-lto)
    --target TARGET         Build specific target only
    --features FEATURES     Build specific feature combination only
    --tui-only              Build only TUI artifacts
    --no-deb                Skip .deb package creation
    --output-dir DIR        Output directory (default: ./release-artifacts)

ENVIRONMENT VARIABLES:
    RUST_VERSION            Rust version to use (default: 1.87.0)
    CREATE_DEB              Create .deb packages (default: true)
    VERSION                 Release version string

EXAMPLES:
    $0                      # Full release build for all targets
    $0 --target x86_64-unknown-linux-gnu  # Build specific target
    $0 --tui-only          # Build only TUI artifacts
    $0 --profile release     # Use standard release profile

TARGETS:
    Primary targets for releases:
    - x86_64-unknown-linux-gnu (standard Linux)
    - x86_64-unknown-linux-musl (static Linux)
    - aarch64-unknown-linux-gnu (ARM64 Linux)
    - armv7-unknown-linux-gnueabihf (ARMv7 Linux)

EOF
}

# Function to check dependencies
check_dependencies() {
    echo -e "${BLUE}🔧 Checking dependencies...${NC}"

    local deps=("cargo" "tar" "gzip" "sha256sum")
    if [[ "$CREATE_DEB" == "true" ]]; then
        deps+=("dpkg-deb")
    fi

    for dep in "${deps[@]}"; do
        if ! command_exists "$dep"; then
            echo -e "${RED}❌ Missing dependency: $dep${NC}"
            exit 1
        fi
    done

    echo -e "${GREEN}✅ All dependencies available${NC}"
}

# Function to setup cross-compilation
setup_cross_compilation() {
    echo -e "${BLUE}🔧 Setting up cross-compilation...${NC}"

    # Install cross if not present
    if ! command_exists cross; then
        echo -e "${YELLOW}Installing cross...${NC}"
        cargo install cross --git https://github.com/cross-rs/cross
    fi

    # Add targets
    for target in "${RELEASE_TARGETS[@]}"; do
        echo -e "${YELLOW}Adding target: $target${NC}"
        rustup target add "$target" || true
    done

    echo -e "${GREEN}✅ Cross-compilation setup complete${NC}"
}

# Function to build package
build_package() {
    local target="$1"
    local features="$2"
    local package="$3"

    local feature_flag=""
    if [[ -n "$features" ]]; then
        feature_flag="--features $features"
    fi

    local profile_flag=""
    if [[ "$BUILD_PROFILE" == "release-lto" ]]; then
        profile_flag="--profile release-lto"
    else
        profile_flag="--release"
    fi

    echo -e "${YELLOW}Building: $package for $target with features: [${features:-default}]${NC}"

    if [[ "$target" == "x86_64-unknown-linux-gnu" ]]; then
        # Use cargo for native builds
        cargo build --target "$target" --package "$package" $feature_flag $profile_flag
    else
        # Use cross for cross-compilation
        cross build --target "$target" --package "$package" $feature_flag $profile_flag
    fi
}

# Function to create release package
create_package() {
    local target="$1"
    local features="$2"
    local package="$3"

    local target_dir="target/$target/$BUILD_PROFILE"
    local binary_name=""

    # Determine binary name
    case "$package" in
        "terraphim_server")
            binary_name="terraphim_server"
            ;;
        "terraphim_mcp_server")
            binary_name="terraphim_mcp_server"
            ;;
        "terraphim_agent")
            binary_name="terraphim-agent"
            ;;
    esac

    if [[ ! -f "$target_dir/$binary_name" ]]; then
        echo -e "${RED}❌ Binary not found: $target_dir/$binary_name${NC}"
        return 1
    fi

    # Create package name
    local features_suffix=""
    if [[ -n "$features" ]]; then
        features_suffix="-${features//,/}"
    fi

    local package_name="terraphim-${package}${features_suffix}-${VERSION}-${target}"
    local package_dir="$OUTPUT_DIR/$package_name"

    echo -e "${YELLOW}Creating package: $package_name${NC}"

    # Create package directory
    mkdir -p "$package_dir"

    # Copy binary
    cp "$target_dir/$binary_name" "$package_dir/"

    # Create metadata
    cat > "$package_dir/README.md" << EOF
# Terraphim $package - Release $VERSION

## Build Information
- Target: $target
- Features: ${features:-default}
- Build Profile: $BUILD_PROFILE
- Build Date: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
- Rust Version: $(rustc --version)

## Usage

### $binary_name
\`\`\`bash
# Make executable
chmod +x $binary_name

# Run with --help for usage information
./$binary_name --help
\`\`\`

## Verification

This release package includes:
- \`$binary_name\` - Main executable
- \`README.md\` - This file
- \`checksums.txt\` - SHA256 checksums

Verify integrity:
\`\`\`bash
sha256sum -c checksums.txt
\`\`\`

## Support

For issues and support, please visit the Terraphim project repository.
EOF

    # Create checksums
    cd "$package_dir"
    sha256sum "$binary_name" README.md > checksums.txt
    cd "$PROJECT_ROOT"

    # Create tar.gz archive
    tar -czf "$OUTPUT_DIR/${package_name}.tar.gz" -C "$OUTPUT_DIR" "$(basename "$package_dir")"

    # Create .zip archive for Windows compatibility
    if [[ "$target" == *"windows"* ]] || [[ "$target" == *"mingw"* ]]; then
        (cd "$OUTPUT_DIR" && zip -r "${package_name}.zip" "$(basename "$package_dir")")
    fi

    # Create .deb package for Linux targets
    if [[ "$CREATE_DEB" == "true" ]] && [[ "$target" == *"linux"* ]]; then
        create_deb_package "$target" "$features" "$package" "$package_name"
    fi

    echo -e "${GREEN}✅ Package created: ${package_name}.tar.gz${NC}"
}

# Function to create .deb package
create_deb_package() {
    local target="$1"
    local features="$2"
    local package="$3"
    local package_name="$4"

    local binary_name=""
    case "$package" in
        "terraphim_server") binary_name="terraphim_server" ;;
        "terraphim_mcp_server") binary_name="terraphim_mcp_server" ;;
        "terraphim_agent") binary_name="terraphim-agent" ;;
    esac

    local deb_dir="$OUTPUT_DIR/deb-build"
    local deb_package_name="terraphim-${package}"

    # Create DEBIAN directory structure
    mkdir -p "$deb_dir/DEBIAN"

    # Determine architecture
    local arch=""
    case "$target" in
        "x86_64-unknown-linux-gnu") arch="amd64" ;;
        "x86_64-unknown-linux-musl") arch="amd64" ;;
        "aarch64-unknown-linux-gnu") arch="arm64" ;;
        "armv7-unknown-linux-gnueabihf") arch="armhf" ;;
        *) arch="unknown" ;;
    esac

    # Create control file
    cat > "$deb_dir/DEBIAN/control" << EOF
Package: $deb_package_name
Version: $VERSION
Section: utils
Priority: optional
Architecture: $arch
Maintainer: Terraphim Project <noreply@terraphim.ai>
Description: Terraphim AI Assistant - $package component
 Terraphim is a privacy-first AI assistant that operates locally.
 This package contains the $package component.
Depends: libc6
EOF

    # Copy binary
    mkdir -p "$deb_dir/usr/bin"
    cp "$OUTPUT_DIR/$package_name/$binary_name" "$deb_dir/usr/bin/"

    # Create .deb package
    dpkg-deb --build "$deb_dir" "$OUTPUT_DIR/${deb_package_name}_${VERSION}_${arch}.deb"

    # Cleanup
    rm -rf "$deb_dir"

    echo -e "${GREEN}✅ DEB package created: ${deb_package_name}_${VERSION}_${arch}.deb${NC}"
}

# Function to create release summary
create_release_summary() {
    echo -e "${BLUE}📋 Creating release summary...${NC}"

    cat > "$OUTPUT_DIR/RELEASE-$VERSION.md" << EOF
# Terraphim Release $VERSION

Generated on: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

## Build Information
- Rust Version: $(rustc --version)
- Build Profile: $BUILD_PROFILE
- Build Targets: ${RELEASE_TARGETS[*]}

## Artifacts

### Binaries
EOF

    # List all artifacts
    for artifact in "$OUTPUT_DIR"/*.tar.gz; do
        if [[ -f "$artifact" ]]; then
            local checksum=$(sha256sum "$artifact" | cut -d' ' -f1)
            local size=$(stat -f%z "$artifact" 2>/dev/null || stat -c%s "$artifact" 2>/dev/null || echo "unknown")
            echo "- $(basename "$artifact") (${size} bytes)" >> "$OUTPUT_DIR/RELEASE-$VERSION.md"
            echo "  - SHA256: $checksum" >> "$OUTPUT_DIR/RELEASE-$VERSION.md"
        fi
    done

    cat >> "$OUTPUT_DIR/RELEASE-$VERSION.md" << EOF

### Verification

All artifacts include \`checksums.txt\` files for integrity verification.

To verify:
\`\`\`bash
sha256sum -c checksums.txt
\`\`\`

## Installation

### Linux (tar.gz)
\`\`\`bash
tar -xzf terraphim-*.tar.gz
cd terraphim-*
sudo cp terraphim-* /usr/local/bin/
\`\`\`

### Debian/Ubuntu (.deb)
\`\`\`bash
sudo dpkg -i terraphim-*.deb
\`\`\`

### TUI Installation
\`\`\`bash
# After extraction
chmod +x terraphim-agent
./terraphim-agent --help
\`\`\`

## Features

This release includes the following feature combinations:
EOF

    for features in "${RELEASE_FEATURES[@]}" "${TUI_FEATURES[@]}"; do
        echo "- ${features:-default}" >> "$OUTPUT_DIR/RELEASE-$VERSION.md"
    done

    cat >> "$OUTPUT_DIR/RELEASE-$VERSION.md" << EOF

## Support

For documentation and support, please refer to the Terraphim project repository.
EOF

    echo -e "${GREEN}✅ Release summary created: RELEASE-$VERSION.md${NC}"
}

# Main function
main() {
    local targets=("${RELEASE_TARGETS[@]}")
    local build_all_packages=true
    local build_tui_only=false
    local specific_features=""
    local specific_target=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help)
                show_usage
                exit 0
                ;;
            --version)
                VERSION="$2"
                shift 2
                ;;
            --profile)
                BUILD_PROFILE="$2"
                shift 2
                ;;
            --target)
                specific_target="$2"
                shift 2
                ;;
            --features)
                specific_features="$2"
                shift 2
                ;;
            --tui-only)
                build_tui_only=true
                build_all_packages=false
                shift
                ;;
            --no-deb)
                CREATE_DEB="false"
                shift
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
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

    # Override targets if specific target provided
    if [[ -n "$specific_target" ]]; then
        targets=("$specific_target")
    fi

    # Setup
    cd "$PROJECT_ROOT"
    check_dependencies
    setup_cross_compilation

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    echo -e "${BLUE}🏗️ Starting release build...${NC}"
    echo "This may take 10-30 minutes depending on targets and features"
    echo ""

    local total_builds=0
    local successful_builds=0

    # Build packages
    local packages=()
    if [[ "$build_tui_only" == "true" ]]; then
        packages=("terraphim_tui")
    elif [[ "$build_all_packages" == "true" ]]; then
        packages=("terraphim_server" "terraphim_mcp_server" "terraphim_tui")
    fi

    local features_to_test=("${RELEASE_FEATURES[@]}")
    if [[ "$build_tui_only" == "true" ]]; then
        features_to_test=("${TUI_FEATURES[@]}")
    fi

    if [[ -n "$specific_features" ]]; then
        features_to_test=("$specific_features")
    fi

    for target in "${targets[@]}"; do
        for package in "${packages[@]}"; do
            for features in "${features_to_test[@]}"; do
                ((total_builds++))

                echo -e "${BLUE}[$total_builds] Building $package for $target${NC}"

                if build_package "$target" "$features" "$package"; then
                    if create_package "$target" "$features" "$package"; then
                        ((successful_builds++))
                    else
                        echo -e "${RED}❌ Failed to create package${NC}"
                    fi
                else
                    echo -e "${RED}❌ Failed to build $package${NC}"
                fi
                echo ""
            done
        done
    done

    # Create release summary
    create_release_summary

    # Summary
    echo -e "${BLUE}=== Release Build Summary ===${NC}"
    echo "Total builds: $total_builds"
    echo -e "${GREEN}Successful: $successful_builds${NC}"
    if [[ $((total_builds - successful_builds)) -gt 0 ]]; then
        echo -e "${RED}Failed: $((total_builds - successful_builds))${NC}"
    fi
    echo ""
    echo "Output directory: $OUTPUT_DIR"
    echo ""

    # List created artifacts
    echo -e "${BLUE}📦 Created Artifacts:${NC}"
    ls -la "$OUTPUT_DIR"/*.tar.gz "$OUTPUT_DIR"/*.deb "$OUTPUT_DIR"/RELEASE-*.md 2>/dev/null || true

    if [[ $successful_builds -eq $total_builds ]]; then
        echo -e "${GREEN}🎉 All release builds completed successfully!${NC}"
        echo -e "${GREEN}📦 Ready for deployment!${NC}"
        exit 0
    else
        echo -e "${RED}❌ Some release builds failed!${NC}"
        echo -e "${YELLOW}Check the logs above for details${NC}"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
