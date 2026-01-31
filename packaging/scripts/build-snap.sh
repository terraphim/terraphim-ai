#!/bin/bash
# packaging/scripts/build-snap.sh
# Build Snap package
# Usage: ./build-snap.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$ROOT/target/snap"
BUILD_DIR="$OUTPUT_DIR/build"

echo "Building Snap package..."

# Check for snapcraft
if ! command -v snapcraft &> /dev/null; then
    echo "Warning: snapcraft not found."
    echo "  Install: sudo snap install snapcraft --classic"
    exit 1
fi

# Create directories
mkdir -p "$BUILD_DIR"

# Get version
VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Create snapcraft.yaml
cat > "$BUILD_DIR/snapcraft.yaml" << EOF
name: terraphim-agent
version: '$VERSION'
summary: Terraphim AI Agent CLI
description: |
  Terraphim Agent - AI Agent CLI Interface for Terraphim.
  Command-line interface with interactive REPL and ASCII graph visualization.
  Supports search, configuration management, and data exploration.

grade: stable
confinement: classic
base: core22

architectures:
  - build-on: [amd64]
  - build-on: [arm64]

parts:
  terraphim-agent:
    plugin: rust
    source: $ROOT
    rust-channel: stable
    build-packages:
      - pkg-config
      - libssl-dev
    stage-packages:
      - libssl3
    override-build: |
      cargo build --release -p terraphim_agent
      mkdir -p \$SNAPCRAFT_PART_INSTALL/bin
      cp target/release/terraphim-agent \$SNAPCRAFT_PART_INSTALL/bin/

apps:
  terraphim-agent:
    command: bin/terraphim-agent
    plugs:
      - home
      - network
      - network-bind
EOF

# Build the snap
echo "Building snap with snapcraft..."
(cd "$BUILD_DIR" && snapcraft) || {
    echo "Snap build failed."
    echo "Try running with --debug for more information."
    exit 1
}

# Copy snap to output directory
find "$BUILD_DIR" -name "*.snap" -exec cp {} "$OUTPUT_DIR/" \;

echo ""
echo "Generated Snap packages:"
find "$OUTPUT_DIR" -maxdepth 1 -name "*.snap" -type f 2>/dev/null | while read -r pkg; do
    echo "  $(basename "$pkg")"
done

echo ""
echo "Snap package built successfully!"
echo "Install with: sudo snap install --classic $OUTPUT_DIR/terraphim-agent_${VERSION}_*.snap"
