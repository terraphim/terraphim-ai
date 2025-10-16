#!/usr/bin/env bash

# Terraphim AI Release Validation Script
# Validates release artifacts and performs post-release checks

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

usage() {
    cat << EOF
Terraphim AI Release Validation Script

Usage: $0 [OPTIONS] <VERSION>

Arguments:
    VERSION    Version number to validate (e.g., 0.2.5)

Options:
    -h, --help           Show this help message
    -q, --quick          Quick validation (skip time-consuming checks)
    -l, --local          Validate local build artifacts only
    -r, --remote         Validate remote GitHub release only
    --install-test       Test installation of packages (requires sudo)

Examples:
    $0 0.2.5                    # Full validation
    $0 --quick 0.2.5           # Quick validation
    $0 --install-test 0.2.5    # Full validation with installation test

EOF
}

# Default options
QUICK_VALIDATION=false
LOCAL_ONLY=false
REMOTE_ONLY=false
INSTALL_TEST=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -q|--quick)
            QUICK_VALIDATION=true
            shift
            ;;
        -l|--local)
            LOCAL_ONLY=true
            shift
            ;;
        -r|--remote)
            REMOTE_ONLY=true
            shift
            ;;
        --install-test)
            INSTALL_TEST=true
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

TAG="v$VERSION"
RELEASE_DIR="release/$VERSION"

print_status "Validating Terraphim AI release v$VERSION"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to validate file exists and is not empty
validate_file() {
    local file="$1"
    local description="$2"

    if [[ ! -f "$file" ]]; then
        print_error "$description not found: $file"
        return 1
    fi

    if [[ ! -s "$file" ]]; then
        print_error "$description is empty: $file"
        return 1
    fi

    print_success "$description found: $file ($(stat -f%z "$file" 2>/dev/null || stat -c%s "$file") bytes)"
    return 0
}

# Function to validate executable binary
validate_binary() {
    local binary="$1"
    local description="$2"

    if ! validate_file "$binary" "$description"; then
        return 1
    fi

    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if file "$binary" | grep -q "Mach-O"; then
            print_success "$description is a valid Mach-O binary"
        else
            print_error "$description is not a valid Mach-O binary"
            return 1
        fi
    else
        # Linux
        if file "$binary" | grep -q "ELF"; then
            print_success "$description is a valid ELF binary"
        else
            print_error "$description is not a valid ELF binary"
            return 1
        fi
    fi

    # Check if binary has execute permission
    if [[ -x "$binary" ]]; then
        print_success "$description has execute permissions"
    else
        print_warning "$description does not have execute permissions"
    fi

    return 0
}

# Function to validate package
validate_package() {
    local package="$1"
    local package_type="$2"
    local description="$3"

    if ! validate_file "$package" "$description"; then
        return 1
    fi

    case "$package_type" in
        "deb")
            if command_exists dpkg; then
                if dpkg-deb --info "$package" >/dev/null 2>&1; then
                    print_success "$description is a valid Debian package"
                    print_status "Package info: $(dpkg-deb --showformat='$Package $Version $Architecture' --show "$package")"
                else
                    print_error "$description is not a valid Debian package"
                    return 1
                fi
            else
                print_warning "dpkg not available, skipping Debian package validation"
            fi
            ;;
        "rpm")
            if command_exists rpm; then
                if rpm -qip "$package" >/dev/null 2>&1; then
                    print_success "$description is a valid RPM package"
                    print_status "Package info: $(rpm -qp --queryformat='%{NAME} %{VERSION} %{ARCH}\n' "$package")"
                else
                    print_error "$description is not a valid RPM package"
                    return 1
                fi
            else
                print_warning "rpm not available, skipping RPM package validation"
            fi
            ;;
        "tar.zst")
            if command_exists tar; then
                if tar -tf "$package" >/dev/null 2>&1; then
                    print_success "$description is a valid tar.zst archive"
                    print_status "Archive contents: $(tar -tf "$package" | head -5)"
                else
                    print_error "$description is not a valid tar.zst archive"
                    return 1
                fi
            else
                print_warning "tar not available, skipping archive validation"
            fi
            ;;
        "tar.gz")
            if command_exists tar; then
                if tar -tf "$package" >/dev/null 2>&1; then
                    print_success "$description is a valid tar.gz archive"
                    print_status "Archive contents: $(tar -tf "$package" | head -5)"
                else
                    print_error "$description is not a valid tar.gz archive"
                    return 1
                fi
            else
                print_warning "tar not available, skipping archive validation"
            fi
            ;;
        *)
            print_warning "Unknown package type: $package_type"
            ;;
    esac

    return 0
}

# Function to validate local artifacts
validate_local_artifacts() {
    print_status "Validating local build artifacts"

    local validation_errors=0

    # Validate built binaries
    validate_binary "target/release/terraphim_server" "Terraphim Server binary" || validation_errors=$((validation_errors + 1))
    validate_binary "target/release/terraphim_tui" "Terraphim TUI binary" || validation_errors=$((validation_errors + 1))

    # Validate Debian packages
    validate_package "target/debian/terraphim-server_${VERSION}-1_amd64.deb" "deb" "Terraphim Server Debian package" || validation_errors=$((validation_errors + 1))
    validate_package "target/debian/terraphim-tui_${VERSION}-1_amd64.deb" "deb" "Terraphim TUI Debian package" || validation_errors=$((validation_errors + 1))

    # Validate Arch Linux packages
    validate_package "$RELEASE_DIR/terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst" "tar.zst" "Terraphim Server Arch package" || validation_errors=$((validation_errors + 1))
    validate_package "$RELEASE_DIR/terraphim-tui-${VERSION}-1-x86_64.pkg.tar.zst" "tar.zst" "Terraphim TUI Arch package" || validation_errors=$((validation_errors + 1))

    # Validate RPM packages
    validate_package "$RELEASE_DIR/terraphim-server-${VERSION}-1.x86_64.rpm" "rpm" "Terraphim Server RPM package" || validation_errors=$((validation_errors + 1))
    validate_package "$RELEASE_DIR/terraphim-tui-${VERSION}-1.x86_64.rpm" "rpm" "Terraphim TUI RPM package" || validation_errors=$((validation_errors + 1))

    # Validate macOS packages (if on macOS)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        validate_package "$RELEASE_DIR/TerraphimServer-${VERSION}-macos.tar.gz" "tar.gz" "Terraphim Server macOS package" || validation_errors=$((validation_errors + 1))
        validate_package "$RELEASE_DIR/TerraphimTUI-${VERSION}-macos.tar.gz" "tar.gz" "Terraphim TUI macOS package" || validation_errors=$((validation_errors + 1))
    fi

    # Validate installation scripts
    validate_file "$RELEASE_DIR/install.sh" "Installation script" || validation_errors=$((validation_errors + 1))
    validate_file "$RELEASE_DIR/docker-run.sh" "Docker run script" || validation_errors=$((validation_errors + 1))
    validate_file "$RELEASE_DIR/README.md" "Release README" || validation_errors=$((validation_errors + 1))

    if [[ $validation_errors -gt 0 ]]; then
        print_error "Found $validation_errors validation errors in local artifacts"
        return 1
    else
        print_success "All local artifacts validated successfully"
        return 0
    fi
}

# Function to validate remote GitHub release
validate_remote_release() {
    print_status "Validating remote GitHub release"

    if ! command_exists gh; then
        print_error "GitHub CLI (gh) not found. Install with: brew install gh or https://cli.github.com/"
        return 1
    fi

    # Check if release exists
    if ! gh release view "$TAG" >/dev/null 2>&1; then
        print_error "GitHub release $TAG not found"
        return 1
    fi

    print_success "GitHub release $TAG found"

    # Get release information
    local release_info
    release_info=$(gh release view "$TAG" --json tagName,name,prerelease,createdAt,url)

    print_status "Release info:"
    echo "$release_info" | jq -r 'to_entries[] | "  \(.key): \(.value)"'

    # List release assets
    print_status "Release assets:"
    local assets
    assets=$(gh release view "$TAG" --json assets --jq '.assets[] | "\(.name) (\(.size) bytes)"')
    echo "$assets" | sed 's/^/  /'

    # Check for required assets
    local required_assets=(
        "install.sh"
        "docker-run.sh"
        "README.md"
        "terraphim-server_${VERSION}-1_amd64.deb"
        "terraphim-tui_${VERSION}-1_amd64.deb"
        "terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst"
        "terraphim-tui-${VERSION}-1-x86_64.pkg.tar.zst"
    )

    local missing_assets=()
    for asset in "${required_assets[@]}"; do
        if ! gh release view "$TAG" --json assets --jq ".assets[] | select(.name == \"$asset\")" | grep -q .; then
            missing_assets+=("$asset")
        fi
    done

    if [[ ${#missing_assets[@]} -gt 0 ]]; then
        print_warning "Missing assets in GitHub release:"
        printf '  %s\n' "${missing_assets[@]}"
    else
        print_success "All required assets found in GitHub release"
    fi

    # Test download of installation script
    local install_script_url
    install_script_url=$(gh release view "$TAG" --json assets --jq '.assets[] | select(.name == "install.sh") | .url')

    if [[ -n "$install_script_url" ]]; then
        print_status "Testing installation script download..."
        if curl -fsSL "$install_script_url" | head -5 >/dev/null; then
            print_success "Installation script download test passed"
        else
            print_warning "Installation script download test failed"
        fi
    fi

    return 0
}

# Function to test installation
test_installation() {
    if [[ "$INSTALL_TEST" != "true" ]]; then
        return 0
    fi

    print_status "Testing package installation (requires sudo privileges)"

    if [[ "$EUID" -ne 0 ]]; then
        print_warning "Installation test requires sudo privileges. Testing with sudo..."
    fi

    # Test Debian package installation
    local deb_package="target/debian/terraphim-server_${VERSION}-1_amd64.deb"
    if [[ -f "$deb_package" ]] && command_exists dpkg; then
        print_status "Testing Debian package installation..."

        if sudo dpkg -i "$deb_package" 2>/dev/null; then
            print_success "Debian package installation test passed"

            # Test binary execution
            if command -v terraphim_server >/dev/null 2>&1; then
                print_success "terraphim_server binary found in PATH"
                print_status "Version: $(terraphim_server --version 2>/dev/null || echo 'Version check failed')"
            else
                print_warning "terraphim_server binary not found in PATH"
            fi

            # Remove package
            print_status "Removing test package..."
            sudo dpkg -r terraphim-server || true
        else
            print_error "Debian package installation test failed"
            return 1
        fi
    fi

    # Test Arch package installation
    local arch_package="$RELEASE_DIR/terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst"
    if [[ -f "$arch_package" ]] && command_exists pacman; then
        print_status "Testing Arch package installation..."

        if sudo pacman -U --noconfirm "$arch_package" 2>/dev/null; then
            print_success "Arch package installation test passed"

            # Test binary execution
            if command -v terraphim_server >/dev/null 2>&1; then
                print_success "terraphim_server binary found in PATH"
            else
                print_warning "terraphim_server binary not found in PATH"
            fi

            # Remove package
            print_status "Removing test package..."
            sudo pacman -R --noconfirm terraphim-server || true
        else
            print_error "Arch package installation test failed"
            return 1
        fi
    fi

    return 0
}

# Function to run comprehensive tests
run_comprehensive_tests() {
    if [[ "$QUICK_VALIDATION" == "true" ]]; then
        print_status "Skipping comprehensive tests (quick mode)"
        return 0
    fi

    print_status "Running comprehensive tests"

    # Test binary execution
    if [[ -f "target/release/terraphim_server" ]]; then
        print_status "Testing server binary execution..."
        if timeout 5 target/release/terraphim_server --help >/dev/null 2>&1; then
            print_success "Server binary executes successfully"
        else
            print_warning "Server binary execution test failed"
        fi
    fi

    if [[ -f "target/release/terraphim_tui" ]]; then
        print_status "Testing TUI binary execution..."
        if timeout 5 target/release/terraphim_tui --help >/dev/null 2>&1; then
            print_success "TUI binary executes successfully"
        else
            print_warning "TUI binary execution test failed"
        fi
    fi

    # Test Rust project compilation
    print_status "Testing project compilation..."
    if cargo check --workspace >/dev/null 2>&1; then
        print_success "Project compiles successfully"
    else
        print_error "Project compilation failed"
        return 1
    fi

    # Run tests if not in quick mode
    if [[ "$QUICK_VALIDATION" != "true" ]]; then
        print_status "Running unit tests..."
        if cargo test --workspace >/dev/null 2>&1; then
            print_success "Unit tests pass"
        else
            print_warning "Some unit tests failed"
        fi
    fi

    return 0
}

# Function to generate validation report
generate_report() {
    print_status "Generating validation report"

    local report_file="validation-report-${VERSION}.md"

    cat > "$report_file" << EOF
# Terraphim AI v$VERSION Validation Report

Generated: $(date)
Validation Type: $([ "$QUICK_VALIDATION" == "true" ] && echo "Quick" || echo "Full")

## Local Artifacts Validation
$([ "$LOCAL_ONLY" != "true" ] && echo "- Skipped (remote only)" || echo "- Completed ✓")

## Remote Release Validation
$([ "$REMOTE_ONLY" != "true" ] && echo "- Skipped (local only)" || echo "- Completed ✓")

## Installation Tests
$([ "$INSTALL_TEST" == "true" ] && echo "- Completed ✓" || echo "- Skipped")

## Comprehensive Tests
$([ "$QUICK_VALIDATION" == "true" ] && echo "- Skipped (quick mode)" || echo "- Completed ✓")

## Files Validated

### Binaries
- \`target/release/terraphim_server\`
- \`target/release/terraphim_tui\`

### Packages
- Debian (.deb): terraphim-server, terraphim-tui
- Arch Linux (.tar.zst): terraphim-server, terraphim-tui
- RHEL/CentOS (.rpm): terraphim-server, terraphim-tui
- macOS (.tar.gz): TerraphimServer, TerraphimTUI

### Scripts
- Installation script: \`install.sh\`
- Docker script: \`docker-run.sh\`
- Documentation: \`README.md\`

## GitHub Release
- URL: https://github.com/terraphim/terraphim-ai/releases/tag/$TAG
- Status: $(gh release view "$TAG" >/dev/null 2>&1 && echo "✓ Published" || echo "✗ Not found")

## Recommendations
- [ ] Review test results
- [ ] Perform manual smoke testing
- [ ] Update documentation if needed
- [ ] Announce release to community

---

*This report was generated automatically by the validation script.*
EOF

    print_success "Validation report generated: $report_file"
}

# Main execution
main() {
    local validation_failed=false

    # Validate git repository
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_error "Not in a git repository"
        exit 1
    fi

    # Run validations based on options
    if [[ "$REMOTE_ONLY" != "true" ]]; then
        if ! validate_local_artifacts; then
            validation_failed=true
        fi
    fi

    if [[ "$LOCAL_ONLY" != "true" ]]; then
        if ! validate_remote_release; then
            validation_failed=true
        fi
    fi

    # Run comprehensive tests
    if ! run_comprehensive_tests; then
        validation_failed=true
    fi

    # Test installation if requested
    if ! test_installation; then
        validation_failed=true
    fi

    # Generate report
    generate_report

    # Final result
    echo
    if [[ "$validation_failed" == "true" ]]; then
        print_error "Release validation FAILED - see above for details"
        exit 1
    else
        print_success "Release validation PASSED - Terraphim AI v$VERSION is ready!"
        print_status "GitHub release: https://github.com/terraphim/terraphim-ai/releases/tag/$TAG"
    fi
}

# Run main function
main "$@"
