#!/bin/bash
# packaging/scripts/build-deb.sh
# Enhanced Debian package build script

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building Debian packages for v$VERSION..."

# Ensure necessary tools
if ! command -v cargo-deb > /dev/null; then
    echo "❌ cargo-deb not found. Installing..."
    cargo install cargo-deb
fi

# Build backend DEB packages
echo "Building terraphim-server DEB..."
cd "$ROOT"
cargo deb -p terraphim_server --version "$VERSION"

echo "Building terraphim-agent DEB..." 
cargo deb -p terraphim_agent --version "$VERSION"

# Build desktop DEB
echo "Building terraphim-desktop DEB..."
cd "$ROOT/desktop"
yarn install --frozen-lockfile
yarn tauri build --bundles deb --target x86_64-unknown-linux-gnu

# Copy all DEB files to release artifacts
mkdir -p "$ROOT/release-artifacts"
cp "$ROOT/target/debian"/*.deb "$ROOT/release-artifacts/" 2>/dev/null || true
cp "$ROOT/desktop/target/x86_64-unknown-linux-gnu/release/bundle/deb"/*.deb "$ROOT/release-artifacts/" 2>/dev/null || true

echo "✅ Debian packages built successfully"
echo "📦 Available DEB files:"
ls -la "$ROOT/release-artifacts"/*.deb 2>/dev/null || echo "No DEB files found"