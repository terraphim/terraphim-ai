#!/bin/bash

# Terraphim AI Cross-Platform Build Script
# This script builds Terraphim AI for multiple platforms manually
# Use this when GitHub Actions workflows are having issues

set -e

VERSION=${1:-"0.2.5-cross-platform"}
BUILD_DIR="build-cross-platform"
RELEASE_DIR="release-$VERSION"

echo "🚀 Building Terraphim AI v$VERSION for multiple platforms..."

# Clean previous builds
rm -rf $BUILD_DIR $RELEASE_DIR
mkdir -p $BUILD_DIR $RELEASE_DIR

# Install cross-compilation tools
echo "📦 Installing cross-compilation tools..."
cargo install cross 2>/dev/null || echo "cross already installed"

# Function to build for a specific target
build_target() {
    local target=$1
    local use_cross=$2

    echo "🔨 Building for $target..."

    if [ "$use_cross" = "true" ]; then
        cross build --release --target $target --bin terraphim_server
        cross build --release --target $target --bin terraphim-tui
    else
        cargo build --release --target $target --bin terraphim_server
        cargo build --release --target $target --bin terraphim-tui
    fi

    # Copy binaries to build directory
    if [[ "$target" == *"windows"* ]]; then
        cp target/$target/release/terraphim_server.exe $BUILD_DIR/terraphim_server-$target.exe
        cp target/$target/release/terraphim-tui.exe $BUILD_DIR/terraphim-tui-$target.exe
    else
        cp target/$target/release/terraphim_server $BUILD_DIR/terraphim_server-$target
        cp target/$target/release/terraphim-tui $BUILD_DIR/terraphim-tui-$target
        chmod +x $BUILD_DIR/terraphim_server-$target $BUILD_DIR/terraphim-tui-$target
    fi
}

# Build for all targets
echo "🏗️  Starting cross-platform builds..."

# Linux builds
build_target "x86_64-unknown-linux-gnu" "false"
build_target "x86_64-unknown-linux-musl" "true"
build_target "aarch64-unknown-linux-musl" "true"
build_target "armv7-unknown-linux-musleabihf" "true"

# macOS builds
build_target "x86_64-apple-darwin" "false"
build_target "aarch64-apple-darwin" "false"

# Windows builds
build_target "x86_64-pc-windows-msvc" "false"

# Create release directory structure
echo "📁 Creating release directory..."
mkdir -p $RELEASE_DIR/{binaries,linux,windows,macos,docs}

# Copy binaries to appropriate directories
cp $BUILD_DIR/*linux* $RELEASE_DIR/linux/
cp $BUILD_DIR/*windows* $RELEASE_DIR/windows/
cp $BUILD_DIR*darwin* $RELEASE_DIR/macos/
cp $BUILD_DIR/* $RELEASE_DIR/binaries/

# Generate checksums
echo "🔐 Generating checksums..."
cd $RELEASE_DIR
find . -type f -exec sha256sum {} + > checksums.txt
cd ..

# Create installation scripts
echo "📜 Creating installation scripts..."

# Linux installation script
cat > $RELEASE_DIR/install-linux.sh << 'EOF'
#!/bin/bash
# Terraphim AI Linux Installation Script

set -e
ARCH=$(uname -m)
VERSION="0.2.5-cross-platform"

echo "Installing Terraphim AI $VERSION for Linux ($ARCH)..."

# Detect architecture and download appropriate binary
case $ARCH in
    x86_64)
        if command -v apt-get >/dev/null 2>&1; then
            echo "Detected Debian/Ubuntu system"
            wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim-server_$VERSION-1_amd64.deb
            sudo dpkg -i terraphim-server_$VERSION-1_amd64.deb
        elif command -v yum >/dev/null 2>&1 || command -v dnf >/dev/null 2>&1; then
            echo "Detected RedHat/Fedora system"
            wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim-server-$VERSION-1.x86_64.rpm
            sudo rpm -i terraphim-server-$VERSION-1.x86_64.rpm
        else
            echo "Using generic binary installation"
            wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim_server-x86_64-unknown-linux-gnu
            chmod +x terraphim_server-x86_64-unknown-linux-gnu
            sudo mv terraphim_server-x86_64-unknown-linux-gnu /usr/local/bin/terraphim_server
        fi
        ;;
    aarch64)
        echo "ARM64 detected - using generic binary"
        wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/terraphim_server-aarch64-unknown-linux-musl
        chmod +x terraphim_server-aarch64-unknown-linux-musl
        sudo mv terraphim_server-aarch64-unknown-linux-musl /usr/local/bin/terraphim_server
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "✅ Installation completed!"
echo "Run 'terraphim_server --help' to get started"
EOF

chmod +x $RELEASE_DIR/install-linux.sh

# macOS installation script
cat > $RELEASE_DIR/install-macos.sh << 'EOF'
#!/bin/bash
# Terraphim AI macOS Installation Script

set -e
ARCH=$(uname -m)
VERSION="0.2.5-cross-platform"

echo "Installing Terraphim AI $VERSION for macOS ($ARCH)..."

# Detect architecture and download appropriate binary
case $ARCH in
    x86_64)
        BINARY="terraphim_server-x86_64-apple-darwin"
        ;;
    arm64)
        BINARY="terraphim_server-aarch64-apple-darwin"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Download and install
wget -q https://github.com/terraphim/terraphim-ai/releases/download/v$VERSION/$BINARY
chmod +x $BINARY
sudo mv $BINARY /usr/local/bin/terraphim_server

echo "✅ Installation completed!"
echo "Run 'terraphim_server --help' to get started"
EOF

chmod +x $RELEASE_DIR/install-macos.sh

# Windows installation script
cat > $RELEASE_DIR/install-windows.ps1 << 'EOF'
# Terraphim AI Windows Installation Script

param(
    [string]$Version = "0.2.5-cross-platform"
)

Write-Host "Installing Terraphim AI $Version for Windows..."

# Download binaries
$ServerUrl = "https://github.com/terraphim/terraphim-ai/releases/download/v$Version/terraphim_server-x86_64-pc-windows-msvc.exe"
$TuiUrl = "https://github.com/terraphim/terraphim-ai/releases/download/v$Version/terraphim-tui-x86_64-pc-windows-msvc.exe"

# Create installation directory
$InstallDir = "$env:ProgramFiles\Terraphim"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force
}

# Download files
Invoke-WebRequest -Uri $ServerUrl -OutFile "$InstallDir\terraphim_server.exe"
Invoke-WebRequest -Uri $TuiUrl -OutFile "$InstallDir\terraphim_tui.exe"

# Add to PATH
$Path = [Environment]::GetEnvironmentVariable("PATH", "Machine")
if ($Path -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", $Path + ";$InstallDir", "Machine")
}

Write-Host "✅ Installation completed!"
Write-Host "Run 'terraphim_server --help' to get started"
Write-Host "Note: You may need to restart PowerShell for PATH changes to take effect."
EOF

# Create README
cat > $RELEASE_DIR/README.md << EOF
# Terraphim AI v$VERSION - Cross-Platform Release

## 🚀 Quick Installation

### Linux
\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release-$VERSION/install-linux.sh | bash
\`\`\`

### macOS
\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release-$VERSION/install-macos.sh | bash
\`\`\`

### Windows (PowerShell)
\`\`\`powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release-$VERSION/install-windows.ps1" -OutFile "install-windows.ps1"
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
.\install-windows.ps1
\`\`\`

## 📦 Available Binaries

### Linux
- \`x86_64-unknown-linux-gnu\`: Standard Linux (glibc)
- \`x86_64-unknown-linux-musl\`: Alpine Linux compatible
- \`aarch64-unknown-linux-musl\`: ARM64 Linux
- \`armv7-unknown-linux-musleabihf\`: ARMv7 Linux

### macOS
- \`x86_64-apple-darwin\`: Intel Mac
- \`aarch64-apple-darwin\`: Apple Silicon (M1/M2)

### Windows
- \`x86_64-pc-windows-msvc\`: Windows 64-bit

## 🔐 Verification

All files can be verified using the provided \`checksums.txt\`:

\`\`\`bash
sha256sum -c checksums.txt
\`\`\`

## 🐳 Docker

\`\`\`bash
docker run -d --name terraphim-server -p 8000:8000 ghcr.io/terraphim/terraphim-server:v$VERSION
\`\`\`

## 📚 Documentation

- Complete documentation: https://docs.terraphim.ai
- GitHub repository: https://github.com/terraphim/terraphim-ai
- Issues and support: https://github.com/terraphim/terraphim-ai/issues

Built on: $(date)
Version: $VERSION
EOF

# Create archive
echo "📦 Creating release archive..."
tar -czf terraphim-ai-$VERSION-cross-platform.tar.gz $RELEASE_DIR/

echo "✅ Cross-platform build completed!"
echo "📁 Release directory: $RELEASE_DIR"
echo "📦 Archive: terraphim-ai-$VERSION-cross-platform.tar.gz"
echo ""
echo "🚀 To upload to GitHub:"
echo "   gh release create v$VERSION --title 'Terraphim AI v$VERSION - Cross-Platform' --notes-file $RELEASE_DIR/README.md"
echo "   gh release upload v$VERSION $RELEASE_DIR/*"
echo ""
echo "🔐 Checksums available in: $RELEASE_DIR/checksums.txt"