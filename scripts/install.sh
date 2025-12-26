#!/bin/bash
# Terraphim AI Universal Installer v1.0.0
# Installs terraphim-agent and optionally terraphim-cli
# Supports: Linux, macOS, Windows (WSL)
# Installation: curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash

set -euo pipefail

# Configuration
INSTALLER_VERSION="1.0.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GITHUB_API_BASE="https://api.github.com/repos/terraphim/terraphim-ai"
GITHUB_RELEASES="https://github.com/terraphim/terraphim-ai/releases/download"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
DEFAULT_TOOLS=("terraphim-agent")
VERSION="${VERSION:-latest}"
SKIP_VERIFY="${SKIP_VERIFY:-false}"
VERBOSE="${VERBOSE:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    if [[ "$VERBOSE" == "true" ]]; then
        echo -e "${BLUE}ℹ${NC} $*"
    fi
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

log_progress() {
    echo -e "${BLUE}➤${NC} $*"
}

# Display banner
show_banner() {
    cat << 'EOF'
╭─────────────────────────────────────────────────────────╮
│           Terraphim AI Installer v1.0.0                │
│    Privacy-first AI assistant with semantic search     │
│                                                      │
│    Installing: terraphim-agent                        │
│    Optional:    terraphim-cli                         │
╰─────────────────────────────────────────────────────────╯
EOF
}

# Parse command line arguments
parse_args() {
    INSTALL_DIR="$DEFAULT_INSTALL_DIR"
    TOOLS_TO_INSTALL=("${DEFAULT_TOOLS[@]}")

    while [[ $# -gt 0 ]]; do
        case $1 in
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --with-cli)
                TOOLS_TO_INSTALL=("terraphim-agent" "terraphim-cli")
                shift
                ;;
            --cli-only)
                TOOLS_TO_INSTALL=("terraphim-cli")
                shift
                ;;
            --version)
                VERSION="$2"
                shift 2
                ;;
            --skip-verify)
                SKIP_VERIFY="true"
                shift
                ;;
            --verbose)
                VERBOSE="true"
                shift
                ;;
            --help|-h)
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

    export INSTALL_DIR TOOLS_TO_INSTALL VERSION SKIP_VERIFY VERBOSE
}

# Show help
show_help() {
    cat << EOF
Terraphim AI Installer

USAGE:
    curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash [OPTIONS]

OPTIONS:
    --install-dir DIR       Installation directory (default: $DEFAULT_INSTALL_DIR)
    --with-cli              Also install terraphim-cli
    --cli-only              Install only terraphim-cli
    --version VERSION       Install specific version (default: latest)
    --skip-verify           Skip checksum verification (not recommended)
    --verbose               Enable verbose logging
    --help, -h              Show this help message

EXAMPLES:
    # Install terraphim-agent (default)
    curl -fsSL ... | bash

    # Install both agent and cli
    curl -fsSL ... | bash --with-cli

    # Install to custom directory
    curl -fsSL ... | bash --install-dir /usr/local/bin

    # Install specific version
    curl -fsSL ... | bash --version v1.2.3
EOF
}

# Load utility functions
load_utils() {
    local utils_dir="$(dirname "${BASH_SOURCE[0]}")"

    # Source utility scripts if they exist
    for util in "platform-detection.sh" "binary-resolution.sh" "security-verification.sh"; do
        if [[ -f "$utils_dir/$util" ]]; then
            log_info "Loading utility: $util"
            source "$utils_dir/$util"
        else
            log_warn "Utility script not found: $util"
        fi
    done
}

# Main installation function
main() {
    # Parse command line arguments
    parse_args "$@"

    # Show banner
    show_banner

    # Load utility functions
    load_utils

    # Detect platform
    log_progress "Detecting platform..."
    detect_platform
    log_success "Platform detected: $OS-$ARCH"

    # Check dependencies
    log_progress "Checking dependencies..."
    check_dependencies

    # Create installation directory
    create_install_directory

    # Install tools
    for tool in "${TOOLS_TO_INSTALL[@]}"; do
        echo
        log_progress "Installing $tool..."

        local asset_url=$(resolve_binary_url "$tool" "$VERSION")
        log_info "Resolved asset URL: $asset_url"

        if [[ "$asset_url" == "source" ]]; then
            install_from_source "$tool" "$VERSION"
        else
            install_binary "$tool" "$asset_url"
        fi

        verify_installation "$tool"
        log_success "$tool installed successfully"
    done

    # Setup configuration and PATH
    setup_configuration
    setup_path "$INSTALL_DIR"

    # Show completion message
    show_completion_message
}

# Platform detection (fallback if not in separate script)
detect_platform() {
    if command -v detect_os_arch >/dev/null 2>&1; then
        detect_os_arch
        return
    fi

    # Fallback implementation
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case $os in
        linux*) OS="linux" ;;
        darwin*) OS="macos" ;;
        cygwin*|mingw*|msys*) OS="windows" ;;
        *)
            log_error "Unsupported OS: $os"
            exit 1
            ;;
    esac

    case $arch in
        x86_64|amd64) ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        armv7*|armv6*) ARCH="armv7" ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    export OS ARCH
}

# Check basic dependencies
check_dependencies() {
    local missing_deps=()

    # Check for curl
    if ! command -v curl >/dev/null 2>&1; then
        missing_deps+=("curl")
    fi

    # Check for sha256sum or shasum
    if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
        missing_deps+=("sha256sum or shasum")
    fi

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_error "Please install the missing dependencies and try again."
        exit 1
    fi

    log_success "All dependencies found"
}

# Create installation directory
create_install_directory() {
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_progress "Creating installation directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    # Check if directory is writable
    if [[ ! -w "$INSTALL_DIR" ]]; then
        log_error "Installation directory is not writable: $INSTALL_DIR"
        log_error "Try running with sudo or specify a different directory with --install-dir"
        exit 1
    fi

    log_success "Installation directory ready: $INSTALL_DIR"
}

# Binary resolution (fallback if not in separate script)
resolve_binary_url() {
    local tool=$1
    local version=${2:-"latest"}

    # Use external script if available
    if command -v resolve_best_asset >/dev/null 2>&1; then
        # Temporarily disable verbose output for clean resolution
        local old_verbose="$VERBOSE"
        VERBOSE=false

        local resolution_output
        resolution_output=$(resolve_best_asset "$tool" "$version" 2>/dev/null)

        # Restore verbose setting
        VERBOSE="$old_verbose"

        # Extract the ASSET_URL from the output
        local asset_url
        asset_url=$(echo "$resolution_output" | grep "^ASSET_URL=" | cut -d'=' -f2-)

        echo "$asset_url"
        return
    fi

    # Fallback implementation
    if [[ "$version" == "latest" ]]; then
        # Get latest release tag
        version=$(curl -s "${GITHUB_API_BASE}/releases/latest" | grep -o '"tag_name": "[^"]*' | sed 's/"tag_name": "//' | sed 's/"//')
        if [[ -z "$version" ]]; then
            log_error "Failed to get latest version from GitHub API"
            exit 1
        fi
    fi

    # Remove 'v' prefix if present
    version=${version#v}

    # Determine asset name
    local asset_name
    if [[ "$OS" == "macos" ]]; then
        asset_name="${tool}-universal-apple-darwin"
    elif [[ "$OS" == "windows" ]]; then
        asset_name="${tool}-windows-x86_64.exe"
    else
        asset_name="${tool}-${OS}-${ARCH}"
    fi

    local asset_url="${GITHUB_RELEASES}/v${version}/${asset_name}"

    # Check if asset exists
    if curl --silent --fail --head "$asset_url" >/dev/null; then
        echo "$asset_url"
    else
        log_warn "Pre-built binary not found: $asset_name"
        echo "source"
    fi
}

# Install binary from URL
install_binary() {
    local tool=$1
    local url=$2
    local filename=$(basename "$url")
    local install_path="$INSTALL_DIR/$filename"

    log_progress "Downloading $tool..."

    # Download with progress
    curl --progress-bar \
         --location \
         --retry 3 \
         --retry-delay 1 \
         --output "$install_path" \
         "$url"

    # Make executable (except for Windows .exe files)
    if [[ ! "$filename" =~ \.exe$ ]]; then
        chmod +x "$install_path"
    fi

    log_success "Downloaded $tool to $install_path"
}

# Install from source (placeholder)
install_from_source() {
    local tool=$1
    local version=${2:-"latest"}

    log_warn "Source compilation not yet implemented for $tool"
    log_warn "Please install Rust toolchain and run: cargo install $tool"
    log_info "For installation instructions, visit: https://docs.terraphim.ai/installation"

    # For now, we'll skip source installation
    log_warn "Skipping $tool installation"
}

# Verify installation
verify_installation() {
    local tool=$1

    # Try to find the binary
    local binary_path=""
    for ext in "" ".exe"; do
        if [[ -f "$INSTALL_DIR/$tool$ext" ]]; then
            binary_path="$INSTALL_DIR/$tool$ext"
            break
        fi
    done

    if [[ -z "$binary_path" ]]; then
        log_error "$tool binary not found in $INSTALL_DIR"
        return 1
    fi

    # Test if binary runs
    if "$binary_path" --version >/dev/null 2>&1; then
        local installed_version=$("$binary_path" --version 2>/dev/null || echo "unknown")
        log_success "$tool is working (version: $installed_version)"
    else
        log_warn "$tool binary installed but failed version check"
    fi
}

# Setup basic configuration
setup_configuration() {
    local config_dir="$HOME/.config/terraphim"

    if [[ ! -d "$config_dir" ]]; then
        log_progress "Creating configuration directory..."
        mkdir -p "$config_dir"
    fi

    # Create default config if it doesn't exist
    local config_file="$config_dir/config.json"
    if [[ ! -f "$config_file" ]]; then
        log_progress "Creating default configuration..."
        cat > "$config_file" << 'EOF'
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "~/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst,*.rs,*.js,*.ts"
      }
    }
  ],
  "update_channel": "stable",
  "auto_update": true
}
EOF
        log_success "Default configuration created: $config_file"
    fi
}

# Setup PATH in shell configs
setup_path() {
    local install_dir=$1

    # Skip if directory is already in PATH
    if echo "$PATH" | grep -q "$install_dir"; then
        log_info "Installation directory already in PATH"
        return
    fi

    log_progress "Adding $install_dir to PATH..."

    # Detect current shell and update config
    local current_shell=$(basename "$SHELL")
    local config_file=""

    case $current_shell in
        bash)
            config_file="$HOME/.bashrc"
            if [[ -f "$HOME/.bash_profile" ]]; then
                config_file="$HOME/.bash_profile"
            fi
            ;;
        zsh)
            config_file="$HOME/.zshrc"
            ;;
        fish)
            config_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            log_warn "Unsupported shell: $current_shell"
            log_warn "Please add $install_dir to your PATH manually"
            return
            ;;
    esac

    # Add to config if not already present
    if [[ -f "$config_file" ]] && ! grep -q "$install_dir" "$config_file"; then
        echo "" >> "$config_file"
        echo "# Terraphim AI" >> "$config_file"
        if [[ "$current_shell" == "fish" ]]; then
            echo "set -gx PATH \$PATH $install_dir" >> "$config_file"
        else
            echo "export PATH=\"\$PATH:$install_dir\"" >> "$config_file"
        fi
        log_success "Added to $config_file"
    fi

    # Update current session
    export PATH="$PATH:$install_dir"
}

# Show completion message
show_completion_message() {
    echo
    log_success "Installation completed successfully!"
    echo
    echo "Installed tools:"
    for tool in "${TOOLS_TO_INSTALL[@]}"; do
        echo "  - $tool"
    done
    echo
    echo "Installation directory: $INSTALL_DIR"
    echo "Configuration directory: $HOME/.config/terraphim"
    echo
    echo "To get started:"
    if [[ " ${TOOLS_TO_INSTALL[@]} " =~ " terraphim-agent " ]]; then
        echo "  terraphim-agent --help"
    fi
    if [[ " ${TOOLS_TO_INSTALL[@]} " =~ " terraphim-cli " ]]; then
        echo "  terraphim-cli --help"
    fi
    echo
    echo "Note: You may need to restart your terminal or run:"
    echo "  source ~/.bashrc  # or ~/.zshrc, depending on your shell"
    echo
    echo "For more information, visit: https://docs.terraphim.ai"
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
