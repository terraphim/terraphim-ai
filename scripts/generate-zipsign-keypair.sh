#!/usr/bin/env bash
#
# Generate Ed25519 key pair for Terraphim AI releases using zipsign
#
# This script generates a new Ed25519 key pair for signing Terraphim AI release binaries.
# The public key will be embedded in the binary, and the private key must be stored securely.
#
# IMPORTANT: This script should be run by maintainers in a secure environment.
# The private key must be kept secret and stored securely (e.g., 1Password, password manager).
#
# Usage: ./scripts/generate-zipsign-keypair.sh
#
# Output:
#   - private.key - Private signing key (SECRET, keep safe!)
#   - public.key - Public verification key (embed in binary)
#
# After running this script:
# 1. Store private.key securely in 1Password or password manager
# 2. Add the public key to crates/terraphim_update/src/signature.rs
# 3. Delete the private key from the filesystem after storing securely
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEY_DIR="$SCRIPT_DIR/../keys"
PRIVATE_KEY="$KEY_DIR/private.key"
PUBLIC_KEY="$KEY_DIR/public.key"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Check if zipsign CLI is installed
if ! command -v zipsign &> /dev/null; then
    print_error "zipsign is not installed"
    echo ""
    echo "Install zipsign:"
    echo "  cargo install zipsign"
    exit 1
fi

# Create keys directory if it doesn't exist
if [[ ! -d "$KEY_DIR" ]]; then
    print_status "Creating keys directory: $KEY_DIR"
    mkdir -p "$KEY_DIR"
    chmod 700 "$KEY_DIR"
fi

# Check if keys already exist
if [[ -f "$PRIVATE_KEY" ]] || [[ -f "$PUBLIC_KEY" ]]; then
    print_warning "Keys already exist:"
    [[ -f "$PRIVATE_KEY" ]] && echo "  - Private key: $PRIVATE_KEY"
    [[ -f "$PUBLIC_KEY" ]] && echo "  - Public key: $PUBLIC_KEY"
    echo ""
    read -p "Do you want to overwrite existing keys? (type 'yes' to confirm): " confirm
    if [[ "$confirm" != "yes" ]]; then
        print_status "Aborted"
        exit 0
    fi
    echo ""

    # Backup existing keys
    if [[ -f "$PRIVATE_KEY" ]]; then
        BACKUP_KEY="${PRIVATE_KEY}.backup.$(date +%Y%m%d%H%M%S)"
        mv "$PRIVATE_KEY" "$BACKUP_KEY"
        print_status "Backed up private key to: $BACKUP_KEY"
    fi
    if [[ -f "$PUBLIC_KEY" ]]; then
        BACKUP_PUB="${PUBLIC_KEY}.backup.$(date +%Y%m%d%H%M%S)"
        mv "$PUBLIC_KEY" "$BACKUP_PUB"
        print_status "Backed up public key to: $BACKUP_PUB"
    fi
    echo ""
fi

print_status "Generating Ed25519 key pair for Terraphim AI releases"
echo ""

# Generate key pair using zipsign
zipsign gen-key "$PRIVATE_KEY" "$PUBLIC_KEY" || {
    print_error "Failed to generate key pair"
    exit 1
}

echo ""
print_status "Key pair generated successfully!"
echo ""

# Set restrictive permissions
chmod 600 "$PRIVATE_KEY"
chmod 644 "$PUBLIC_KEY"

# Display public key
echo "=========================================="
echo "PUBLIC KEY (embed this in binary):"
echo "=========================================="
cat "$PUBLIC_KEY"
echo "=========================================="
echo ""

print_warning "IMPORTANT SECURITY NOTES:"
echo ""
echo "1. Store the PRIVATE key securely:"
echo "   - Upload to 1Password or password manager"
echo "   - File location: $PRIVATE_KEY"
echo "   - DO NOT commit to git!"
echo "   - Delete from filesystem after storing"
echo ""
echo "2. Add the PUBLIC key to the codebase:"
echo "   - Edit: crates/terraphim_update/src/signature.rs"
echo "   - Add to: get_embedded_public_key() function"
echo "   - The key is base64-encoded, use as-is"
echo ""
echo "3. Update .gitignore:"
echo "   - Add 'keys/' to .gitignore"
echo "   - Ensure private.key is never committed"
echo ""
echo "4. Test signing:"
echo "   - tar -czf /tmp/test.tar.gz Cargo.toml"
echo "   - zipsign sign tar /tmp/test.tar.gz $PRIVATE_KEY"
echo "   - zipsign verify tar /tmp/test.tar.gz $PUBLIC_KEY"
echo ""

print_status "Done! Key pair is ready for use."
echo ""
echo "Next steps:"
echo "  1. Store private.key in 1Password"
echo "  2. Add public key to crates/terraphim_update/src/signature.rs"
echo "  3. Add 'keys/' to .gitignore"
echo "  4. Delete $PRIVATE_KEY from filesystem"
echo "  5. Commit the public key changes"
