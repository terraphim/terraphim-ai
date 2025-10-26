#!/bin/bash
#
# Enhanced Git pre-commit hook for Terraphim AI
# Provides comprehensive, fast, and reliable pre-commit checks
#
set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Configuration
readonly MAX_FILE_SIZE_KB=1000
readonly MAX_COMMIT_TITLE_LENGTH=72
readonly MIN_COMMIT_DESCRIPTION_LENGTH=10
readonly RUST_CHECK_TIMEOUT=60
readonly JS_CHECK_TIMEOUT=30

# Global status tracking
EXIT_CODE=0

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case "$status" in
        "SUCCESS") echo -e "${GREEN}✓${NC} $message" ;;
        "FAIL")
            echo -e "${RED}✗${NC} $message"
            EXIT_CODE=1
            ;;
        "WARN") echo -e "${YELLOW}⚠${NC} $message" ;;
        "INFO") echo -e "${BLUE}ℹ${NC} $message" ;;
        *) echo -e "$message" ;;
    esac
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to run command with timeout and proper error handling
run_with_timeout() {
    local timeout=$1
    local cmd="$2"
    local description="$3"

    if timeout "$timeout" bash -c "$cmd" >/dev/null 2>&1; then
        print_status "SUCCESS" "$description"
        return 0
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            print_status "WARN" "$description timed out after ${timeout}s"
        else
            print_status "FAIL" "$description failed"
        fi
        return 1
    fi
}

# Function to get staged files by type
get_staged_files() {
    local pattern=$1
    git diff --cached --name-only --diff-filter=ACM | grep -E "$pattern" || true
}

# Function to check file size
check_file_sizes() {
    print_status "INFO" "Checking for large files..."

    local large_files=""
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            local size=$(du -k "$file" 2>/dev/null | cut -f1) || size=0
            if [ "$size" -gt "$MAX_FILE_SIZE_KB" ]; then
                large_files="$large_files $file (${size}KB)"
            fi
        fi
    done <<< "$(git diff --cached --name-only --diff-filter=A)"

    if [ -n "$large_files" ]; then
        print_status "FAIL" "Large files detected (>${MAX_FILE_SIZE_KB}KB):$large_files"
        print_status "INFO" "Use git-lfs for large files or reduce file size"
        return 1
    fi

    print_status "SUCCESS" "No large files found"
    return 0
}

# Function to check for secrets
check_secrets() {
    print_status "INFO" "Checking for secrets and sensitive data..."

    # Enhanced secret patterns
    local -a secret_patterns=(
        "password\s*=\s*['\"][^'\"]{8,}['\"]"
        "api_key\s*=\s*['\"][^'\"]{10,}['\"]"
        "secret_key\s*=\s*['\"][^'\"]{10,}['\"]"
        "private_key\s*=\s*['\"][^'\"]{10,}['\"]"
        "-----BEGIN.*PRIVATE KEY-----"
        "-----BEGIN.*CERTIFICATE-----"
        "AKIA[0-9A-Z]{16}"
        "sk_live_[0-9a-zA-Z]{24}"
        "ghp_[0-9a-zA-Z]{36}"
        "xoxb-[0-9]{10,}-[0-9]{10,}"
        "AIza[0-9A-Za-z_-]{35}"
        "SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}"
    )

    local secrets_found=false
    local staged_files=$(git diff --cached --name-only)

    for pattern in "${secret_patterns[@]}"; do
        if echo "$staged_files" | xargs grep -l -E -i "$pattern" 2>/dev/null; then
            print_status "FAIL" "Potential secret found matching pattern: ${pattern:0:30}..."
            secrets_found=true
        fi
    done

    if [ "$secrets_found" = true ]; then
        print_status "FAIL" "Secrets detected! Please remove them before committing."
        return 1
    fi

    print_status "SUCCESS" "No secrets detected"
    return 0
}

# Function to check Rust code
check_rust_code() {
    local rust_files=$(get_staged_files '\.(rs|toml)$')

    if [ -z "$rust_files" ]; then
        print_status "INFO" "No Rust files in staged changes, skipping Rust checks"
        return 0
    fi

    print_status "INFO" "Rust files detected, running comprehensive checks..."

    # Check formatting
    if ! run_with_timeout $RUST_CHECK_TIMEOUT "cargo fmt --all -- --check" "Rust formatting check"; then
        print_status "INFO" "Run 'cargo fmt' to fix formatting issues"
        return 1
    fi

    # Check compilation (faster than full build)
    if ! run_with_timeout $RUST_CHECK_TIMEOUT "cargo check --workspace --all-targets" "Cargo compilation check"; then
        print_status "INFO" "Run 'cargo check --workspace --all-targets' to see details"
        return 1
    fi

    # Check linting
    if ! run_with_timeout $RUST_CHECK_TIMEOUT "cargo clippy --workspace --all-targets -- -D warnings" "Rust linting (clippy)"; then
        print_status "INFO" "Run 'cargo clippy --workspace --all-targets' to see details"
        return 1
    fi

    print_status "SUCCESS" "All Rust checks passed"
    return 0
}

# Function to check JavaScript/TypeScript
check_js_code() {
    if ! git diff --cached --name-only | grep -q "^desktop/"; then
        return 0
    fi

    if ! command_exists npx || [ ! -f "desktop/biome.json" ]; then
        print_status "WARN" "Biome not available, skipping JavaScript/TypeScript checks"
        return 0
    fi

    print_status "INFO" "Checking JavaScript/TypeScript with Biome..."

    cd desktop
    if ! run_with_timeout $JS_CHECK_TIMEOUT "npx @biomejs/biome check --no-errors-on-unmatched" "Biome formatting and linting"; then
        print_status "INFO" "Run 'cd desktop && npx @biomejs/biome check --write' to fix issues"
        cd ..
        return 1
    fi
    cd ..

    print_status "SUCCESS" "JavaScript/TypeScript checks passed"
    return 0
}

# Function to check trailing whitespace
check_trailing_whitespace() {
    print_status "INFO" "Checking for trailing whitespace..."

    if git diff --cached --check >/dev/null 2>&1; then
        print_status "SUCCESS" "No trailing whitespace found"
        return 0
    else
        print_status "FAIL" "Trailing whitespace found"
        print_status "INFO" "Fix with: git diff --cached --check"
        print_status "INFO" "Or run: git diff --cached --name-only | xargs sed -i 's/[[:space:]]*$//'"
        return 1
    fi
}

# Function to check configuration files
check_config_syntax() {
    print_status "INFO" "Checking configuration file syntax..."

    local yaml_files=$(get_staged_files '\.(yml|yaml)$')
    local toml_files=$(get_staged_files '\.toml$')

    # Check YAML files
    if [ -n "$yaml_files" ]; then
        if command_exists python3; then
            for file in $yaml_files; do
                if ! python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
                    print_status "FAIL" "Invalid YAML syntax in $file"
                    return 1
                fi
            done
        else
            print_status "WARN" "Python3 not found, skipping YAML syntax check"
        fi
    fi

    # Check TOML files
    if [ -n "$toml_files" ]; then
        for file in $toml_files; do
            if [[ "$file" == *"Cargo.toml" ]] && command_exists cargo; then
                if ! cargo metadata --manifest-path="$file" --format-version 1 >/dev/null 2>&1; then
                    print_status "FAIL" "Invalid TOML syntax in $file"
                    return 1
                fi
            fi
        done
    fi

    print_status "SUCCESS" "Configuration file syntax checks passed"
    return 0
}

# Function to validate commit message format
validate_commit_message() {
    local commit_msg_file="$1"

    if [ ! -f "$commit_msg_file" ]; then
        return 0
    fi

    print_status "INFO" "Validating commit message format..."
    local commit_msg=$(head -n1 "$commit_msg_file")

    # Skip for merge commits
    if [[ $commit_msg =~ ^Merge.* ]]; then
        print_status "SUCCESS" "Merge commit detected, skipping validation"
        return 0
    fi

    # Enhanced conventional commit pattern
    local conventional_pattern='^(feat|fix|docs|style|refactor|perf|test|chore|build|ci|revert)(\([a-zA-Z0-9_-]+\))?!?: .{1,72}$'

    if [[ ! $commit_msg =~ $conventional_pattern ]]; then
        print_status "FAIL" "Commit message does not follow conventional commit format!"
        echo ""
        echo "Expected format: type(optional-scope): description"
        echo ""
        echo "Valid types: feat, fix, docs, style, refactor, perf, test, chore, build, ci, revert"
        echo "Scope: optional (e.g., api, ui, config)"
        echo "Description: lowercase, no period, max 72 characters"
        echo ""
        echo "Examples:"
        echo "  feat: add user authentication system"
        echo "  fix(api): resolve memory leak in request handler"
        echo "  docs(readme): update installation instructions"
        echo "  chore(deps): bump tokio from 1.34.0 to 1.35.0"
        echo ""
        echo "Current message: '$commit_msg'"
        return 1
    fi

    # Extract description for additional checks
    local description=$(echo "$commit_msg" | sed -E 's/^[a-z]+(\([^)]+\))?!?: (.*)/\2/')
    local desc_length=${#description}

    # Check description length
    if [ $desc_length -lt $MIN_COMMIT_DESCRIPTION_LENGTH ]; then
        print_status "WARN" "Commit description is quite short ($desc_length characters)"
        print_status "INFO" "Consider adding more detail to explain the change"
    fi

    # Check description case
    if [[ $description =~ ^[A-Z] ]]; then
        print_status "WARN" "Commit description should start with lowercase"
        print_status "INFO" "Current: '$description'"
        print_status "INFO" "Suggested: '${description,}'"
    fi

    # Check for period at end
    if [[ $description =~ \.$ ]]; then
        print_status "WARN" "Commit description shouldn't end with a period"
    fi

    print_status "SUCCESS" "Commit message format is valid!"
    return 0
}

# Main execution
main() {
    echo "Running enhanced Terraphim AI pre-commit checks..."
    echo ""

    # Run all checks
    check_file_sizes || true
    check_secrets || true
    check_rust_code || true
    check_js_code || true
    check_trailing_whitespace || true
    check_config_syntax || true

    echo ""
    if [ $EXIT_CODE -eq 0 ]; then
        print_status "SUCCESS" "All pre-commit checks passed!"
        echo ""
    else
        print_status "FAIL" "Some pre-commit checks failed!"
        echo ""
        exit $EXIT_CODE
    fi
}

# Run main function
main "$@"