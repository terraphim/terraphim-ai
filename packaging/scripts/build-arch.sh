#!/bin/bash
# packaging/scripts/build-arch.sh
# Build Arch Linux packages using makepkg
# Usage: ./build-arch.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$ROOT/target/arch"
BUILD_DIR="$OUTPUT_DIR/build"

echo "Building Arch Linux packages..."

# Check for makepkg
if ! command -v makepkg &> /dev/null; then
    echo "Warning: makepkg not found. This script requires Arch Linux or makepkg."
    echo "On non-Arch systems, consider using docker with an Arch image."
    exit 1
fi

# Create directories
mkdir -p "$BUILD_DIR"

# Get version from Cargo.toml
VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Create PKGBUILD
cat > "$BUILD_DIR/PKGBUILD" << 'PKGBUILD_EOF'
# Maintainer: Terraphim Contributors <team@terraphim.ai>
pkgname=terraphim-agent
pkgver=VERSION_PLACEHOLDER
pkgrel=1
pkgdesc="Terraphim AI Agent CLI - Command-line interface with interactive REPL"
arch=('x86_64' 'aarch64')
url="https://terraphim.ai"
license=('Apache-2.0')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=()
sha256sums=()

build() {
    cd "$srcdir/../../../.."
    cargo build --release -p terraphim_agent
}

package() {
    cd "$srcdir/../../../.."
    install -Dm755 "target/release/terraphim-agent" "$pkgdir/usr/bin/terraphim-agent"
    install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
}
PKGBUILD_EOF

# Replace version placeholder
sed -i "s/VERSION_PLACEHOLDER/$VERSION/" "$BUILD_DIR/PKGBUILD"

# Build the package
echo "Running makepkg..."
(cd "$BUILD_DIR" && makepkg -sf --noconfirm) || {
    echo "makepkg failed. Check the PKGBUILD."
    exit 1
}

# Copy package to output directory
find "$BUILD_DIR" -name "*.pkg.tar*" -exec cp {} "$OUTPUT_DIR/" \;

echo ""
echo "Generated Arch packages:"
find "$OUTPUT_DIR" -maxdepth 1 -name "*.pkg.tar*" -type f 2>/dev/null | while read -r pkg; do
    echo "  $(basename "$pkg")"
done

echo ""
echo "Arch Linux packages built successfully!"
