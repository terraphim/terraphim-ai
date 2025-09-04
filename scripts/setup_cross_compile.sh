#!/bin/bash
# Cross-compilation setup script for Terraphim
# Installs necessary cross-compilation toolchains

set -e

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

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command -v apt-get &> /dev/null; then
            OS="ubuntu"
        elif command -v yum &> /dev/null; then
            OS="rhel"
        elif command -v pacman &> /dev/null; then
            OS="arch"
        else
            OS="linux"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        OS="unknown"
    fi

    log_info "Detected OS: $OS"
}

# Install system dependencies for Ubuntu/Debian
install_ubuntu_deps() {
    log_info "Installing cross-compilation toolchains for Ubuntu/Debian..."

    sudo apt-get update -qq

    # Basic build tools
    sudo apt-get install -y \
        build-essential \
        pkg-config \
        libssl-dev \
        ca-certificates \
        curl \
        git

    # Cross-compilation toolchains
    sudo apt-get install -y \
        gcc-aarch64-linux-gnu \
        libc6-dev-arm64-cross \
        gcc-arm-linux-gnueabihf \
        libc6-dev-armhf-cross \
        musl-tools \
        musl-dev

    log_success "Ubuntu/Debian dependencies installed"
}

# Install system dependencies for macOS
install_macos_deps() {
    log_info "Installing cross-compilation toolchains for macOS..."

    if ! command -v brew &> /dev/null; then
        log_error "Homebrew not found. Please install Homebrew first."
        exit 1
    fi

    # Install basic dependencies
    brew install pkg-config openssl

    # For cross-compilation on macOS, we mainly rely on Docker or GitHub Actions
    log_warning "Cross-compilation on macOS is limited. Consider using Docker or GitHub Actions for multi-arch builds."

    log_success "macOS dependencies installed"
}

# Install system dependencies for RHEL/CentOS/Fedora
install_rhel_deps() {
    log_info "Installing cross-compilation toolchains for RHEL/CentOS/Fedora..."

    # Use dnf for Fedora, yum for CentOS/RHEL
    local pkg_manager="yum"
    if command -v dnf &> /dev/null; then
        pkg_manager="dnf"
    fi

    sudo $pkg_manager update -y
    sudo $pkg_manager groupinstall -y "Development Tools"
    sudo $pkg_manager install -y \
        pkg-config \
        openssl-devel \
        ca-certificates \
        curl \
        git

    # Cross-compilation support is more limited on RHEL-based systems
    log_warning "Cross-compilation support on RHEL-based systems may be limited."
    log_warning "Consider using Docker for multi-arch builds."

    log_success "RHEL/CentOS/Fedora dependencies installed"
}

# Install system dependencies for Arch Linux
install_arch_deps() {
    log_info "Installing cross-compilation toolchains for Arch Linux..."

    sudo pacman -Syu --noconfirm
    sudo pacman -S --noconfirm \
        base-devel \
        pkg-config \
        openssl \
        ca-certificates \
        curl \
        git

    # Install AUR helper if not present (optional)
    if ! command -v yay &> /dev/null && ! command -v paru &> /dev/null; then
        log_warning "No AUR helper found. Cross-compilation packages may need manual installation."
    fi

    log_success "Arch Linux dependencies installed"
}

# Install Rust toolchain and targets
setup_rust() {
    log_info "Setting up Rust toolchain and cross-compilation targets..."

    # Install Rust if not present
    if ! command -v rustc &> /dev/null; then
        log_info "Installing Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
        source "$HOME/.cargo/env"
    fi

    # Add cross-compilation targets
    local targets=(
        "x86_64-unknown-linux-gnu"
        "aarch64-unknown-linux-gnu"
        "armv7-unknown-linux-gnueabihf"
        "x86_64-unknown-linux-musl"
        "aarch64-unknown-linux-musl"
        "armv7-unknown-linux-musleabihf"
    )

    for target in "${targets[@]}"; do
        log_info "Adding Rust target: $target"
        rustup target add "$target" || log_warning "Failed to add target: $target"
    done

    # Install useful Rust tools
    log_info "Installing cargo-deb for Debian package creation..."
    cargo install cargo-deb || log_warning "Failed to install cargo-deb"

    log_info "Installing cross for advanced cross-compilation..."
    cargo install cross --locked || log_warning "Failed to install cross"

    log_success "Rust cross-compilation setup complete"
}

# Set up Docker for multi-arch builds
setup_docker() {
    log_info "Setting up Docker for multi-architecture builds..."

    if ! command -v docker &> /dev/null; then
        case "$OS" in
            "ubuntu")
                log_info "Installing Docker on Ubuntu..."
                curl -fsSL https://get.docker.com -o get-docker.sh
                sudo sh get-docker.sh
                sudo usermod -aG docker "$USER"
                rm get-docker.sh
                ;;
            "macos")
                log_warning "Please install Docker Desktop from https://docker.com/products/docker-desktop"
                return 0
                ;;
            *)
                log_warning "Please install Docker manually for your system"
                return 0
                ;;
        esac
    fi

    # Set up Docker Buildx
    if docker buildx version &> /dev/null; then
        log_info "Setting up Docker Buildx multi-arch builder..."

        # Create a new builder instance
        docker buildx create --name terraphim-builder --driver docker-container --bootstrap || true
        docker buildx use terraphim-builder

        # Enable experimental features
        export DOCKER_CLI_EXPERIMENTAL=enabled

        log_success "Docker Buildx setup complete"
    else
        log_warning "Docker Buildx not available. Multi-arch builds may be limited."
    fi
}

# Create configuration files
create_configs() {
    log_info "Creating cross-compilation configuration files..."

    # Create .cargo/config.toml for cross-compilation settings
    mkdir -p .cargo

    cat > .cargo/config.toml << 'EOF'
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.x86_64-unknown-linux-musl]
linker = "musl-gcc"

[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"

[target.armv7-unknown-linux-musleabihf]
linker = "arm-linux-musleabihf-gcc"

# Enable faster builds
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
jobs = 4

# Enable incremental compilation
[profile.dev]
incremental = true

[profile.release]
# Optimize for size and performance
lto = "thin"
codegen-units = 1
panic = "abort"
EOF

    log_success "Cargo configuration created"

    # Create a build environment script
    cat > build-env.sh << 'EOF'
#!/bin/bash
# Build environment setup script
# Source this file to set up cross-compilation environment variables

# Cross-compilation environment variables
export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
export CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

export CC_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-gcc"
export CXX_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-g++"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="arm-linux-gnueabihf-gcc"

export CC_x86_64_unknown_linux_musl="musl-gcc"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="musl-gcc"

# OpenSSL configuration for cross-compilation
export PKG_CONFIG_ALLOW_CROSS=1

echo "Cross-compilation environment configured"
echo "Available targets:"
echo "  - x86_64-unknown-linux-gnu (native)"
echo "  - aarch64-unknown-linux-gnu (ARM64)"
echo "  - armv7-unknown-linux-gnueabihf (ARMv7)"
echo "  - x86_64-unknown-linux-musl (musl)"

echo ""
echo "Usage:"
echo "  source build-env.sh"
echo "  cargo build --target <target> --release"
EOF

    chmod +x build-env.sh
    log_success "Build environment script created: build-env.sh"
}

# Main setup function
main() {
    log_info "Setting up cross-compilation environment for Terraphim..."

    detect_os

    case "$OS" in
        "ubuntu")
            install_ubuntu_deps
            ;;
        "macos")
            install_macos_deps
            ;;
        "rhel")
            install_rhel_deps
            ;;
        "arch")
            install_arch_deps
            ;;
        *)
            log_warning "Unsupported OS: $OS"
            log_warning "Manual setup may be required"
            ;;
    esac

    setup_rust
    setup_docker
    create_configs

    log_success "Cross-compilation setup complete!"

    echo ""
    log_info "Next steps:"
    echo "1. Source the build environment: source build-env.sh"
    echo "2. Build for specific target: cargo build --target aarch64-unknown-linux-gnu --release"
    echo "3. Use the CI build script: ./scripts/ci_build.sh"
    echo "4. For Docker multi-arch builds: docker buildx build --platform linux/amd64,linux/arm64 ."
    echo ""
    log_info "For GitHub Actions CI/CD, see .github/workflows/ci-native.yml"
}

# Check if running as script (not sourced)
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
