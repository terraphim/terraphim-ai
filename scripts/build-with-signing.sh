#!/bin/bash
# build-with-signing.sh - Build Tauri app with 1Password secret injection

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DESKTOP_DIR="$PROJECT_ROOT/desktop"
TAURI_DIR="$DESKTOP_DIR/src-tauri"

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
    
    if [[ ! -f "$PROJECT_ROOT/.env.tauri-release" ]]; then
        print_error "Environment template not found: .env.tauri-release"
        exit 1
    fi
    
    if [[ ! -f "$TAURI_DIR/tauri.conf.json.template" ]]; then
        print_error "Tauri config template not found: tauri.conf.json.template"
        exit 1
    fi
    
    print_success "Prerequisites validated"
}

inject_tauri_config() {
    print_info "Injecting secrets into Tauri configuration..."
    
    # Create temporary config with injected secrets
    TEMP_CONFIG=$(mktemp)
    trap "rm -f '$TEMP_CONFIG'" EXIT
    
    if ! op inject -i "$TAURI_DIR/tauri.conf.json.template" -o "$TEMP_CONFIG"; then
        print_error "Failed to inject secrets into Tauri config"
        exit 1
    fi
    
    # Replace the main config with injected version
    cp "$TEMP_CONFIG" "$TAURI_DIR/tauri.conf.json"
    chmod 600 "$TAURI_DIR/tauri.conf.json"
    
    print_success "Injected secrets into Tauri configuration"
}

restore_tauri_config() {
    print_info "Restoring original Tauri configuration..."
    
    # Reset to placeholder version
    sed -i '' 's/"pubkey": ".*"/"pubkey": "PLACEHOLDER_PUBLIC_KEY"/' "$TAURI_DIR/tauri.conf.json"
    
    print_success "Restored original configuration"
}

build_tauri_app() {
    print_info "Building Tauri application with signing..."
    
    cd "$DESKTOP_DIR"
    
    # Use op run to inject environment variables and build
    if ! op run --env-file="$PROJECT_ROOT/.env.tauri-release" -- yarn run tauri build "$@"; then
        print_error "Tauri build failed"
        restore_tauri_config
        exit 1
    fi
    
    print_success "Tauri build completed successfully"
}

main() {
    print_info "🔐 Building Terraphim Desktop with 1Password signing"
    
    validate_prerequisites
    inject_tauri_config
    
    # Set up cleanup trap
    trap 'restore_tauri_config' EXIT
    
    build_tauri_app "$@"
    
    print_success "🎉 Build completed successfully!"
    print_info "Artifacts location: $DESKTOP_DIR/target/release/bundle/"
}

main "$@"