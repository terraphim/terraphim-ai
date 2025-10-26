#!/bin/bash
# Terraphim AI Linux Installation Script

set -e
ARCH=$(uname -m)
VERSION="0.2.5-cross-platform"

echo "Installing Terraphim AI $VERSION for Linux ($ARCH)..."

# Detect architecture and download appropriate binary
case $ARCH in
    x86_64)
        echo "Using generic binary installation"
        wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim_server-x86_64-unknown-linux-gnu
        chmod +x terraphim_server-x86_64-unknown-linux-gnu
        sudo mv terraphim_server-x86_64-unknown-linux-gnu /usr/local/bin/terraphim_server

        wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim-tui-x86_64-unknown-linux-gnu
        chmod +x terraphim-tui-x86_64-unknown-linux-gnu
        sudo mv terraphim-tui-x86_64-unknown-linux-gnu /usr/local/bin/terraphim-tui
        ;;
    aarch64)
        echo "ARM64 detected - binaries not yet available for this architecture"
        echo "Please build from source or check for a future release"
        exit 1
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "✅ Installation completed!"
echo "Run 'terraphim_server --help' to get started"
echo "Run 'terraphim-tui --help' for the terminal interface"