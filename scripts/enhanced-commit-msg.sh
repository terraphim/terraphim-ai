#!/bin/bash
#
# Enhanced Git commit-msg hook for Terraphim AI
# Provides comprehensive commit message validation and suggestions
#
set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Configuration
readonly MAX_TITLE_LENGTH=72
readonly MIN_TITLE_LENGTH=10
readonly MAX_BODY_LINE_LENGTH=72

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

# Function to validate commit title format
validate_commit_title() {
    local title="$1"

    # Skip for merge commits and revert commits
    if [[ $title =~ ^Merge.* ]] || [[ $title =~ ^Revert.* ]]; then
        print_status "SUCCESS" "Merge/Revert commit detected, skipping validation"
        return 0
    fi

    # Enhanced conventional commit pattern
    local conventional_pattern='^(feat|fix|docs|style|refactor|perf|test|chore|build|ci|revert)(\([a-zA-Z0-9_-]+\))?!?: .{1,72}$'

    if [[ ! $title =~ $conventional_pattern ]]; then
        print_status "FAIL" "Commit title does not follow conventional commit format!"
        echo ""
        echo "Expected format: type(optional-scope): description"
        echo ""
        echo "Valid types:"
        echo "  feat:     A new feature"
        echo "  fix:      A bug fix"
        echo "  docs:     Documentation only changes"
        echo "  style:    Changes that don't affect meaning of code (formatting, etc.)"
        echo "  refactor: A code change that neither fixes a bug nor adds a feature"
        echo "  perf:     A code change that improves performance"
        echo "  test:     Adding missing tests or correcting existing tests"
        echo "  chore:    Changes to build process, dependencies, or auxiliary tools"
        echo "  build:    Changes that affect the build system or external dependencies"
        echo "  ci:       Changes to CI configuration files and scripts"
        echo "  revert:   Reverts a previous commit"
        echo ""
        echo "Scope (optional): specific area of change (e.g., api, ui, config)"
        echo "Breaking changes: add '!' after type (e.g., feat!)"
        echo ""
        echo "Examples:"
        echo "  feat: add user authentication system"
        echo "  fix(api): resolve memory leak in request handler"
        echo "  docs(readme): update installation instructions"
        echo "  chore(deps): bump tokio from 1.34.0 to 1.35.0"
        echo "  feat!: remove deprecated API endpoint"
        echo ""
        echo "Current title: '$title'"
        return 1
    fi

    # Extract components for detailed validation
    local commit_type=$(echo "$title" | sed -E 's/^([a-z]+)(\([^)]+\))?!?: .*/\1/')
    local scope=$(echo "$title" | sed -E 's/^[a-z]+\(([^)]+)\)?: .*/\1/' | grep -v '^[a-z]\+$' || echo "")
    local description=$(echo "$title" | sed -E 's/^[a-z]+(\([^)]+\))?!?: (.*)/\2/')
    local is_breaking=false

    if [[ $title =~ !: ]]; then
        is_breaking=true
    fi

    # Validate description
    local desc_length=${#description}

    if [ $desc_length -lt $MIN_TITLE_LENGTH ]; then
        print_status "WARN" "Commit description is quite short ($desc_length characters)"
        print_status "INFO" "Consider adding more detail to explain the change"
    fi

    if [ $desc_length -gt $MAX_TITLE_LENGTH ]; then
        print_status "WARN" "Commit description is long ($desc_length characters)"
        print_status "INFO" "Consider using commit body for additional details"
    fi

    # Check description case (should be lowercase)
    if [[ $description =~ ^[A-Z] ]]; then
        print_status "WARN" "Commit description should start with lowercase"
        print_status "INFO" "Current: '$description'"
        print_status "INFO" "Suggested: '${description,}'"
    fi

    # Check for period at end (shouldn't have)
    if [[ $description =~ \.$ ]]; then
        print_status "WARN" "Commit description shouldn't end with a period"
    fi

    # Check for common issues
    if [[ $description =~ (fixes|fix|resolves|closes)\s+[0-9]+ ]]; then
        print_status "INFO" "Consider using issue references in commit body instead of title"
    fi

    # Validate breaking changes
    if [ "$is_breaking" = true ]; then
        print_status "INFO" "Breaking change detected! Make sure to:"
        print_status "INFO" "  1. Update documentation"
        print_status "INFO" "  2. Add migration guide if needed"
        print_status "INFO" "  3. Consider version bump (major)"
    fi

    # Type-specific suggestions
    case "$commit_type" in
        "feat")
            if [[ ! $description =~ (add|create|implement|introduce|support) ]]; then
                print_status "INFO" "Feature commits often start with action verbs: add, create, implement, etc."
            fi
            ;;
        "fix")
            if [[ ! $description =~ (fix|resolve|prevent|handle|correct) ]]; then
                print_status "INFO" "Fix commits often describe the problem solved"
            fi
            ;;
        "docs")
            if [[ ! $description =~ (update|add|remove|improve|document) ]]; then
                print_status "INFO" "Docs commits often describe documentation changes"
            fi
            ;;
        "test")
            if [[ ! $description =~ (add|test|fix|improve|cover) ]]; then
                print_status "INFO" "Test commits often describe test coverage or fixes"
            fi
            ;;
        "chore")
            if [[ $description =~ (deps|dependencies|version|bump|update) ]]; then
                print_status "INFO" "Consider using scope: chore(deps)"
            fi
            ;;
    esac

    print_status "SUCCESS" "Commit title format is valid!"
    return 0
}

# Function to validate commit body
validate_commit_body() {
    local body="$1"

    if [ -z "$body" ]; then
        return 0  # No body is acceptable
    fi

    print_status "INFO" "Validating commit body..."

    # Check line length in body
    local long_lines=false
    while IFS= read -r line; do
        if [ ${#line} -gt $MAX_BODY_LINE_LENGTH ]; then
            print_status "WARN" "Line in commit body is too long (${#line} > $MAX_BODY_LINE_LENGTH)"
            long_lines=true
        fi
    done <<< "$body"

    if [ "$long_lines" = true ]; then
        print_status "INFO" "Consider wrapping commit body lines to $MAX_BODY_LINE_LENGTH characters"
    fi

    # Check for breaking change marker
    if [[ $body =~ BREAKING[[:space:]]+CHANGE: ]]; then
        print_status "INFO" "Breaking change found in commit body"
    fi

    # Check for issue references
    if [[ $body =~ (Closes|Fixes|Resolves)\s+#?[0-9]+ ]]; then
        print_status "INFO" "Issue references found in commit body"
    fi

    print_status "SUCCESS" "Commit body validation passed"
    return 0
}

# Function to provide commit suggestions
provide_suggestions() {
    local title="$1"
    local body="$2"

    # Suggest improvements based on common patterns
    if [[ $title =~ (wip|work[[:space:]]+in[[:space:]]+progress) ]]; then
        print_status "INFO" "Consider using 'wip:' prefix for work-in-progress commits"
    fi

    if [[ $title =~ (temp|temporary|hack|workaround) ]]; then
        print_status "INFO" "Consider adding TODO comments for temporary solutions"
    fi

    if [[ $title =~ (refactor|cleanup|improve) ]] && [ -z "$body" ]; then
        print_status "INFO" "Refactor commits often benefit from detailed explanations"
    fi
}

# Main validation function
main() {
    local commit_file="$1"

    if [ ! -f "$commit_file" ]; then
        print_status "FAIL" "Commit file not found: $commit_file"
        exit 1
    fi

    # Read commit message
    local commit_msg=$(cat "$commit_file")

    # Split title and body
    local title=$(echo "$commit_msg" | head -n1)
    local body=$(echo "$commit_msg" | tail -n +2 | sed '/^$/d')

    echo "Validating commit message..."
    echo "Title: $title"
    if [ -n "$body" ]; then
        echo "Body: $(echo "$body" | head -n3 | tr '\n' ' ')"
        if [ $(echo "$body" | wc -l) -gt 3 ]; then
            echo "       ($(echo "$body" | wc -l) more lines)"
        fi
    fi
    echo ""

    # Run validations
    validate_commit_title "$title" || true
    validate_commit_body "$body" || true
    provide_suggestions "$title" "$body"

    echo ""
    if [ $EXIT_CODE -eq 0 ]; then
        print_status "SUCCESS" "Commit message validation passed!"
        echo ""
    else
        print_status "FAIL" "Commit message validation failed!"
        echo ""
        exit $EXIT_CODE
    fi
}

# Run main function
main "$@"