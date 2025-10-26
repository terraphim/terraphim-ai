#!/bin/bash
#
# Setup enhanced pre-commit hooks for Terraphim AI
# Replaces default hooks with improved versions
#
set -euo pipefail

# Colors for output
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly RED='\033[0;31m'
readonly NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    case "$status" in
        "SUCCESS") echo -e "${GREEN}✓${NC} $message" ;;
        "WARN") echo -e "${YELLOW}⚠${NC} $message" ;;
        "FAIL") echo -e "${RED}✗${NC} $message" ;;
        *) echo -e "$message" ;;
    esac
}

# Check if we're in a git repository
if ! git rev-parse --git-dir >/dev/null 2>&1; then
    print_status "FAIL" "Not in a git repository"
    exit 1
fi

# Get git hooks directory
HOOKS_DIR=$(git rev-parse --git-dir)/hooks
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

print_status "INFO" "Setting up enhanced pre-commit hooks..."
print_status "INFO" "Hooks directory: $HOOKS_DIR"
print_status "INFO" "Scripts directory: $SCRIPT_DIR"

# Backup existing hooks
backup_hook() {
    local hook_name="$1"
    local hook_path="$HOOKS_DIR/$hook_name"

    if [ -f "$hook_path" ]; then
        local backup_path="${hook_path}.backup.$(date +%Y%m%d_%H%M%S)"
        cp "$hook_path" "$backup_path"
        print_status "INFO" "Backed up existing $hook_name to $(basename "$backup_path")"
    fi
}

# Install enhanced hooks
install_hook() {
    local hook_name="$1"
    local script_name="$2"
    local hook_path="$HOOKS_DIR/$hook_name"
    local script_path="$SCRIPT_DIR/$script_name"

    if [ ! -f "$script_path" ]; then
        print_status "FAIL" "Script not found: $script_path"
        return 1
    fi

    backup_hook "$hook_name"
    cp "$script_path" "$hook_path"
    chmod +x "$hook_path"
    print_status "SUCCESS" "Installed enhanced $hook_name hook"
}

# Install hooks
install_hook "pre-commit" "enhanced-pre-commit.sh"
install_hook "commit-msg" "enhanced-commit-msg.sh"

# Create configuration for enhanced hooks
cat > "$HOOKS_DIR/enhanced-hooks-config.yaml" << 'EOF'
# Enhanced Git Hooks Configuration for Terraphim AI
#
# This file configures the behavior of enhanced pre-commit and commit-msg hooks

# Pre-commit configuration
pre_commit:
  # File size limits
  max_file_size_kb: 1000

  # Timeout settings (seconds)
  rust_check_timeout: 60
  js_check_timeout: 30

  # Enable/disable specific checks
  checks:
    file_sizes: true
    secrets: true
    rust_code: true
    js_code: true
    trailing_whitespace: true
    config_syntax: true

# Commit message configuration
commit_msg:
  # Length limits
  max_title_length: 72
  min_title_length: 10
  max_body_line_length: 72

  # Enable/disable specific validations
  validations:
    conventional_format: true
    title_case: true
    description_length: true
    body_line_length: true
    breaking_changes: true
    issue_references: true

  # Suggestion level (strict, normal, relaxed)
  suggestion_level: normal

# Conventional commit types
commit_types:
  - feat      # New feature
  - fix       # Bug fix
  - docs      # Documentation only
  - style     # Formatting changes
  - refactor  # Code change without new features
  - perf      # Performance improvement
  - test      # Adding or fixing tests
  - chore     # Build process or dependency changes
  - build     # Build system changes
  - ci        # CI configuration changes
  - revert    # Revert previous commit

# Common scopes (examples)
common_scopes:
  - api       # API changes
  - ui        # User interface
  - config    # Configuration
  - deps      # Dependencies
  - docs      # Documentation
  - test      # Tests
  - build     # Build system
  - ci        # CI/CD
EOF

print_status "SUCCESS" "Enhanced hooks configuration created"

# Test the hooks
print_status "INFO" "Testing enhanced hooks..."

# Test pre-commit hook syntax
if bash -n "$HOOKS_DIR/pre-commit"; then
    print_status "SUCCESS" "Pre-commit hook syntax is valid"
else
    print_status "FAIL" "Pre-commit hook has syntax errors"
    exit 1
fi

# Test commit-msg hook syntax
if bash -n "$HOOKS_DIR/commit-msg"; then
    print_status "SUCCESS" "Commit-msg hook syntax is valid"
else
    print_status "FAIL" "Commit-msg hook has syntax errors"
    exit 1
fi

print_status "SUCCESS" "Enhanced pre-commit hooks setup completed!"
echo ""
print_status "INFO" "Features of enhanced hooks:"
echo "  • Faster and more reliable checks"
echo "  • Better error messages and suggestions"
echo "  • Configurable timeouts and limits"
echo "  • Enhanced commit message validation"
echo "  • Improved secret detection"
echo "  • Comprehensive Rust and JavaScript checks"
echo ""
print_status "INFO" "To customize settings, edit: $HOOKS_DIR/enhanced-hooks-config.yaml"
print_status "INFO" "To restore original hooks, use: git config core.hooksPath .git/hooks-original"
