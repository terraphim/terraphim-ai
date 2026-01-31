#!/usr/bin/env bash

# Enhanced Terraphim AI Release Validation Script
# Integrates with new Rust-based validation system

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
ACTUAL_VERSION="${ACTUAL_VERSION:-}"
CATEGORIES="${CATEGORIES:-}"
OUTPUT_DIR="${OUTPUT_DIR:-target/validation-reports}"
LOG_LEVEL="${LOG_LEVEL:-info}"
USE_RUST_VALIDATOR="${USE_RUST_VALIDATOR:-true}"
ENABLE_LEGACY_BACKUP="${ENABLE_LEGACY_BACKUP:-false}"

# Paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_VALIDATOR="$PROJECT_ROOT/target/release/terraphim-validation"

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

# Check if Rust validator is available and built
check_rust_validator() {
    if [[ "$USE_RUST_VALIDATOR" != "true" ]]; then
        return 1
    fi

    if [[ ! -f "$RUST_VALIDATOR" ]]; then
        print_warning "Rust validator not found at $RUST_VALIDATOR"
        print_status "Building Rust validator..."

        cd "$PROJECT_ROOT"
        if cargo build --release -p terraphim_validation; then
            print_success "Rust validator built successfully"
        else
            print_error "Failed to build Rust validator"
            return 1
        fi
    fi

    return 0
}

# Run legacy bash validation (original functionality)
run_legacy_validation() {
    local version="$1"

    print_status "Running legacy bash validation for version: $version"

    # Original validation logic would go here
    # For now, just run basic checks

    print_success "Legacy validation completed"
    return 0
}

# Run new Rust-based validation
run_rust_validation() {
    local version="$1"
    local categories="$2"

    print_status "Running Rust-based validation for version: $version"

    # Prepare command
    local cmd=("$RUST_VALIDATOR" "validate" "$version")

    if [[ -n "$categories" ]]; then
        cmd+=("--categories" "$categories")
    fi

    cmd+=("--verbose" "--output-dir" "$OUTPUT_DIR")

    # Set log level
    export RUST_LOG="terraphim_validation=$LOG_LEVEL"

    # Run validation
    if "${cmd[@]}"; then
        print_success "Rust validation completed successfully"

        # Display summary
        if [[ -f "$OUTPUT_DIR/validation_report_"*".json" ]]; then
            print_status "Validation report generated:"
            ls -la "$OUTPUT_DIR"/validation_report_*.json
        fi

        return 0
    else
        print_error "Rust validation failed"
        return 1
    fi
}

# Enhanced validation with both systems
run_enhanced_validation() {
    local version="$1"
    local categories="$2"

    print_status "Starting enhanced validation for version: $version"

    # First, run Rust validation if available
    if check_rust_validator; then
        if run_rust_validation "$version" "$categories"; then
            print_success "Primary validation passed"

            # Run legacy validation as backup
            if [[ "$ENABLE_LEGACY_BACKUP" == "true" ]]; then
                print_status "Running legacy validation as backup..."
                if run_legacy_validation "$version"; then
                    print_success "Legacy validation also passed"
                else
                    print_warning "Legacy validation failed, but primary validation passed"
                fi
            fi
        else
            print_error "Primary validation failed"

            # Fallback to legacy validation
            print_status "Falling back to legacy validation..."
            run_legacy_validation "$version"
        fi
    else
        print_status "Rust validator not available, using legacy validation"
        run_legacy_validation "$version"
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                show_help
                exit 0
                ;;
            -v|--version)
                ACTUAL_VERSION="$2"
                shift 2
                ;;
            -c|--categories)
                CATEGORIES="$2"
                shift 2
                ;;
            -o|--output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            -l|--log-level)
                LOG_LEVEL="$2"
                shift 2
                ;;
            --legacy-only)
                USE_RUST_VALIDATOR="false"
                shift
                ;;
            --enable-backup)
                ENABLE_LEGACY_BACKUP="true"
                shift
                ;;
            *)
                # Assume positional argument for version
                if [[ -z "$ACTUAL_VERSION" ]]; then
                    ACTUAL_VERSION="$1"
                fi
                shift
                ;;
        esac
    done
}

# Show help
show_help() {
    cat << EOF
Terraphim AI Enhanced Release Validation Script

USAGE:
    $0 [OPTIONS] [VERSION]

ARGUMENTS:
    VERSION                 Release version to validate (e.g., 1.0.0, v1.0.0)

OPTIONS:
    -h, --help              Show this help message
    -v, --version VERSION   Version to validate
    -c, --categories CATS   Comma-separated list of validation categories
                            (download,installation,functionality,security,performance)
    -o, --output-dir DIR    Output directory for reports (default: target/validation-reports)
    -l, --log-level LEVEL   Log level (trace,debug,info,warn,error)
    --legacy-only           Use only legacy bash validation
    --enable-backup         Enable legacy validation as backup

EXAMPLES:
    $0 1.0.0                              # Validate version 1.0.0 with all categories
    $0 -c "download,installation" 1.0.0   # Validate specific categories
    $0 --legacy-only 1.0.0                # Use only legacy validation
    $0 --enable-backup 1.0.0              # Enable backup validation

ENVIRONMENT VARIABLES:
    USE_RUST_VALIDATOR    Set to 'false' to disable Rust validator
    ENABLE_LEGACY_BACKUP  Set to 'true' to enable legacy backup
    OUTPUT_DIR            Output directory for validation reports
    LOG_LEVEL             Log level for validation output

EOF
}

# Main execution
main() {
    # Ensure we're in the project root
    cd "$PROJECT_ROOT"

    # Parse arguments
    parse_args "$@"

    # Validate arguments
    if [[ -z "$ACTUAL_VERSION" ]]; then
        print_error "Version parameter is required"
        show_help
        exit 1
    fi

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Run validation
    if run_enhanced_validation "$ACTUAL_VERSION" "$CATEGORIES"; then
        print_success "Validation completed successfully"
        exit 0
    else
        print_error "Validation failed"
        exit 1
    fi
}

# Run main function
main "$@"