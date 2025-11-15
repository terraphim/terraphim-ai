#!/bin/bash
# Comprehensive Release Pipeline Validation Script

set -e

VERSION=${1:-"0.2.3"}
RELEASE_TAG="v$VERSION"
TEST_DIR="/tmp/terraphim-pipeline-test"
RESULTS_FILE="$TEST_DIR/validation-results.log"

echo "🔄 Terraphim AI Release Pipeline Validation"
echo "=========================================="
echo "Version: $RELEASE_TAG"
echo "Test Directory: $TEST_DIR"
echo ""

# Colors for output
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

# Initialize test environment
init_test_env() {
    print_status "Initializing test environment..."

    # Clean up previous tests
    rm -rf "$TEST_DIR"
    mkdir -p "$TEST_DIR"
    cd "$TEST_DIR"

    # Create results file
    echo "Terraphim AI Release Pipeline Validation - $(date)" > "$RESULTS_FILE"
    echo "Version: $RELEASE_TAG" >> "$RESULTS_FILE"
    echo "========================================" >> "$RESULTS_FILE"

    print_success "Test environment initialized"
}

# Validate Git repository
validate_git_repo() {
    print_status "Validating Git repository..."

    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "Not in a Git repository"
        return 1
    fi

    # Check if release tag exists
    if ! git tag | grep -q "^$RELEASE_TAG$"; then
        print_error "Release tag $RELEASE_TAG not found"
        echo "ERROR: Release tag $RELEASE_TAG not found" >> "$RESULTS_FILE"
        return 1
    fi

    print_success "Git repository validation passed"
    echo "✓ Git repository: Valid" >> "$RESULTS_FILE"
    return 0
}

# Validate GitHub release
validate_github_release() {
    print_status "Validating GitHub release..."

    # Check if release exists on GitHub
    if ! gh release view "$RELEASE_TAG" > /dev/null 2>&1; then
        print_error "GitHub release $RELEASE_TAG not found"
        echo "ERROR: GitHub release $RELEASE_TAG not found" >> "$RESULTS_FILE"
        return 1
    fi

    # Get release assets
    local assets=$(gh release view "$RELEASE_TAG" --json assets --jq '.assets | length')
    if [ "$assets" -lt 5 ]; then
        print_warning "GitHub release has fewer assets than expected ($assets)"
    fi

    print_success "GitHub release validation passed"
    echo "✓ GitHub release: Valid ($assets assets)" >> "$RESULTS_FILE"
    return 0
}

# Download and validate artifacts
validate_artifacts() {
    print_status "Downloading and validating artifacts..."

    local artifacts=(
        "terraphim-server_${VERSION}-1_amd64.deb"
        "terraphim-agent_${VERSION}-1_amd64.deb"
        "terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst"
        "terraphim-agent-${VERSION}-1-x86_64.pkg.tar.zst"
        "install.sh"
        "docker-run.sh"
        "README.md"
    )

    local download_success=0

    for artifact in "${artifacts[@]}"; do
        print_status "Downloading $artifact..."

        if [[ "$artifact" == *.deb ]]; then
            wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_TAG/$artifact"
        elif [[ "$artifact" == *.pkg.tar.zst ]]; then
            wget -q "https://github.com/terraphim/terraphim-ai/releases/download/$RELEASE_TAG/$artifact"
        elif [[ "$artifact" == install.sh || "$artifact" == docker-run.sh ]]; then
            wget -q "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/$artifact"
        elif [[ "$artifact" == README.md ]]; then
            wget -q "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/$VERSION/$artifact"
        fi

        if [ -f "$artifact" ]; then
            print_success "✓ $artifact downloaded"
            echo "✓ $artifact: Downloaded" >> "$RESULTS_FILE"
            ((download_success++))
        else
            print_error "✗ Failed to download $artifact"
            echo "✗ $artifact: Download failed" >> "$RESULTS_FILE"
        fi
    done

    if [ "$download_success" -eq ${#artifacts[@]} ]; then
        print_success "All artifacts downloaded successfully"
        echo "✓ Artifacts: All $download_success artifacts downloaded" >> "$RESULTS_FILE"
        return 0
    else
        print_error "Only $download_success/${#artifacts[@]} artifacts downloaded"
        return 1
    fi
}

# Validate Debian packages
validate_debian_packages() {
    print_status "Validating Debian packages..."

    for deb_file in terraphim-server_${VERSION}-1_amd64.deb terraphim-agent_${VERSION}-1_amd64.deb; do
        if [ -f "$deb_file" ]; then
            print_status "Validating $deb_file..."

            # Check package info
            local pkg_info=$(dpkg-deb -I "$deb_file" 2>/dev/null || echo "INVALID")
            if [[ "$pkg_info" == *"INVALID"* ]]; then
                print_error "✗ $deb_file is not a valid Debian package"
                echo "✗ $deb_file: Invalid Debian package" >> "$RESULTS_FILE"
                return 1
            fi

            # Check package contents
            local has_binary=$(dpkg-deb -c "$deb_file" | grep -c "usr/bin/" || echo "0")
            if [ "$has_binary" -eq 0 ]; then
                print_error "✗ $deb_file missing binary in usr/bin/"
                echo "✗ $deb_file: Missing binary" >> "$RESULTS_FILE"
                return 1
            fi

            print_success "✓ $deb_file is valid"
            echo "✓ $deb_file: Valid Debian package" >> "$RESULTS_FILE"
        else
            print_error "✗ $deb_file not found"
            echo "✗ $deb_file: Not found" >> "$RESULTS_FILE"
            return 1
        fi
    done

    return 0
}

# Validate Arch Linux packages
validate_arch_packages() {
    print_status "Validating Arch Linux packages..."

    for pkg_file in terraphim-server-${VERSION}-1-x86_64.pkg.tar.zst terraphim-agent-${VERSION}-1-x86_64.pkg.tar.zst; do
        if [ -f "$pkg_file" ]; then
            print_status "Validating $pkg_file..."

            # Check if package can be listed
            if ! tar -I 'zstd -d' -tf "$pkg_file" > /dev/null 2>&1; then
                print_error "✗ $pkg_file is not a valid Arch package"
                echo "✗ $pkg_file: Invalid Arch package" >> "$RESULTS_FILE"
                return 1
            fi

            # Check for PKGINFO
            if ! tar -I 'zstd -d' -tf "$pkg_file" | grep -q ".PKGINFO"; then
                print_error "✗ $pkg_file missing .PKGINFO"
                echo "✗ $pkg_file: Missing PKGINFO" >> "$RESULTS_FILE"
                return 1
            fi

            # Check for binary
            local has_binary=$(tar -I 'zstd -d' -tf "$pkg_file" | grep -c "usr/bin/" || echo "0")
            if [ "$has_binary" -eq 0 ]; then
                print_error "✗ $pkg_file missing binary in usr/bin/"
                echo "✗ $pkg_file: Missing binary" >> "$RESULTS_FILE"
                return 1
            fi

            print_success "✓ $pkg_file is valid"
            echo "✓ $pkg_file: Valid Arch package" >> "$RESULTS_FILE"
        else
            print_error "✗ $pkg_file not found"
            echo "✗ $pkg_file: Not found" >> "$RESULTS_FILE"
            return 1
        fi
    done

    return 0
}

# Validate installation scripts
validate_installation_scripts() {
    print_status "Validating installation scripts..."

    for script in install.sh docker-run.sh; do
        if [ -f "$script" ]; then
            # Check if script is executable
            if [ ! -x "$script" ]; then
                print_warning "⚠ $script is not executable, fixing..."
                chmod +x "$script"
            fi

            # Check script syntax
            if bash -n "$script" 2>/dev/null; then
                print_success "✓ $script syntax is valid"
                echo "✓ $script: Valid syntax" >> "$RESULTS_FILE"
            else
                print_error "✗ $script has syntax errors"
                echo "✗ $script: Syntax errors" >> "$RESULTS_FILE"
                return 1
            fi
        else
            print_error "✗ $script not found"
            echo "✗ $script: Not found" >> "$RESULTS_FILE"
            return 1
        fi
    done

    return 0
}

# Validate documentation
validate_documentation() {
    print_status "Validating documentation..."

    if [ -f "README.md" ]; then
        # Check if README contains installation instructions
        if grep -q "Installation Options" README.md; then
            print_success "✓ README.md contains installation instructions"
            echo "✓ README.md: Contains installation instructions" >> "$RESULTS_FILE"
        else
            print_warning "⚠ README.md missing installation instructions"
            echo "⚠ README.md: Missing installation instructions" >> "$RESULTS_FILE"
        fi
    else
        print_error "✗ README.md not found"
        echo "✗ README.md: Not found" >> "$RESULTS_FILE"
        return 1
    fi

    return 0
}

# Test Docker deployment (optional)
test_docker_deployment() {
    print_status "Testing Docker deployment..."

    # Check if Docker is available
    if ! command -v docker &> /dev/null; then
        print_warning "⚠ Docker not available, skipping Docker test"
        echo "⚠ Docker: Not available" >> "$RESULTS_FILE"
        return 0
    fi

    # Try to pull the Docker image
    if docker pull ghcr.io/terraphim/terraphim-server:$VERSION > /dev/null 2>&1; then
        print_success "✓ Docker image can be pulled"
        echo "✓ Docker: Image pullable" >> "$RESULTS_FILE"
    else
        print_warning "⚠ Docker image not available (expected for new releases)"
        echo "⚠ Docker: Image not available" >> "$RESULTS_FILE"
    fi

    return 0
}

# Generate validation report
generate_report() {
    print_status "Generating validation report..."

    local total_tests=$(grep -c "✓\|✗\|⚠" "$RESULTS_FILE" || echo "0")
    local passed_tests=$(grep -c "✓" "$RESULTS_FILE" || echo "0")
    local failed_tests=$(grep -c "✗" "$RESULTS_FILE" || echo "0")
    local warnings=$(grep -c "⚠" "$RESULTS_FILE" || echo "0")

    echo ""
    echo "=========================================="
    echo "📊 VALIDATION REPORT"
    echo "=========================================="
    echo "Total Tests: $total_tests"
    echo "Passed: $passed_tests"
    echo "Failed: $failed_tests"
    echo "Warnings: $warnings"
    echo ""

    if [ "$failed_tests" -eq 0 ]; then
        print_success "🎉 All validation tests passed!"
        echo "✅ Status: PASSED" >> "$RESULTS_FILE"
    else
        print_error "❌ $failed_tests validation test(s) failed!"
        echo "❌ Status: FAILED" >> "$RESULTS_FILE"
    fi

    echo ""
    echo "Detailed results saved to: $RESULTS_FILE"
    echo ""

    # Show failed tests
    if [ "$failed_tests" -gt 0 ]; then
        echo "Failed Tests:"
        grep "✗" "$RESULTS_FILE" | sed 's/✗/  - /'
        echo ""
    fi

    # Show warnings
    if [ "$warnings" -gt 0 ]; then
        echo "Warnings:"
        grep "⚠" "$RESULTS_FILE" | sed 's/⚠/  - /'
        echo ""
    fi
}

# Main validation function
main() {
    echo "Starting comprehensive release pipeline validation..."
    echo ""

    # Initialize test environment
    init_test_env

    # Run validation tests
    local tests=(
        "validate_git_repo"
        "validate_github_release"
        "validate_artifacts"
        "validate_debian_packages"
        "validate_arch_packages"
        "validate_installation_scripts"
        "validate_documentation"
        "test_docker_deployment"
    )

    local failed_tests=0

    for test in "${tests[@]}"; do
        if ! $test; then
            ((failed_tests++))
        fi
        echo ""
    done

    # Generate report
    generate_report

    # Exit with appropriate code
    if [ "$failed_tests" -eq 0 ]; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
