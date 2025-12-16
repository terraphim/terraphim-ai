#!/bin/bash
# packaging/scripts/build-arch.sh
# Arch Linux package build script

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building Arch Linux packages for v$VERSION..."

# Check for required tools
if ! command -v makepkg > /dev/null; then
    echo "❌ makepkg not found. Please install base-devel package."
    exit 1
fi

# Create Arch build directory
ARCH_BUILD_DIR="$ROOT/target/arch"
mkdir -p "$ARCH_BUILD_DIR"

# Function to build PKGBUILD
build_pkgbuild() {
    local pkgname="$1"
    local pkgbuild_file="$PACKAGING_ROOT/arch/PKGBUILD-$pkgname"
    
    echo "Building $pkgname AUR package..."
    
    # Create source tarball
    cd "$ROOT"
    git archive --format=tar.gz --prefix="$pkgname-$VERSION/" "v$VERSION" > "$ARCH_BUILD_DIR/$pkgname-$VERSION.tar.gz"
    
    # Copy PKGBUILD to build directory
    cp "$pkgbuild_file" "$ARCH_BUILD_DIR/PKGBUILD"
    
    # Build package
    cd "$ARCH_BUILD_DIR"
    makepkg -f --skipinteg
    
    # Copy resulting package to release artifacts
    cp "$pkgname"*.pkg.tar.zst "$ROOT/release-artifacts/" 2>/dev/null || true
}

# Build server package
build_pkgbuild "server"

# Build agent package  
build_pkgbuild "agent"

# For desktop, we'll use a different approach since it's Rust-based
echo "Building terraphim-desktop AUR package..."
cd "$ROOT"
git archive --format=tar.gz --prefix="terraphim-desktop-$VERSION/" "v$VERSION" > "$ARCH_BUILD_DIR/terraphim-desktop-$VERSION.tar.gz"

# Copy desktop PKGBUILD
cp "$PACKAGING_ROOT/arch/PKGBUILD-desktop" "$ARCH_BUILD_DIR/PKGBUILD"

# Build desktop package
cd "$ARCH_BUILD_DIR"
makepkg -f --skipinteg

# Copy desktop package to release artifacts
cp "terraphim-desktop"*.pkg.tar.zst "$ROOT/release-artifacts/" 2>/dev/null || true

echo "✅ Arch Linux packages built successfully"
echo "📦 Available Arch packages:"
ls -la "$ROOT/release-artifacts"/*.pkg.tar.zst 2>/dev/null || echo "No Arch packages found"