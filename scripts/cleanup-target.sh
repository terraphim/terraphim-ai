#!/bin/bash
# Target Directory Cleanup Script for Terraphim AI
# Reduces disk usage by removing old build artifacts
# Expected savings: 20-30 GB per runner

set -euo pipefail

# Configuration
DRY_RUN=${DRY_RUN:-false}
TARGET_DIR=${TARGET_DIR:-"target"}
CARGO_CACHE_DIR=${CARGO_CACHE_DIR:-"$HOME/.cargo"}
RETENTION_DAYS=${RETENTION_DAYS:-7}
INCREMENTAL_RETENTION_DAYS=${INCREMENTAL_RETENTION_DAYS:-3}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show current disk usage
show_disk_usage() {
    log_info "Current disk usage:"
    df -h | grep -E '(Filesystem|/dev/)' || true
    
    if [[ -d "$TARGET_DIR" ]]; then
        log_info "Target directory size: $(du -sh "$TARGET_DIR" 2>/dev/null | cut -f1 || echo 'N/A')"
    fi
    
    if [[ -d "$CARGO_CACHE_DIR/registry" ]]; then
        log_info "Cargo registry size: $(du -sh "$CARGO_CACHE_DIR/registry" 2>/dev/null | cut -f1 || echo 'N/A')"
    fi
}

# Clean old .rlib files
clean_rlibs() {
    log_info "Cleaning .rlib files older than $RETENTION_DAYS days..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -name "*.rlib" -type f -mtime +$RETENTION_DAYS -print 2>/dev/null | head -20
        log_warn "Dry run mode - no files deleted"
    else
        local count=$(find "$TARGET_DIR" -name "*.rlib" -type f -mtime +$RETENTION_DAYS -delete -print 2>/dev/null | wc -l)
        log_info "Deleted $count old .rlib files"
    fi
}

# Clean old .rmeta files
clean_rmeta() {
    log_info "Cleaning .rmeta files older than $RETENTION_DAYS days..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -name "*.rmeta" -type f -mtime +$RETENTION_DAYS -print 2>/dev/null | head -20
        log_warn "Dry run mode - no files deleted"
    else
        local count=$(find "$TARGET_DIR" -name "*.rmeta" -type f -mtime +$RETENTION_DAYS -delete -print 2>/dev/null | wc -l)
        log_info "Deleted $count old .rmeta files"
    fi
}

# Clean incremental compilation data
clean_incremental() {
    log_info "Cleaning incremental compilation data older than $INCREMENTAL_RETENTION_DAYS days..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -path "*/incremental/*" -type d -mtime +$INCREMENTAL_RETENTION_DAYS -print 2>/dev/null | head -20
        log_warn "Dry run mode - no directories deleted"
    else
        local count=0
        while IFS= read -r dir; do
            if [[ -d "$dir" ]]; then
                rm -rf "$dir"
                ((count++))
            fi
        done < <(find "$TARGET_DIR" -path "*/incremental/*" -type d -mtime +$INCREMENTAL_RETENTION_DAYS 2>/dev/null)
        log_info "Deleted $count incremental directories"
    fi
}

# Clean object files
clean_objects() {
    log_info "Cleaning object files (.o)..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -name "*.o" -type f -print 2>/dev/null | head -20
        log_warn "Dry run mode - no files deleted"
    else
        local count=$(find "$TARGET_DIR" -name "*.o" -type f -delete -print 2>/dev/null | wc -l)
        log_info "Deleted $count object files"
    fi
}

# Clean dependency files
clean_deps() {
    log_info "Cleaning dependency files (.d) older than 1 day..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -name "*.d" -type f -mtime +1 -print 2>/dev/null | head -20
        log_warn "Dry run mode - no files deleted"
    else
        local count=$(find "$TARGET_DIR" -name "*.d" -type f -mtime +1 -delete -print 2>/dev/null | wc -l)
        log_info "Deleted $count dependency files"
    fi
}

# Clean old build artifacts for specific targets
clean_old_targets() {
    log_info "Cleaning old target-specific artifacts..."
    
    # List of targets to clean (keep only recent builds)
    local targets=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-unknown-linux-musl")
    
    for target in "${targets[@]}"; do
        local target_path="$TARGET_DIR/$target"
        if [[ -d "$target_path" ]]; then
            log_info "Checking $target_path..."
            
            # Clean old .rlib files in target directory
            if [[ "$DRY_RUN" == "true" ]]; then
                find "$target_path" -name "*.rlib" -type f -mtime +$RETENTION_DAYS -print 2>/dev/null | head -10
            else
                local count=$(find "$target_path" -name "*.rlib" -type f -mtime +$RETENTION_DAYS -delete -print 2>/dev/null | wc -l)
                if [[ $count -gt 0 ]]; then
                    log_info "Deleted $count old .rlib files from $target"
                fi
            fi
        fi
    done
}

# Clean cargo registry cache
clean_cargo_cache() {
    log_info "Cleaning Cargo registry cache..."
    
    if command -v cargo-cache &> /dev/null; then
        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "Would run: cargo cache --autoclean"
        else
            cargo cache --autoclean 2>/dev/null || log_warn "cargo cache autoclean failed"
        fi
    else
        log_warn "cargo-cache not installed, skipping registry cleanup"
        log_info "Install with: cargo install cargo-cache"
    fi
}

# Remove empty directories
remove_empty_dirs() {
    log_info "Removing empty directories..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        find "$TARGET_DIR" -type d -empty -print 2>/dev/null | head -20
        log_warn "Dry run mode - no directories deleted"
    else
        local count=0
        # Keep running until no more empty directories are found
        while true; do
            local deleted=$(find "$TARGET_DIR" -type d -empty -delete -print 2>/dev/null | wc -l)
            if [[ $deleted -eq 0 ]]; then
                break
            fi
            ((count+=deleted))
        done
        log_info "Removed $count empty directories"
    fi
}

# Show help
show_help() {
    cat << EOF
Terraphim AI Target Directory Cleanup Script

Usage: $0 [OPTIONS]

OPTIONS:
    -d, --dry-run           Show what would be deleted without deleting
    -t, --target-dir DIR    Target directory to clean (default: target)
    -r, --retention DAYS    Retention period in days (default: 7)
    -h, --help              Show this help message

ENVIRONMENT VARIABLES:
    DRY_RUN                 Set to 'true' for dry run mode
    TARGET_DIR              Target directory path
    CARGO_CACHE_DIR         Cargo cache directory path
    RETENTION_DAYS          Number of days to retain files
    INCREMENTAL_RETENTION_DAYS  Number of days to retain incremental data

EXAMPLES:
    # Dry run to see what would be deleted
    $0 --dry-run

    # Clean with custom retention period
    $0 --retention 3

    # Clean specific target directory
    $0 --target-dir /path/to/target

EOF
}

# Main function
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -t|--target-dir)
                TARGET_DIR="$2"
                shift 2
                ;;
            -r|--retention)
                RETENTION_DAYS="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    log_info "Starting target directory cleanup..."
    log_info "Target directory: $TARGET_DIR"
    log_info "Retention period: $RETENTION_DAYS days"
    log_info "Incremental retention: $INCREMENTAL_RETENTION_DAYS days"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warn "DRY RUN MODE - No files will be deleted"
    fi

    # Show initial disk usage
    show_disk_usage

    # Perform cleanup
    if [[ -d "$TARGET_DIR" ]]; then
        clean_rlibs
        clean_rmeta
        clean_incremental
        clean_objects
        clean_deps
        clean_old_targets
        remove_empty_dirs
    else
        log_warn "Target directory not found: $TARGET_DIR"
    fi

    # Clean cargo cache
    clean_cargo_cache

    # Show final disk usage
    log_info "Cleanup complete!"
    show_disk_usage
}

# Run main function
main "$@"
