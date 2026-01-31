#!/bin/bash
# packaging/scripts/build-appimage.sh
# Build AppImage using appimagetool or Tauri
# Usage: ./build-appimage.sh [--cli|--desktop]

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$ROOT/target/appimage"
TARGET="${1:---desktop}"

echo "Building AppImage packages..."

# Create output directory
mkdir -p "$OUTPUT_DIR"

case "$TARGET" in
    "--cli")
        # Build CLI AppImage using appimagetool
        if ! command -v appimagetool &> /dev/null; then
            echo "Downloading appimagetool..."
            ARCH=$(uname -m)
            wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$ARCH.AppImage" \
                -O /tmp/appimagetool
            chmod +x /tmp/appimagetool
            APPIMAGETOOL="/tmp/appimagetool"
        else
            APPIMAGETOOL="appimagetool"
        fi

        # Build release binary
        echo "Building release binary..."
        cargo build --release -p terraphim_agent

        # Create AppDir structure
        APPDIR="$OUTPUT_DIR/terraphim-agent.AppDir"
        mkdir -p "$APPDIR/usr/bin"
        mkdir -p "$APPDIR/usr/share/applications"
        mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

        # Copy binary
        cp "$ROOT/target/release/terraphim-agent" "$APPDIR/usr/bin/"

        # Create desktop file
        cat > "$APPDIR/terraphim-agent.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Terraphim Agent
Exec=terraphim-agent
Icon=terraphim-agent
Categories=Utility;Development;
Terminal=true
EOF
        cp "$APPDIR/terraphim-agent.desktop" "$APPDIR/usr/share/applications/"

        # Create AppRun
        cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/terraphim-agent" "$@"
EOF
        chmod +x "$APPDIR/AppRun"

        # Create placeholder icon if not exists
        if [[ ! -f "$APPDIR/terraphim-agent.png" ]]; then
            # Create simple placeholder - in production use actual icon
            echo "Note: Using placeholder icon. Replace with actual icon."
            convert -size 256x256 xc:navy -fill white -gravity center \
                -pointsize 48 -annotate 0 "T" \
                "$APPDIR/terraphim-agent.png" 2>/dev/null || \
                touch "$APPDIR/terraphim-agent.png"
        fi

        # Build AppImage
        VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
        ARCH=$(uname -m)
        "$APPIMAGETOOL" "$APPDIR" "$OUTPUT_DIR/terraphim-agent-$VERSION-$ARCH.AppImage"
        ;;

    "--desktop"|*)
        # Build desktop AppImage using Tauri
        echo "Building desktop AppImage using Tauri..."
        if [[ ! -d "$ROOT/desktop/src-tauri" ]]; then
            echo "Error: Tauri project not found at desktop/src-tauri"
            exit 1
        fi

        (cd "$ROOT/desktop" && yarn tauri build --bundles appimage) || {
            echo "Tauri build failed. Ensure yarn and tauri-cli are installed."
            exit 1
        }

        # Copy AppImage to output
        find "$ROOT/desktop/src-tauri/target/release/bundle" -name "*.AppImage" \
            -exec cp {} "$OUTPUT_DIR/" \;
        ;;
esac

echo ""
echo "Generated AppImage packages:"
find "$OUTPUT_DIR" -maxdepth 1 -name "*.AppImage" -type f 2>/dev/null | while read -r img; do
    echo "  $(basename "$img")"
done

echo ""
echo "AppImage packages built successfully!"
