#!/usr/bin/env bash
#
# Sign release binaries using zipsign
#
# This script signs all release archives (.tar.gz, .tar.zst) in a directory
# using Ed25519 signatures via zipsign CLI. Signatures are embedded directly
# in the archives (as GZIP comments for .tar.gz files).
#
# Usage: ./scripts/sign-release.sh <release_directory> [private_key_path]
#
# Arguments:
#   release_directory - Directory containing release archives to sign
#   private_key_path  - Path to private signing key (optional, defaults to
#                       $ZIPSIGN_PRIVATE_KEY or keys/private.key)
#
# Environment Variables:
#   ZIPSIGN_PRIVATE_KEY - Path to private signing key
#
# Output:
#   Each archive is signed in-place, with the signature embedded in the archive
#
# Example:
#   ./scripts/sign-release.sh target/release/
#   ./scripts/sign-release.sh target/release/ keys/production.key
#   ZIPSIGN_PRIVATE_KEY=keys/production.key ./scripts/sign-release.sh target/release/
#
# Requirements:
#   - zipsign CLI installed (cargo install zipsign)
#   - Private signing key file exists
#   - Release directory exists with archives to sign
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default private key location
DEFAULT_KEY_DIR="$PROJECT_ROOT/keys"
DEFAULT_PRIVATE_KEY="$DEFAULT_KEY_DIR/private.key"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

# Usage function
usage() {
    cat <<EOF
Usage: $0 <release_directory> [private_key_path]

Sign all release archives in a directory using Ed25519 signatures.

Arguments:
  release_directory  Directory containing release archives (.tar.gz, .tar.zst)
  private_key_path   Path to private signing key (optional)

Environment Variables:
  ZIPSIGN_PRIVATE_KEY  Path to private signing key (overrides default)

Examples:
  $0 target/release/
  $0 target/release/ keys/production.key
  ZIPSIGN_PRIVATE_KEY=keys/production.key $0 target/release/

Requirements:
  - zipsign CLI installed (cargo install zipsign)
  - Private signing key file exists
  - Release directory exists with archives to sign

EOF
    exit 1
}

# Check if zipsign CLI is installed
if ! command -v zipsign &> /dev/null; then
    print_error "zipsign is not installed"
    echo ""
    echo "Install zipsign:"
    echo "  cargo install zipsign"
    exit 1
fi

# Parse arguments
RELEASE_DIR="${1:-}"
PRIVATE_KEY="${2:-${ZIPSIGN_PRIVATE_KEY:-$DEFAULT_PRIVATE_KEY}}"

# Validate arguments
if [[ -z "$RELEASE_DIR" ]]; then
    print_error "Release directory not specified"
    echo ""
    usage
fi

if [[ ! -d "$RELEASE_DIR" ]]; then
    print_error "Release directory does not exist: $RELEASE_DIR"
    exit 1
fi

if [[ ! -f "$PRIVATE_KEY" ]]; then
    print_error "Private key not found: $PRIVATE_KEY"
    echo ""
    echo "Generate a key pair:"
    echo "  ./scripts/generate-zipsign-keypair.sh"
    echo ""
    echo "Or specify a custom key path:"
    echo "  $0 $RELEASE_DIR /path/to/private.key"
    exit 1
fi

# Check private key permissions
KEY_PERMS=$(stat -c "%a" "$PRIVATE_KEY" 2>/dev/null || stat -f "%OLp" "$PRIVATE_KEY" 2>/dev/null || echo "000")
if [[ "$KEY_PERMS" != "600" ]] && [[ "$KEY_PERMS" != "400" ]]; then
    print_warning "Private key has insecure permissions: $KEY_PERMS (should be 600)"
    read -p "Continue anyway? (type 'yes' to confirm): " confirm
    if [[ "$confirm" != "yes" ]]; then
        print_status "Aborted"
        exit 0
    fi
fi

print_status "Signing release archives in: $RELEASE_DIR"
print_info "Using private key: $PRIVATE_KEY"
echo ""

# Find and sign archives
SIGNED_COUNT=0
FAILED_COUNT=0
SKIPPED_COUNT=0

# Create array of archive files
mapfile -t ARCHIVES < <(find "$RELEASE_DIR" -maxdepth 1 \( -name "*.tar.gz" -o -name "*.tar.zst" \) -type f 2>/dev/null | sort)

if [[ ${#ARCHIVES[@]} -eq 0 ]]; then
    print_warning "No archives found in $RELEASE_DIR"
    echo ""
    echo "Looking for files matching: *.tar.gz, *.tar.zst"
    echo "Release directory contents:"
    ls -la "$RELEASE_DIR"
    exit 0
fi

print_status "Found ${#ARCHIVES[@]} archive(s) to sign"
echo ""

# Sign each archive
for archive in "${ARCHIVES[@]}"; do
    archive_name=$(basename "$archive")
    print_info "Signing: $archive_name"

    # Check if already signed
    if zipsign verify tar "$archive" "$PRIVATE_KEY" &>/dev/null; then
        print_warning "  Already signed, skipping"
        ((SKIPPED_COUNT++))
        continue
    fi

    # Sign the archive
    if zipsign sign tar "$archive" "$PRIVATE_KEY" 2>&1 | while IFS= read -r line; do
        echo "  $line"
    done; then
        print_status "  ✓ Signed successfully"
        ((SIGNED_COUNT++))
    else
        print_error "  ✗ Failed to sign"
        ((FAILED_COUNT++))
    fi
    echo ""
done

# Summary
echo "=========================================="
print_status "Signing complete!"
echo ""
echo "  Signed:    $SIGNED_COUNT"
echo "  Skipped:   $SKIPPED_COUNT"
echo "  Failed:    $FAILED_COUNT"
echo "  Total:     ${#ARCHIVES[@]}"
echo "=========================================="
echo ""

# Verify signed archives
if [[ $SIGNED_COUNT -gt 0 ]]; then
    print_status "Verifying signatures..."
    echo ""

    VERIFY_FAILED=0
    for archive in "${ARCHIVES[@]}"; do
        archive_name=$(basename "$archive")
        print_info "Verifying: $archive_name"

        if zipsign verify tar "$archive" "$PRIVATE_KEY" 2>&1 | while IFS= read -r line; do
            echo "  $line"
        done; then
            print_status "  ✓ Signature valid"
        else
            print_error "  ✗ Signature invalid"
            ((VERIFY_FAILED++))
        fi
        echo ""
    done

    if [[ $VERIFY_FAILED -gt 0 ]]; then
        print_error "Verification failed for $VERIFY_FAILED archive(s)"
        exit 1
    fi
fi

if [[ $FAILED_COUNT -gt 0 ]]; then
    print_error "Failed to sign $FAILED_COUNT archive(s)"
    exit 1
fi

print_status "All archives signed and verified successfully!"
echo ""
print_info "You can now upload the signed archives to GitHub Releases"
echo ""
echo "Example upload command:"
echo "  gh release create v1.0.0 $RELEASE_DIR/*.tar.gz"
