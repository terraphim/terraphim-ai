#!/bin/bash
# packaging/scripts/build-all-formats.sh
# Universal build script for all Linux package formats
# Usage: ./build-all-formats.sh [version]

set -euo pipefail

VERSION="${1:-1.0.0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACKAGING_ROOT="$ROOT/packaging"

echo "====================================================================="
echo "🚀 Building all Linux package formats for Terraphim AI v$VERSION"
echo "====================================================================="
echo ""

# Create release directory
mkdir -p "$ROOT/release-artifacts"

# Function to build specific format
build_format() {
    local format="$1"
    echo "🔧 Building $format packages..."
    
    case "$format" in
        "deb")
            "$PACKAGING_ROOT/scripts/build-deb.sh"
            ;;
        "rpm")
            "$PACKAGING_ROOT/scripts/build-rpm.sh"
            ;;
        "arch")
            "$PACKAGING_ROOT/scripts/build-arch.sh"
            ;;
        "appimage")
            "$PACKAGING_ROOT/scripts/build-appimage.sh"
            ;;
        "flatpak")
            "$PACKAGING_ROOT/scripts/build-flatpak.sh"
            ;;
        "snap")
            "$PACKAGING_ROOT/scripts/build-snap.sh"
            ;;
        *)
            echo "❌ Unknown format: $format"
            return 1
            ;;
    esac
    
    echo "✅ $format build complete"
    echo ""
}

# Build all formats
FORMATS=("deb" "rpm" "arch" "appimage" "flatpak" "snap")

for format in "${FORMATS[@]}"; do
    build_format "$format"
done

# Move all artifacts to release directory
echo "📦 Collecting artifacts..."
find "$PACKAGING_ROOT" -name "*.$format" -o -name "*.AppImage" -o -name "*.flatpak" -o -name "*.snap" | while read -r artifact; do
    cp "$artifact" "$ROOT/release-artifacts/"
done

# Generate checksums
echo "🔐 Generating checksums..."
cd "$ROOT/release-artifacts"
sha256sum * > checksums.txt

# Display results
echo ""
echo "====================================================================="
echo "📋 Build Summary"
echo "====================================================================="
echo "Release artifacts created:"
ls -la

echo ""
echo "🔐 Checksums available in: checksums.txt"

# Verify package sizes
echo ""
echo "📊 Package sizes:"
for file in *.deb *.rpm *.pkg.tar* *.AppImage *.flatpak *.snap; do
    if [[ -f "$file" ]]; then
        size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo "unknown")
        echo "  $file: $(numfmt --to=iec-i --suffix=B "$size")"
    fi
done

echo ""
echo "🎉 All package formats built successfully!"
echo "====================================================================="