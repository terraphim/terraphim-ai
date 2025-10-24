#!/bin/bash

# API Key Detection Pre-commit Hook
# This script scans for common API key patterns in staged files
# to prevent accidental credential commits

set -e

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Flag to track if any violations are found
VIOLATIONS_FOUND=false

# Function to print colored output
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

# API key patterns to detect
declare -A API_PATTERNS=(
    # Cloudflare patterns
    ["cloudflare_account_id"]="[0-9a-fA-F]{32}"
    ["cloudflare_api_token"]="[A-Za-z0-9_-]{40,}"
    ["cloudflare_global_api_key"]="[0-9a-fA-F]{37}"

    # AWS patterns
    ["aws_access_key"]="AKIA[0-9A-Z]{16}"
    ["aws_secret_key"]="[A-Za-z0-9/+=]{40}"

    # GitHub patterns
    ["github_token"]="gh[pousr]_[A-Za-z0-9_]{36,}"
    ["github_classic_token"]="ghp_[A-Za-z0-9]{36}"

    # Google API patterns
    ["google_api_key"]="AIza[0-9A-Za-z-_]{35}"

    # Generic patterns
    ["generic_api_key"]="[\"']?[aA][pP][iI][_-]?[kK][eE][yY][\"']?[[:space:]]*[:=][[:space:]]*[\"'][A-Za-z0-9_-]{16,}[\"']"
    ["generic_secret"]="[\"']?[sS][eE][cC][rR][eE][tT][\"']?[[:space:]]*[:=][[:space:]]*[\"'][A-Za-z0-9_-]{16,}[\"']"
    ["generic_token"]="[\"']?[tT][oO][kK][eE][nN][\"']?[[:space:]]*[:=][[:space:]]*[\"'][A-Za-z0-9_-]{16,}[\"']"

    # Hardcoded credential patterns (like the ones we just fixed)
    ["hardcoded_account_id"]="ACCOUNT_ID[[:space:]]*[:=][[:space:]]*[\"'][0-9a-fA-F]{32}[\"']"
    ["hardcoded_api_token"]="API_TOKEN[[:space:]]*[:=][[:space:]]*[\"'][A-Za-z0-9_-]{40,}[\"']"
)

# File extensions to check
FILE_EXTENSIONS=("js" "ts" "jsx" "tsx" "py" "rb" "go" "java" "php" "cs" "cpp" "c" "h" "hpp" "rs" "toml" "yaml" "yml" "json" "env" "config" "conf" "ini" "properties" "sh" "bash" "zsh" "fish")

# Files and directories to exclude
EXCLUDE_PATTERNS=(
    ".git/"
    "node_modules/"
    "target/"
    "dist/"
    "build/"
    ".vscode/"
    ".idea/"
    "*.log"
    "*.cache"
    "*.tmp"
    "test-fixtures/"
    "scripts/check-api-keys.sh"  # Exclude this script itself
    "*/tests/*_test.rs"  # Exclude test files (function names can be long)
    "tests/"  # Exclude test directories
)

# Function to check if file should be excluded
should_exclude_file() {
    local file="$1"

    for pattern in "${EXCLUDE_PATTERNS[@]}"; do
        if [[ "$file" == *"$pattern"* ]]; then
            return 0  # Should exclude
        fi
    done

    return 1  # Should not exclude
}

# Function to check if file extension should be scanned
should_scan_extension() {
    local file="$1"

    for ext in "${FILE_EXTENSIONS[@]}"; do
        if [[ "$file" == *".$ext" ]]; then
            return 0  # Should scan
        fi
    done

    return 1  # Should not scan
}

# Function to scan a file for API key patterns
scan_file() {
    local file="$1"
    local violations_in_file=false

    if should_exclude_file "$file"; then
        return 0
    fi

    if ! should_scan_extension "$file"; then
        return 0
    fi

    if [[ ! -f "$file" ]]; then
        return 0
    fi

    # Read file content
    local content
    if ! content=$(cat "$file" 2>/dev/null); then
        print_warning "Could not read file: $file"
        return 0
    fi

    # Check each pattern
    for pattern_name in "${!API_PATTERNS[@]}"; do
        local pattern="${API_PATTERNS[$pattern_name]}"

        # Use grep to find matches with line numbers
        local matches
        if matches=$(echo "$content" | grep -n -E "$pattern" 2>/dev/null); then
            if [[ -n "$matches" ]]; then
                if [[ "$violations_in_file" == false ]]; then
                    print_error "Potential API key found in: $file"
                    violations_in_file=true
                    VIOLATIONS_FOUND=true
                fi

                echo "  Pattern: $pattern_name"
                while IFS= read -r match; do
                    local line_num=$(echo "$match" | cut -d: -f1)
                    local line_content=$(echo "$match" | cut -d: -f2-)
                    echo "    Line $line_num: $(echo "$line_content" | sed 's/^[[:space:]]*//')"
                done <<< "$matches"
                echo ""
            fi
        fi
    done

    # Additional specific checks for common hardcoded patterns
    local hardcoded_patterns=(
        "const.*API[_-]?KEY.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
        "const.*SECRET.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
        "const.*TOKEN.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
        "const.*ACCOUNT[_-]?ID.*=.*[\"'][0-9a-fA-F]{32}[\"']"
        "[A-Z_]*API[_-]?KEY[A-Z_]*.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
        "[A-Z_]*SECRET[A-Z_]*.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
        "[A-Z_]*TOKEN[A-Z_]*.*=.*[\"'][A-Za-z0-9_-]{16,}[\"']"
    )

    for pattern in "${hardcoded_patterns[@]}"; do
        if matches=$(echo "$content" | grep -n -E "$pattern" 2>/dev/null); then
            if [[ -n "$matches" ]]; then
                if [[ "$violations_in_file" == false ]]; then
                    print_error "Hardcoded credential pattern found in: $file"
                    violations_in_file=true
                    VIOLATIONS_FOUND=true
                fi

                echo "  Hardcoded pattern detected:"
                while IFS= read -r match; do
                    local line_num=$(echo "$match" | cut -d: -f1)
                    local line_content=$(echo "$match" | cut -d: -f2-)
                    echo "    Line $line_num: $(echo "$line_content" | sed 's/^[[:space:]]*//')"
                done <<< "$matches"
                echo ""
            fi
        fi
    done
}

# Main execution
main() {
    print_info "🔍 Scanning for API keys and credentials..."
    echo ""

    # Get list of staged files if running as git hook, otherwise scan all files
    local files_to_scan=()

    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        # We're in a git repository
        if git diff --cached --name-only >/dev/null 2>&1; then
            # Get staged files
            while IFS= read -r file; do
                if [[ -n "$file" ]]; then
                    files_to_scan+=("$file")
                fi
            done < <(git diff --cached --name-only)

            if [[ ${#files_to_scan[@]} -eq 0 ]]; then
                print_info "No staged files to scan"
                return 0
            fi
        else
            # Not in git hook context, scan all tracked files
            while IFS= read -r file; do
                if [[ -n "$file" ]]; then
                    files_to_scan+=("$file")
                fi
            done < <(git ls-files)
        fi
    else
        # Not a git repository, scan current directory
        while IFS= read -r -d '' file; do
            files_to_scan+=("$file")
        done < <(find . -type f -print0)
    fi

    if [[ ${#files_to_scan[@]} -eq 0 ]]; then
        print_info "No files to scan"
        return 0
    fi

    print_info "Scanning ${#files_to_scan[@]} files..."
    echo ""

    # Scan each file
    for file in "${files_to_scan[@]}"; do
        scan_file "$file"
    done

    echo ""

    if [[ "$VIOLATIONS_FOUND" == true ]]; then
        print_error "🚨 API key violations detected!"
        echo ""
        echo "Potential API keys or credentials were found in your code."
        echo "Please:"
        echo "1. Remove any hardcoded API keys, tokens, or secrets"
        echo "2. Use environment variables or secure configuration files"
        echo "3. Add sensitive patterns to .gitignore if needed"
        echo "4. Consider using a secrets management system"
        echo ""
        echo "For browser extensions, store credentials in chrome.storage.sync"
        echo "and load them dynamically rather than hardcoding them."
        echo ""
        return 1
    else
        print_success "✅ No API key violations detected"
        return 0
    fi
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "API Key Detection Script"
        echo ""
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --version, -v  Show version"
        echo ""
        echo "This script scans for common API key patterns in your code"
        echo "to prevent accidental credential commits."
        ;;
    --version|-v)
        echo "API Key Detection Script v1.0.0"
        ;;
    *)
        main
        ;;
esac
