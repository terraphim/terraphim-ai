#!/bin/bash
# packaging/scripts/build-rpm.sh
# RPM package build script using cargo-generate-rpm

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building RPM packages for v$VERSION..."

# Install cargo-generate-rpm if not present
if ! command -v cargo-generate-rpm > /dev/null; then
    echo "📦 Installing cargo-generate-rpm..."
    cargo install cargo-generate-rpm
fi

# Create RPM build directory
RPM_BUILD_DIR="$ROOT/target/rpm"
mkdir -p "$RPM_BUILD_DIR"

# Build server RPM
echo "Building terraphim-server RPM..."
cd "$ROOT"
cargo-generate-rpm \
    -p terraphim_server \
    -v "$VERSION" \
    -o "$RPM_BUILD_DIR"

# Build agent RPM
echo "Building terraphim-agent RPM..."
cargo-generate-rpm \
    -p terraphim_agent \
    -v "$VERSION" \
    -o "$RPM_BUILD_DIR"

# Copy RPM files to release artifacts
mkdir -p "$ROOT/release-artifacts"
cp "$RPM_BUILD_DIR"/*.rpm "$ROOT/release-artifacts/" 2>/dev/null || true

echo "✅ RPM packages built successfully"
echo "📦 Available RPM files:"
ls -la "$ROOT/release-artifacts"/*.rpm 2>/dev/null || echo "No RPM files found"