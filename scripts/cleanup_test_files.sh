#!/bin/bash

# 🧹 Terraphim Test File Cleanup Utility
# =====================================
#
# This script automatically cleans up temporary files generated during test runs,
# including config files, logs, and other artifacts.

set -euo pipefail

# Configuration
TEST_TEMP_DIR="/tmp"
TERRAPHIM_PREFIX="terraphim_test_matrix_"
LOG_PREFIX="terraphim_"
MAX_AGE_HOURS=24
DRY_RUN=false
VERBOSE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    cat << EOF
🧹 Terraphim Test File Cleanup Utility

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -h, --help          Show this help message
    -n, --dry-run       Show what would be deleted without actually deleting
    -v, --verbose       Enable verbose output
    -a, --age HOURS     Delete files older than HOURS (default: $MAX_AGE_HOURS)
    -d, --dir DIR       Temporary directory to clean (default: $TEST_TEMP_DIR)
    --force             Delete all matching files regardless of age

EXAMPLES:
    $0                  # Clean up files older than 24 hours
    $0 --dry-run        # Preview what would be deleted
    $0 --age 1          # Clean up files older than 1 hour
    $0 --force          # Delete all test files immediately

DESCRIPTION:
    This script cleans up temporary files created during Terraphim test runs:
    • Test configuration files (/tmp/terraphim_test_matrix_*.json)
    • Log files (/tmp/terraphim_*.log)
    • Performance report files (performance_report.*)
    • Other temporary test artifacts

EOF
}

log() {
    local level=$1
    shift
    case $level in
        "INFO")  echo -e "${BLUE}ℹ️  $*${NC}" ;;
        "WARN")  echo -e "${YELLOW}⚠️  $*${NC}" ;;
        "ERROR") echo -e "${RED}❌ $*${NC}" ;;
        "SUCCESS") echo -e "${GREEN}✅ $*${NC}" ;;
        "DEBUG") [[ $VERBOSE == true ]] && echo -e "${NC}🔍 $*${NC}" ;;
    esac
}

find_test_files() {
    local base_dir=$1
    local age_constraint=""

    if [[ $FORCE != true ]]; then
        age_constraint="-mtime +$(echo "$MAX_AGE_HOURS / 24" | bc -l | cut -d. -f1)"
    fi

    log "DEBUG" "Searching for test files in $base_dir with age constraint: $age_constraint"

    # Find all types of test files
    {
        # Test matrix config files
        find "$base_dir" -name "${TERRAPHIM_PREFIX}*.json" $age_constraint 2>/dev/null || true

        # Log files
        find "$base_dir" -name "${LOG_PREFIX}*.log" $age_constraint 2>/dev/null || true

        # Performance report files
        find "$base_dir" -name "performance_report.*" $age_constraint 2>/dev/null || true

        # Temporary config files for testing
        find "$base_dir" -name "test_config*.json" $age_constraint 2>/dev/null || true
        find "$base_dir" -name "invalid_config*.json" $age_constraint 2>/dev/null || true

        # Benchmark result files
        find "$base_dir" -name "benchmark_*.json" $age_constraint 2>/dev/null || true
        find "$base_dir" -name "*.flamegraph" $age_constraint 2>/dev/null || true
    } | sort | uniq
}

get_file_info() {
    local file=$1
    local size=$(stat -f%z "$file" 2>/dev/null || echo "0")
    local modified=$(stat -f%Sm "$file" 2>/dev/null || echo "unknown")
    echo "$size bytes, modified: $modified"
}

cleanup_files() {
    local files_to_delete=()
    local total_size=0
    local file_count=0

    log "INFO" "Scanning for temporary test files..."

    while IFS= read -r -d '' file; do
        if [[ -f "$file" ]]; then
            files_to_delete+=("$file")
            local size=$(stat -f%z "$file" 2>/dev/null || echo "0")
            total_size=$((total_size + size))
            file_count=$((file_count + 1))

            if [[ $VERBOSE == true ]]; then
                log "DEBUG" "Found: $file ($(get_file_info "$file"))"
            fi
        fi
    done < <(find_test_files "$TEST_TEMP_DIR" -print0)

    if [[ ${#files_to_delete[@]} -eq 0 ]]; then
        log "INFO" "No temporary test files found to clean up"
        return 0
    fi

    # Convert total size to human readable
    local human_size
    if [[ $total_size -gt 1048576 ]]; then
        human_size="$(echo "scale=1; $total_size / 1048576" | bc)MB"
    elif [[ $total_size -gt 1024 ]]; then
        human_size="$(echo "scale=1; $total_size / 1024" | bc)KB"
    else
        human_size="${total_size}B"
    fi

    log "INFO" "Found $file_count test files totaling $human_size"

    if [[ $DRY_RUN == true ]]; then
        log "WARN" "DRY RUN - would delete the following files:"
        for file in "${files_to_delete[@]}"; do
            echo "  - $file"
        done
        log "INFO" "Run without --dry-run to actually delete these files"
        return 0
    fi

    # Confirm deletion (unless forced)
    if [[ $FORCE != true ]] && [[ -t 0 ]]; then  # Only prompt if running interactively
        echo -n "Delete $file_count files ($human_size)? [y/N]: "
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            log "INFO" "Cleanup cancelled"
            return 0
        fi
    fi

    # Delete files
    local deleted_count=0
    local failed_count=0

    for file in "${files_to_delete[@]}"; do
        if rm "$file" 2>/dev/null; then
            deleted_count=$((deleted_count + 1))
            log "DEBUG" "Deleted: $file"
        else
            failed_count=$((failed_count + 1))
            log "ERROR" "Failed to delete: $file"
        fi
    done

    if [[ $failed_count -eq 0 ]]; then
        log "SUCCESS" "Successfully deleted $deleted_count test files ($human_size freed)"
    else
        log "WARN" "Deleted $deleted_count files, failed to delete $failed_count files"
    fi
}

# Parse command line arguments
FORCE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -n|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -a|--age)
            MAX_AGE_HOURS="$2"
            shift 2
            ;;
        -d|--dir)
            TEST_TEMP_DIR="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        *)
            log "ERROR" "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate inputs
if [[ ! -d "$TEST_TEMP_DIR" ]]; then
    log "ERROR" "Directory does not exist: $TEST_TEMP_DIR"
    exit 1
fi

if ! [[ "$MAX_AGE_HOURS" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
    log "ERROR" "Invalid age value: $MAX_AGE_HOURS (must be a number)"
    exit 1
fi

# Check for required tools
for tool in find stat bc; do
    if ! command -v $tool &> /dev/null; then
        log "ERROR" "Required tool not found: $tool"
        exit 1
    fi
done

# Main execution
log "INFO" "Terraphim Test File Cleanup"
log "INFO" "Cleaning directory: $TEST_TEMP_DIR"
log "INFO" "Max age: $MAX_AGE_HOURS hours"
if [[ $DRY_RUN == true ]]; then
    log "INFO" "Mode: DRY RUN (no files will be deleted)"
fi

cleanup_files

log "SUCCESS" "Cleanup completed"
