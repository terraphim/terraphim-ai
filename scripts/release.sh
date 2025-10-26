#!/usr/bin/env bash

# Terraphim AI Release Script
# Automates the complete release process including version bumping, package creation, and GitHub release

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to display usage
usage() {
    cat << EOF
Terraphim AI Release Script

Usage: $0 [OPTIONS] <VERSION>

Arguments:
    VERSION    Version number (e.g., 0.2.5)

Options:
    -h, --help           Show this help message
    -n, --dry-run        Show what would be done without executing
    -s, --skip-tests     Skip running tests before release
    -b, --skip-build     Skip building packages (use existing)
    -p, --push           Push changes to remote repository
    -r, --remote REMOTE  Remote repository name (default: origin)
    --no-docker          Skip Docker image creation
    --windows            Include Windows installer (experimental)
    --beta               Mark release as beta/prerelease

Examples:
    $0 0.2.5                    # Basic release
    $0 --dry-run 0.2.5         # Preview what would be done
    $0 --push --skip-tests 0.2.5  # Release without tests and push
    $0 --beta 0.3.0-beta.1      # Beta release

Requirements:
    - GitHub CLI (gh)
    - cargo-deb (for Debian packages)
    - docker (with buildx)
    - Rust toolchain
    - Proper git configuration

EOF
}

# Default options
DRY_RUN=false
SKIP_TESTS=false
SKIP_BUILD=false
PUSH_CHANGES=false
REMOTE="origin"
SKIP_DOCKER=false
INCLUDE_WINDOWS=false
BETA_RELEASE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -n|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -s|--skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        -b|--skip-build)
            SKIP_BUILD=true
            shift
            ;;
        -p|--push)
            PUSH_CHANGES=true
            shift
            ;;
        -r|--remote)
            REMOTE="$2"
            shift 2
            ;;
        --no-docker)
            SKIP_DOCKER=true
            shift
            ;;
        --windows)
            INCLUDE_WINDOWS=true
            shift
            ;;
        --beta)
            BETA_RELEASE=true
            shift
            ;;
        -*)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
        *)
            VERSION="$1"
            shift
            ;;
    esac
done

# Validate version parameter
if [[ -z "${VERSION:-}" ]]; then
    print_error "Version parameter is required"
    usage
    exit 1
fi

# Validate version format
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    print_error "Invalid version format: $VERSION (expected: X.Y.Z or X.Y.Z-prerelease)"
    exit 1
fi

# Set release tag
TAG="v$VERSION"
RELEASE_DIR="release/$VERSION"

print_status "Starting Terraphim AI release process for version $VERSION"
print_status "Release tag: $TAG"
print_status "Release directory: $RELEASE_DIR"

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    print_error "Not in a git repository"
    exit 1
fi

# Check if git working directory is clean
if [[ -n $(git status --porcelain) ]]; then
    print_error "Working directory is not clean. Please commit or stash changes first."
    exit 1
fi

# Check required tools
check_requirements() {
    local tools=("git" "cargo" "gh" "docker")
    local missing_tools=()

    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            missing_tools+=("$tool")
        fi
    done

    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        print_error "Missing required tools: ${missing_tools[*]}"
        print_error "Please install the missing tools and try again"
        exit 1
    fi

    # Check for cargo-deb
    if ! cargo deb --help &> /dev/null; then
        print_warning "cargo-deb not found. Install with: cargo install cargo-deb"
    fi
}

# Function to execute command or show what would be executed
execute() {
    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "[DRY-RUN] Would execute: $*"
        return 0
    else
        print_status "Executing: $*"
        "$@"
    fi
}

# Function to update version numbers
update_versions() {
    print_status "Updating version numbers to $VERSION"

    # Update Cargo.toml files
    find . -name "Cargo.toml" -type f -not -path "./target/*" | while read -r cargo_toml; do
        if grep -q '^version = ' "$cargo_toml"; then
            execute sed -i.bak "s/^version = .*/version = \"$VERSION\"/" "$cargo_toml"
            if [[ "$DRY_RUN" == "false" ]]; then
                rm -f "${cargo_toml}.bak"
            fi
        fi
    done

    # Update version in package.json if exists
    if [[ -f "desktop/package.json" ]]; then
        execute npm version "$VERSION" --no-git-tag-version --prefix desktop
    fi

    # Update rust-toolchain.toml if exists
    if [[ -f "rust-toolchain.toml" ]]; then
        # Note: This assumes you want to update the channel in sync with releases
        # Adjust as needed for your versioning strategy
        print_status "Checking rust-toolchain.toml for version updates"
    fi
}

# Function to run tests
run_tests() {
    if [[ "$SKIP_TESTS" == "true" ]]; then
        print_warning "Skipping tests as requested"
        return 0
    fi

    print_status "Running tests"

    # Rust tests
    execute cargo test --workspace

    # Frontend tests if desktop directory exists
    if [[ -d "desktop" ]]; then
        execute npm test --prefix desktop
    fi
}

# Function to build release binaries
build_binaries() {
    if [[ "$SKIP_BUILD" == "true" ]]; then
        print_warning "Skipping binary build as requested"
        return 0
    fi

    print_status "Building release binaries"

    # Clean previous builds
    execute cargo clean

    # Build server
    execute cargo build --release --package terraphim_server

    # Build TUI
    execute cargo build --release --package terraphim_tui --features repl-full
}

# Function to create Debian packages
create_deb_packages() {
    print_status "Creating Debian packages"

    # Create server package
    execute cargo deb --package terraphim_server --no-build

    # Create TUI package
    execute cargo deb --package terraphim_tui --no-build
}

# Function to create Arch Linux packages
create_arch_packages() {
    print_status "Creating Arch Linux packages"

    # Create release directory
    execute mkdir -p "$RELEASE_DIR"

    # Copy built binaries
    if [[ -f "target/release/terraphim_server" ]]; then
        execute cp target/release/terraphim_server "$RELEASE_DIR/"
    fi

    if [[ -f "target/release/terraphim_tui" ]]; then
        execute cp target/release/terraphim_tui "$RELEASE_DIR/"
    fi

    # Create PKGBUILD for server
    cat > "${RELEASE_DIR}/PKGBUILD-server" << EOF
pkgname=terraphim-server
pkgver=$VERSION
pkgrel=1
pkgdesc="Terraphim AI Server - Privacy-first AI assistant"
arch=('x86_64')
url="https://github.com/terraphim/terraphim-ai"
license=('Apache-2.0')
depends=('openssl')
conflicts=('terraphim-server-bin')
provides=('terraphim-server')

package() {
    install -Dm755 "\$srcdir/terraphim_server" "\$pkgdir/usr/bin/terraphim_server"
}
EOF

    # Create PKGBUILD for TUI
    cat > "${RELEASE_DIR}/PKGBUILD-tui" << EOF
pkgname=terraphim-tui
pkgver=$VERSION
pkgrel=1
pkgdesc="Terraphim AI TUI - Terminal User Interface"
arch=('x86_64')
url="https://github.com/terraphim/terraphim-ai"
license=('Apache-2.0')
depends=('openssl')
conflicts=('terraphim-tui-bin')
provides=('terraphim-tui')

package() {
    install -Dm755 "\$srcdir/terraphim_tui" "\$pkgdir/usr/bin/terraphim_tui"
}
EOF

    # Build packages (simplified approach)
    if [[ "$DRY_RUN" == "false" ]]; then
        cd "$RELEASE_DIR"

        # Build server package
        if [[ -f "terraphim_server" ]]; then
            tar -cJf "terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst" \
                --owner=root --group=root terraphim_server PKGBUILD-server
        fi

        # Build TUI package
        if [[ -f "terraphim_tui" ]]; then
            tar -cJf "terraphim-tui-${VERSION}-1-x86_64.pkg.tar.zst" \
                --owner=root --group=root terraphim_tui PKGBUILD-tui
        fi

        cd - > /dev/null
    fi
}

# Function to create RPM packages
create_rpm_packages() {
    print_status "Creating RPM packages (using alien)"

    if command -v alien &> /dev/null; then
        # Convert Debian packages to RPM
        for deb_file in target/debian/*.deb; do
            if [[ -f "$deb_file" ]]; then
                execute alien -r -c "$deb_file"
            fi
        done

        # Move RPM files to release directory
        execute mkdir -p "$RELEASE_DIR"
        execute mv *.rpm "$RELEASE_DIR/" 2>/dev/null || true
    else
        print_warning "alien not found, skipping RPM package creation"
        print_warning "Install with: sudo apt-get install alien"
    fi
}

# Function to create macOS packages
create_macos_packages() {
    print_status "Creating macOS packages"

    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_warning "Not on macOS, skipping macOS package creation"
        return 0
    fi

    # Run macOS build script if it exists
    if [[ -f "scripts/build-macos-bundles.sh" ]]; then
        execute scripts/build-macos-bundles.sh
    else
        print_warning "macOS build script not found, skipping"
    fi
}

# Function to build Docker images
build_docker_images() {
    if [[ "$SKIP_DOCKER" == "true" ]]; then
        print_warning "Skipping Docker image creation as requested"
        return 0
    fi

    print_status "Building Docker images"

    # Check if buildx is available
    if ! docker buildx version &> /dev/null; then
        print_warning "docker buildx not available, using regular docker build"
        execute docker build -t "terraphim-server:$TAG" .
    else
        # Use multi-architecture build
        execute docker buildx build --platform linux/amd64,linux/arm64 \
            -t "terraphim-server:$TAG" \
            -t "terraphim-server:latest" \
            --push .
    fi
}

# Function to create GitHub release
create_github_release() {
    print_status "Creating GitHub release"

    # Prepare release notes
    local release_notes
    if [[ "$BETA_RELEASE" == "true" ]]; then
        release_notes="Terraphim AI v$VERSION (Beta Release)

This is a pre-release version of Terraphim AI. Please report any issues you encounter.

## Installation
\`\`\`bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/install.sh | bash
\`\`\`

## Changes
- [Add your changelog here]
"
    else
        release_notes="Terraphim AI v$VERSION

## Installation
\`\`\`bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/install.sh | bash
\`\`\`

## Changes
- [Add your changelog here]
"
    fi

    # Create release
    local prerelease_flag=""
    if [[ "$BETA_RELEASE" == "true" ]]; then
        prerelease_flag="--prerelease"
    fi

    execute gh release create "$TAG" \
        --title "Terraphim AI v$VERSION" \
        --notes "$release_notes" \
        $prerelease_flag \
        "$RELEASE_DIR"/*.{deb,tar.zst,rpm,tar.gz} 2>/dev/null || {
        print_warning "Some package files may not exist, creating release without them"
        execute gh release create "$TAG" \
            --title "Terraphim AI v$VERSION" \
            --notes "$release_notes" \
            $prerelease_flag
    }
}

# Function to push changes
push_changes() {
    if [[ "$PUSH_CHANGES" != "true" ]]; then
        print_warning "Skipping git push as requested"
        return 0
    fi

    print_status "Pushing changes to remote repository"

    # Push commits
    execute git push "$REMOTE" "$(git branch --show-current)"

    # Push tag
    execute git push "$REMOTE" "$TAG"
}

# Function to commit version changes
commit_changes() {
    print_status "Committing version changes"

    execute git add .
    execute git commit -m "chore: release v$VERSION"
}

# Function to create git tag
create_tag() {
    print_status "Creating git tag: $TAG"

    execute git tag -a "$TAG" -m "Release v$VERSION"
}

# Function to create release directory structure
create_release_structure() {
    print_status "Creating release directory structure"

    execute mkdir -p "$RELEASE_DIR"

    # Copy installation scripts
    if [[ -f "release/install.sh" ]]; then
        execute cp release/install.sh "$RELEASE_DIR/"
    fi

    if [[ -f "release/docker-run.sh" ]]; then
        execute cp release/docker-run.sh "$RELEASE_DIR/"
    fi

    # Copy README for release
    cat > "$RELEASE_DIR/README.md" << EOF
# Terraphim AI v$VERSION Release

This release contains packages for installing Terraphim AI on various platforms.

## Installation Options

### Quick Install (Recommended)
\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/install.sh | bash
\`\`\`

### Package Manager Installation

#### Debian/Ubuntu (.deb packages)
\`\`\`bash
wget https://github.com/terraphim/terraphim-ai/releases/download/$TAG/terraphim-server_$VERSION-1_amd64.deb
sudo dpkg -i terraphim-server_$VERSION-1_amd64.deb

wget https://github.com/terraphim/terraphim-ai/releases/download/$TAG/terraphim-tui_$VERSION-1_amd64.deb
sudo dpkg -i terraphim-tui_$VERSION-1_amd64.deb
\`\`\`

#### Arch Linux (.tar.zst packages)
\`\`\`bash
wget https://github.com/terraphim/terraphim-ai/releases/download/$TAG/terraphim-server-$VERSION-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-server-$VERSION-1-x86_64.pkg.tar.zst
\`\`\`

#### RHEL/CentOS/Fedora (.rpm packages)
\`\`\`bash
wget https://github.com/terraphim/terraphim-ai/releases/download/$TAG/terraphim-server-$VERSION-1.x86_64.rpm
sudo yum localinstall terraphim-server-$VERSION-1.x86_64.rpm
\`\`\`

#### macOS (.app bundles)
\`\`\`bash
wget https://github.com/terraphim/terraphim-ai/releases/download/$TAG/TerraphimServer-$VERSION-macos.tar.gz
tar -xzf TerraphimServer-$VERSION-macos.tar.gz
cp -r TerraphimServer.app /Applications/
\`\`\`

### Docker Installation
\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/docker-run.sh | bash
\`\`\`

## Verification

After installation, you can verify the installation:

\`\`\`bash
terraphim_server --version
terraphim_tui --version
\`\`\`

## Support

- Documentation: https://github.com/terraphim/terraphim-ai/blob/main/docs/
- Issues: https://github.com/terraphim/terraphim-ai/issues
- Discussions: https://github.com/terraphim/terraphim-ai/discussions
EOF
}

# Main execution flow
main() {
    check_requirements

    if [[ "$DRY_RUN" == "true" ]]; then
        print_status "Running in DRY-RUN mode - no changes will be made"
    fi

    # Check if tag already exists
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        print_error "Tag $TAG already exists"
        exit 1
    fi

    # Create release directory structure
    create_release_structure

    # Update versions
    update_versions

    # Run tests
    run_tests

    # Build binaries
    build_binaries

    # Create packages
    create_deb_packages
    create_arch_packages
    create_rpm_packages
    create_macos_packages

    # Build Docker images
    build_docker_images

    # Commit changes
    commit_changes

    # Create tag
    create_tag

    # Create GitHub release
    create_github_release

    # Push changes
    push_changes

    print_success "Release v$VERSION completed successfully!"
    print_status "GitHub release: https://github.com/terraphim/terraphim-ai/releases/tag/$TAG"
}

# Run main function
main "$@"
