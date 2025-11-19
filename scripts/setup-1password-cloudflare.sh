#!/usr/bin/env bash
# Setup 1Password integration for Cloudflare Pages deployment
# Uses: op://TerraphimPlatform/terraphim-md-book-cloudflare/workers-api-token

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 1Password vault paths
OP_VAULT="TerraphimPlatform"
OP_ITEM="terraphim-md-book-cloudflare"
OP_API_TOKEN_PATH="op://${OP_VAULT}/${OP_ITEM}/workers-api-token"
OP_ACCOUNT_ID_PATH="op://${OP_VAULT}/${OP_ITEM}/account-id"
OP_ZONE_ID_PATH="op://${OP_VAULT}/${OP_ITEM}/zone-id"

check_op_cli() {
    log_info "Checking 1Password CLI..."

    if ! command -v op &> /dev/null; then
        log_error "1Password CLI (op) is not installed"
        log_info "Install from: https://developer.1password.com/docs/cli/get-started/"
        exit 1
    fi

    # Check if signed in
    if ! op account list &> /dev/null; then
        log_warning "Not signed in to 1Password"
        log_info "Please run: op signin"
        exit 1
    fi

    log_success "1Password CLI is available and authenticated"
}

verify_vault_access() {
    log_info "Verifying access to vault: ${OP_VAULT}..."

    if ! op vault get "${OP_VAULT}" &> /dev/null; then
        log_error "Cannot access vault: ${OP_VAULT}"
        log_info "Ensure you have access to the vault and are signed in"
        exit 1
    fi

    log_success "Vault access verified"
}

verify_item_exists() {
    log_info "Verifying item exists: ${OP_ITEM}..."

    if ! op item get "${OP_ITEM}" --vault "${OP_VAULT}" &> /dev/null; then
        log_warning "Item '${OP_ITEM}' not found in vault '${OP_VAULT}'"
        log_info "Creating the item with required fields..."
        create_item
    else
        log_success "Item exists"
    fi
}

create_item() {
    log_info "Creating 1Password item for Cloudflare credentials..."

    echo ""
    echo "Please provide the following Cloudflare credentials:"
    echo ""

    read -p "Cloudflare API Token (for Pages Edit): " api_token
    read -p "Cloudflare Account ID: " account_id
    read -p "Cloudflare Zone ID (optional, press Enter to skip): " zone_id

    # Create the item
    op item create \
        --category=login \
        --title="${OP_ITEM}" \
        --vault="${OP_VAULT}" \
        "workers-api-token=${api_token}" \
        "account-id=${account_id}" \
        "zone-id=${zone_id:-}" \
        "url=https://dash.cloudflare.com" \
        "notes=Cloudflare credentials for docs.terraphim.ai deployment"

    log_success "Item created successfully"
}

test_credentials() {
    log_info "Testing Cloudflare credentials..."

    # Read credentials from 1Password
    local api_token account_id

    api_token=$(op read "${OP_API_TOKEN_PATH}")
    account_id=$(op read "${OP_ACCOUNT_ID_PATH}")

    if [[ -z "$api_token" || -z "$account_id" ]]; then
        log_error "Failed to read credentials from 1Password"
        exit 1
    fi

    # Test API access
    local response
    response=$(curl -s -X GET "https://api.cloudflare.com/client/v4/user/tokens/verify" \
        -H "Authorization: Bearer ${api_token}" \
        -H "Content-Type: application/json")

    if echo "$response" | grep -q '"success":true'; then
        log_success "API token is valid"
    else
        log_error "API token verification failed"
        echo "$response" | jq . 2>/dev/null || echo "$response"
        exit 1
    fi
}

create_env_file() {
    log_info "Creating .env.1password file..."

    cat > "${PROJECT_ROOT}/docs/.env.1password" << EOF
# 1Password environment file for Cloudflare Pages deployment
# Use with: op run --env-file=.env.1password -- <command>

CLOUDFLARE_API_TOKEN=${OP_API_TOKEN_PATH}
CLOUDFLARE_ACCOUNT_ID=${OP_ACCOUNT_ID_PATH}
CLOUDFLARE_ZONE_ID=${OP_ZONE_ID_PATH}
PROJECT_NAME=terraphim-docs
CUSTOM_DOMAIN=docs.terraphim.ai
EOF

    log_success "Created .env.1password file"
    log_info "Use with: op run --env-file=docs/.env.1password -- ./scripts/deploy-docs.sh"
}

setup_github_actions() {
    log_info "Setting up GitHub Actions integration..."

    echo ""
    echo "=== GitHub Actions 1Password Integration ==="
    echo ""
    echo "To use 1Password in GitHub Actions, you need to:"
    echo ""
    echo "1. Install 1Password GitHub Actions integration:"
    echo "   https://github.com/1Password/load-secrets-action"
    echo ""
    echo "2. Add the following secrets to your GitHub repository:"
    echo "   - OP_SERVICE_ACCOUNT_TOKEN: Your 1Password service account token"
    echo ""
    echo "3. Create a service account with access to vault: ${OP_VAULT}"
    echo "   https://developer.1password.com/docs/service-accounts/"
    echo ""
    echo "4. The workflow will automatically read credentials from:"
    echo "   - API Token: ${OP_API_TOKEN_PATH}"
    echo "   - Account ID: ${OP_ACCOUNT_ID_PATH}"
    echo "   - Zone ID: ${OP_ZONE_ID_PATH}"
    echo ""
}

show_usage() {
    echo ""
    echo "=== 1Password Integration Complete ==="
    echo ""
    echo "Usage examples:"
    echo ""
    echo "1. Deploy with 1Password credentials:"
    echo "   op run --env-file=docs/.env.1password -- ./scripts/deploy-docs.sh production"
    echo ""
    echo "2. Manual credential injection:"
    echo "   export CLOUDFLARE_API_TOKEN=\$(op read '${OP_API_TOKEN_PATH}')"
    echo "   export CLOUDFLARE_ACCOUNT_ID=\$(op read '${OP_ACCOUNT_ID_PATH}')"
    echo "   ./scripts/deploy-docs.sh production"
    echo ""
    echo "3. GitHub Actions (automated):"
    echo "   Push to main branch or create PR to trigger deployment"
    echo ""
}

main() {
    echo ""
    echo "============================================"
    echo "  1Password Setup for Cloudflare Deployment"
    echo "============================================"
    echo ""

    check_op_cli
    verify_vault_access
    verify_item_exists
    test_credentials
    create_env_file
    setup_github_actions
    show_usage

    log_success "1Password integration setup complete!"
}

main "$@"
