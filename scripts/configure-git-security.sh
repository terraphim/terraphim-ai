#!/bin/bash
#
# Git Security Configuration Script for Terraphim AI
# Configures git settings to enhance security and prevent accidental pushes
#
# Usage: ./scripts/configure-git-security.sh [--apply]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "SUCCESS" ]; then
        echo -e "${GREEN}✓${NC} $message"
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}✗${NC} $message"
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠${NC} $message"
    elif [ "$status" = "INFO" ]; then
        echo -e "${BLUE}ℹ${NC} $message"
    else
        echo -e "$message"
    fi
}

# Function to check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_status "FAIL" "Not in a git repository"
        exit 1
    fi
}

# Function to backup current git config
backup_git_config() {
    local backup_file=".git/config.backup.$(date +%Y%m%d-%H%M%S)"
    cp .git/config "$backup_file"
    print_status "INFO" "Git config backed up to: $backup_file"
}

# Function to configure push defaults
configure_push_defaults() {
    local apply=${1:-false}
    
    print_status "INFO" "Configuring push defaults..."
    
    # Configuration commands
    local configs=(
        "push.default simple"
        "push.followTags false"
        "push.autoSetupRemote false"
        "branch.autoSetupMerge false"
        "branch.autoSetupRebase false"
        "pull.rebase false"
        "merge.ff only"
    )
    
    for config in "${configs[@]}"; do
        local key=$(echo "$config" | cut -d' ' -f1)
        local value=$(echo "$config" | cut -d' ' -f2-)
        
        local current_value=$(git config --local "$key" 2>/dev/null || echo "not set")
        
        if [[ "$apply" = true ]]; then
            git config --local "$key" "$value"
            print_status "SUCCESS" "Set $key = $value"
        else
            print_status "INFO" "Would set: $key = $value (current: $current_value)"
        fi
    done
}

# Function to configure branch-specific settings
configure_branch_settings() {
    local apply=${1:-false}
    
    print_status "INFO" "Configuring branch-specific settings..."
    
    # Get list of existing branches
    local branches=$(git branch -a | grep -E '(private-|internal-|client-|secret-)' | sed 's/^[* ] //' | sed 's/remotes\///' || true)
    
    if [[ -n "$branches" ]]; then
        while IFS= read -r branch; do
            # Skip remote branches that are just references
            [[ "$branch" =~ ^(origin|private|upstream)/ ]] && continue
            
            # Clean branch name
            branch=$(echo "$branch" | sed 's/origin\///' | sed 's/private\///')
            
            if [[ "$apply" = true ]]; then
                # Set private branches to push to private remote by default
                git config --local "branch.$branch.remote" "private"
                git config --local "branch.$branch.pushRemote" "private"
                print_status "SUCCESS" "Configured branch '$branch' to push to private remote"
            else
                print_status "INFO" "Would configure branch '$branch' to push to private remote"
            fi
        done <<< "$branches"
    else
        print_status "INFO" "No private branches found to configure"
    fi
    
    # Configure public branches to use origin
    local public_branches=$(git branch | grep -E '^[* ] (main|master|develop|feature-|fix-|docs-|release-)' | sed 's/^[* ] //' || true)
    
    if [[ -n "$public_branches" ]]; then
        while IFS= read -r branch; do
            if [[ "$apply" = true ]]; then
                git config --local "branch.$branch.remote" "origin"
                git config --local "branch.$branch.pushRemote" "origin"
                print_status "SUCCESS" "Configured branch '$branch' to push to origin remote"
            else
                print_status "INFO" "Would configure branch '$branch' to push to origin remote"
            fi
        done <<< "$public_branches"
    fi
}

# Function to set up push URL restrictions
configure_push_urls() {
    local apply=${1:-false}
    
    print_status "INFO" "Configuring push URL restrictions..."
    
    # Get current remotes
    local origin_url=$(git remote get-url origin 2>/dev/null || echo "not configured")
    local private_url=$(git remote get-url private 2>/dev/null || echo "not configured")
    
    print_status "INFO" "Current remotes:"
    print_status "INFO" "  origin: $origin_url"
    print_status "INFO" "  private: $private_url"
    
    # Validate remote URLs
    if [[ "$origin_url" == *"terraphim/terraphim-ai"* ]]; then
        print_status "SUCCESS" "Origin remote correctly points to public repository"
    else
        print_status "WARN" "Origin remote may not be configured correctly"
    fi
    
    if [[ "$private_url" == *"zestic-ai/terraphim-private"* ]]; then
        print_status "SUCCESS" "Private remote correctly configured"
    else
        print_status "WARN" "Private remote may not be configured correctly"
    fi
}

# Function to create git aliases for safe pushing
create_git_aliases() {
    local apply=${1:-false}
    
    print_status "INFO" "Creating git aliases for safe operations..."
    
    local aliases=(
        "alias.push-public !git push origin"
        "alias.push-private !git push private" 
        "alias.validate-push !./scripts/validate-push.sh"
        "alias.safe-push !./scripts/validate-push.sh && git push"
        "alias.check-private !git log --oneline | grep -iE '(private|internal|secret|client)' || echo 'No private markers found'"
    )
    
    for alias_config in "${aliases[@]}"; do
        local key=$(echo "$alias_config" | cut -d' ' -f1)
        local value=$(echo "$alias_config" | cut -d' ' -f2-)
        
        if [[ "$apply" = true ]]; then
            git config --local "$key" "$value"
            local alias_name=$(echo "$key" | sed 's/alias\.//')
            print_status "SUCCESS" "Created alias: git $alias_name"
        else
            local alias_name=$(echo "$key" | sed 's/alias\.//')
            print_status "INFO" "Would create alias: git $alias_name"
        fi
    done
}

# Function to show current configuration
show_current_config() {
    print_status "INFO" "Current git security configuration:"
    echo ""
    
    # Show push settings
    echo "Push Settings:"
    echo "  push.default: $(git config push.default 2>/dev/null || echo "not set")"
    echo "  push.followTags: $(git config push.followTags 2>/dev/null || echo "not set")" 
    echo "  push.autoSetupRemote: $(git config push.autoSetupRemote 2>/dev/null || echo "not set")"
    echo ""
    
    # Show remotes
    echo "Configured Remotes:"
    git remote -v | sed 's/^/  /'
    echo ""
    
    # Show branch configurations
    echo "Branch Configurations:"
    for branch in $(git branch | sed 's/^[* ] //'); do
        local remote=$(git config "branch.$branch.remote" 2>/dev/null || echo "default")
        local pushRemote=$(git config "branch.$branch.pushRemote" 2>/dev/null || echo "not set")
        echo "  $branch -> remote: $remote, pushRemote: $pushRemote"
    done
    echo ""
    
    # Show aliases
    echo "Git Aliases:"
    git config --list | grep '^alias\.' | sed 's/^/  /' || echo "  No aliases configured"
    echo ""
}

# Function to validate hooks
validate_hooks() {
    print_status "INFO" "Validating git hooks..."
    
    if [[ -x ".git/hooks/pre-push" ]]; then
        print_status "SUCCESS" "Pre-push hook is installed and executable"
    else
        print_status "WARN" "Pre-push hook is not installed or not executable"
    fi
    
    if [[ -x ".git/hooks/pre-commit" ]]; then
        print_status "SUCCESS" "Pre-commit hook is installed and executable"  
    else
        print_status "WARN" "Pre-commit hook is not installed or not executable"
    fi
    
    if [[ -f ".gitprivateignore" ]]; then
        print_status "SUCCESS" ".gitprivateignore file exists"
        local pattern_count=$(grep -v '^#' .gitprivateignore | grep -v '^$' | wc -l)
        print_status "INFO" "  Contains $pattern_count patterns"
    else
        print_status "WARN" ".gitprivateignore file not found"
    fi
}

# Main function
main() {
    local apply=false
    
    # Parse arguments
    case "${1:-}" in
        --apply)
            apply=true
            ;;
        --help|-h)
            cat << EOF
Git Security Configuration Script for Terraphim AI
==================================================

Usage: $0 [OPTIONS]

Options:
  --apply       Actually apply the configuration changes
  --help, -h    Show this help message

Without --apply, this script will show what changes would be made
without actually applying them (dry-run mode).

This script configures:
1. Push defaults and safety settings
2. Branch-specific push remotes
3. Push URL restrictions
4. Helpful git aliases
5. Security validation

Run without --apply first to see what would be changed.
EOF
            exit 0
            ;;
        "")
            print_status "INFO" "Running in dry-run mode. Use --apply to actually make changes."
            ;;
        *)
            print_status "FAIL" "Unknown option: $1"
            print_status "INFO" "Use --help for usage information"
            exit 1
            ;;
    esac
    
    echo "🔧 Terraphim AI Git Security Configuration"
    echo "=========================================="
    echo ""
    
    # Check prerequisites
    check_git_repo
    
    # Backup config if applying changes
    if [[ "$apply" = true ]]; then
        backup_git_config
        echo ""
    fi
    
    # Show current state
    show_current_config
    
    # Configure settings
    configure_push_defaults "$apply"
    echo ""
    configure_branch_settings "$apply"
    echo ""
    configure_push_urls "$apply"
    echo ""
    create_git_aliases "$apply"
    echo ""
    validate_hooks
    echo ""
    
    if [[ "$apply" = true ]]; then
        print_status "SUCCESS" "Git security configuration applied successfully!"
        print_status "INFO" "You can now use these commands:"
        print_status "INFO" "  git safe-push         - Validate and push safely"
        print_status "INFO" "  git push-public       - Push to public remote (origin)"
        print_status "INFO" "  git push-private      - Push to private remote"
        print_status "INFO" "  git validate-push     - Run validation without pushing"
        print_status "INFO" "  git check-private     - Check for private markers in history"
    else
        print_status "INFO" "This was a dry-run. Use --apply to make actual changes."
        print_status "INFO" "Review the proposed changes above before applying."
    fi
}

# Run main function
main "$@"