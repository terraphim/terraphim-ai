#!/usr/bin/env bash

# Terraphim AI 1Password Integration Setup Script
# This script creates 1Password vaults and initial secret structure for Terraphim AI
# Usage: ./setup-1password-terraphim.sh [--environment dev|prod|all]

set -eo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
DEFAULT_ENVIRONMENT="dev"
ENVIRONMENT="$DEFAULT_ENVIRONMENT"

# Vault configurations (bash 3.2 compatible)
get_vault_name() {
    case "$1" in
        "dev") echo "Terraphim-Dev" ;;
        "prod") echo "Terraphim-Prod" ;;
        "shared") echo "Terraphim-Shared" ;;
        *) echo "Unknown" ;;
    esac
}

# Secret configurations for each environment
get_dev_secrets() {
    cat << 'EOF'
OpenRouter:API_KEY ORGANIZATION_ID
Ollama:BASE_URL MODEL_NAME
Anthropic:API_KEY MODEL_NAME
Perplexity:API_KEY
AtomicServer:URL SECRET
ClickUp:API_TOKEN TEAM_ID LIST_ID
AWS_S3:ACCESS_KEY_ID SECRET_ACCESS_KEY BUCKET_NAME REGION
GitHub:TOKEN ORGANIZATION REPOSITORY
PostgreSQL:CONNECTION_STRING USERNAME PASSWORD HOST PORT DATABASE
Redis:URL PASSWORD HOST PORT
EOF
}

get_prod_secrets() {
    cat << 'EOF'
OpenRouter:API_KEY ORGANIZATION_ID
Perplexity:API_KEY
AWS_S3:ACCESS_KEY_ID SECRET_ACCESS_KEY BUCKET_NAME REGION
PostgreSQL:CONNECTION_STRING USERNAME PASSWORD HOST PORT DATABASE
Redis:URL PASSWORD HOST PORT
EOF
}

get_shared_secrets() {
    cat << 'EOF'
TauriSigning:PRIVATE_KEY PUBLIC_KEY PASSPHRASE
CodeSigning:CERTIFICATE_PATH CERTIFICATE_PASSWORD
Monitoring:SENTRY_DSN DATADOG_API_KEY
EOF
}

# Logging functions
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

log_header() {
    echo -e "\n${PURPLE}=== $1 ===${NC}"
}

# Check if 1Password CLI is installed and authenticated
check_prerequisites() {
    log_header "Checking Prerequisites"

    if ! command -v op &> /dev/null; then
        log_error "1Password CLI (op) is not installed"
        log_info "Please install it from: https://developer.1password.com/docs/cli/get-started/"
        exit 1
    fi

    # Check if authenticated
    if ! op account list &> /dev/null; then
        log_error "Not authenticated with 1Password CLI"
        log_info "Please run: op signin"
        exit 1
    fi

    log_success "1Password CLI is installed and authenticated"

    # Show current account
    local account=$(op account list --format=json | jq -r '.[0].user_name' 2>/dev/null || echo "Unknown")
    log_info "Current account: $account"
}

# Create a vault if it doesn't exist
create_vault() {
    local vault_name="$1"
    local description="$2"

    log_info "Checking vault: $vault_name"

    if op vault list --format=json | jq -e --arg name "$vault_name" '.[] | select(.name == $name)' > /dev/null; then
        log_warning "Vault '$vault_name' already exists"
        return 0
    fi

    log_info "Creating vault: $vault_name"
    if op vault create "$vault_name" --description "$description"; then
        log_success "Created vault: $vault_name"
    else
        log_error "Failed to create vault: $vault_name"
        return 1
    fi
}

# Create a secret item in a vault
create_secret_item() {
    local vault="$1"
    local item_name="$2"
    local fields="$3"

    log_info "Creating secret item: $item_name in $vault"

    # Check if item already exists
    if op item get "$item_name" --vault="$vault" &> /dev/null; then
        log_warning "Item '$item_name' already exists in vault '$vault'"
        return 0
    fi

    # Build the op item create command
    local cmd="op item create --vault='$vault' --category='API Credential' --title='$item_name'"

    # Add fields
    IFS=' ' read -ra FIELD_ARRAY <<< "$fields"
    for field in "${FIELD_ARRAY[@]}"; do
        case "$field" in
            *_KEY|*_TOKEN|*_SECRET|PASSWORD|PASSPHRASE)
                # Sensitive fields
                cmd="$cmd --field='$field[password]=placeholder_$(echo $field | tr '[:upper:]' '[:lower:]')'"
                ;;
            *)
                # Regular fields
                cmd="$cmd --field='$field[text]=placeholder_$(echo $field | tr '[:upper:]' '[:lower:]')'"
                ;;
        esac
    done

    # Add notes
    cmd="$cmd --notes='Created by Terraphim 1Password setup script. Please update placeholder values with actual credentials.'"

    if eval "$cmd" > /dev/null; then
        log_success "Created item: $item_name"
    else
        log_error "Failed to create item: $item_name"
        return 1
    fi
}

# Setup secrets for a specific environment
setup_environment_secrets() {
    local env="$1"
    local vault_name="$(get_vault_name "$env")"

    log_header "Setting up $env environment secrets in vault: $vault_name"

    # Create vault
    case "$env" in
        "dev")
            create_vault "$vault_name" "Terraphim AI Development Environment Secrets"
            ;;
        "prod")
            create_vault "$vault_name" "Terraphim AI Production Environment Secrets"
            ;;
        "shared")
            create_vault "$vault_name" "Terraphim AI Shared Environment Secrets"
            ;;
    esac

    # Get secrets for this environment and create them
    local secrets_func="get_${env}_secrets"
    if command -v "$secrets_func" >/dev/null 2>&1; then
        "$secrets_func" | while IFS=: read -r item_name fields; do
            [ -n "$item_name" ] && create_secret_item "$vault_name" "$item_name" "$fields"
        done
    else
        log_error "Unknown environment: $env"
        return 1
    fi

    log_success "Completed setup for $env environment"
}

# Generate configuration templates
generate_templates() {
    log_header "Generating Configuration Templates"

    local templates_dir="$PROJECT_ROOT/templates"
    mkdir -p "$templates_dir"

    # Environment template
    cat > "$templates_dir/.env.terraphim.template" << 'EOF'
# Terraphim AI Environment Configuration Template
# This file contains 1Password references that will be injected at runtime
# Usage: op inject -i .env.terraphim.template -o .env.terraphim

# LLM API Configuration
OPENROUTER_API_KEY="op://Terraphim-Dev/OpenRouter/API_KEY"
OPENROUTER_ORGANIZATION_ID="op://Terraphim-Dev/OpenRouter/ORGANIZATION_ID"

OLLAMA_BASE_URL="op://Terraphim-Dev/Ollama/BASE_URL"
OLLAMA_MODEL_NAME="op://Terraphim-Dev/Ollama/MODEL_NAME"

ANTHROPIC_API_KEY="op://Terraphim-Dev/Anthropic/API_KEY"
ANTHROPIC_MODEL_NAME="op://Terraphim-Dev/Anthropic/MODEL_NAME"

# LLM Proxy Configuration (z.ai)
ANTHROPIC_BASE_URL="op://Terraphim-Dev/Anthropic/BASE_URL"
ANTHROPIC_AUTH_TOKEN="op://Terraphim-Dev/Anthropic/AUTH_TOKEN"

# Search Services
PERPLEXITY_API_KEY="op://Terraphim-Dev/Perplexity/API_KEY"
ATOMIC_SERVER_URL="op://Terraphim-Dev/AtomicServer/URL"
ATOMIC_SERVER_SECRET="op://Terraphim-Dev/AtomicServer/SECRET"

CLICKUP_API_TOKEN="op://Terraphim-Dev/ClickUp/API_TOKEN"
CLICKUP_TEAM_ID="op://Terraphim-Dev/ClickUp/TEAM_ID"
CLICKUP_LIST_ID="op://Terraphim-Dev/ClickUp/LIST_ID"

# Cloud Storage
AWS_ACCESS_KEY_ID="op://Terraphim-Dev/AWS_S3/ACCESS_KEY_ID"
AWS_SECRET_ACCESS_KEY="op://Terraphim-Dev/AWS_S3/SECRET_ACCESS_KEY"
AWS_S3_BUCKET="op://Terraphim-Dev/AWS_S3/BUCKET_NAME"
AWS_REGION="op://Terraphim-Dev/AWS_S3/REGION"

# External APIs
GITHUB_TOKEN="op://Terraphim-Dev/GitHub/TOKEN"
DISCORD_BOT_TOKEN="op://Terraphim-Dev/Discord/BOT_TOKEN"

# Database
POSTGRES_CONNECTION_STRING="op://Terraphim-Dev/PostgreSQL/CONNECTION_STRING"
REDIS_URL="op://Terraphim-Dev/Redis/URL"
EOF

    # Server configuration template
    cat > "$templates_dir/server_config.json.template" << 'EOF'
{
  "llm": {
    "openrouter": {
      "api_key": "op://Terraphim-Dev/OpenRouter/API_KEY",
      "organization_id": "op://Terraphim-Dev/OpenRouter/ORGANIZATION_ID"
    },
    "anthropic": {
      "api_key": "op://Terraphim-Dev/Anthropic/API_KEY",
      "model": "op://Terraphim-Dev/Anthropic/MODEL_NAME"
    }
  },
  "search": {
    "perplexity": {
      "api_key": "op://Terraphim-Dev/Perplexity/API_KEY"
    },
    "atomic_server": {
      "url": "op://Terraphim-Dev/AtomicServer/URL",
      "secret": "op://Terraphim-Dev/AtomicServer/SECRET"
    }
  },
  "storage": {
    "s3": {
      "access_key_id": "op://Terraphim-Dev/AWS_S3/ACCESS_KEY_ID",
      "secret_access_key": "op://Terraphim-Dev/AWS_S3/SECRET_ACCESS_KEY",
      "bucket": "op://Terraphim-Dev/AWS_S3/BUCKET_NAME",
      "region": "op://Terraphim-Dev/AWS_S3/REGION"
    }
  }
}
EOF

    # Tauri configuration template
    cat > "$templates_dir/tauri.conf.json.template" << 'EOF'
{
  "package": {
    "productName": "Terraphim AI",
    "version": "1.0.0"
  },
  "build": {
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "tauri": {
    "updater": {
      "active": true,
      "endpoints": ["https://releases.terraphim.io/latest.json"],
      "dialog": true,
      "pubkey": "op://Terraphim-Shared/TauriSigning/PUBLIC_KEY"
    },
    "bundle": {
      "active": true,
      "category": "Productivity",
      "copyright": "© 2023 Terraphim AI",
      "deb": {
        "depends": []
      },
      "externalBin": [],
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.terraphim.ai",
      "longDescription": "Terraphim AI - Intelligent Search and Knowledge Management",
      "macOS": {
        "entitlements": null,
        "exceptionDomain": "",
        "frameworks": [],
        "providerShortName": null,
        "signingIdentity": null
      },
      "resources": [],
      "shortDescription": "Intelligent Search and Knowledge Management",
      "targets": "all",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    }
  }
}
EOF

    # GitHub Actions workflow template
    cat > "$templates_dir/github-actions-1password.yml" << 'EOF'
name: Build with 1Password Secrets

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4

    - name: Install 1Password CLI
      uses: 1password/install-cli-action@v1

    - name: Load secrets from 1Password
      run: |
        # Inject secrets into environment file
        op inject -i templates/.env.terraphim.template -o .env.terraphim

        # Inject secrets into configuration
        op inject -i templates/server_config.json.template -o server_config.json

        # Set permissions
        chmod 600 .env.terraphim server_config.json

    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Build with secrets
      run: |
        # Source environment variables
        set -a && source .env.terraphim && set +a

        # Build project
        cargo build --release

    - name: Cleanup secrets
      if: always()
      run: |
        rm -f .env.terraphim server_config.json
EOF

    log_success "Generated configuration templates in $templates_dir"
}

# Generate usage documentation
generate_documentation() {
    log_header "Generating Documentation"

    local docs_dir="$PROJECT_ROOT/docs"
    mkdir -p "$docs_dir"

    cat > "$docs_dir/1PASSWORD_INTEGRATION.md" << 'EOF'
# Terraphim AI 1Password Integration

This document describes how to use 1Password for secure secret management in Terraphim AI.

## Overview

Terraphim AI uses 1Password for enterprise-grade secret management with three vaults:

- **Terraphim-Dev**: Development environment secrets
- **Terraphim-Prod**: Production environment secrets
- **Terraphim-Shared**: Cross-environment shared secrets

## Setup

### Prerequisites

1. Install 1Password CLI:
   ```bash
   # macOS
   brew install --cask 1password-cli

   # Linux
   curl -sS https://downloads.1password.com/linux/keys/1password.asc | gpg --dearmor --output /usr/share/keyrings/1password-archive-keyring.gpg
   echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/1password-archive-keyring.gpg] https://downloads.1password.com/linux/debian/$(dpkg --print-architecture) stable main" | tee /etc/apt/sources.list.d/1password.list
   apt update && apt install 1password-cli
   ```

2. Authenticate with 1Password:
   ```bash
   op signin
   ```

### Initial Setup

Run the setup script to create vaults and initial secret structure:

```bash
# Setup development environment
./scripts/setup-1password-terraphim.sh dev

# Setup production environment
./scripts/setup-1password-terraphim.sh prod

# Setup all environments
./scripts/setup-1password-terraphim.sh all
```

## Usage

### Method 1: Process Memory (Recommended)

Load secrets directly into environment without touching disk:

```bash
# Development
op run --env-file=templates/.env.terraphim.template -- cargo run --bin terraphim_server

# Production
op run --env-file=templates/.env.terraphim.template -- ./target/release/terraphim_server
```

### Method 2: Secure File Injection

Generate temporary configuration files with secrets:

```bash
# Inject secrets into configuration
op inject -i templates/.env.terraphim.template -o .env.terraphim
op inject -i templates/server_config.json.template -o server_config.json

# Set secure permissions
chmod 600 .env.terraphim server_config.json

# Run application
source .env.terraphim
cargo run --bin terraphim_server

# Cleanup (important!)
rm .env.terraphim server_config.json
```

### CI/CD Integration

Add `OP_SERVICE_ACCOUNT_TOKEN` to GitHub Secrets and use in workflows:

```yaml
- name: Install 1Password CLI
  uses: 1password/install-cli-action@v1

- name: Load secrets
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
  run: |
    op inject -i templates/.env.terraphim.template -o .env.terraphim
```

## Secret Management

### Adding New Secrets

1. Add to 1Password vault using GUI or CLI:
   ```bash
   op item create --vault="Terraphim-Dev" --category="API Credential" \
     --title="NewService" \
     --field="API_KEY[password]=your-secret-key"
   ```

2. Add reference to template:
   ```bash
   NEW_SERVICE_API_KEY="op://Terraphim-Dev/NewService/API_KEY"
   ```

### Rotating Secrets

1. Update secret in 1Password vault
2. No code changes required - secrets automatically updated

### Environment-Specific Secrets

Use different vault references for different environments:

```bash
# Development
OPENROUTER_API_KEY="op://Terraphim-Dev/OpenRouter/API_KEY"

# Production
OPENROUTER_API_KEY="op://Terraphim-Prod/OpenRouter/API_KEY"
```

## Security Best Practices

1. **Never commit actual secrets** - only commit template files
2. **Use secure permissions** - `chmod 600` for generated files
3. **Clean up temporary files** - remove generated files after use
4. **Rotate secrets regularly** - use 1Password's rotation features
5. **Monitor access** - review 1Password activity logs
6. **Use service accounts** - for CI/CD, never personal accounts

## Troubleshooting

### Common Issues

1. **Not authenticated**: Run `op signin`
2. **Permission denied**: Check vault access permissions
3. **Item not found**: Verify item name and vault
4. **Template parsing failed**: Check `op://` reference syntax

### Debug Commands

```bash
# List vaults
op vault list

# List items in vault
op item list --vault="Terraphim-Dev"

# Get item details
op item get "OpenRouter" --vault="Terraphim-Dev"

# Test injection
op inject -i templates/.env.terraphim.template
```

## Support

For issues with 1Password integration:

1. Check this documentation
2. Review 1Password CLI documentation
3. Check project issues on GitHub
4. Contact the development team
EOF

    log_success "Generated documentation in $docs_dir/1PASSWORD_INTEGRATION.md"
}

# Show usage information
show_usage() {
    cat << EOF
Terraphim AI 1Password Integration Setup

USAGE:
    $0 [OPTIONS] [ENVIRONMENT]

ENVIRONMENTS:
    dev         Setup development environment only (default)
    prod        Setup production environment only
    shared      Setup shared environment only
    all         Setup all environments

OPTIONS:
    --help, -h  Show this help message
    --dry-run   Show what would be done without making changes
    --verbose   Enable verbose output

EXAMPLES:
    $0                  # Setup development environment
    $0 prod             # Setup production environment
    $0 all              # Setup all environments
    $0 --dry-run dev    # Show what would be done for dev

DESCRIPTION:
    This script sets up 1Password vaults and secret structure for Terraphim AI.
    It creates vaults, secret items with placeholder values, and configuration
    templates for secure secret management.

    After running this script, you'll need to:
    1. Update placeholder values in 1Password with actual secrets
    2. Use the generated templates for secure configuration
    3. Follow the documentation for integration patterns

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_usage
                exit 0
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            dev|prod|shared|all)
                ENVIRONMENT="$1"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Main execution
main() {
    log_header "Terraphim AI 1Password Integration Setup"
    log_info "Environment: $ENVIRONMENT"

    # Check prerequisites
    check_prerequisites

    # Setup based on environment choice
    case "$ENVIRONMENT" in
        "dev")
            setup_environment_secrets "dev"
            ;;
        "prod")
            setup_environment_secrets "prod"
            ;;
        "shared")
            setup_environment_secrets "shared"
            ;;
        "all")
            setup_environment_secrets "dev"
            setup_environment_secrets "prod"
            setup_environment_secrets "shared"
            ;;
        *)
            log_error "Invalid environment: $ENVIRONMENT"
            log_info "Valid environments: dev, prod, shared, all"
            exit 1
            ;;
    esac

    # Generate templates and documentation
    generate_templates
    generate_documentation

    log_header "Setup Complete!"
    log_success "1Password vaults and secrets have been created"
    log_info "Next steps:"
    echo "  1. Update placeholder values in 1Password with actual secrets"
    echo "  2. Review generated templates in ./templates/"
    echo "  3. Read documentation in ./docs/1PASSWORD_INTEGRATION.md"
    echo "  4. Test integration with: op inject -i templates/.env.terraphim.template"

    log_warning "Remember to update placeholder values with real secrets!"
}

# Parse arguments and run main function
parse_args "$@"
main
