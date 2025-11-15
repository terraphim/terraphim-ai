#!/usr/bin/env bash

# Terraphim AI Release Manager
# Master script for managing all release-related tasks

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${PURPLE}"
    echo "███████╗███████╗███████╗████████╗██╗   ██╗██████╗"
    echo "██╔════╝██╔════╝██╔════╝╚══██╔══╝╚██╗ ██╔╝██╔══██╗"
    echo "███████╗█████╗  ███████╗   ██║    ╚████╔╝ ██████╔╝"
    echo "╚════██║██╔══╝  ╚════██║   ██║     ╚██╔╝  ██╔══██╗"
    echo "███████║███████╗███████║   ██║      ██║   ██████╔╝"
    echo "╚══════╝╚══════╝╚══════╝   ╚═╝      ╚═╝   ╚═════╝"
    echo -e "${CYAN}Terraphim AI Release Manager${NC}"
    echo -e "${BLUE}=======================================${NC}"
}

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

print_menu() {
    echo -e "${CYAN}Select an action:${NC}"
    echo "1) 🚀 Create New Release"
    echo "2) ✅ Validate Existing Release"
    echo "3) 📝 Generate Changelog"
    echo "4) 🐳 Build Docker Images"
    echo "5) 📦 Build Packages Only"
    echo "6) 🏷️  Create Git Tag"
    echo "7) 📢 Create GitHub Release"
    echo "8) 🧹 Cleanup Build Artifacts"
    echo "9) 📊 Show Release Status"
    echo "10) 🔧 Configuration Setup"
    echo "11) ❓ Help"
    echo "12) 🚪 Exit"
    echo
}

usage() {
    cat << EOF
Terraphim AI Release Manager

Usage: $0 [OPTIONS] [COMMAND] [ARGS]

Commands:
    release VERSION         Create a new release with VERSION
    validate VERSION       Validate release VERSION
    changelog [FROM] [TO]  Generate changelog between tags
    docker                 Build Docker images
    packages VERSION       Build packages for VERSION
    tag VERSION            Create git tag for VERSION
    github-release VERSION Create GitHub release for VERSION
    cleanup                Clean build artifacts
    status                 Show release status
    config                 Show configuration setup
    help                   Show this help message

Options:
    -h, --help            Show this help message
    -i, --interactive     Interactive mode (default)
    -n, --non-interactive Non-interactive mode

Examples:
    $0                           # Interactive mode
    $0 release 0.2.5            # Create release 0.2.5
    $0 validate 0.2.4            # Validate release 0.2.4
    $0 changelog v0.2.3 v0.2.4   # Generate changelog
    $0 docker                    # Build Docker images

EOF
}

# Function to get script directory
get_script_dir() {
    cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

# Function to check dependencies
check_dependencies() {
    local missing_deps=()
    local optional_deps=()

    # Required dependencies
    local required=("git" "cargo")
    for dep in "${required[@]}"; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            missing_deps+=("$dep")
        fi
    done

    # Optional dependencies
    local optional=("gh" "docker" "npm" "cargo-deb" "alien")
    for dep in "${optional[@]}"; do
        if ! command -v "$dep" >/dev/null 2>&1; then
            optional_deps+=("$dep")
        fi
    done

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        print_error "Missing required dependencies: ${missing_deps[*]}"
        print_status "Install missing dependencies and try again"
        return 1
    fi

    if [[ ${#optional_deps[@]} -gt 0 ]]; then
        print_warning "Missing optional dependencies: ${optional_deps[*]}"
        print_status "Some features may not be available without these tools"
    fi

    return 0
}

# Function to show configuration setup
show_config_setup() {
    print_header
    echo -e "${CYAN}Configuration Setup${NC}"
    echo

    print_status "Required setup steps:"
    echo "1. Install GitHub CLI and authenticate:"
    echo "   gh auth login"
    echo
    echo "2. Configure git user information:"
    echo "   git config --global user.name 'Your Name'"
    echo "   git config --global user.email 'your.email@example.com'"
    echo
    echo "3. Install optional tools:"
    echo "   - cargo-deb: cargo install cargo-deb"
    echo "   - Docker: https://docs.docker.com/get-docker/"
    echo "   - alien (for RPM): sudo apt-get install alien"
    echo

    print_status "Environment variables (optional):"
    echo "- GITHUB_TOKEN: For GitHub API access"
    echo "- DOCKER_REGISTRY: For custom Docker registry"
    echo

    print_status "Current configuration:"
    echo "- Git user: $(git config user.name || echo 'Not configured')"
    echo "- Git email: $(git config user.email || echo 'Not configured')"
    echo "- GitHub CLI: $(command -v gh >/dev/null 2>&1 && echo 'Installed' || echo 'Not installed')"
    echo "- Docker: $(command -v docker >/dev/null 2>&1 && echo 'Installed' || echo 'Not installed')"
    echo "- cargo-deb: $(cargo deb --help >/dev/null 2>&1 && echo 'Installed' || echo 'Not installed')"
    echo

    read -p "Press Enter to continue..."
}

# Function to show release status
show_release_status() {
    print_header
    echo -e "${CYAN}Release Status${NC}"
    echo

    # Show current git status
    print_status "Git Status:"
    git status --porcelain || print_warning "Not a git repository or git status failed"
    echo

    # Show latest tags
    print_status "Latest Release Tags:"
    git tag --list "v*" --sort=-version:refname | head -5 || print_warning "No release tags found"
    echo

    # Show current version in Cargo.toml
    if [[ -f "Cargo.toml" ]]; then
        local current_version
        current_version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/' || echo "Unknown")
        print_status "Current version in Cargo.toml: $current_version"
    fi
    echo

    # Show release directories
    print_status "Release Directories:"
    for dir in release/*/; do
        if [[ -d "$dir" ]]; then
            local file_count
            file_count=$(find "$dir" -type f | wc -l)
            echo "- $dir ($file_count files)"
        fi
    done
    echo

    # Show Docker images
    if command -v docker >/dev/null 2>&1; then
        print_status "Docker Images:"
        docker images --filter "reference=terraphim-*" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}" 2>/dev/null || print_warning "No terraphim Docker images found"
    else
        print_warning "Docker not available"
    fi
    echo

    read -p "Press Enter to continue..."
}

# Function to cleanup build artifacts
cleanup_artifacts() {
    print_header
    echo -e "${CYAN}Cleanup Build Artifacts${NC}"
    echo

    print_status "Cleaning Rust build artifacts..."
    cargo clean || print_warning "cargo clean failed"

    print_status "Cleaning release directories..."
    for dir in release/*/; do
        if [[ -d "$dir" && "$dir" != "release/v0.2.3/" && "$dir" != "release/v0.2.4/" ]]; then
            print_status "Removing $dir"
            rm -rf "$dir" || print_warning "Failed to remove $dir"
        fi
    done

    print_status "Cleaning temporary files..."
    find . -name "*.bak" -delete 2>/dev/null || true
    find . -name "*.tmp" -delete 2>/dev/null || true
    find . -name ".DS_Store" -delete 2>/dev/null || true

    print_status "Pruning Docker images..."
    if command -v docker >/dev/null 2>&1; then
        docker image prune -f 2>/dev/null || true
    fi

    print_success "Cleanup completed!"
    echo
    read -p "Press Enter to continue..."
}

# Function to build packages only
build_packages() {
    local version="$1"

    print_header
    echo -e "${CYAN}Building Packages for v$version${NC}"
    echo

    local script_dir
    script_dir=$(get_script_dir)

    print_status "Building Debian packages..."
    if cargo deb --package terraphim_server --no-build; then
        print_success "Debian server package built"
    else
        print_error "Failed to build Debian server package"
        return 1
    fi

    if cargo deb --package terraphim_agent --no-build; then
        print_success "Debian TUI package built"
    else
        print_error "Failed to build Debian TUI package"
        return 1
    fi

    print_status "Building Arch Linux packages..."
    if "$script_dir/release.sh" --skip-tests --skip-build --no-docker "$version" 2>/dev/null; then
        print_success "Arch Linux packages built"
    else
        print_error "Failed to build Arch Linux packages"
        return 1
    fi

    print_success "All packages built successfully!"
    echo
    read -p "Press Enter to continue..."
}

# Function to build Docker images
build_docker_images() {
    print_header
    echo -e "${CYAN}Building Docker Images${NC}"
    echo

    local script_dir
    script_dir=$(get_script_dir)

    # Check if Docker is available
    if ! command -v docker >/dev/null 2>&1; then
        print_error "Docker not found. Please install Docker first."
        return 1
    fi

    print_status "Building multi-architecture Docker images..."

    if docker buildx build --builder multiarch-builder --platform linux/amd64,linux/arm64 \
        -f Dockerfile.multiarch \
        -t terraphim-server:latest \
        --push . 2>/dev/null; then
        print_success "Multi-architecture Docker images built and pushed"
    else
        print_warning "Multi-architecture build failed, trying single architecture..."

        if docker build -f Dockerfile.multiarch -t terraphim-server:latest .; then
            print_success "Single architecture Docker image built"
        else
            print_error "Failed to build Docker images"
            return 1
        fi
    fi

    print_success "Docker build completed!"
    echo
    read -p "Press Enter to continue..."
}

# Function to create git tag only
create_git_tag() {
    local version="$1"
    local tag="v$version"

    print_header
    echo -e "${CYAN}Creating Git Tag: $tag${NC}"
    echo

    # Check if tag already exists
    if git rev-parse "$tag" >/dev/null 2>&1; then
        print_error "Tag $tag already exists"
        return 1
    fi

    print_status "Creating tag $tag..."
    if git tag -a "$tag" -m "Release v$version"; then
        print_success "Tag $tag created successfully"
    else
        print_error "Failed to create tag $tag"
        return 1
    fi

    read -p "Push tag to remote? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Pushing tag to origin..."
        if git push origin "$tag"; then
            print_success "Tag pushed to origin"
        else
            print_error "Failed to push tag"
            return 1
        fi
    fi

    echo
    read -p "Press Enter to continue..."
}

# Function to create GitHub release only
create_github_release() {
    local version="$1"
    local tag="v$version"

    print_header
    echo -e "${CYAN}Creating GitHub Release: $tag${NC}"
    echo

    if ! command -v gh >/dev/null 2>&1; then
        print_error "GitHub CLI (gh) not found. Please install it first."
        return 1
    fi

    # Check if release directory exists
    local release_dir="release/$version"
    if [[ ! -d "$release_dir" ]]; then
        print_error "Release directory not found: $release_dir"
        print_status "Please build packages first using option 5"
        return 1
    fi

    print_status "Creating GitHub release..."
    local release_notes="Terraphim AI v$version

## Installation
\`\`\`bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$version/install.sh | bash
\`\`\`

## Download
- Download packages from: https://github.com/terraphim/terraphim-ai/releases/tag/$TAG
"

    if gh release create "$tag" \
        --title "Terraphim AI v$version" \
        --notes "$release_notes" \
        "$release_dir"/* 2>/dev/null; then
        print_success "GitHub release created successfully"
        print_status "Release URL: https://github.com/terraphim/terraphim-ai/releases/tag/$tag"
    else
        print_error "Failed to create GitHub release"
        return 1
    fi

    echo
    read -p "Press Enter to continue..."
}

# Function to generate changelog
generate_changelog_interactive() {
    print_header
    echo -e "${CYAN}Generate Changelog${NC}"
    echo

    local script_dir
    script_dir=$(get_script_dir)

    # Show recent tags
    print_status "Recent release tags:"
    local tags
    tags=$(git tag --list "v*" --sort=-version:refname | head -5)
    echo "$tags" | nl -nln
    echo

    read -p "Enter from tag (or leave empty for auto-detect): " from_tag
    read -p "Enter to tag (or leave empty for HEAD): " to_tag

    if [[ -z "$from_tag" ]]; then
        from_tag=$(echo "$tags" | head -1)
        print_status "Using from tag: $from_tag"
    fi

    if [[ -z "$to_tag" ]]; then
        to_tag="HEAD"
        print_status "Using to tag: HEAD (unreleased changes)"
    fi

    print_status "Generating changelog from $from_tag to $to_tag..."

    if [[ -f "$script_dir/changelog.sh" ]]; then
        local changelog
        changelog=$("$script_dir/changelog.sh" "$from_tag" "$to_tag")

        echo
        echo -e "${CYAN}Generated Changelog:${NC}"
        echo "$changelog"
        echo

        read -p "Save to CHANGELOG.md? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "$changelog" >> CHANGELOG.md
            print_success "Changelog appended to CHANGELOG.md"
        fi
    else
        print_error "Changelog script not found"
        return 1
    fi

    echo
    read -p "Press Enter to continue..."
}

# Function to validate release
validate_release_interactive() {
    print_header
    echo -e "${CYAN}Validate Release${NC}"
    echo

    read -p "Enter version to validate: " version

    if [[ -z "$version" ]]; then
        print_error "Version is required"
        return 1
    fi

    local script_dir
    script_dir=$(get_script_dir)

    print_status "Validating release v$version..."

    if [[ -f "$script_dir/validate-release.sh" ]]; then
        if "$script_dir/validate-release.sh" --quick "$version"; then
            print_success "Release validation passed!"
        else
            print_error "Release validation failed"
            return 1
        fi
    else
        print_error "Validation script not found"
        return 1
    fi

    echo
    read -p "Press Enter to continue..."
}

# Function to create new release
create_release_interactive() {
    print_header
    echo -e "${CYAN}Create New Release${NC}"
    echo

    read -p "Enter version (e.g., 0.2.5): " version

    if [[ -z "$version" ]]; then
        print_error "Version is required"
        return 1
    fi

    # Validate version format
    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
        print_error "Invalid version format. Use X.Y.Z or X.Y.Z-prerelease"
        return 1
    fi

    local script_dir
    script_dir=$(get_script_dir)

    print_status "Creating release v$version..."

    if [[ -f "$script_dir/release.sh" ]]; then
        read -p "Run in dry-run mode first? (Y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            print_status "Running dry-run..."
            "$script_dir/release.sh" --dry-run "$version"
            echo
            read -p "Continue with actual release? (y/N): " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                print_status "Release cancelled"
                return 0
            fi
        fi

        if "$script_dir/release.sh" "$version"; then
            print_success "Release v$version completed successfully!"
        else
            print_error "Release failed"
            return 1
        fi
    else
        print_error "Release script not found"
        return 1
    fi

    echo
    read -p "Press Enter to continue..."
}

# Function to show help
show_help() {
    print_header
    echo -e "${CYAN}Release Manager Help${NC}"
    echo

    usage
    echo

    print_status "Available Scripts:"
    local script_dir
    script_dir=$(get_script_dir)
    echo "- release.sh: Main release automation script"
    echo "- validate-release.sh: Release validation script"
    echo "- changelog.sh: Changelog generation script"
    echo "- build-macos-bundles.sh: macOS package building script"
    echo

    print_status "Common Workflows:"
    echo "1. New Release: Use option 1 or run: $0 release VERSION"
    echo "2. Validate Release: Use option 2 or run: $0 validate VERSION"
    echo "3. Generate Changelog: Use option 3 or run: $0 changelog"
    echo "4. Quick Package Build: Use option 5 or run: $0 packages VERSION"
    echo

    read -p "Press Enter to continue..."
}

# Interactive mode
interactive_mode() {
    while true; do
        print_header
        print_menu

        read -p "Enter your choice (1-12): " -n 1 -r
        echo
        echo

        case $REPLY in
            1) create_release_interactive ;;
            2) validate_release_interactive ;;
            3) generate_changelog_interactive ;;
            4) build_docker_images ;;
            5)
                read -p "Enter version: " version
                build_packages "$version"
                ;;
            6)
                read -p "Enter version: " version
                create_git_tag "$version"
                ;;
            7)
                read -p "Enter version: " version
                create_github_release "$version"
                ;;
            8) cleanup_artifacts ;;
            9) show_release_status ;;
            10) show_config_setup ;;
            11) show_help ;;
            12)
                print_status "Goodbye!"
                exit 0
                ;;
            *)
                print_error "Invalid choice. Please enter 1-12."
                echo
                read -p "Press Enter to continue..."
                ;;
        esac
    done
}

# Command line mode
command_mode() {
    local command="${1:-}"
    shift || true

    case "$command" in
        "release")
            if [[ $# -eq 0 ]]; then
                print_error "Version parameter required for release command"
                usage
                exit 1
            fi
            local script_dir
            script_dir=$(get_script_dir)
            "$script_dir/release.sh" "$@"
            ;;
        "validate")
            if [[ $# -eq 0 ]]; then
                print_error "Version parameter required for validate command"
                usage
                exit 1
            fi
            local script_dir
            script_dir=$(get_script_dir)
            "$script_dir/validate-release.sh" "$@"
            ;;
        "changelog")
            local script_dir
            script_dir=$(get_script_dir)
            "$script_dir/changelog.sh" "$@"
            ;;
        "docker")
            build_docker_images
            ;;
        "packages")
            if [[ $# -eq 0 ]]; then
                print_error "Version parameter required for packages command"
                usage
                exit 1
            fi
            build_packages "$1"
            ;;
        "tag")
            if [[ $# -eq 0 ]]; then
                print_error "Version parameter required for tag command"
                usage
                exit 1
            fi
            create_git_tag "$1"
            ;;
        "github-release")
            if [[ $# -eq 0 ]]; then
                print_error "Version parameter required for github-release command"
                usage
                exit 1
            fi
            create_github_release "$1"
            ;;
        "cleanup")
            cleanup_artifacts
            ;;
        "status")
            show_release_status
            ;;
        "config")
            show_config_setup
            ;;
        "help")
            usage
            ;;
        *)
            print_error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

# Main execution
main() {
    # Default to interactive mode
    local interactive=true

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -i|--interactive)
                interactive=true
                shift
                ;;
            -n|--non-interactive)
                interactive=false
                shift
                ;;
            -*)
                print_error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                # Command detected, switch to command mode
                interactive=false
                command_mode "$@"
                exit $?
                ;;
        esac
    done

    # Check dependencies
    if ! check_dependencies; then
        exit 1
    fi

    # Run interactive mode if no command provided
    if [[ "$interactive" == "true" ]]; then
        interactive_mode
    fi
}

# Run main function
main "$@"
