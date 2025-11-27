#!/usr/bin/env bash

# GitHub Token Validation Script using 1Password
# This script validates GitHub personal access tokens retrieved from 1Password op URLs

set -eo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
VERBOSE=false
DRY_RUN=false
GITHUB_API_URL="https://api.github.com"

# Function to print colored output
print_error() {
    echo -e "${RED}ERROR: $1${NC}" >&2
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

print_verbose() {
    if [[ "$VERBOSE" == true ]]; then
        echo -e "${BLUE}VERBOSE: $1${NC}"
    fi
}

# Function to show usage
show_usage() {
    cat << EOF
GitHub Token Validation Script using 1Password

USAGE:
    $0 [OPTIONS] <OP_URL>

ARGUMENTS:
    OP_URL         1Password op:// URL for the GitHub token
                  Example: op://vault/item/field

OPTIONS:
    -v, --verbose     Enable verbose output
    -d, --dry-run     Show what would be done without executing
    -u, --api-url     GitHub API URL (default: https://api.github.com)
    -h, --help        Show this help message

EXAMPLES:
    # Validate token from 1Password
    $0 op://GitHub/tokens/personal-access-token/token

    # Dry run to see what would happen
    $0 --dry-run op://GitHub/tokens/personal-access-token/token

    # Verbose output
    $0 --verbose op://GitHub/tokens/personal-access-token/token

EXIT CODES:
    0    Token is valid
    1    Token is invalid or error occurred
    2    Usage error
    3    1Password CLI not found or not authenticated

EOF
}

# Function to check dependencies
check_dependencies() {
    print_verbose "Checking dependencies..."
    
    # Check for 1Password CLI
    if ! command -v op >/dev/null 2>&1; then
        print_error "1Password CLI (op) not found. Please install it first."
        return 3
    fi
    
    # Check if op is authenticated
    if ! op account get >/dev/null 2>&1; then
        print_error "1Password CLI not authenticated. Please run 'op signin' first."
        return 3
    fi
    
    # Check for curl
    if ! command -v curl >/dev/null 2>&1; then
        print_error "curl command not found. Please install curl first."
        return 1
    fi
    
    print_verbose "All dependencies satisfied"
    return 0
}

# Function to validate op URL format
validate_op_url() {
    local op_url="$1"
    
    if [[ ! "$op_url" =~ ^op:// ]]; then
        print_error "Invalid 1Password URL format. Must start with 'op://'"
        return 2
    fi
    
    print_verbose "1Password URL format is valid: $op_url"
    return 0
}

# Function to retrieve token from 1Password
get_token_from_op() {
    local op_url="$1"
    
    print_verbose "Retrieving token from 1Password: $op_url"
    
    if [[ "$DRY_RUN" == true ]]; then
        print_info "[DRY RUN] Would retrieve token from: $op_url"
        echo "dry-run-token-placeholder"
        return 0
    fi
    
    local token
    if ! token=$(op read "$op_url" 2>/dev/null); then
        print_error "Failed to retrieve token from 1Password"
        print_info "Please check:"
        print_info "1. The op:// URL is correct"
        print_info "2. You have access to the vault and item"
        print_info "3. The field exists and contains a token"
        return 1
    fi
    
    if [[ -z "$token" ]]; then
        print_error "Retrieved token is empty"
        return 1
    fi
    
    print_verbose "Token retrieved successfully (length: ${#token})"
    echo "$token"
}

# Function to validate GitHub token format
validate_github_token_format() {
    local token="$1"
    
    print_verbose "Validating GitHub token format..."
    
    # GitHub personal access tokens (classic)
    if [[ "$token" =~ ^ghp_[a-zA-Z0-9]{36}$ ]]; then
        print_verbose "Token format: GitHub Personal Access Token (Classic)"
        return 0
    fi
    
    # GitHub fine-grained tokens
    if [[ "$token" =~ ^github_pat_[a-zA-Z0-9_]{82}$ ]]; then
        print_verbose "Token format: GitHub Fine-Grained Personal Access Token"
        return 0
    fi
    
    print_warning "Token format doesn't match known GitHub token patterns"
    return 1
}

# Function to test GitHub token against API
test_github_token() {
    local token="$1"
    local api_url="$2"
    
    print_verbose "Testing token against GitHub API: $api_url"
    
    if [[ "$DRY_RUN" == true ]]; then
        print_info "[DRY RUN] Would test token against GitHub API"
        return 0
    fi
    
    # Test the token by making a request to the user endpoint
    local response_body
    local http_code
    
    print_verbose "Making request to: $api_url/user"
    
    # Make the request and capture response body and HTTP code separately
    http_code=$(curl -s -o /tmp/github_response_$$.json -w "%{http_code}" \
        -H "Authorization: token $token" \
        -H "Accept: application/vnd.github.v3+json" \
        "$api_url/user" 2>/dev/null)
    
    # Read the response body
    if [[ -f "/tmp/github_response_$$.json" ]]; then
        response_body=$(cat "/tmp/github_response_$$.json")
        rm -f "/tmp/github_response_$$.json"
    else
        response_body=""
    fi
    
    print_verbose "HTTP Status Code: $http_code"
    
    case "$http_code" in
        200)
            print_verbose "Token is valid and active"
            
            # Parse user info if verbose
            if [[ "$VERBOSE" == true ]]; then
                local login=$(echo "$response_body" | grep -o '"login":"[^"]*"' | cut -d'"' -f4)
                local name=$(echo "$response_body" | grep -o '"name":"[^"]*"' | cut -d'"' -f4)
                
                print_info "Token Details:"
                print_info "  Username: $login"
                [[ -n "$name" ]] && print_info "  Name: $name"
            fi
            
            return 0
            ;;
        401)
            print_error "Token is invalid, expired, or revoked"
            return 1
            ;;
        403)
            print_error "Token is valid but lacks required permissions"
            return 1
            ;;
        000)
            print_error "Network error or API endpoint unreachable"
            return 1
            ;;
        *)
            print_error "Unexpected HTTP status code: $http_code"
            print_verbose "Response: $response_body"
            return 1
            ;;
    esac
}

# Main function
main() {
    local op_url=""
    local api_url="$GITHUB_API_URL"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -u|--api-url)
                api_url="$2"
                shift 2
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            -*)
                print_error "Unknown option: $1"
                show_usage
                exit 2
                ;;
            *)
                if [[ -z "$op_url" ]]; then
                    op_url="$1"
                else
                    print_error "Multiple op URLs provided"
                    show_usage
                    exit 2
                fi
                shift
                ;;
        esac
    done
    
    # Validate required arguments
    if [[ -z "$op_url" ]]; then
        print_error "1Password op:// URL is required"
        show_usage
        exit 2
    fi
    
    print_info "🔍 GitHub Token Validation using 1Password"
    print_info "====================================="
    print_info "1Password URL: $op_url"
    print_info "GitHub API: $api_url"
    [[ "$DRY_RUN" == true ]] && print_info "Mode: Dry Run"
    echo
    
    # Check dependencies
    if ! check_dependencies; then
        exit $?
    fi
    
    # Validate op URL format
    if ! validate_op_url "$op_url"; then
        exit $?
    fi
    
    # Get token from 1Password
    print_info "Retrieving token from 1Password..."
    local token
    if ! token=$(get_token_from_op "$op_url"); then
        exit $?
    fi
    
    # Validate token format
    print_info "Validating token format..."
    if ! validate_github_token_format "$token"; then
        print_warning "Token format validation failed, but proceeding with API test..."
    fi
    
    # Test token against GitHub API
    print_info "Testing token against GitHub API..."
    if ! test_github_token "$token" "$api_url"; then
        print_error "❌ GitHub token validation failed"
        exit 1
    fi
    
    # Success
    echo
    print_success "✅ GitHub token is valid and working"
    print_info "Token successfully retrieved from 1Password and validated against GitHub API"
    
    if [[ "$DRY_RUN" == false ]]; then
        print_info "You can now use this token for GitHub operations"
    fi
    
    exit 0
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        show_usage
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac