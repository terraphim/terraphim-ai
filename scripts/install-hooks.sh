#!/bin/bash
#
# Install script for pre-commit hooks in Terraphim AI
# Supports multiple hook managers: pre-commit, prek, lefthook, or native Git hooks
#
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "SUCCESS" ]; then
        echo -e "${GREEN}✓${NC} $message"
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}✗${NC} $message"
    elif [ "$status" = "INFO" ]; then
        echo -e "${BLUE}ℹ${NC} $message"
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠${NC} $message"
    else
        echo -e "$message"
    fi
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

print_status "INFO" "Setting up git hooks for Terraphim AI..."
echo ""

# Check if we're in a git repository
if [ ! -d ".git" ]; then
    print_status "FAIL" "Not in a git repository! Please run this script from the project root."
    exit 1
fi

# Ensure hooks directory exists
mkdir -p .git/hooks

# Function to install native hooks
install_native_hooks() {
    print_status "INFO" "Installing native Git hooks..."
    
    if [ -f "scripts/hooks/pre-commit" ]; then
        cp scripts/hooks/pre-commit .git/hooks/pre-commit
        chmod +x .git/hooks/pre-commit
        print_status "SUCCESS" "Installed native pre-commit hook"
    else
        print_status "WARN" "Native pre-commit hook not found at scripts/hooks/pre-commit"
    fi
    
    if [ -f "scripts/hooks/commit-msg" ]; then
        cp scripts/hooks/commit-msg .git/hooks/commit-msg
        chmod +x .git/hooks/commit-msg
        print_status "SUCCESS" "Installed native commit-msg hook"
    else
        print_status "WARN" "Native commit-msg hook not found at scripts/hooks/commit-msg"
    fi
}

# Function to setup Biome
setup_biome() {
    if [ -d "desktop" ] && [ -f "desktop/package.json" ]; then
        print_status "INFO" "Setting up Biome for JavaScript/TypeScript..."
        cd desktop
        
        # Check if Biome is already installed
        if ! command_exists npx || ! npx @biomejs/biome --version >/dev/null 2>&1; then
            print_status "INFO" "Installing Biome..."
            if command_exists npm; then
                npm install --save-dev @biomejs/biome
                print_status "SUCCESS" "Biome installed via npm"
            elif command_exists yarn; then
                yarn add --dev @biomejs/biome
                print_status "SUCCESS" "Biome installed via yarn"
            else
                print_status "WARN" "Neither npm nor yarn found, please install Biome manually:"
                print_status "INFO" "  npm install --save-dev @biomejs/biome"
            fi
        else
            print_status "SUCCESS" "Biome is already available"
        fi
        
        cd ..
    else
        print_status "INFO" "Desktop directory not found, skipping Biome setup"
    fi
}

# Function to create secrets baseline
create_secrets_baseline() {
    if [ ! -f ".secrets.baseline" ]; then
        print_status "INFO" "Creating secrets baseline..."
        cat > .secrets.baseline << 'EOF'
{
  "version": "1.4.0",
  "plugins_used": [
    {
      "name": "ArtifactoryDetector"
    },
    {
      "name": "AWSKeyDetector"
    },
    {
      "name": "Base64HighEntropyString",
      "limit": 4.5
    },
    {
      "name": "BasicAuthDetector"
    },
    {
      "name": "CloudantDetector"
    },
    {
      "name": "GitHubTokenDetector"
    },
    {
      "name": "HexHighEntropyString",
      "limit": 3.0
    },
    {
      "name": "IbmCloudIamDetector"
    },
    {
      "name": "IbmCosHmacDetector"
    },
    {
      "name": "JwtTokenDetector"
    },
    {
      "name": "KeywordDetector",
      "keyword_exclude": ""
    },
    {
      "name": "MailchimpDetector"
    },
    {
      "name": "PrivateKeyDetector"
    },
    {
      "name": "SlackDetector"
    },
    {
      "name": "SoftlayerDetector"
    },
    {
      "name": "StripeDetector"
    },
    {
      "name": "TwilioKeyDetector"
    }
  ],
  "filters_used": [
    {
      "path": "detect_secrets.filters.allowlist.is_line_allowlisted"
    },
    {
      "path": "detect_secrets.filters.common.is_baseline_file"
    },
    {
      "path": "detect_secrets.filters.common.is_ignored_due_to_verification_policies",
      "min_level": 2
    },
    {
      "path": "detect_secrets.filters.heuristic.is_indirect_reference"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_likely_id_string"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_lock_file"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_not_alphanumeric_string"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_potential_uuid"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_prefixed_with_dollar_sign"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_sequential_string"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_swagger_file"
    },
    {
      "path": "detect_secrets.filters.heuristic.is_templated_secret"
    }
  ],
  "results": {},
  "generated_at": "2024-01-01T00:00:00Z"
}
EOF
        print_status "SUCCESS" "Created .secrets.baseline file"
    else
        print_status "INFO" "Secrets baseline already exists"
    fi
}

# Main installation logic
HOOK_MANAGER_INSTALLED=false

# Check for various hook managers and install
if command_exists pre-commit; then
    print_status "SUCCESS" "Found pre-commit, installing hooks..."
    pre-commit install
    pre-commit install --hook-type commit-msg
    HOOK_MANAGER_INSTALLED=true
    print_status "SUCCESS" "pre-commit hooks installed"
    
elif command_exists prek; then
    print_status "SUCCESS" "Found prek, installing hooks..."
    prek install
    prek install --hook-type commit-msg
    HOOK_MANAGER_INSTALLED=true
    print_status "SUCCESS" "prek hooks installed"
    
elif command_exists lefthook; then
    print_status "SUCCESS" "Found lefthook, installing hooks..."
    lefthook install
    HOOK_MANAGER_INSTALLED=true
    print_status "SUCCESS" "lefthook hooks installed"
    
    # Create lefthook configuration if it doesn't exist
    if [ ! -f "lefthook.yml" ]; then
        print_status "INFO" "Creating lefthook.yml configuration..."
        cat > lefthook.yml << 'EOF'
pre-commit:
  parallel: true
  commands:
    trailing-whitespace:
      run: git diff --cached --check
      stage_fixed: true
    
    cargo-fmt:
      glob: "*.rs"
      run: cargo fmt --all -- --check
      stage_fixed: true
    
    cargo-clippy:
      glob: "*.rs"
      run: cargo clippy --workspace --all-targets --all-features -- -D warnings
    
    biome:
      glob: "desktop/**/*.{js,ts,tsx,jsx,json,jsonc}"
      root: desktop/
      run: npx @biomejs/biome check --write false --no-errors-on-unmatched
      stage_fixed: true
    
    secrets:
      run: ./scripts/hooks/pre-commit

commit-msg:
  commands:
    conventional:
      run: ./scripts/hooks/commit-msg {1}
EOF
        print_status "SUCCESS" "Created lefthook.yml configuration"
    fi
    
else
    print_status "WARN" "No hook manager found (pre-commit, prek, or lefthook)"
    print_status "INFO" "Installing native Git hooks as fallback..."
fi

# Always install native hooks as fallback
install_native_hooks

# Setup additional dependencies
setup_biome
create_secrets_baseline

echo ""
print_status "SUCCESS" "Hook installation complete!"
echo ""

# Provide installation instructions for missing tools
if [ "$HOOK_MANAGER_INSTALLED" = false ]; then
    print_status "INFO" "For better hook management, consider installing one of these tools:"
    echo ""
    print_status "INFO" "Option 1 - prek (Rust-based, no Python required):"
    echo "  curl --proto '=https' --tlsv1.2 -LsSf \\"
    echo "    https://github.com/j178/prek/releases/download/v0.1.4/prek-installer.sh | sh"
    echo "  Then run: prek install"
    echo ""
    print_status "INFO" "Option 2 - lefthook (Go-based, single binary):"
    echo "  curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh"
    echo "  Then run: lefthook install"
    echo ""
    print_status "INFO" "Option 3 - pre-commit (Python-based):"
    echo "  pip install pre-commit"
    echo "  Then run: pre-commit install"
    echo ""
fi

# Show usage information
print_status "INFO" "Usage:"
echo "  • Hooks will run automatically on git commit"
echo "  • Run manually: ./scripts/hooks/pre-commit"
echo "  • Skip hooks (emergency): git commit --no-verify"
echo "  • Check all files: pre-commit run --all-files (if using pre-commit/prek)"

# Check if Rust tools are available
if ! command_exists cargo; then
    echo ""
    print_status "WARN" "Cargo not found - Rust checks will be skipped"
    print_status "INFO" "Install Rust: https://rustup.rs/"
fi

echo ""
print_status "SUCCESS" "Setup complete! Your commits will now be validated automatically."