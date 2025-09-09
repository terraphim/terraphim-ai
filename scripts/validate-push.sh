#!/bin/bash
#
# Comprehensive validation script for Terraphim AI
# Validates commits before pushing to public repository
# Can be used standalone or integrated with CI/CD
#
# Usage: ./scripts/validate-push.sh [branch] [target-remote]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VALIDATION_LOG="$REPO_ROOT/.git/validation-audit.log"

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

# Function to log validation attempts
log_validation() {
    local result=$1
    local branch=$2
    local remote=$3
    local reason=$4
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $result: Branch '$branch' to '$remote' - $reason" >> "$VALIDATION_LOG"
}

# Function to check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_status "FAIL" "Not in a git repository"
        exit 1
    fi
}

# Function to validate branch naming conventions
validate_branch_naming() {
    local branch=$1
    local target_remote=$2
    
    print_status "INFO" "Validating branch naming for '$branch' -> '$target_remote'"
    
    # Define private branch patterns
    local private_patterns=(
        "^private-"
        "^internal-"
        "^client-"
        "^secret-"
        "^wip-private-"
        "^customer-"
        "^proprietary-"
        "^confidential-"
    )
    
    # Check if pushing to public remote
    local public_remotes=("origin" "upstream" "public")
    local is_public=false
    
    for remote in "${public_remotes[@]}"; do
        if [[ "$target_remote" == "$remote" ]]; then
            is_public=true
            break
        fi
    done
    
    if [[ "$is_public" = true ]]; then
        for pattern in "${private_patterns[@]}"; do
            if [[ "$branch" =~ $pattern ]]; then
                print_status "FAIL" "Branch '$branch' matches private pattern '$pattern'"
                log_validation "FAIL" "$branch" "$target_remote" "Private branch pattern"
                return 1
            fi
        done
        
        # Legacy private branches
        local legacy_private=("sqlite_haystack" "private-feature" "internal-docs" "customer-data")
        for private_branch in "${legacy_private[@]}"; do
            if [[ "$branch" == "$private_branch" ]]; then
                print_status "FAIL" "Branch '$branch' is explicitly marked as private"
                log_validation "FAIL" "$branch" "$target_remote" "Legacy private branch"
                return 1
            fi
        done
    fi
    
    print_status "SUCCESS" "Branch naming validation passed"
    return 0
}

# Function to scan commit messages for private markers
scan_commit_messages() {
    local branch=$1
    local target_remote=$2
    
    print_status "INFO" "Scanning commit messages for private markers"
    
    # Determine range of commits to check
    local range="origin/${branch}..HEAD"
    if ! git rev-parse --verify "origin/${branch}" >/dev/null 2>&1; then
        # If remote branch doesn't exist, check last 50 commits
        range="HEAD~50..HEAD"
    fi
    
    # Private markers to search for
    local private_markers=(
        "\[PRIVATE\]"
        "\[INTERNAL\]"
        "\[CLIENT\]"
        "\[DO-NOT-PUSH\]"
        "\[CONFIDENTIAL\]"
        "\[CUSTOMER\]"
        "private:"
        "internal:"
        "client:"
        "secret:"
        "confidential:"
        "zestic-only:"
        "customer-specific:"
    )
    
    # Create regex pattern
    local pattern=$(IFS="|"; echo "${private_markers[*]}")
    
    # Check commits
    local violations=$(git log --oneline $range 2>/dev/null | grep -i -E "$pattern" || true)
    
    if [[ -n "$violations" ]]; then
        print_status "FAIL" "Found commits with private markers:"
        echo "$violations"
        log_validation "FAIL" "$branch" "$target_remote" "Private markers in commits"
        return 1
    fi
    
    print_status "SUCCESS" "No private markers found in commit messages"
    return 0
}

# Function to check for sensitive files
check_sensitive_files() {
    local branch=$1
    local target_remote=$2
    
    print_status "INFO" "Checking for sensitive file patterns"
    
    # Determine range of commits to check
    local range="origin/${branch}..HEAD"
    if ! git rev-parse --verify "origin/${branch}" >/dev/null 2>&1; then
        range="HEAD~50..HEAD"
    fi
    
    # Check .gitprivateignore file
    if [[ -f "$REPO_ROOT/.gitprivateignore" ]]; then
        local violations=""
        local exclusions=()
        local patterns=()
        
        # First pass: collect patterns and exclusions
        while IFS= read -r line || [[ -n "$line" ]]; do
            # Skip comments and empty lines
            [[ "$line" =~ ^#.*$ || -z "$line" ]] && continue
            
            if [[ "$line" =~ ^! ]]; then
                # This is an exclusion pattern (remove the !)
                exclusions+=("${line#!}")
            else
                # This is a regular pattern
                patterns+=("$line")
            fi
        done < "$REPO_ROOT/.gitprivateignore"
        
        # Second pass: check patterns and apply exclusions
        for pattern in "${patterns[@]}"; do
            local matches=$(git diff --name-only $range 2>/dev/null | grep "$pattern" || true)
            
            if [[ -n "$matches" ]]; then
                # Apply exclusions
                local filtered_matches=""
                while IFS= read -r match; do
                    [[ -z "$match" ]] && continue
                    
                    local excluded=false
                    for exclusion in "${exclusions[@]}"; do
                        if [[ "$match" =~ $exclusion ]]; then
                            excluded=true
                            break
                        fi
                    done
                    
                    if [[ "$excluded" = false ]]; then
                        filtered_matches="$filtered_matches$match\n"
                    fi
                done <<< "$matches"
                
                if [[ -n "$filtered_matches" ]]; then
                    violations="$violations\n$pattern: $filtered_matches"
                fi
            fi
        done
        
        if [[ -n "$violations" ]]; then
            print_status "FAIL" "Sensitive file patterns found:"
            echo -e "$violations"
            log_validation "FAIL" "$branch" "$target_remote" "Sensitive files detected"
            return 1
        fi
        
        print_status "SUCCESS" "No sensitive file patterns found"
    else
        print_status "WARN" "No .gitprivateignore file found"
    fi
    
    return 0
}

# Function to scan file contents for sensitive data
scan_file_contents() {
    local branch=$1
    local target_remote=$2
    
    print_status "INFO" "Scanning file contents for sensitive data"
    
    # Determine range of commits to check
    local range="origin/${branch}..HEAD"
    if ! git rev-parse --verify "origin/${branch}" >/dev/null 2>&1; then
        range="HEAD~50..HEAD"
    fi
    
    # Sensitive keywords to search for
    local sensitive_keywords=(
        "customer_api_key"
        "client_secret"
        "private_key_prod"
        "internal_endpoint"
        "zestic.*api.*key"
        "customer.*password"
        "prod.*secret"
        "production.*key"
        "internal.*token"
        "confidential.*key"
        "proprietary.*secret"
    )
    
    # Check each keyword
    for keyword in "${sensitive_keywords[@]}"; do
        if git diff $range 2>/dev/null | grep -i -q "$keyword"; then
            print_status "FAIL" "Sensitive keyword '$keyword' found in file changes"
            
            # Show context for debugging (but not the actual sensitive data)
            print_status "INFO" "Files containing sensitive content:"
            git diff --name-only $range 2>/dev/null | xargs grep -l -i "$keyword" 2>/dev/null || true
            
            log_validation "FAIL" "$branch" "$target_remote" "Sensitive keyword: $keyword"
            return 1
        fi
    done
    
    print_status "SUCCESS" "No sensitive keywords found in file contents"
    return 0
}

# Function to check commit author and email
check_commit_metadata() {
    local branch=$1
    local target_remote=$2
    
    print_status "INFO" "Checking commit metadata"
    
    # Determine range of commits to check
    local range="origin/${branch}..HEAD"
    if ! git rev-parse --verify "origin/${branch}" >/dev/null 2>&1; then
        range="HEAD~10..HEAD"  # Check last 10 commits
    fi
    
    # Check for internal email domains that shouldn't appear in public
    local internal_domains=("zestic.ai" "internal.terraphim" "customer.local")
    
    for domain in "${internal_domains[@]}"; do
        local violations=$(git log --format="%H %ae %ce" $range 2>/dev/null | grep "$domain" || true)
        if [[ -n "$violations" ]]; then
            print_status "FAIL" "Internal email domain '$domain' found in commits"
            echo "$violations"
            log_validation "FAIL" "$branch" "$target_remote" "Internal email domain: $domain"
            return 1
        fi
    done
    
    print_status "SUCCESS" "Commit metadata validation passed"
    return 0
}

# Function to generate validation report
generate_report() {
    local branch=$1
    local target_remote=$2
    local result=$3
    
    local report_file="$REPO_ROOT/.git/validation-report-$(date +%Y%m%d-%H%M%S).txt"
    
    cat > "$report_file" << EOF
TERRAPHIM AI VALIDATION REPORT
==============================
Date: $(date)
Branch: $branch
Target Remote: $target_remote
Result: $result

Repository Info:
- Current HEAD: $(git rev-parse HEAD)
- Commits being validated: $(git rev-list --count HEAD ^origin/$branch 2>/dev/null || echo "N/A")
- Modified files: $(git diff --name-only origin/$branch..HEAD 2>/dev/null | wc -l || echo "N/A")

Validation Steps Performed:
1. Branch naming conventions ✓
2. Commit message scanning ✓
3. Sensitive file detection ✓
4. File content analysis ✓
5. Commit metadata check ✓

$(if [[ "$result" = "PASS" ]]; then
    echo "✅ All validation checks passed"
    echo "✅ Safe to push to public remote"
else
    echo "❌ Validation failed"
    echo "❌ DO NOT push to public remote"
    echo ""
    echo "To fix issues:"
    echo "1. Remove sensitive content from commits"
    echo "2. Update commit messages to remove private markers"
    echo "3. Rename branch if it matches private patterns"
    echo "4. Push to private remote instead: git push private $branch"
fi)

For detailed logs, see: $VALIDATION_LOG
EOF

    print_status "INFO" "Validation report saved to: $report_file"
}

# Main validation function
main() {
    local branch=${1:-$(git symbolic-ref --short HEAD 2>/dev/null || echo "HEAD")}
    local target_remote=${2:-"origin"}
    
    echo "🔍 Terraphim AI Repository Validation"
    echo "======================================"
    print_status "INFO" "Validating branch '$branch' for push to '$target_remote'"
    print_status "INFO" "Repository: $(pwd)"
    echo ""
    
    # Check if we're in a git repository
    check_git_repo
    
    # Change to repository root
    cd "$REPO_ROOT"
    
    local validation_failed=false
    
    # Run all validation checks
    validate_branch_naming "$branch" "$target_remote" || validation_failed=true
    scan_commit_messages "$branch" "$target_remote" || validation_failed=true
    check_sensitive_files "$branch" "$target_remote" || validation_failed=true
    scan_file_contents "$branch" "$target_remote" || validation_failed=true
    check_commit_metadata "$branch" "$target_remote" || validation_failed=true
    
    echo ""
    
    if [[ "$validation_failed" = true ]]; then
        print_status "FAIL" "Validation FAILED - DO NOT push to public remote"
        log_validation "FAIL" "$branch" "$target_remote" "Multiple validation failures"
        generate_report "$branch" "$target_remote" "FAIL"
        exit 1
    else
        print_status "SUCCESS" "All validations PASSED - Safe to push"
        log_validation "PASS" "$branch" "$target_remote" "All checks passed"
        generate_report "$branch" "$target_remote" "PASS"
        exit 0
    fi
}

# Help function
show_help() {
    cat << EOF
Terraphim AI Repository Validation Script
==========================================

Usage: $0 [OPTIONS] [BRANCH] [REMOTE]

Arguments:
  BRANCH    Branch to validate (default: current branch)
  REMOTE    Target remote (default: origin)

Options:
  -h, --help    Show this help message

Examples:
  $0                                 # Validate current branch for origin
  $0 feature-branch                  # Validate specific branch for origin
  $0 feature-branch public           # Validate specific branch for specific remote

Validation Checks:
  1. Branch naming conventions (prevents private-* branches to public remotes)
  2. Commit message scanning (looks for [PRIVATE], [INTERNAL] etc.)
  3. Sensitive file detection (uses .gitprivateignore patterns)
  4. File content analysis (scans for API keys, secrets)
  5. Commit metadata verification (checks for internal email domains)

Exit Codes:
  0 - All validations passed
  1 - One or more validations failed

For more information, see SECURITY.md
EOF
}

# Parse command line arguments
case "${1:-}" in
    -h|--help)
        show_help
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac