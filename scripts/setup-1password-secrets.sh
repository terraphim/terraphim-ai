#!/bin/bash
# setup-1password-secrets.sh - Create 1Password vault and items for Terraphim AI deployment

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VAULT="Terraphim-Deployment"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

validate_prerequisites() {
    print_info "Validating prerequisites..."
    
    if ! command -v op &> /dev/null; then
        print_error "1Password CLI not found. Install with: brew install --cask 1password-cli"
        exit 1
    fi
    
    if ! op whoami &> /dev/null; then
        print_error "Not authenticated with 1Password. Run: op signin"
        exit 1
    fi
    
    if ! command -v npm &> /dev/null; then
        print_error "npm not found. Required for Tauri key generation."
        exit 1
    fi
    
    print_success "Prerequisites validated"
}

create_vault() {
    print_info "Creating 1Password vault: $VAULT"
    
    if op vault get "$VAULT" &> /dev/null; then
        print_warning "Vault '$VAULT' already exists"
    else
        op vault create "$VAULT"
        print_success "Created vault: $VAULT"
    fi
}

generate_tauri_keys() {
    print_info "Generating Tauri signing keys..."
    
    cd "$PROJECT_ROOT/desktop"
    
    # Generate temporary key to get the format
    TEMP_KEY_FILE=$(mktemp)
    trap "rm -f '$TEMP_KEY_FILE'" EXIT
    
    # Generate private key
    npm run tauri signer generate -- -w "$TEMP_KEY_FILE"
    
    PRIVATE_KEY=$(cat "$TEMP_KEY_FILE")
    PUBLIC_KEY=$(echo "$PRIVATE_KEY" | npm run tauri signer show-public-key 2>/dev/null || echo "")
    
    if [[ -z "$PUBLIC_KEY" ]]; then
        # Alternative method to extract public key
        PUBLIC_KEY=$(echo "$PRIVATE_KEY" | grep -A 10 "public key:" | tail -n +2 | head -1)
    fi
    
    # Generate a secure password
    KEY_PASSWORD=$(openssl rand -base64 32)
    
    print_success "Generated Tauri signing keys"
    return 0
}

create_tauri_signing_item() {
    print_info "Creating Tauri Update Signing item in 1Password..."
    
    if op item get "Tauri Update Signing" --vault "$VAULT" &> /dev/null; then
        print_warning "Tauri Update Signing item already exists. Updating..."
        op item edit "Tauri Update Signing" --vault "$VAULT" \
            "TAURI_PRIVATE_KEY[concealed]=$PRIVATE_KEY" \
            "TAURI_KEY_PASSWORD[concealed]=$KEY_PASSWORD" \
            "TAURI_PUBLIC_KEY[text]=$PUBLIC_KEY"
    else
        op item create \
            --category "API Credential" \
            --title "Tauri Update Signing" \
            --vault "$VAULT" \
            --field "label=TAURI_PRIVATE_KEY,type=concealed,value=$PRIVATE_KEY" \
            --field "label=TAURI_KEY_PASSWORD,type=concealed,value=$KEY_PASSWORD" \
            --field "label=TAURI_PUBLIC_KEY,type=text,value=$PUBLIC_KEY"
    fi
    
    print_success "Created Tauri Update Signing item"
}

create_github_token_item() {
    print_info "Creating GitHub Release Token item..."
    
    echo "Please enter your GitHub Personal Access Token (with repo permissions):"
    read -s GITHUB_TOKEN
    
    if [[ -z "$GITHUB_TOKEN" ]]; then
        print_warning "No GitHub token provided. Skipping..."
        return 0
    fi
    
    if op item get "GitHub Release Token" --vault "$VAULT" &> /dev/null; then
        print_warning "GitHub Release Token item already exists. Updating..."
        op item edit "GitHub Release Token" --vault "$VAULT" \
            "GITHUB_TOKEN[concealed]=$GITHUB_TOKEN"
    else
        op item create \
            --category "API Credential" \
            --title "GitHub Release Token" \
            --vault "$VAULT" \
            --field "label=GITHUB_TOKEN,type=concealed,value=$GITHUB_TOKEN"
    fi
    
    print_success "Created GitHub Release Token item"
}

update_tauri_config() {
    print_info "Updating tauri.conf.json with public key..."
    
    local config_file="$PROJECT_ROOT/desktop/src-tauri/tauri.conf.json"
    
    # Use jq to update the public key if available
    if command -v jq &> /dev/null; then
        jq ".tauri.updater.pubkey = \"$PUBLIC_KEY\"" "$config_file" > "$config_file.tmp"
        mv "$config_file.tmp" "$config_file"
        print_success "Updated tauri.conf.json with public key"
    else
        print_warning "jq not found. Please manually update the public key in tauri.conf.json"
        echo "Public key: $PUBLIC_KEY"
    fi
}

create_service_account_instructions() {
    print_info "Creating service account setup instructions..."
    
    cat > "$PROJECT_ROOT/SERVICE_ACCOUNT_SETUP.md" << EOF
# 1Password Service Account Setup for Terraphim AI CI/CD

## Create Service Account

1. Go to 1Password web interface
2. Navigate to Developer Tools > Service Accounts
3. Click "Create Service Account"
4. Name: "Terraphim CI/CD"
5. Description: "Service account for Terraphim AI automated deployments"

## Grant Vault Access

1. In the service account settings, add vault access:
   - Vault: $VAULT
   - Permissions: Read
   
## Copy Service Account Token

1. Copy the service account token (starts with 'ops_...')
2. Add to GitHub repository secrets:
   - Name: OP_SERVICE_ACCOUNT_TOKEN
   - Value: <copied_token>

## Test Access

Test the service account locally:
\`\`\`bash
export OP_SERVICE_ACCOUNT_TOKEN="ops_..."
op item get "Tauri Update Signing" --vault "$VAULT" --field "TAURI_PUBLIC_KEY"
\`\`\`

If this returns the public key, the service account is configured correctly.
EOF
    
    print_success "Created SERVICE_ACCOUNT_SETUP.md with instructions"
}

main() {
    print_info "🔐 Setting up 1Password secrets for Terraphim AI"
    
    validate_prerequisites
    create_vault
    generate_tauri_keys
    create_tauri_signing_item
    create_github_token_item
    update_tauri_config
    create_service_account_instructions
    
    print_success "🎉 1Password setup completed successfully!"
    echo ""
    print_info "Next steps:"
    echo "1. Review SERVICE_ACCOUNT_SETUP.md and create the service account"
    echo "2. Add OP_SERVICE_ACCOUNT_TOKEN to GitHub repository secrets"
    echo "3. Test the setup with: op run --env-file=.env.tauri-release -- echo 'Test'"
    echo ""
    print_info "Your secrets are now securely stored in 1Password vault: $VAULT"
}

main "$@"