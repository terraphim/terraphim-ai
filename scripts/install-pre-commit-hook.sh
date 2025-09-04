#!/bin/bash

# Pre-commit Hook Installation Script
# This script installs the API key detection pre-commit hook

set -e

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_info() {
    echo -e "INFO: $1"
}

# Get the git repository root
if ! GIT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    print_error "This script must be run from within a git repository"
    exit 1
fi

# Paths
HOOKS_DIR="$GIT_ROOT/.git/hooks"
PRE_COMMIT_HOOK="$HOOKS_DIR/pre-commit"
API_KEY_SCRIPT="$GIT_ROOT/scripts/check-api-keys.sh"

print_info "🔧 Installing API key detection pre-commit hook..."
echo ""

# Check if the API key detection script exists
if [[ ! -f "$API_KEY_SCRIPT" ]]; then
    print_error "API key detection script not found at: $API_KEY_SCRIPT"
    print_error "Please ensure the script exists before installing the hook."
    exit 1
fi

# Make sure the API key script is executable
if [[ ! -x "$API_KEY_SCRIPT" ]]; then
    print_info "Making API key detection script executable..."
    chmod +x "$API_KEY_SCRIPT"
    print_success "✅ Made $API_KEY_SCRIPT executable"
fi

# Check if hooks directory exists
if [[ ! -d "$HOOKS_DIR" ]]; then
    print_error "Git hooks directory not found: $HOOKS_DIR"
    print_error "Are you sure this is a git repository?"
    exit 1
fi

# Backup existing pre-commit hook if it exists
if [[ -f "$PRE_COMMIT_HOOK" ]]; then
    BACKUP_FILE="$PRE_COMMIT_HOOK.backup.$(date +%Y%m%d_%H%M%S)"
    print_warning "Existing pre-commit hook found"
    print_info "Backing up to: $BACKUP_FILE"
    cp "$PRE_COMMIT_HOOK" "$BACKUP_FILE"
    print_success "✅ Backup created"
fi

# Create the pre-commit hook
print_info "Creating new pre-commit hook..."

cat > "$PRE_COMMIT_HOOK" << 'EOF'
#!/bin/bash

# Git Pre-commit Hook - API Key Detection
# This hook runs the API key detection script before allowing commits

set -e

# Get the directory of this script (the .git/hooks directory)
HOOK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get the git repository root
GIT_ROOT="$(git rev-parse --show-toplevel)"

# Path to the API key detection script
API_KEY_SCRIPT="$GIT_ROOT/scripts/check-api-keys.sh"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

print_error() {
    echo -e "${RED}$1${NC}"
}

print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_info() {
    echo -e "$1"
}

# Check if the API key detection script exists
if [[ ! -f "$API_KEY_SCRIPT" ]]; then
    print_error "❌ API key detection script not found at: $API_KEY_SCRIPT"
    print_error "Please ensure the script exists and is executable."
    exit 1
fi

# Make sure the script is executable
if [[ ! -x "$API_KEY_SCRIPT" ]]; then
    print_error "❌ API key detection script is not executable: $API_KEY_SCRIPT"
    print_error "Run: chmod +x $API_KEY_SCRIPT"
    exit 1
fi

print_info "🛡️  Running pre-commit API key detection..."
echo ""

# Run the API key detection script
if "$API_KEY_SCRIPT"; then
    print_success "✅ Pre-commit hook passed - no API keys detected"
    echo ""
    exit 0
else
    print_error "❌ Pre-commit hook failed - API keys detected!"
    echo ""
    print_error "Commit aborted to prevent credential leakage."
    print_error "Please fix the issues above and try again."
    echo ""
    print_info "To bypass this check (NOT recommended):"
    print_info "git commit --no-verify"
    echo ""
    exit 1
fi
EOF

# Make the hook executable
chmod +x "$PRE_COMMIT_HOOK"

print_success "✅ Pre-commit hook installed successfully!"
echo ""

# Test the hook
print_info "🧪 Testing the pre-commit hook..."
if "$API_KEY_SCRIPT"; then
    print_success "✅ Hook test passed!"
else
    print_warning "⚠️  Hook test detected issues - check your code for API keys"
fi

echo ""
print_info "📋 Installation Summary:"
print_info "  • Pre-commit hook: $PRE_COMMIT_HOOK"
print_info "  • API key scanner: $API_KEY_SCRIPT"
print_info "  • Status: Active and ready"
echo ""
print_info "🛡️  Your repository is now protected against accidental API key commits!"
print_info "The hook will automatically scan for API keys before each commit."
echo ""
print_info "To temporarily bypass the hook (not recommended):"
print_info "  git commit --no-verify"
echo ""
print_info "To uninstall the hook:"
print_info "  rm $PRE_COMMIT_HOOK"
