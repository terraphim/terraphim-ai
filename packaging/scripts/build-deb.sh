#!/bin/bash
# packaging/scripts/build-deb.sh
# Build Debian packages using cargo-deb
# Usage: ./build-deb.sh [--all|--agent|--server]

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET="${1:---all}"

echo "Building Debian packages..."

# Ensure cargo-deb is installed
if ! command -v cargo-deb &> /dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb
fi

build_package() {
    local crate_path="$1"
    local crate_name="$(basename "$crate_path")"

    if [[ -f "$crate_path/Cargo.toml" ]] && grep -q '\[package.metadata.deb\]' "$crate_path/Cargo.toml"; then
        echo "  Building $crate_name..."
        (cd "$ROOT" && cargo deb -p "$crate_name" --no-build)
    else
        echo "  Skipping $crate_name (no deb metadata)"
    fi
}

# Build release binaries first
echo "Building release binaries..."
cargo build --release -p terraphim_agent
cargo build --release -p terraphim_server

case "$TARGET" in
    "--agent")
        build_package "$ROOT/crates/terraphim_agent"
        ;;
    "--server")
        build_package "$ROOT/terraphim_server"
        ;;
    "--all"|*)
        # Build all crates with deb metadata
        build_package "$ROOT/crates/terraphim_agent"
        # Add more crates here as they get deb metadata
        ;;
esac

# List generated packages
echo ""
echo "Generated .deb packages:"
find "$ROOT/target/debian" -name "*.deb" -type f 2>/dev/null | while read -r deb; do
    echo "  $(basename "$deb")"
done

echo ""
echo "Debian packages built successfully!"
