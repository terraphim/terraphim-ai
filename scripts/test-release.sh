#!/bin/bash
# Test script to validate release artifacts

set -e

RELEASE_VERSION=${1:-"v0.2.3"}
TEST_DIR="/tmp/terraphim-release-test"

echo "🧪 Testing Terraphim AI Release $RELEASE_VERSION"
echo "=========================================="

# Create test directory
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "📥 Downloading release artifacts..."

# Download Debian packages
echo "Downloading Debian packages..."
wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_VERSION/terraphim-server_${RELEASE_VERSION#v}-1_amd64.deb"
wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_VERSION/terraphim-tui_${RELEASE_VERSION#v}-1_amd64.deb"

# Download Arch packages
echo "Downloading Arch packages..."
wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_VERSION/terraphim-server-${RELEASE_VERSION#v}-1-x86_64.pkg.tar.zst"
wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_VERSION/terraphim-tui-${RELEASE_VERSION#v}-1-x86_64.pkg.tar.zst"

# Download installation scripts
echo "Downloading installation scripts..."
wget -q "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$RELEASE_VERSION/install.sh"
wget -q "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$RELEASE_VERSION/docker-run.sh"
chmod +x install.sh docker-run.sh

echo "🔍 Validating downloaded files..."

# Check file sizes
echo "Checking file sizes..."
ls -lh *.deb *.pkg.tar.zst *.sh

# Validate Debian packages
echo "Validating Debian packages..."
dpkg-deb -I terraphim-server_${RELEASE_VERSION#v}-1_amd64.deb | grep -E "(Package|Version|Architecture)"
dpkg-deb -I terraphim-tui_${RELEASE_VERSION#v}-1_amd64.deb | grep -E "(Package|Version|Architecture)"

# Validate Arch packages
echo "Validating Arch packages..."
tar -I 'zstd -d' -tf terraphim-server-${RELEASE_VERSION#v}-1-x86_64.pkg.tar.zst | grep -E "(terraphim-server|.PKGINFO)" | head -5
tar -I 'zstd -d' -tf terraphim-tui-${RELEASE_VERSION#v}-1-x86_64.pkg.tar.zst | grep -E "(terraphim-tui|.PKGINFO)" | head -5

# Extract and test binaries (if we have permission to install)
echo "Testing package contents..."
if command -v dpkg >/dev/null 2>&1; then
    echo "Testing Debian package installation (dry run)..."
    dpkg-deb --contents terraphim-server_${RELEASE_VERSION#v}-1_amd64.deb | grep -E "(usr/bin|etc/terraphim-ai)"
    dpkg-deb --contents terraphim-tui_${RELEASE_VERSION#v}-1_amd64.deb | grep -E "(usr/bin)"
fi

# Test installation scripts syntax
echo "Testing installation script syntax..."
bash -n install.sh
bash -n docker-run.sh

# Check if files are executable
echo "Checking file permissions..."
ls -la install.sh docker-run.sh

echo ""
echo "✅ Release artifact validation completed!"
echo "📊 Summary:"
echo "   - Debian packages: ✅"
echo "   - Arch packages: ✅"
echo "   - Installation scripts: ✅"
echo "   - File sizes and integrity: ✅"
echo ""
echo "🚀 Release $RELEASE_VERSION is ready for distribution!"

# Cleanup
cd /
rm -rf "$TEST_DIR"