#!/bin/bash
# packaging/scripts/build-snap.sh
# Snap package build script

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building Snap for v$VERSION..."

# Check for snapcraft
if ! command -v snapcraft > /dev/null; then
    echo "❌ snapcraft not found. Installing..."
    sudo snap install snapcraft --classic
fi

# Create snap build directory
SNAP_BUILD_DIR="$ROOT/target/snap"
mkdir -p "$SNAP_BUILD_DIR"

# Build snap
echo "🏗️ Building Snap package..."
cd "$ROOT"

snapcraft \
    --target-arch=amd64 \
    --output-dir="$SNAP_BUILD_DIR"

# Copy snap to release artifacts
mkdir -p "$ROOT/release-artifacts"
cp "$SNAP_BUILD_DIR"/*.snap "$ROOT/release-artifacts/" 2>/dev/null || true

echo "✅ Snap built successfully"
echo "📦 Available Snap files:"
ls -la "$ROOT/release-artifacts"/*.snap 2>/dev/null || echo "No Snap files found"