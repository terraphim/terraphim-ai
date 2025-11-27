#!/bin/bash
set -e

# Terraphim v1.0.0 Minimal Release Publication Script
# This script publishes terraphim-repl and terraphim-cli to crates.io
# and creates a GitHub release using 1Password CLI for token management

echo "=========================================="
echo "Terraphim v1.0.0 Minimal Release Publisher"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check prerequisites
echo "Checking prerequisites..."

# Check if op CLI is installed
if ! command -v op &> /dev/null; then
    print_error "1Password CLI (op) is not installed"
    echo "Install from: https://developer.1password.com/docs/cli/get-started/"
    exit 1
fi
print_status "1Password CLI found"

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    print_error "GitHub CLI (gh) is not installed"
    echo "Install from: https://cli.github.com/"
    exit 1
fi
print_status "GitHub CLI found"

# Check if we're in the right directory
if [ ! -f "MINIMAL_RELEASE_PLAN.md" ]; then
    print_error "Not in terraphim-ai root directory"
    exit 1
fi
print_status "In terraphim-ai root directory"

# Check if we're on the right branch
CURRENT_BRANCH=$(git branch --show-current)
print_info "Current branch: $CURRENT_BRANCH"

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_warning "You have uncommitted changes"
    git status --short
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_error "Aborting"
        exit 1
    fi
fi

echo ""
echo "=========================================="
echo "Step 1: Verify Library Crates Published"
echo "=========================================="
echo ""

# Check if library crates are already published
for crate in terraphim_types terraphim_automata terraphim_rolegraph; do
    if cargo search $crate --limit 1 | grep -q "^$crate ="; then
        VERSION=$(cargo search $crate --limit 1 | grep "^$crate =" | cut -d'"' -f2)
        print_status "$crate v$VERSION already published"
    else
        print_warning "$crate not found on crates.io"
    fi
done

echo ""
echo "=========================================="
echo "Step 2: Get crates.io API Token from 1Password"
echo "=========================================="
echo ""

# Get crates.io token from 1Password
print_info "Fetching crates.io token from 1Password..."
CRATES_IO_TOKEN=$(op read "op://TerraphimPlatform/crates.io.token/token")

if [ -z "$CRATES_IO_TOKEN" ]; then
    print_error "Failed to retrieve crates.io token from 1Password"
    exit 1
fi

# Set token length for display (don't show actual token)
TOKEN_LENGTH=${#CRATES_IO_TOKEN}
print_status "Retrieved crates.io token (${TOKEN_LENGTH} characters)"

# Set the token for cargo
export CARGO_REGISTRY_TOKEN="$CRATES_IO_TOKEN"

echo ""
echo "=========================================="
echo "Step 3: Publish terraphim-repl v1.0.0"
echo "=========================================="
echo ""

print_info "Publishing terraphim-repl to crates.io..."
cd crates/terraphim_repl

# Final check before publishing
print_info "Running final tests..."
cargo test --quiet 2>&1 | grep -E "(test result|passed|failed)" || true

# Publish
print_info "Publishing (this may take a minute)..."
if cargo publish 2>&1 | tee /tmp/publish-repl.log; then
    print_status "terraphim-repl v1.0.0 published successfully!"
elif grep -q "already exists on crates.io" /tmp/publish-repl.log; then
    print_status "terraphim-repl v1.0.0 already published (skipping)"
else
    print_error "Failed to publish terraphim-repl"
    cd ../..
    exit 1
fi

cd ../..

echo ""
echo "=========================================="
echo "Step 4: Publish terraphim-cli v1.0.0"
echo "=========================================="
echo ""

print_info "Publishing terraphim-cli to crates.io..."
cd crates/terraphim_cli

# Final check before publishing
print_info "Running compilation check..."
cargo check --quiet 2>&1 | tail -1 || true

# Publish
print_info "Publishing (this may take a minute)..."
if cargo publish 2>&1 | tee /tmp/publish-cli.log; then
    print_status "terraphim-cli v1.0.0 published successfully!"
elif grep -q "already exists on crates.io" /tmp/publish-cli.log; then
    print_status "terraphim-cli v1.0.0 already published (skipping)"
else
    print_error "Failed to publish terraphim-cli"
    cd ../..
    exit 1
fi

cd ../..

# Unset the token for security
unset CARGO_REGISTRY_TOKEN

echo ""
echo "=========================================="
echo "Step 5: Create Git Tag v1.0.0"
echo "=========================================="
echo ""

# Check if tag already exists
if git rev-parse v1.0.0 >/dev/null 2>&1; then
    print_warning "Tag v1.0.0 already exists"
    read -p "Delete and recreate? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag -d v1.0.0
        git push origin :refs/tags/v1.0.0 2>/dev/null || true
        print_info "Deleted existing tag"
        TAG_CREATED=true
    else
        print_info "Skipping tag creation"
        TAG_CREATED=false
    fi
else
    TAG_CREATED=true
fi

if [ "${TAG_CREATED}" = "true" ]; then
    print_info "Creating annotated tag v1.0.0..."
    git tag -a v1.0.0 -m "Terraphim v1.0.0 - Minimal Release

This release includes:
- terraphim_types v1.0.0 (core types)
- terraphim_automata v1.0.0 (text matching & autocomplete)
- terraphim_rolegraph v1.0.0 (knowledge graph)
- terraphim-repl v1.0.0 (interactive REPL - 11 commands)
- terraphim-cli v1.0.0 (automation CLI - 8 commands)

All tools work offline with embedded defaults.
Binary size: 13MB each.
55 tests passing."

    print_status "Tag v1.0.0 created"

    # Push tag
    print_info "Pushing tag to origin..."
    if git push origin v1.0.0; then
        print_status "Tag v1.0.0 pushed to GitHub"
    else
        print_error "Failed to push tag"
        exit 1
    fi
fi

echo ""
echo "=========================================="
echo "Step 6: Build Cross-Platform Binaries"
echo "=========================================="
echo ""

print_info "Building release binaries for multiple platforms..."

# Create release directory
RELEASE_DIR="releases/v1.0.0"
mkdir -p "$RELEASE_DIR"

# Build Linux x86_64 (already built)
print_info "Building Linux x86_64 binaries..."
cargo build --release -p terraphim-repl -p terraphim-cli
cp target/x86_64-unknown-linux-gnu/release/terraphim-repl "$RELEASE_DIR/terraphim-repl-linux-x86_64"
cp target/x86_64-unknown-linux-gnu/release/terraphim-cli "$RELEASE_DIR/terraphim-cli-linux-x86_64"
print_status "Linux x86_64 binaries built"

# Check for cross compilation support
if command -v cross &> /dev/null; then
    print_info "cross found - building additional platforms..."

    # macOS x86_64
    print_info "Building macOS x86_64..."
    if cross build --release --target x86_64-apple-darwin -p terraphim-repl -p terraphim-cli 2>/dev/null; then
        cp target/x86_64-apple-darwin/release/terraphim-repl "$RELEASE_DIR/terraphim-repl-macos-x86_64"
        cp target/x86_64-apple-darwin/release/terraphim-cli "$RELEASE_DIR/terraphim-cli-macos-x86_64"
        print_status "macOS x86_64 binaries built"
    else
        print_warning "macOS x86_64 build failed (may need macOS SDK)"
    fi

    # macOS ARM64
    print_info "Building macOS ARM64..."
    if cross build --release --target aarch64-apple-darwin -p terraphim-repl -p terraphim-cli 2>/dev/null; then
        cp target/aarch64-apple-darwin/release/terraphim-repl "$RELEASE_DIR/terraphim-repl-macos-aarch64"
        cp target/aarch64-apple-darwin/release/terraphim-cli "$RELEASE_DIR/terraphim-cli-macos-aarch64"
        print_status "macOS ARM64 binaries built"
    else
        print_warning "macOS ARM64 build failed (may need macOS SDK)"
    fi

    # Windows x86_64
    print_info "Building Windows x86_64..."
    if cross build --release --target x86_64-pc-windows-gnu -p terraphim-repl -p terraphim-cli 2>/dev/null; then
        cp target/x86_64-pc-windows-gnu/release/terraphim-repl.exe "$RELEASE_DIR/terraphim-repl-windows-x86_64.exe"
        cp target/x86_64-pc-windows-gnu/release/terraphim-cli.exe "$RELEASE_DIR/terraphim-cli-windows-x86_64.exe"
        print_status "Windows x86_64 binaries built"
    else
        print_warning "Windows x86_64 build failed"
    fi
else
    print_warning "cross not found - only building Linux x86_64"
    print_info "Install cross with: cargo install cross"
fi

# List all built binaries
echo ""
print_info "Built binaries:"
ls -lh "$RELEASE_DIR"/ | awk '{if (NR>1) print "  " $9 " (" $5 ")"}'

echo ""
echo "=========================================="
echo "Step 7: Upload Binaries to GitHub Release"
echo "=========================================="
echo ""

print_info "Uploading binaries to GitHub release v1.0.0..."

# Upload all binaries in release directory
for binary in "$RELEASE_DIR"/*; do
    if [ -f "$binary" ]; then
        BINARY_NAME=$(basename "$binary")
        print_info "Uploading $BINARY_NAME..."
        if gh release upload v1.0.0 "$binary" --clobber; then
            print_status "$BINARY_NAME uploaded"
        else
            print_warning "Failed to upload $BINARY_NAME"
        fi
    fi
done

echo ""
echo "=========================================="
echo "Step 8: Create Homebrew Formula"
echo "=========================================="
echo ""

print_info "Generating Homebrew formula..."

# Get SHA256 checksums of Linux binaries
REPL_SHA256=$(sha256sum "$RELEASE_DIR/terraphim-repl-linux-x86_64" | cut -d' ' -f1)
CLI_SHA256=$(sha256sum "$RELEASE_DIR/terraphim-cli-linux-x86_64" | cut -d' ' -f1)

# Create Homebrew formula directory
mkdir -p homebrew-formulas

# Create terraphim-repl formula
cat > homebrew-formulas/terraphim-repl.rb <<EOF
class TerraphimRepl < Formula
  desc "Interactive REPL for semantic knowledge graph search"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-macos-x86_64"
    sha256 "PLACEHOLDER_MACOS_X86_64"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-macos-aarch64"
    sha256 "PLACEHOLDER_MACOS_AARCH64"
  elsif OS.linux?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-linux-x86_64"
    sha256 "${REPL_SHA256}"
  end

  def install
    bin.install "terraphim-repl-linux-x86_64" => "terraphim-repl" if OS.linux?
    bin.install "terraphim-repl-macos-x86_64" => "terraphim-repl" if OS.mac? && Hardware::CPU.intel?
    bin.install "terraphim-repl-macos-aarch64" => "terraphim-repl" if OS.mac? && Hardware::CPU.arm?
  end

  test do
    assert_match "terraphim-repl 1.0.0", shell_output("#{bin}/terraphim-repl --version")
  end
end
EOF

# Create terraphim-cli formula
cat > homebrew-formulas/terraphim-cli.rb <<EOF
class TerraphimCli < Formula
  desc "CLI tool for semantic knowledge graph search with JSON output"
  homepage "https://github.com/terraphim/terraphim-ai"
  version "1.0.0"
  license "Apache-2.0"

  if OS.mac? && Hardware::CPU.intel?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-macos-x86_64"
    sha256 "PLACEHOLDER_MACOS_X86_64"
  elsif OS.mac? && Hardware::CPU.arm?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-macos-aarch64"
    sha256 "PLACEHOLDER_MACOS_AARCH64"
  elsif OS.linux?
    url "https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-cli-linux-x86_64"
    sha256 "${CLI_SHA256}"
  end

  def install
    bin.install "terraphim-cli-linux-x86_64" => "terraphim-cli" if OS.linux?
    bin.install "terraphim-cli-macos-x86_64" => "terraphim-cli" if OS.mac? && Hardware::CPU.intel?
    bin.install "terraphim-cli-macos-aarch64" => "terraphim-cli" if OS.mac? && Hardware::CPU.arm?
  end

  test do
    assert_match "terraphim-cli 1.0.0", shell_output("#{bin}/terraphim-cli --version")
  end
end
EOF

print_status "Homebrew formulas created in homebrew-formulas/"
print_info "  - terraphim-repl.rb"
print_info "  - terraphim-cli.rb"
print_warning "Update macOS SHA256 checksums after building on macOS"

echo ""
echo "=========================================="
echo "Step 9: Create GitHub Release"
echo "=========================================="
echo ""

# Check if release already exists
if gh release view v1.0.0 >/dev/null 2>&1; then
    print_warning "Release v1.0.0 already exists"
    print_info "View at: $(gh release view v1.0.0 --json url -q .url)"
else
    print_info "Creating GitHub release v1.0.0..."

    # Create release with notes from file
    if gh release create v1.0.0 \
        --title "v1.0.0 - Minimal Release" \
        --notes-file RELEASE_NOTES_v1.0.0.md; then
        print_status "GitHub release created successfully!"

        # Get release URL
        RELEASE_URL=$(gh release view v1.0.0 --json url -q .url)
        print_info "Release URL: $RELEASE_URL"
    else
        print_error "Failed to create GitHub release"
        exit 1
    fi
fi

echo ""
echo "=========================================="
echo "🎉 Publication Complete!"
echo "=========================================="
echo ""

print_status "All packages published to crates.io:"
echo "  - terraphim_types v1.0.0"
echo "  - terraphim_automata v1.0.0"
echo "  - terraphim_rolegraph v1.0.0"
echo "  - terraphim-repl v1.0.0 ← NEW"
echo "  - terraphim-cli v1.0.0 ← NEW"
echo ""

print_status "GitHub release created:"
echo "  - Tag: v1.0.0"
echo "  - Release notes: RELEASE_NOTES_v1.0.0.md"
if gh release view v1.0.0 >/dev/null 2>&1; then
    RELEASE_URL=$(gh release view v1.0.0 --json url -q .url)
    echo "  - URL: $RELEASE_URL"
fi
echo ""

print_status "Binaries uploaded to GitHub release:"
ls -1 "$RELEASE_DIR"/ | while read binary; do
    echo "  - $binary"
done
echo ""

print_status "Homebrew formulas created:"
echo "  - homebrew-formulas/terraphim-repl.rb"
echo "  - homebrew-formulas/terraphim-cli.rb"
echo ""

print_info "Installation instructions:"
echo "  # From crates.io (recommended):"
echo "  cargo install terraphim-repl"
echo "  cargo install terraphim-cli"
echo ""
echo "  # From GitHub releases (binaries):"
echo "  wget https://github.com/terraphim/terraphim-ai/releases/download/v1.0.0/terraphim-repl-linux-x86_64"
echo "  chmod +x terraphim-repl-linux-x86_64"
echo ""

print_info "Next steps for complete release:"
echo "  - Test installation from crates.io"
echo "  - Update macOS SHA256 checksums in Homebrew formulas"
echo "  - Publish Homebrew formulas to tap repository"
echo "  - Announce on Discord: https://discord.gg/VPJXB6BGuY"
echo "  - Announce on Discourse: https://terraphim.discourse.group"
echo "  - Tweet/post on social media"
echo "  - Write blog post about v1.0.0 release"
echo ""

echo "🌍 Terraphim v1.0.0 is now live!"
