#!/usr/bin/env bash

# Terraphim AI Changelog Generator Script
# Generates changelog entries based on git commits since last release

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    cat << EOF
Terraphim AI Changelog Generator

Usage: $0 [OPTIONS] [FROM_TAG] [TO_TAG]

Arguments:
    FROM_TAG    Starting git tag (default: last release tag)
    TO_TAG      Ending git tag (default: HEAD)

Options:
    -h, --help           Show this help message
    -o, --output FILE    Output to file instead of stdout
    -f, --format FORMAT  Output format: markdown (default), json, or plain
    -w, --web            Generate web-compatible changelog
    -s, --skip-merge     Skip merge commits
    -a, --all-commits    Include all commits (not just conventional)

Examples:
    $0                                    # Auto-detect tags
    $0 v0.2.3 v0.2.4                      # Custom tag range
    $0 --output CHANGELOG.md             # Output to file
    $0 --format json v0.2.3              # JSON format
    $0 --web v0.2.3                      # Web-friendly format

EOF
}

# Default options
OUTPUT_FILE=""
FORMAT="markdown"
WEB_FORMAT=false
SKIP_MERGE=true
ALL_COMMITS=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -f|--format)
            FORMAT="$2"
            shift 2
            ;;
        -w|--web)
            WEB_FORMAT=true
            shift
            ;;
        -s|--skip-merge)
            SKIP_MERGE=true
            shift
            ;;
        -a|--all-commits)
            ALL_COMMITS=true
            shift
            ;;
        -*)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
        *)
            if [[ -z "${FROM_TAG:-}" ]]; then
                FROM_TAG="$1"
            elif [[ -z "${TO_TAG:-}" ]]; then
                TO_TAG="$1"
            else
                print_error "Too many arguments"
                usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Function to get last release tag
get_last_release_tag() {
    git tag --list "v*" --sort=-version:refname | head -1
}

# Function to get commits in range
get_commits() {
    local from="$1"
    local to="$2"

    local git_args=("log" "--no-merges" "--pretty=format:%H|%s|%an|%ad" "--date=short")

    if [[ "$SKIP_MERGE" == "true" ]]; then
        git_args+=("--no-merges")
    fi

    git "${git_args[@]}" "${from}..${to}"
}

# Function to parse conventional commit
parse_conventional_commit() {
    local message="$1"

    # Check if it's a conventional commit
    if [[ "$message" =~ ^([a-z]+)(\([^\)]+\))?:\s+(.+)$ ]]; then
        local type="${BASH_REMATCH[1]}"
        local scope="${BASH_REMATCH[2]}"
        local description="${BASH_REMATCH[3]}"

        echo "$type|$scope|$description"
        return 0
    else
        echo "other||$message"
        return 0
    fi
}

# Function to categorize commit
categorize_commit() {
    local type="$1"

    case "$type" in
        "feat")
            echo "✨ Features"
            ;;
        "fix")
            echo "🐛 Bug Fixes"
            ;;
        "docs")
            echo "📝 Documentation"
            ;;
        "style")
            echo "💄 Styling"
            ;;
        "refactor")
            echo "♻️ Refactoring"
            ;;
        "perf")
            echo "⚡ Performance"
            ;;
        "test")
            echo "🧪 Testing"
            ;;
        "chore"|"build"|"ci")
            echo "🔧 Build & CI"
            ;;
        *)
            echo "🔄 Other"
            ;;
    esac
}

# Function to format commit entry
format_commit_entry() {
    local type="$1"
    local scope="$2"
    local description="$3"
    local author="$4"
    local date="$5"

    case "$FORMAT" in
        "markdown")
            if [[ "$WEB_FORMAT" == "true" ]]; then
                echo "- **$description** ([\`$author\`](https://github.com/$author))"
            else
                echo "- $description"
            fi
            ;;
        "json")
            echo "    {\"type\": \"$type\", \"scope\": \"$scope\", \"description\": \"$description\", \"author\": \"$author\", \"date\": \"$date\"}"
            ;;
        "plain")
            echo "  $description ($author, $date)"
            ;;
    esac
}

# Function to generate changelog in markdown format
generate_markdown_changelog() {
    local from_tag="$1"
    local to_tag="$2"
    local commits="$3"

    local title
    if [[ "$to_tag" == "HEAD" ]]; then
        title="## [Unreleased]"
    else
        title="## [$to_tag]"
    fi

    echo "$title"
    echo ""

    if [[ -z "$commits" ]]; then
        echo "No changes since $from_tag"
        echo ""
        return 0
    fi

    # Group commits by category
    declare -A categories

    while IFS='|' read -r hash message author date; do
        if [[ "$ALL_COMMITS" == "false" ]]; then
            local parsed
            parsed=$(parse_conventional_commit "$message")
            IFS='|' read -r type scope description <<< "$parsed"
        else
            type="other"
            scope=""
            description="$message"
        fi

        local category
        category=$(categorize_commit "$type")

        local entry
        entry=$(format_commit_entry "$type" "$scope" "$description" "$author" "$date")

        categories["$category"]+="$entry"$'\n'
    done <<< "$commits"

    # Output categories in preferred order
    local order=(
        "✨ Features"
        "🐛 Bug Fixes"
        "📝 Documentation"
        "⚡ Performance"
        "🧪 Testing"
        "🔧 Build & CI"
        "♻️ Refactoring"
        "💄 Styling"
        "🔄 Other"
    )

    for category in "${order[@]}"; do
        if [[ -n "${categories[$category]:-}" ]]; then
            echo "### $category"
            echo ""
            echo -n "${categories[$category]}"
            echo ""
        fi
    done
}

# Function to generate changelog in JSON format
generate_json_changelog() {
    local from_tag="$1"
    local to_tag="$2"
    local commits="$3"

    echo "{"
    echo "  \"from_tag\": \"$from_tag\","
    echo "  \"to_tag\": \"$to_tag\","
    echo "  \"generated_at\": \"$(date -Iseconds)\","
    echo "  \"commits\": ["

    local first=true
    while IFS='|' read -r hash message author date; do
        if [[ "$ALL_COMMITS" == "false" ]]; then
            local parsed
            parsed=$(parse_conventional_commit "$message")
            IFS='|' read -r type scope description <<< "$parsed"
        else
            type="other"
            scope=""
            description="$message"
        fi

        if [[ "$first" == "true" ]]; then
            first=false
        else
            echo ","
        fi

        format_commit_entry "$type" "$scope" "$description" "$author" "$date"
    done <<< "$commits"

    echo ""
    echo "  ]"
    echo "}"
}

# Function to generate changelog in plain format
generate_plain_changelog() {
    local from_tag="$1"
    local to_tag="$2"
    local commits="$3"

    echo "Changelog from $from_tag to $to_tag"
    echo "Generated: $(date)"
    echo ""

    if [[ -z "$commits" ]]; then
        echo "No changes since $from_tag"
        return 0
    fi

    # Group commits by category
    declare -A categories

    while IFS='|' read -r hash message author date; do
        if [[ "$ALL_COMMITS" == "false" ]]; then
            local parsed
            parsed=$(parse_conventional_commit "$message")
            IFS='|' read -r type scope description <<< "$parsed"
        else
            type="other"
            scope=""
            description="$message"
        fi

        local category
        category=$(categorize_commit "$type")

        local entry
        entry=$(format_commit_entry "$type" "$scope" "$description" "$author" "$date")

        categories["$category"]+="$entry"$'\n'
    done <<< "$commits"

    # Output categories
    for category in $(printf '%s\n' "${!categories[@]}" | sort); do
        if [[ -n "${categories[$category]:-}" ]]; then
            echo "$category:"
            echo ""
            echo -n "${categories[$category]}"
            echo ""
        fi
    done
}

# Function to auto-detect tags
auto_detect_tags() {
    if [[ -z "${FROM_TAG:-}" ]]; then
        FROM_TAG=$(get_last_release_tag)
        if [[ -z "$FROM_TAG" ]]; then
            print_error "Could not find any release tags"
            exit 1
        fi
        print_status "Auto-detected FROM_TAG: $FROM_TAG"
    fi

    if [[ -z "${TO_TAG:-}" ]]; then
        TO_TAG="HEAD"
        print_status "Using TO_TAG: HEAD (unreleased changes)"
    fi

    # Validate tags exist
    if ! git rev-parse "$FROM_TAG" >/dev/null 2>&1; then
        print_error "Tag $FROM_TAG not found"
        exit 1
    fi

    if [[ "$TO_TAG" != "HEAD" ]] && ! git rev-parse "$TO_TAG" >/dev/null 2>&1; then
        print_error "Tag $TO_TAG not found"
        exit 1
    fi
}

# Main execution
main() {
    # Check if we're in a git repository
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_error "Not in a git repository"
        exit 1
    fi

    # Auto-detect tags if not provided
    auto_detect_tags

    print_status "Generating changelog from $FROM_TAG to $TO_TAG"

    # Get commits
    local commits
    commits=$(get_commits "$FROM_TAG" "$TO_TAG")

    if [[ -z "$commits" ]]; then
        print_warning "No commits found in range $FROM_TAG..$TO_TAG"
    else
        local commit_count
        commit_count=$(echo "$commits" | wc -l)
        print_status "Found $commit_count commits"
    fi

    # Generate changelog based on format
    local changelog
    case "$FORMAT" in
        "markdown")
            changelog=$(generate_markdown_changelog "$FROM_TAG" "$TO_TAG" "$commits")
            ;;
        "json")
            changelog=$(generate_json_changelog "$FROM_TAG" "$TO_TAG" "$commits")
            ;;
        "plain")
            changelog=$(generate_plain_changelog "$FROM_TAG" "$TO_TAG" "$commits")
            ;;
        *)
            print_error "Unknown format: $FORMAT"
            exit 1
            ;;
    esac

    # Output changelog
    if [[ -n "$OUTPUT_FILE" ]]; then
        echo "$changelog" > "$OUTPUT_FILE"
        print_success "Changelog written to $OUTPUT_FILE"
    else
        echo "$changelog"
    fi

    print_success "Changelog generation completed"
}

# Run main function
main "$@"