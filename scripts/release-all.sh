#!/bin/bash
# release-all.sh - Comprehensive release script for Terraphim AI with 1Password integration

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_header() {
    echo -e "${BOLD}${BLUE}🚀 $1${NC}"
}

usage() {
    echo "Usage: $0 <version> [options]"
    echo ""
    echo "Arguments:"
    echo "  version         Release version (e.g., 0.3.0, 1.0.0-beta.1)"
    echo ""
    echo "Options:"
    echo "  --dry-run       Show what would be done without making changes"
    echo "  --skip-build    Skip building artifacts (useful for testing)"
    echo "  --skip-tests    Skip running tests"
    echo "  --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 0.3.0"
    echo "  $0 1.0.0-beta.1 --dry-run"
    echo "  $0 0.2.1 --skip-tests"
    exit 0
}

validate_prerequisites() {
    print_info "Validating prerequisites..."

    # Check 1Password CLI
    if ! command -v op &> /dev/null; then
        print_error "1Password CLI not found. Install with: brew install --cask 1password-cli"
        exit 1
    fi

    if ! op whoami &> /dev/null; then
        print_error "Not authenticated with 1Password. Run: op signin"
        exit 1
    fi

    # Check required tools
    local required_tools=("git" "cargo" "node" "yarn" "jq")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            print_error "$tool not found. Please install it first."
            exit 1
        fi
    done

    # Verify 1Password vault access
    if ! op vault get "Terraphim-Deployment" &> /dev/null; then
        print_error "Cannot access 1Password vault 'Terraphim-Deployment'"
        print_info "Run: ./scripts/setup-1password-secrets.sh"
        exit 1
    fi

    # Check git status
    if [[ -n $(git status --porcelain) ]]; then
        print_error "Working directory is not clean. Please commit or stash changes."
        exit 1
    fi

    print_success "Prerequisites validated"
}

validate_version() {
    local version="$1"

    if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
        print_error "Invalid version format: $version"
        print_info "Expected format: X.Y.Z or X.Y.Z-suffix (e.g., 1.0.0, 1.0.0-beta.1)"
        exit 1
    fi

    # Check if tag already exists
    if git tag -l | grep -q "^v$version$"; then
        print_error "Tag v$version already exists"
        exit 1
    fi

    print_success "Version $version is valid and available"
}

update_version_files() {
    local version="$1"
    local dry_run="$2"

    print_info "Updating version files to $version..."

    local files_to_update=(
        "Cargo.toml:version = \".*\""
        "desktop/src-tauri/tauri.conf.json:\"version\": \".*\""
        "desktop/package.json:\"version\": \".*\""
        "browser_extensions/TerraphimAIParseExtension/manifest.json:\"version\": \".*\""
        "browser_extensions/TerraphimAIContext/manifest.json:\"version\": \".*\""
    )

    for file_pattern in "${files_to_update[@]}"; do
        IFS=':' read -r file pattern <<< "$file_pattern"
        local full_path="$PROJECT_ROOT/$file"

        if [[ -f "$full_path" ]]; then
            if [[ "$dry_run" == "true" ]]; then
                print_info "[DRY RUN] Would update $file"
            else
                if [[ "$file" == *.json ]]; then
                    # Use jq for JSON files
                    jq ".version = \"$version\"" "$full_path" > "$full_path.tmp"
                    mv "$full_path.tmp" "$full_path"
                else
                    # Use sed for other files
                    if [[ "$OSTYPE" == "darwin"* ]]; then
                        sed -i '' "s/$pattern/version = \"$version\"/" "$full_path"
                    else
                        sed -i "s/$pattern/version = \"$version\"/" "$full_path"
                    fi
                fi
                print_success "Updated $file"
            fi
        else
            print_warning "File not found: $file"
        fi
    done
}

run_tests() {
    local skip_tests="$1"

    if [[ "$skip_tests" == "true" ]]; then
        print_warning "Skipping tests as requested"
        return 0
    fi

    print_header "Running Tests"

    print_info "Running Rust tests..."
    if ! cargo test --workspace; then
        print_error "Rust tests failed"
        exit 1
    fi

    print_info "Running frontend tests..."
    cd "$PROJECT_ROOT/desktop"
    if ! yarn test; then
        print_error "Frontend tests failed"
        exit 1
    fi
    cd "$PROJECT_ROOT"

    print_success "All tests passed"
}

build_artifacts() {
    local skip_build="$1"
    local dry_run="$2"

    if [[ "$skip_build" == "true" ]]; then
        print_warning "Skipping build as requested"
        return 0
    fi

    print_header "Building Release Artifacts"

    if [[ "$dry_run" == "true" ]]; then
        print_info "[DRY RUN] Would build all release artifacts"
        return 0
    fi

    # Build Rust binaries
    print_info "Building Rust binaries..."
    if ! cargo build --release --workspace; then
        print_error "Rust build failed"
        exit 1
    fi

    # Build desktop app with 1Password signing
    print_info "Building desktop application with signing..."
    if ! "$SCRIPT_DIR/build-with-signing.sh"; then
        print_error "Desktop build failed"
        exit 1
    fi

    # Package browser extensions
    print_info "Packaging browser extensions..."
    if [[ -f "$SCRIPT_DIR/package-browser-extensions.sh" ]]; then
        if ! "$SCRIPT_DIR/package-browser-extensions.sh"; then
            print_error "Browser extension packaging failed"
            exit 1
        fi
    else
        print_warning "Browser extension packaging script not found"
    fi

    print_success "All artifacts built successfully"
}

create_release_commit() {
    local version="$1"
    local dry_run="$2"

    print_info "Creating release commit..."

    if [[ "$dry_run" == "true" ]]; then
        print_info "[DRY RUN] Would create commit with message: 'chore: release v$version'"
        print_info "[DRY RUN] Would create tag: v$version"
        print_info "[DRY RUN] Would push to origin"
        return 0
    fi

    # Add all version file changes
    git add \
        Cargo.toml \
        Cargo.lock \
        "desktop/src-tauri/tauri.conf.json" \
        "desktop/package.json" \
        "browser_extensions/*/manifest.json" \
        2>/dev/null || true

    # Create commit
    git commit -m "chore: release v$version"

    # Create annotated tag
    git tag -a "v$version" -m "Release v$version

This release includes:
- Desktop application with auto-update support
- CLI tools with self-update capability
- Browser extensions
- Comprehensive security via 1Password integration

See CHANGELOG.md for detailed changes."

    print_success "Created release commit and tag"
}

push_release() {
    local version="$1"
    local dry_run="$2"

    print_info "Pushing release to GitHub..."

    if [[ "$dry_run" == "true" ]]; then
        print_info "[DRY RUN] Would push main branch and tag v$version"
        return 0
    fi

    # Push main branch and tag
    git push origin main
    git push origin "v$version"

    print_success "Release pushed to GitHub"
    print_info "GitHub Actions will now build and publish the release"
}

generate_changelog_entry() {
    local version="$1"

    print_info "Generating changelog entry..."

    # Get commits since last tag
    local last_tag
    last_tag=$(git describe --tags --abbrev=0 HEAD~1 2>/dev/null || echo "")

    if [[ -n "$last_tag" ]]; then
        print_info "Changes since $last_tag:"
        git log --oneline --no-merges "$last_tag..HEAD" | head -20
    else
        print_info "First release - showing recent commits:"
        git log --oneline --no-merges HEAD | head -10
    fi

    echo ""
    print_info "Consider updating CHANGELOG.md with these changes"
}

main() {
    local version=""
    local dry_run="false"
    local skip_build="false"
    local skip_tests="false"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                dry_run="true"
                shift
                ;;
            --skip-build)
                skip_build="true"
                shift
                ;;
            --skip-tests)
                skip_tests="true"
                shift
                ;;
            --help)
                usage
                ;;
            -*)
                print_error "Unknown option: $1"
                usage
                ;;
            *)
                if [[ -z "$version" ]]; then
                    version="$1"
                else
                    print_error "Multiple version arguments provided"
                    usage
                fi
                shift
                ;;
        esac
    done

    if [[ -z "$version" ]]; then
        print_error "Version argument is required"
        usage
    fi

    print_header "Terraphim AI Release v$version"

    if [[ "$dry_run" == "true" ]]; then
        print_warning "DRY RUN MODE - No changes will be made"
    fi

    validate_prerequisites
    validate_version "$version"

    # Show what will be done
    print_info "Release plan:"
    echo "  • Update version files to $version"
    echo "  • Run tests (skip: $skip_tests)"
    echo "  • Build artifacts (skip: $skip_build)"
    echo "  • Create release commit and tag"
    echo "  • Push to GitHub"
    echo "  • Trigger automated release via GitHub Actions"

    if [[ "$dry_run" == "false" ]]; then
        echo ""
        read -p "Continue with release? (y/N): " -r
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Release cancelled"
            exit 0
        fi
    fi

    echo ""
    update_version_files "$version" "$dry_run"
    run_tests "$skip_tests"
    build_artifacts "$skip_build" "$dry_run"
    create_release_commit "$version" "$dry_run"

    if [[ "$dry_run" == "false" ]]; then
        generate_changelog_entry "$version"
        echo ""
        read -p "Push release to GitHub? (y/N): " -r
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            push_release "$version" "$dry_run"
        else
            print_info "Release created locally. Push manually when ready:"
            print_info "  git push origin main"
            print_info "  git push origin v$version"
        fi
    else
        push_release "$version" "$dry_run"
    fi

    print_success "🎉 Release v$version completed successfully!"

    if [[ "$dry_run" == "false" ]]; then
        echo ""
        print_info "Next steps:"
        echo "  • Monitor GitHub Actions for build progress"
        echo "  • Test auto-update functionality"
        echo "  • Update documentation if needed"
        echo "  • Announce release to team/users"
    fi
}

main "$@"
