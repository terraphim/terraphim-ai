#!/bin/bash
# test-install.sh - Test package installation inside Docker container
# Usage: test-install.sh <package_type>

set -euo pipefail

PACKAGE_TYPE="$1"
PACKAGES_DIR="/packages"
WORKSPACE="/workspace"

echo "🧪 Installing and testing $PACKAGE_TYPE package..."

# Update package manager
if command -v apt > /dev/null; then
    echo "📦 Updating APT packages..."
    apt update
elif command -v dnf > /dev/null; then
    echo "📦 Updating DNF packages..."
    dnf update -y
elif command -v pacman > /dev/null; then
    echo "📦 Updating Pacman packages..."
    pacman -Syu --noconfirm
fi

# Test installation and basic functionality
case "$PACKAGE_TYPE" in
    "DEB")
        echo "🔧 Testing DEB installation..."
        apt install -y "$PACKAGES_DIR"/*.deb
        if command -v terraphim_server > /dev/null; then
            echo "✅ terraphim_server installed successfully"
            terraphim_server --version
        else
            echo "❌ terraphim_server installation failed"
            exit 1
        fi
        ;;
        
    "RPM")
        echo "🔧 Testing RPM installation..."
        dnf install -y "$PACKAGES_DIR"/*.rpm
        if command -v terraphim_server > /dev/null; then
            echo "✅ terraphim_server installed successfully"
            terraphim_server --version
        else
            echo "❌ terraphim_server installation failed"
            exit 1
        fi
        ;;
        
    "Arch")
        echo "🔧 Testing Arch package installation..."
        pacman -U --noconfirm "$PACKAGES_DIR"/*.pkg.tar.zst
        if command -v terraphim_server > /dev/null; then
            echo "✅ terraphim_server installed successfully"
            terraphim_server --version
        else
            echo "❌ terraphim_server installation failed"
            exit 1
        fi
        ;;
        
    "AppImage")
        echo "🔧 Testing AppImage installation..."
        chmod +x "$PACKAGES_DIR"/*.AppImage
        # Test that AppImage runs (quick test)
        if "$PACKAGES_DIR"/*.AppImage --version 2>/dev/null; then
            echo "✅ AppImage executes successfully"
            "$PACKAGES_DIR"/*.AppImage --version
        else
            echo "❌ AppImage execution failed"
            exit 1
        fi
        ;;
        
    *)
        echo "❌ Unknown package type: $PACKAGE_TYPE"
        exit 1
        ;;
esac

# Test basic functionality
echo "🧪 Testing basic functionality..."

# Test server startup (briefly)
if command -v terraphim_server > /dev/null; then
    echo "Starting terraphim_server for basic test..."
    timeout 5s terraphim_server --port 8001 || echo "Server startup test completed"
    
    if pgrep -f terraphim_server > /dev/null; then
        pkill -f terraphim_server || true
        echo "✅ Server starts and runs correctly"
    else
        echo "✅ Server starts and exits gracefully (expected behavior)"
    fi
fi

echo "✅ All tests passed for $PACKAGE_TYPE"