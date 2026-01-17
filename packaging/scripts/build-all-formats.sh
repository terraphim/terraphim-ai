#!/bin/bash
# packaging/scripts/build-all-formats.sh
# Universal build script for all Linux package formats
# Usage: ./build-all-formats.sh [version] [formats...]
#
# Examples:
#   ./build-all-formats.sh                    # Build all formats
#   ./build-all-formats.sh 1.4.10            # Build all formats with specific version
#   ./build-all-formats.sh 1.4.10 deb rpm    # Build only deb and rpm
#
# Supported formats: deb, rpm, arch, appimage, flatpak, snap

set -euo pipefail

VERSION="${1:-}"
shift || true

# Get version from Cargo.toml if not provided
if [[ -z "$VERSION" ]]; then
    ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACKAGING_ROOT="$ROOT/packaging"
RELEASE_DIR="$ROOT/release-artifacts"

echo "====================================================================="
echo "Building all Linux package formats for Terraphim AI v$VERSION"
echo "====================================================================="
echo ""

# Create release directory
mkdir -p "$RELEASE_DIR"

# Setup Tauri signing if available
if [[ -f "$HOME/.tauri/tauriconfig" ]]; then
    # shellcheck source=/dev/null
    source "$HOME/.tauri/tauriconfig"
    echo "Using configured Tauri signing keys"
else
    echo "Note: Tauri signing not configured, building unsigned packages"
fi

# Determine which formats to build
if [[ $# -gt 0 ]]; then
    FORMATS=("$@")
else
    FORMATS=("deb" "rpm" "arch" "appimage" "flatpak" "snap")
fi

# Track build results
declare -A BUILD_RESULTS

# Function to build specific format
build_format() {
    local format="$1"
    local script="$PACKAGING_ROOT/scripts/build-$format.sh"

    echo "---------------------------------------------------------------------"
    echo "Building $format packages..."
    echo "---------------------------------------------------------------------"

    if [[ ! -x "$script" ]]; then
        if [[ -f "$script" ]]; then
            chmod +x "$script"
        else
            echo "Warning: Build script not found: $script"
            BUILD_RESULTS[$format]="skipped"
            return 0
        fi
    fi

    if "$script"; then
        BUILD_RESULTS[$format]="success"
        echo "$format build complete"
    else
        BUILD_RESULTS[$format]="failed"
        echo "Warning: $format build failed"
    fi
    echo ""
}

# Build release binaries first (shared by multiple formats)
echo "Building release binaries..."
cargo build --release -p terraphim_agent 2>&1 || {
    echo "Error: Failed to build release binaries"
    exit 1
}
echo ""

# Build each format
for format in "${FORMATS[@]}"; do
    build_format "$format" || true
done

# Collect all artifacts to release directory
echo "====================================================================="
echo "Collecting artifacts..."
echo "====================================================================="

# Collect from various output directories
collect_artifacts() {
    local src_dir="$1"
    local pattern="$2"

    if [[ -d "$src_dir" ]]; then
        find "$src_dir" -maxdepth 2 -name "$pattern" -type f 2>/dev/null | while read -r artifact; do
            cp -v "$artifact" "$RELEASE_DIR/" 2>/dev/null || true
        done
    fi
}

# Collect all package types
collect_artifacts "$ROOT/target/debian" "*.deb"
collect_artifacts "$ROOT/target/rpm" "*.rpm"
collect_artifacts "$ROOT/target/arch" "*.pkg.tar*"
collect_artifacts "$ROOT/target/appimage" "*.AppImage"
collect_artifacts "$ROOT/target/flatpak" "*.flatpak"
collect_artifacts "$ROOT/target/snap" "*.snap"

# Also collect from Tauri bundle directory
if [[ -d "$ROOT/desktop/src-tauri/target/release/bundle" ]]; then
    collect_artifacts "$ROOT/desktop/src-tauri/target/release/bundle/deb" "*.deb"
    collect_artifacts "$ROOT/desktop/src-tauri/target/release/bundle/rpm" "*.rpm"
    collect_artifacts "$ROOT/desktop/src-tauri/target/release/bundle/appimage" "*.AppImage"
fi

# Generate checksums
if [[ -n "$(ls -A "$RELEASE_DIR" 2>/dev/null)" ]]; then
    echo ""
    echo "Generating checksums..."
    (cd "$RELEASE_DIR" && sha256sum ./* > checksums.txt 2>/dev/null) || true
fi

# Display results
echo ""
echo "====================================================================="
echo "Build Summary"
echo "====================================================================="

echo ""
echo "Build results:"
for format in "${!BUILD_RESULTS[@]}"; do
    status="${BUILD_RESULTS[$format]}"
    case "$status" in
        "success") marker="[OK]" ;;
        "failed")  marker="[FAIL]" ;;
        "skipped") marker="[SKIP]" ;;
        *)         marker="[?]" ;;
    esac
    printf "  %-10s %s\n" "$format:" "$marker"
done

echo ""
echo "Release artifacts:"
if [[ -d "$RELEASE_DIR" ]] && [[ -n "$(ls -A "$RELEASE_DIR" 2>/dev/null)" ]]; then
    ls -lh "$RELEASE_DIR"
else
    echo "  (no artifacts found)"
fi

echo ""
echo "Checksums available in: $RELEASE_DIR/checksums.txt"

# Package sizes
echo ""
echo "Package sizes:"
shopt -s nullglob
for ext in deb rpm pkg.tar.zst pkg.tar.xz AppImage flatpak snap; do
    for file in "$RELEASE_DIR"/*."$ext"; do
        if [[ -f "$file" ]]; then
            size=$(stat -c%s "$file" 2>/dev/null || stat -f%z "$file" 2>/dev/null || echo "0")
            if command -v numfmt &> /dev/null; then
                size_human=$(numfmt --to=iec-i --suffix=B "$size" 2>/dev/null || echo "${size}B")
            else
                size_human="${size}B"
            fi
            printf "  %-40s %s\n" "$(basename "$file"):" "$size_human"
        fi
    done
done
shopt -u nullglob

# Final status
echo ""
echo "====================================================================="
failed_count=0
for status in "${BUILD_RESULTS[@]}"; do
    [[ "$status" == "failed" ]] && ((failed_count++)) || true
done

if [[ $failed_count -eq 0 ]]; then
    echo "All requested package formats built successfully!"
else
    echo "Warning: $failed_count format(s) failed to build."
fi
echo "====================================================================="

exit 0
