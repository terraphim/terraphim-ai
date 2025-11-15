#!/bin/bash
# Terraphim AI v0.2.3 Installation Script
# This script downloads and installs Terraphim AI on Linux systems

set -e

VERSION="v0.2.3"
INSTALL_DIR="$HOME/.local/bin"
TEMP_DIR="/tmp/terraphim-install"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running on Linux
if [[ "$OSTYPE" != "linux-gnu"* && "$OSTYPE" != "linux-musl"* ]]; then
    print_error "This installer is designed for Linux systems. For other platforms, please see the manual installation guide."
    exit 1
fi

# Check dependencies
check_dependencies() {
    print_status "Checking dependencies..."

    local missing_deps=()

    if ! command -v curl &> /dev/null; then
        missing_deps+=("curl")
    fi

    if ! command -v git &> /dev/null; then
        missing_deps+=("git")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        print_status "Please install them using your package manager:"
        print_status "  Ubuntu/Debian: sudo apt-get install curl git"
        print_status "  CentOS/RHEL: sudo yum install curl git"
        print_status "  Arch Linux: sudo pacman -S curl git"
        exit 1
    fi
}

# Create installation directory
create_install_dir() {
    print_status "Creating installation directory..."
    mkdir -p "$INSTALL_DIR"

    # Add to PATH if not already there
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$HOME/.bashrc"
        print_status "Added $INSTALL_DIR to PATH. Please run: source ~/.bashrc"
    fi
}

# Download and build from source
build_from_source() {
    print_status "Building Terraphim from source..."

    # Clean up any existing temp directory
    rm -rf "$TEMP_DIR"
    mkdir -p "$TEMP_DIR"

    # Clone the repository
    print_status "Cloning Terraphim repository..."
    cd "$TEMP_DIR"
    git clone https://github.com/terraphim/terraphim-ai.git .
    git checkout "$VERSION"

    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_status "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    # Build the binaries
    print_status "Building Terraphim Server..."
    cargo build --release --package terraphim_server

    print_status "Building Terraphim TUI..."
    cargo build --release --package terraphim_agent --features repl-full

    # Copy binaries to installation directory
    print_status "Installing binaries..."
    cp target/release/terraphim_server "$INSTALL_DIR/"
    cp target/release/terraphim-agent "$INSTALL_DIR/"

    # Make them executable
    chmod +x "$INSTALL_DIR/terraphim_server"
    chmod +x "$INSTALL_DIR/terraphim-agent"

    # Clean up
    cd /
    rm -rf "$TEMP_DIR"
}

# Verify installation
verify_installation() {
    print_status "Verifying installation..."

    if command -v terraphim_server &> /dev/null; then
        print_status "✓ terraphim_server is installed"
        terraphim_server --version
    else
        print_error "terraphim_server installation failed"
        return 1
    fi

    if command -v terraphim-agent &> /dev/null; then
        print_status "✓ terraphim-agent is installed"
        terraphim-agent --version
    else
        print_error "terraphim-agent installation failed"
        return 1
    fi
}

# Create configuration directory
setup_config() {
    print_status "Setting up configuration..."
    mkdir -p "$HOME/.config/terraphim"

    # Copy default configuration
    if [ ! -f "$HOME/.config/terraphim/config.json" ]; then
        cat > "$HOME/.config/terraphim/config.json" << 'EOF'
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "$HOME/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    }
  ]
}
EOF
        print_status "Created default configuration at $HOME/.config/terraphim/config.json"
    fi
}

# Main installation process
main() {
    print_status "Installing Terraphim AI $VERSION"

    check_dependencies
    create_install_dir
    build_from_source
    verify_installation
    setup_config

    print_status ""
    print_status "🎉 Installation completed successfully!"
    print_status ""
    print_status "To get started:"
    print_status "  terraphim-agent --help                 # Show TUI help"
    print_status "  terraphim-agent search 'rust'          # Search with TUI"
    print_status "  terraphim_server --config ~/.config/terraphim/config.json  # Start server"
    print_status ""
    print_status "Documentation: https://github.com/terraphim/terraphim-ai/wiki"
    print_status "Support: https://github.com/terraphim/terraphim-ai/issues"
}

# Run main function
main "$@"
