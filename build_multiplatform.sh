#!/bin/bash

echo "====================================================================="
echo "BUILDING TERRAPHIM AI v1.0.2 - MULTI-PLATFORM RELEASE"
echo "====================================================================="
echo ""

# Create release directory structure
mkdir -p releases/v1.0.2/{macos,linux,windows,desktop}

# Function to build and save artifacts
build_target() {
    local TARGET=$1
    local OS=$2
    local ARCH=$3

    echo "Building for $TARGET..."

    # Build all three binaries
    cargo build --release --target $TARGET \
        --package terraphim_server \
        --package terraphim_mcp_server \
        --package terraphim_agent \
        --package terraphim-cli 2>/dev/null

    if [ $? -eq 0 ]; then
        echo "✅ Built successfully for $TARGET"

        # Create target directory
        mkdir -p releases/v1.0.2/$OS/$ARCH

        # Copy binaries with proper naming
        if [ "$OS" = "windows" ]; then
            cp target/$TARGET/release/terraphim_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/terraphim_server.exe 2>/dev/null
            cp target/$TARGET/release/terraphim_mcp_server.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/terraphim_mcp_server.exe 2>/dev/null
            cp target/$TARGET/release/terraphim-agent.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim-agent releases/v1.0.2/$OS/$ARCH/terraphim-agent.exe 2>/dev/null
            cp target/$TARGET/release/terraphim-cli.exe releases/v1.0.2/$OS/$ARCH/ 2>/dev/null || cp target/$TARGET/release/terraphim-cli releases/v1.0.2/$OS/$ARCH/terraphim-cli.exe 2>/dev/null
        else
            cp target/$TARGET/release/terraphim_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
            cp target/$TARGET/release/terraphim_mcp_server releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
            cp target/$TARGET/release/terraphim-agent releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
            cp target/$TARGET/release/terraphim-cli releases/v1.0.2/$OS/$ARCH/ 2>/dev/null
        fi

        # Create tar.gz archive for Unix systems
        if [ "$OS" != "windows" ]; then
            cd releases/v1.0.2/$OS/$ARCH
            tar -czf ../terraphim-ai-v1.0.2-$OS-$ARCH.tar.gz *
            cd ../../../../
            echo "📦 Created releases/v1.0.2/$OS/terraphim-ai-v1.0.2-$OS-$ARCH.tar.gz"
        else
            # Create zip for Windows
            cd releases/v1.0.2/$OS/$ARCH
            zip -q ../terraphim-ai-v1.0.2-$OS-$ARCH.zip *.exe
            cd ../../../../
            echo "📦 Created releases/v1.0.2/$OS/terraphim-ai-v1.0.2-$OS-$ARCH.zip"
        fi
    else
        echo "❌ Failed to build for $TARGET"
    fi
    echo ""
}

echo "1. BUILDING NATIVE BINARIES (current platform)"
echo "------------------------------------------------"
cargo build --release \
    --package terraphim_server \
    --package terraphim_mcp_server \
    --package terraphim_agent \
    --package terraphim-cli

# Detect current platform and save native binaries
if [[ "$OSTYPE" == "darwin"* ]]; then
    ARCH=$(uname -m)
    if [ "$ARCH" = "arm64" ]; then
        mkdir -p releases/v1.0.2/macos/aarch64
        cp target/release/terraphim_server releases/v1.0.2/macos/aarch64/
        cp target/release/terraphim_mcp_server releases/v1.0.2/macos/aarch64/
        cp target/release/terraphim-agent releases/v1.0.2/macos/aarch64/
        cp target/release/terraphim-cli releases/v1.0.2/macos/aarch64/
        echo "✅ Saved native macOS ARM64 binaries"
    else
        mkdir -p releases/v1.0.2/macos/x86_64
        cp target/release/terraphim_server releases/v1.0.2/macos/x86_64/
        cp target/release/terraphim_mcp_server releases/v1.0.2/macos/x86_64/
        cp target/release/terraphim-agent releases/v1.0.2/macos/x86_64/
        cp target/release/terraphim-cli releases/v1.0.2/macos/x86_64/
        echo "✅ Saved native macOS x86_64 binaries"
    fi
fi

echo ""
echo "2. CROSS-COMPILING FOR OTHER PLATFORMS"
echo "----------------------------------------"

# macOS targets
build_target "x86_64-apple-darwin" "macos" "x86_64"
build_target "aarch64-apple-darwin" "macos" "aarch64"

# Linux targets
build_target "x86_64-unknown-linux-gnu" "linux" "x86_64"
build_target "aarch64-unknown-linux-gnu" "linux" "aarch64"

# Windows targets
build_target "x86_64-pc-windows-gnu" "windows" "x86_64"
build_target "x86_64-pc-windows-msvc" "windows" "x86_64-msvc"

echo "3. BUILDING DESKTOP APP (Tauri)"
echo "---------------------------------"
cd desktop
echo "Installing dependencies..."
yarn install

echo "Building desktop app..."
yarn tauri build

# Copy desktop builds to release directory
if [ -d "target/release/bundle" ]; then
    cp -r target/release/bundle/* ../releases/v1.0.2/desktop/ 2>/dev/null
    echo "✅ Desktop app built and saved"
else
    echo "❌ Desktop app build failed or bundle not found"
fi

cd ..

echo ""
echo "4. CREATING UNIVERSAL BINARIES (macOS)"
echo "----------------------------------------"
if [ -f "releases/v1.0.2/macos/x86_64/terraphim_server" ] && [ -f "releases/v1.0.2/macos/aarch64/terraphim_server" ]; then
    mkdir -p releases/v1.0.2/macos/universal
    lipo -create \
        releases/v1.0.2/macos/x86_64/terraphim_server \
        releases/v1.0.2/macos/aarch64/terraphim_server \
        -output releases/v1.0.2/macos/universal/terraphim_server
    lipo -create \
        releases/v1.0.2/macos/x86_64/terraphim_mcp_server \
        releases/v1.0.2/macos/aarch64/terraphim_mcp_server \
        -output releases/v1.0.2/macos/universal/terraphim_mcp_server
    lipo -create \
        releases/v1.0.2/macos/x86_64/terraphim-agent \
        releases/v1.0.2/macos/aarch64/terraphim-agent \
        -output releases/v1.0.2/macos/universal/terraphim-agent
    lipo -create \
        releases/v1.0.2/macos/x86_64/terraphim-cli \
        releases/v1.0.2/macos/aarch64/terraphim-cli \
        -output releases/v1.0.2/macos/universal/terraphim-cli

    cd releases/v1.0.2/macos/universal
    tar -czf ../terraphim-ai-v1.0.2-macos-universal.tar.gz *
    cd ../../../../
    echo "✅ Created universal macOS binaries"
fi

echo ""
echo "====================================================================="
echo "BUILD SUMMARY"
echo "====================================================================="
echo "Release artifacts saved to: releases/v1.0.2/"
echo ""
echo "Available builds:"
ls -la releases/v1.0.2/*/terraphim-ai-v1.0.2-*.{tar.gz,zip} 2>/dev/null || echo "No archives found"
echo ""
echo "Desktop app bundles:"
ls -la releases/v1.0.2/desktop/ 2>/dev/null || echo "No desktop bundles found"
echo ""
echo "====================================================================="
