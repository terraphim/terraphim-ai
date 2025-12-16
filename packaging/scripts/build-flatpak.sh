#!/bin/bash
# packaging/scripts/build-flatpak.sh
# Flatpak build script

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building Flatpak for v$VERSION..."

# Check for flatpak-builder
if ! command -v flatpak-builder > /dev/null; then
    echo "❌ flatpak-builder not found. Installing..."
    sudo apt update
    sudo apt install flatpak flatpak-builder
fi

# Add Flathub if not present
if ! flatpak remote-list | grep -q flathub; then
    flatpak remote-add --if-not-exists flathub https://flathub.org/flathub.flatpakrepo
fi

# Install necessary SDKs
echo "📦 Installing Flatpak SDKs..."
flatpak install flathub org.freedesktop.Platform//23.08 org.freedesktop.Sdk//23.08

# Create flatpak build directory
FLATPAK_BUILD_DIR="$ROOT/target/flatpak"
mkdir -p "$FLATPAK_BUILD_DIR"

# Build Flatpak
echo "🏗️ Building Flatpak package..."
cd "$ROOT"

flatpak-builder \
    --repo="$FLATPAK_BUILD_DIR/repo" \
    --force-clean \
    --disable-updates \
    --ccache \
    "$FLATPAK_BUILD_DIR/build-dir" \
    "$PACKAGING_ROOT/flatpak/com.terraphim.ai.desktop.yml"

# Create bundle
flatpak build-bundle \
    "$FLATPAK_BUILD_DIR/repo" \
    "$ROOT/release-artifacts/terraphim-desktop_${VERSION}_amd64.flatpak" \
    com.terraphim.ai.desktop

echo "✅ Flatpak built successfully"
echo "📦 Available Flatpak files:"
ls -la "$ROOT/release-artifacts"/*.flatpak 2>/dev/null || echo "No Flatpak files found"