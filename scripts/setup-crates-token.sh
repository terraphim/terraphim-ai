#!/bin/bash
# setup-crates-token.sh - Set up CARGO_REGISTRY_TOKEN from 1Password

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Terraphim crates.io Token Setup ===${NC}"
echo ""

# Function to check if 1Password CLI is available
check_op_cli() {
    if ! command -v op >/dev/null 2>&1; then
        echo -e "${RED}❌ 1Password CLI not found. Please install it first:${NC}"
        echo "https://developer.1password.com/docs/cli/get-started/"
        exit 1
    fi

    echo -e "${GREEN}✅ 1Password CLI found${NC}"
}

# Function to check if user is signed in to 1Password
check_op_auth() {
    if ! op account list >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠️  Not signed in to 1Password. Please sign in:${NC}"
        echo "op signin <account-shorthand>"
        echo ""
        echo "Available accounts:"
        op account list 2>/dev/null || echo "No accounts found"
        exit 1
    fi

    echo -e "${GREEN}✅ Signed in to 1Password${NC}"
}

# Function to get the token from 1Password
get_token_from_1password() {
    local account="${1:-}"

    if [[ -n "$account" ]]; then
        token=$(op read "op://TerraphimPlatform/crates.io.token/token" --account "$account" 2>/dev/null)
    else
        # Try without specifying account (uses default)
        token=$(op read "op://TerraphimPlatform/crates.io.token/token" 2>/dev/null)
    fi

    if [[ -z "$token" ]]; then
        echo -e "${RED}❌ Could not read crates.io token from 1Password${NC}"
        echo "Please check:"
        echo "1. You're signed in to the correct 1Password account"
        echo "2. The secret 'op://TerraphimPlatform/crates.io.token/token' exists"
        echo "3. You have permission to access this secret"
        exit 1
    fi

    echo "$token"
}

# Function to update .env file
update_env_file() {
    local token="$1"

    if [[ -f ".env" ]]; then
        echo -e "${YELLOW}⚠️  .env file already exists${NC}"
        read -p "Do you want to update it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Aborted."
            exit 0
        fi
    fi

    # Create/update .env file
    cat > .env << EOF
# Environment Variables for Terraphim Development
# Generated on: $(date)

# crates.io token for publishing Rust crates
# Retrieved from 1Password: op://TerraphimPlatform/crates.io.token/token
CARGO_REGISTRY_TOKEN=${token}

# Optional: Local development overrides
# TERRAPHIM_CONFIG=./terraphim_engineer_config.json
# TERRAPHIM_DATA_DIR=./data
# LOG_LEVEL=debug
EOF

    echo -e "${GREEN}✅ .env file updated${NC}"
}

# Function to export token for current session
export_token() {
    local token="$1"
    export CARGO_REGISTRY_TOKEN="$token"
    echo -e "${GREEN}✅ CARGO_REGISTRY_TOKEN exported for current session${NC}"
    echo -e "${YELLOW}💡 To make this permanent, add it to your shell profile (.bashrc, .zshrc, etc.)${NC}"
}

# Function to test the token
test_token() {
    echo -e "${BLUE}🧪 Testing crates.io token...${NC}"

    if cargo publish --dry-run --package terraphim_types >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Token is valid and ready for publishing${NC}"
    else
        echo -e "${RED}❌ Token validation failed${NC}"
        echo "Please check if the token is correct and has publishing permissions"
        exit 1
    fi
}

# Main execution
main() {
    local account=""
    local update_env=false
    local export_only=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --account)
                account="$2"
                shift 2
                ;;
            --update-env)
                update_env=true
                shift
                ;;
            --export-only)
                export_only=true
                shift
                ;;
            --help)
                cat << EOF
Usage: $0 [OPTIONS]

Setup CARGO_REGISTRY_TOKEN from 1Password for publishing Rust crates.

OPTIONS:
    --account ACCOUNT     Use specific 1Password account
    --update-env          Update .env file with token
    --export-only         Export token for current session only
    --help                Show this help message

EXAMPLES:
    $0 --update-env                    # Update .env file
    $0 --export-only                    # Export for current session
    $0 --account zesticailtd --update-env  # Use specific account and update .env

REQUIREMENTS:
    - 1Password CLI installed and signed in
    - Access to op://TerraphimPlatform/crates.io.token/token

EOF
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}" >&2
                exit 1
                ;;
        esac
    done

    echo "Checking prerequisites..."
    check_op_cli
    check_op_auth
    echo ""

    echo "Retrieving crates.io token from 1Password..."
    token=$(get_token_from_1password "$account")
    echo -e "${GREEN}✅ Token retrieved successfully${NC}"
    echo ""

    if [[ "$export_only" == "true" ]]; then
        export_token "$token"
    else
        update_env_file "$token"
    fi

    echo ""
    test_token
    echo ""
    echo -e "${GREEN}🎉 Setup complete!${NC}"

    if [[ "$export_only" != "true" ]]; then
        echo -e "${BLUE}Next steps:${NC}"
        echo "1. Source the .env file: source .env"
        echo "2. Or run: export CARGO_REGISTRY_TOKEN=\$(op read \"op://TerraphimPlatform/crates.io.token/token\")"
        echo "3. Test publishing: cargo publish --dry-run --package terraphim_types"
    fi
}

# Run main function with all arguments
main "$@"
