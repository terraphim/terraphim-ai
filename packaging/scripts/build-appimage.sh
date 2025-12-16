#!/bin/bash
# packaging/scripts/build-appimage.sh
# AppImage build script for Terraphim desktop

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${1:-1.0.0}"

echo "🔧 Building AppImage for v$VERSION..."

# Check for appimagetool
APPIMAGE_TOOL="$HOME/bin/appimagetool"
if [[ ! -x "$APPIMAGE_TOOL" ]]; then
    echo "📦 Installing appimagetool..."
    mkdir -p "$(dirname "$APPIMAGE_TOOL")"
    wget -O "$APPIMAGE_TOOL" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
    chmod +x "$APPIMAGE_TOOL"
fi

export PATH="$(dirname "$APPIMAGE_TOOL"):$PATH"
export TAURI_BUNDLE_APPIMAGE_BUNDLE_BIN="$APPIMAGE_TOOL"

# Create AppDir
APPDIR="$ROOT/target/appimage/terraphim-desktop.AppDir"
mkdir -p "$APPDIR"/{usr/bin,usr/share/applications,usr/share/icons/hicolor/256x256/apps}

# Build desktop application
echo "📱 Building desktop application..."
cd "$ROOT/desktop"
yarn install --frozen-lockfile

# Try Tauri AppImage build first
if yarn tauri build --bundles appimage --target x86_64-unknown-linux-gnu; then
    echo "✅ Tauri AppImage build succeeded"
    
    # Copy generated AppImage
    find . -name "*.AppImage" -copy "$ROOT/release-artifacts/" 2>/dev/null || true
else
    echo "⚠️ Tauri AppImage failed, attempting manual build..."
    
    # Manual AppImage build process
    yarn tauri build --target x86_64-unknown-linux-gnu
    
    # Copy binary to AppDir
    cp "$ROOT/desktop/target/x86_64-unknown-linux-gnu/release/terraphim-desktop" "$APPDIR/usr/bin/"
    
    # Create desktop file
    cat > "$APPDIR/usr/share/applications/com.terraphim.ai.desktop" << EOF
[Desktop Entry]
Type=Application
Name=Terraphim AI
Comment=Privacy-first AI assistant
Exec=terraphim-desktop
Icon=terraphim-desktop
Terminal=false
Categories=Development;Office;
EOF
    
    # Copy icon
    mkdir -p "$(dirname "$APPDIR/usr/share/icons/hicolor/256x256/apps")"
    cp "$ROOT/desktop/src-tauri/icons/128x128.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/terraphim-desktop.png"
    
    # Create AppImage
    cd "$(dirname "$APPDIR")"
    appimagetool "$APPDIR" "terraphim-desktop_${VERSION}_amd64.AppImage"
    
    # Copy AppImage to release artifacts
    cp "terraphim-desktop_${VERSION}_amd64.AppImage" "$ROOT/release-artifacts/"
fi

# Make executable
chmod +x "$ROOT/release-artifacts"/*.AppImage

echo "✅ AppImage built successfully"
echo "📦 Available AppImage files:"
ls -la "$ROOT/release-artifacts"/*.AppImage 2>/dev/null || echo "No AppImage files found"