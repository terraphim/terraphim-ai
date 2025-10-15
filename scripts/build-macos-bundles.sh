#!/bin/bash
# Build macOS App Bundles for Terraphim AI

set -e

VERSION="0.2.3"
BUNDLE_DIR="macos-bundles"
BINARIES_DIR="target/release"

echo "🍎 Building macOS App Bundles for Terraphim AI v$VERSION"
echo "=================================================="

# Check if we have the required binaries
if [ ! -f "$BINARIES_DIR/terraphim_server" ]; then
    echo "🔨 Building Rust binaries..."
    cargo build --release --package terraphim_server
    cargo build --release --package terraphim_tui --features repl-full
fi

# Create server app bundle
echo "📦 Creating Terraphim Server app bundle..."

if [ -f "$BINARIES_DIR/terraphim_server" ]; then
    cp "$BINARIES_DIR/terraphim_server" "$BUNDLE_DIR/TerraphimServer.app/Contents/MacOS/"
    chmod +x "$BUNDLE_DIR/TerraphimServer.app/Contents/MacOS/terraphim_server"
    echo "✅ Terraphim Server app bundle created"
else
    echo "❌ terraphim_server binary not found"
    exit 1
fi

# Create TUI app bundle
echo "📱 Creating Terraphim TUI app bundle..."

if [ -f "$BINARIES_DIR/terraphim-tui" ]; then
    cp "$BINARIES_DIR/terraphim-tui" "$BUNDLE_DIR/TerraphimTUI.app/Contents/MacOS/terraphim-tui"
    chmod +x "$BUNDLE_DIR/TerraphimTUI.app/Contents/MacOS/terraphim-tui"
    echo "✅ Terraphim TUI app bundle created"
else
    echo "❌ terraphim-tui binary not found"
    exit 1
fi

# Create application scripts for Terminal launching
echo "📝 Creating launch scripts..."

# Server launch script that opens Terminal
cat > "$BUNDLE_DIR/TerraphimServer.app/Contents/MacOS/launch-server.sh" << 'EOF'
#!/bin/bash
# Launch Terraphim Server in Terminal

# Get the directory where the app bundle is located
APP_DIR="$(dirname "$(dirname "$(readlink "$0")")")"

# Create necessary directories
mkdir -p "$HOME/Library/Application Support/Terraphim"
mkdir -p "$HOME/Library/Application Support/Terraphim/data"
mkdir -p "$HOME/Library/Application Support/Terraphim/config"

# Set environment variables
export RUST_LOG=info
export LOG_LEVEL=info

# Launch Terminal and run the server
osascript -e 'tell application "Terminal"
    do script
        "'"'"'"$APP_DIR/MacOS/terraphim_server"'"'"
    end script
    activate
end tell'
EOF

# TUI launch script that opens Terminal
cat > "$BUNDLE_DIR/TerraphimTUI.app/Contents/MacOS/launch-tui.sh" << 'EOF'
#!/bin/bash
# Launch Terraphim TUI in Terminal

# Get the directory where the app bundle is located
APP_DIR="$(dirname "$(dirname "$(readlink "$0")")")"

# Create necessary directories
mkdir -p "$HOME/Library/Application Support/Terraphim"
mkdir -p "$HOME/Library/Application Support/Terraphim/data"
mkdir -p "$HOME/Library/Application Support/Terraphim/config"

# Set environment variables
export RUST_LOG=info
export LOG_LEVEL=info

# Launch Terminal and run the TUI
osascript -e 'tell application "Terminal"
    do script
        "'"'"'"$APP_DIR/MacOS/teraphim-tui"'"'"
    end script
    activate
end tell'
EOF

# Make launch scripts executable
chmod +x "$BUNDLE_DIR/TerraphimServer.app/Contents/MacOS/launch-server.sh"
chmod +x "$BUNDLE_DIR/TerraphimTUI.app/Contents/MacOS/launch-tui.sh"

# Update Info.plist to use launch scripts
echo "⚙️ Updating app bundle configurations..."

# Update server Info.plist to use launch script
sed -i '' 's|<string>terraphim_server</string>|<string>launch-server.sh</string>|' "$BUNDLE_DIR/TerraphimServer.app/Contents/Info.plist"

# Update TUI Info.plist to use launch script
sed -i '' 's|<string>terraphim-tui</string>|<string>launch-tui.sh</string>|' "$BUNDLE_DIR/TerraphimTUI.app/Contents/Info.plist"

# Create simple icon placeholder (text-based)
echo "🎨 Creating app icons..."

# Create a simple icon (placeholder)
cat > "$BUNDLE_DIR/TerraphimServer.app/Contents/Resources/AppIcon.icns" << 'EOF'
placeholder for server icon
EOF

cat > "$BUNDLE_DIR/TerraphimTUI.app/Contents/Resources/AppIcon.icns" << 'EOF'
placeholder for TUI icon
EOF

# Create DMG distribution script
echo "💿 Creating DMG distribution script..."

cat > "$BUNDLE_DIR/create-dmg.sh" << 'EOF'
#!/bin/bash
# Create macOS DMG for distribution

VERSION="0.2.3"
DMG_NAME="Terraphim-AI-$VERSION"
APP_DIR="macos-bundles"

# Create DMG
hdiutil create -volname "$DMG_NAME" \
    -srcfolder "$APP_DIR" \
    -ov -format UDZO \
    "$DMG_NAME.dmg"

echo "✅ DMG created: $DMG_NAME.dmg"
EOF

chmod +x "$BUNDLE_DIR/create-dmg.sh"

# Create installation script
echo "📋 Creating installation instructions..."

cat > "$BUNDLE_DIR/INSTALL-macOS.md" << 'EOF'
# Terraphim AI macOS Installation Guide

## Method 1: App Bundle (Easy)

1. Download the Terraphim AI DMG
2. Mount the DMG and drag the apps to Applications
3. Launch from Applications folder

## Method 2: Terminal Installation

1. Install using Homebrew (if available):
   ```bash
   brew install terraphim-ai
   ```

2. Or build from source:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai.git
   cd terraphim-ai
   cargo build --release
   ```

## First Launch

When you first launch Terraphim AI:
1. The Terminal will open automatically
2. Configuration files will be created in:
   - `~/Library/Application Support/Terraphim/config.json`
   - `~/Library/Application Support/Terraphim/data/`

## Access Points

### Terraphim Server
- **Web Interface**: http://localhost:8000
- **API**: http://localhost:8000/api
- **Health Check**: http://localhost:8000/health

### Terraphim TUI
- Opens in Terminal with interactive interface
- Type `help` for available commands
- Use `search "query"` to search your knowledge

## Configuration

Edit `~/Library/Application Support/Terraphim/config.json` to:
- Add data sources (documents, code repositories)
- Configure search parameters
- Set up AI providers (Ollama, OpenRouter)

## Troubleshooting

If apps don't launch:
1. Check Gatekeeper settings: System Preferences > Security & Privacy > General
2. Right-click app > Open if blocked
3. Ensure Terminal app has permission to run scripts

## Support
- Documentation: https://docs.terraphim.ai
- Issues: https://github.com/terraphim/terraphim-ai/issues
- Community: https://discord.gg/VPJXB6BGuY
EOF

echo ""
echo "🎉 macOS App Bundles Created Successfully!"
echo "=================================="
echo "Created bundles:"
echo "  - TerraphimServer.app"
echo "  - TerraphimTUI.app"
echo ""
echo "Bundle location: $(pwd)/macos-bundles/"
echo ""
echo "Next steps:"
echo "1. Test the app bundles"
echo "2. Create DMG: ./macos-bundles/create-dmg.sh"
echo "3. Upload to GitHub release"
echo ""
echo "Note: These are basic app bundles. For production distribution,"
echo "      consider codesigning the apps and notarizing them with Apple."