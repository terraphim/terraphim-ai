#!/bin/bash
# packaging/scripts/build-flatpak.sh
# Build Flatpak package
# Usage: ./build-flatpak.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$ROOT/target/flatpak"
BUILD_DIR="$OUTPUT_DIR/build"

echo "Building Flatpak package..."

# Check for flatpak-builder
if ! command -v flatpak-builder &> /dev/null; then
    echo "Warning: flatpak-builder not found."
    echo "  Install: sudo apt install flatpak-builder"
    echo "  Or: sudo dnf install flatpak-builder"
    exit 1
fi

# Create directories
mkdir -p "$BUILD_DIR"
mkdir -p "$OUTPUT_DIR/repo"

# Get version
VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Create Flatpak manifest
cat > "$BUILD_DIR/ai.terraphim.Agent.yml" << EOF
app-id: ai.terraphim.Agent
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: terraphim-agent
finish-args:
  - --share=network
  - --share=ipc
  - --filesystem=home
  - --socket=fallback-x11
  - --socket=wayland
modules:
  - name: terraphim-agent
    buildsystem: simple
    build-options:
      append-path: /usr/lib/sdk/rust-stable/bin
      env:
        CARGO_HOME: /run/build/terraphim-agent/cargo
    build-commands:
      - cargo --offline fetch --manifest-path Cargo.toml --verbose
      - cargo --offline build --release -p terraphim_agent --verbose
      - install -Dm755 target/release/terraphim-agent /app/bin/terraphim-agent
    sources:
      - type: dir
        path: $ROOT
EOF

# Build the Flatpak
echo "Building Flatpak with flatpak-builder..."
flatpak-builder --force-clean --repo="$OUTPUT_DIR/repo" \
    "$BUILD_DIR/app" "$BUILD_DIR/ai.terraphim.Agent.yml" || {
    echo "Flatpak build failed."
    echo "Note: Flatpak requires all dependencies to be available offline."
    echo "Run 'cargo vendor' and update the manifest for offline builds."
    exit 1
}

# Create distributable bundle
echo "Creating Flatpak bundle..."
flatpak build-bundle "$OUTPUT_DIR/repo" \
    "$OUTPUT_DIR/terraphim-agent-$VERSION.flatpak" \
    ai.terraphim.Agent

echo ""
echo "Generated Flatpak packages:"
find "$OUTPUT_DIR" -maxdepth 1 -name "*.flatpak" -type f 2>/dev/null | while read -r pkg; do
    echo "  $(basename "$pkg")"
done

echo ""
echo "Flatpak package built successfully!"
echo "Install with: flatpak install $OUTPUT_DIR/terraphim-agent-$VERSION.flatpak"
