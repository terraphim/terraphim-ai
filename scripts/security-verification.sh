#!/bin/bash
# Security Verification Utilities for Terraphim AI Installer
# Provides checksum verification, binary validation, and security checks

# Configuration
SKIP_VERIFY="${SKIP_VERIFY:-false}"
GITHUB_RELEASES="${GITHUB_RELEASES:-https://github.com/terraphim/terraphim-ai/releases/download}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

# Get available checksum command
get_checksum_command() {
    if command -v sha256sum >/dev/null 2>&1; then
        echo "sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        echo "shasum -a 256"
    elif command -v openssl >/dev/null 2>&1; then
        echo "openssl dgst -sha256"
    else
        echo ""
    fi
}

# Calculate SHA256 checksum of a file
calculate_checksum() {
    local file_path=$1

    if [[ ! -f "$file_path" ]]; then
        log_error "File not found: $file_path"
        return 1
    fi

    local checksum_cmd
    checksum_cmd=$(get_checksum_command)

    if [[ -z "$checksum_cmd" ]]; then
        log_error "No checksum command available (sha256sum, shasum, or openssl required)"
        return 1
    fi

    log_info "Calculating checksum for: $file_path"

    local checksum
    case $checksum_cmd in
        "sha256sum")
            checksum=$(sha256sum "$file_path" | cut -d' ' -f1)
            ;;
        "shasum -a 256")
            checksum=$(shasum -a 256 "$file_path" | cut -d' ' -f1)
            ;;
        "openssl dgst -sha256")
            checksum=$(openssl dgst -sha256 "$file_path" | cut -d' ' -f2)
            ;;
    esac

    if [[ -n "$checksum" ]]; then
        echo "$checksum"
        return 0
    else
        log_error "Failed to calculate checksum"
        return 1
    fi
}

# Download checksum file for a release
download_checksums() {
    local version=$1
    local checksum_url="${GITHUB_RELEASES}/v${version#v}/checksums.txt"
    local temp_checksum_file="/tmp/terraphim-checksums-${version}.txt"

    log_info "Downloading checksums for version $version..."

    if curl --silent --fail --location --retry 3 --output "$temp_checksum_file" "$checksum_url"; then
        log_success "Checksums downloaded to: $temp_checksum_file"
        echo "$temp_checksum_file"
        return 0
    else
        log_warn "No checksum file found for version $version"
        return 1
    fi
}

# Extract checksum for a specific asset from checksums file
extract_checksum_from_file() {
    local checksum_file=$1
    local asset_name=$2

    if [[ ! -f "$checksum_file" ]]; then
        log_error "Checksum file not found: $checksum_file"
        return 1
    fi

    local checksum
    checksum=$(grep "$asset_name" "$checksum_file" | head -1 | awk '{print $1}')

    if [[ -n "$checksum" ]]; then
        echo "$checksum"
        return 0
    else
        log_warn "Checksum not found for $asset_name"
        return 1
    fi
}

# Verify file checksum against expected value
verify_checksum() {
    local file_path=$1
    local expected_checksum=$2

    if [[ "$SKIP_VERIFY" == "true" ]]; then
        log_warn "Skipping checksum verification (SKIP_VERIFY=true)"
        return 0
    fi

    log_info "Verifying checksum for: $(basename "$file_path")"

    if [[ -z "$expected_checksum" ]]; then
        log_warn "No expected checksum provided, skipping verification"
        return 0
    fi

    local actual_checksum
    actual_checksum=$(calculate_checksum "$file_path")

    if [[ $? -ne 0 ]]; then
        log_error "Failed to calculate actual checksum"
        return 1
    fi

    log_info "Expected: $expected_checksum"
    log_info "Actual:   $actual_checksum"

    if [[ "$actual_checksum" == "$expected_checksum" ]]; then
        log_success "Checksum verification passed"
        return 0
    else
        log_error "Checksum verification failed!"
        log_error "The file may be corrupted or tampered with."
        return 1
    fi
}

# Verify downloaded binary with checksums file
verify_binary_with_checksums() {
    local file_path=$1
    local asset_name=$2
    local version=$3

    log_info "Verifying binary with official checksums..."

    # Download checksums file
    local checksum_file
    checksum_file=$(download_checksums "$version")

    if [[ $? -ne 0 ]]; then
        log_warn "Could not download checksums file, skipping verification"
        return 0
    fi

    # Extract expected checksum
    local expected_checksum
    expected_checksum=$(extract_checksum_from_file "$checksum_file" "$asset_name")

    if [[ $? -ne 0 ]]; then
        log_warn "Could not find checksum for $asset_name, skipping verification"
        return 0
    fi

    # Verify checksum
    verify_checksum "$file_path" "$expected_checksum"
    local result=$?

    # Cleanup
    rm -f "$checksum_file"

    return $result
}

# Verify binary file type and basic properties
verify_binary_properties() {
    local file_path=$1
    local os=${2:-$(uname -s | tr '[:upper:]' '[:lower:]')}

    log_info "Verifying binary properties for: $(basename "$file_path")"

    if [[ ! -f "$file_path" ]]; then
        log_error "File not found: $file_path"
        return 1
    fi

    # Check file size (should be greater than 0)
    local file_size
    file_size=$(stat -f%z "$file_path" 2>/dev/null || stat -c%s "$file_path" 2>/dev/null || echo "0")

    if [[ "$file_size" -eq 0 ]]; then
        log_error "File is empty: $file_path"
        return 1
    fi

    log_info "File size: $file_size bytes"

    # Check if file is executable (for Unix-like systems)
    if [[ "$os" != "windows" && ! "$file_path" =~ \.exe$ ]]; then
        if [[ ! -x "$file_path" ]]; then
            log_warn "File is not executable, fixing permissions..."
            chmod +x "$file_path"
        fi
    fi

    # Use file command to check file type (if available)
    if command -v file >/dev/null 2>&1; then
        local file_type
        file_type=$(file "$file_path")

        log_info "File type: $file_type"

        # Basic validation of file type
        case $os in
            linux*)
                if [[ ! "$file_type" =~ (ELF|executable) ]]; then
                    log_warn "File doesn't appear to be a Linux executable"
                fi
                ;;
            darwin*)
                if [[ ! "$file_type" =~ (Mach-O|executable) ]]; then
                    log_warn "File doesn't appear to be a macOS executable"
                fi
                ;;
            windows*)
                if [[ ! "$file_type" =~ (PE32|executable) ]] && [[ ! "$file_path" =~ \.exe$ ]]; then
                    log_warn "File doesn't appear to be a Windows executable"
                fi
                ;;
        esac
    fi

    log_success "Binary properties verification completed"
    return 0
}

# Quick security check of the download URL
verify_download_url() {
    local url=$1

    log_info "Verifying download URL security..."

    # Check if using HTTPS
    if [[ ! "$url" =~ ^https:// ]]; then
        log_error "Download URL must use HTTPS: $url"
        return 1
    fi

    # Check if it's from the expected domain
    if [[ ! "$url" =~ github\.com/terraphim/terraphim-ai ]]; then
        log_error "Download URL is not from the official repository: $url"
        return 1
    fi

    log_success "Download URL security check passed"
    return 0
}

# Perform comprehensive verification of a downloaded binary
comprehensive_verify() {
    local file_path=$1
    local asset_name=$2
    local version=$3
    local download_url=$4

    log_info "Starting comprehensive verification..."

    local verification_passed=true

    # 1. Verify download URL security
    if ! verify_download_url "$download_url"; then
        verification_passed=false
    fi

    # 2. Verify binary properties
    if ! verify_binary_properties "$file_path"; then
        verification_passed=false
    fi

    # 3. Verify checksum
    if ! verify_binary_with_checksums "$file_path" "$asset_name" "$version"; then
        verification_passed=false
    fi

    # 4. Basic functionality test (try to run --version if it's a binary)
    if [[ -x "$file_path" ]] && ! [[ "$file_path" =~ \.exe$ ]]; then
        log_info "Testing basic binary functionality..."
        if timeout 5 "$file_path" --version >/dev/null 2>&1; then
            log_success "Binary functionality test passed"
        else
            log_warn "Binary functionality test failed (may be normal for some tools)"
        fi
    fi

    if [[ "$verification_passed" == "true" ]]; then
        log_success "Comprehensive verification passed"
        return 0
    else
        log_error "Comprehensive verification failed"
        return 1
    fi
}

# Generate checksum file for local testing (development only)
generate_checksum_file() {
    local directory=$1
    local output_file=$2

    log_info "Generating checksums for files in: $directory"

    if [[ ! -d "$directory" ]]; then
        log_error "Directory not found: $directory"
        return 1
    fi

    local checksum_cmd
    checksum_cmd=$(get_checksum_command)

    if [[ -z "$checksum_cmd" ]]; then
        log_error "No checksum command available"
        return 1
    fi

    cd "$directory"
    $checksum_cmd * > "$output_file" 2>/dev/null

    log_success "Checksums generated: $output_file"
}

# Security audit of the installation process
security_audit() {
    log_info "Performing security audit..."

    local audit_passed=true

    # Check if running with elevated privileges
    if [[ $EUID -eq 0 ]]; then
        log_warn "Running with root privileges - ensure this is intentional"
    fi

    # Check if PATH is secure
    if echo "$PATH" | grep -q "::"; then
        log_error "Insecure PATH detected (empty directory in PATH)"
        audit_passed=false
    fi

    # Check for suspicious environment variables
    local suspicious_vars=("LD_PRELOAD" "DYLD_INSERT_LIBRARIES" "IFS")
    for var in "${suspicious_vars[@]}"; do
        if [[ -n "${!var:-}" ]]; then
            log_warn "Suspicious environment variable set: $var"
        fi
    done

    # Check if we're in a secure directory
    if [[ "$(pwd)" =~ \ |\' ]]; then
        log_warn "Current directory contains spaces - may cause issues"
    fi

    if [[ "$audit_passed" == "true" ]]; then
        log_success "Security audit passed"
        return 0
    else
        log_error "Security audit failed"
        return 1
    fi
}

# Main function for testing
main() {
    local test_file=${1:-""}
    local version=${2:-"latest"}

    echo "=== Security Verification Test ==="
    if [[ -n "$test_file" ]]; then
        echo "Test file: $test_file"
        echo "Version: $version"
    fi
    echo "================================="

    # Test checksum command
    local checksum_cmd
    checksum_cmd=$(get_checksum_command)
    echo "Checksum command: $checksum_cmd"

    # Test security audit
    security_audit

    if [[ -n "$test_file" && -f "$test_file" ]]; then
        echo
        echo "Testing verification on: $test_file"
        verify_binary_properties "$test_file"

        local checksum
        checksum=$(calculate_checksum "$test_file")
        echo "Checksum: $checksum"

        verify_checksum "$test_file" "$checksum"
    fi
}

# If script is executed directly, run main
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
