#!/bin/bash
# Update versions across all crates and project files
# Ensures consistent versioning for releases

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}$1${NC}"
}

log_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Validate arguments
if [ $# -lt 1 ]; then
    echo "Usage: $0 <version> [--dry-run]"
    echo "Example: $0 1.2.3"
    echo "Example: $0 1.2.3 --dry-run"
    exit 1
fi

VERSION="$1"
DRY_RUN="${2:-}"

# Validate version format
if [[ ! $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    log_error "Invalid version format: $VERSION"
    echo "Expected format: X.Y.Z (e.g., 1.2.3)"
    exit 1
fi

log_info "Updating version to: $VERSION ${DRY_RUN:+(DRY RUN)}"

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Files to update
declare -a FILES=(
    "Cargo.toml"
    "package.json"
    "desktop/package.json"
    "desktop/src-tauri/Cargo.toml"
    "terraphim_ai_nodejs/package.json"
    "terraphim_server/Cargo.toml"
)

# Function to update version in Cargo.toml
update_cargo_toml() {
    local file="$1"

    if [ ! -f "$file" ]; then
        return 0
    fi

    log_info "Updating $file"

    if [ -n "$DRY_RUN" ]; then
        echo "Would update $file to version $VERSION"
        return 0
    fi

    # Backup original file
    cp "$file" "$file.bak"

    # Update workspace package version if it exists
    if grep -q '\[workspace\.package\]' "$file"; then
        sed -i "s/^version = .*/version = \"$VERSION\"/" "$file"
    fi

    # Update package version if it exists
    if grep -q '\[package\]' "$file"; then
        sed -i "s/^version = .*/version = \"$VERSION\"/" "$file"
    fi

    # Update terraphim dependency versions
    sed -i "s/terraphim_[a-zA-Z_-]* = { version = \"[0-9]\+\.[0-9]\+\.[0-9]\+/&/" "$file" | \
    sed -i "s/\(terraphim_[a-zA-Z_-]* = { version = \"\)[0-9]\+\.[0-9]\+\.[0-9]\+/\1$VERSION/" "$file"

    # Restore file if update failed
    if ! grep -q "version = \"$VERSION\"" "$file"; then
        log_warning "Failed to update $file, restoring backup"
        mv "$file.bak" "$file"
        return 1
    fi

    rm -f "$file.bak"
    log_success "Updated $file"
    return 0
}

# Function to update version in package.json
update_package_json() {
    local file="$1"

    if [ ! -f "$file" ]; then
        return 0
    fi

    log_info "Updating $file"

    if [ -n "$DRY_RUN" ]; then
        echo "Would update $file to version $VERSION"
        return 0
    fi

    # Backup original file
    cp "$file" "$file.bak"

    # Update version using Node.js if available, else use sed
    if command -v node &> /dev/null; then
        node -e "
            const fs = require('fs');
            const pkg = JSON.parse(fs.readFileSync('$file', 'utf8'));
            pkg.version = '$VERSION';
            if (pkg.dependencies) {
                Object.keys(pkg.dependencies).forEach(key => {
                    if (key.startsWith('terraphim-') && !key.includes('-types')) {
                        pkg.dependencies[key] = '$VERSION';
                    }
                });
            }
            fs.writeFileSync('$file', JSON.stringify(pkg, null, 2) + '\n');
        "
    else
        # Fallback to sed (less reliable for JSON)
        sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" "$file"
    fi

    # Verify update
    if grep -q "\"version\": \"$VERSION\"" "$file"; then
        rm -f "$file.bak"
        log_success "Updated $file"
        return 0
    else
        log_warning "Failed to update $file, restoring backup"
        mv "$file.bak" "$file"
        return 1
    fi
}

# Function to update versions in individual crate Cargo.toml files
update_crate_versions() {
    log_info "Updating crate versions"

    if [ -n "$DRY_RUN" ]; then
        echo "Would update versions in crates/"
        return 0
    fi

    # Find all Cargo.toml files in crates
    find "$PROJECT_ROOT/crates" -name "Cargo.toml" -type f | while read -r crate_file; do
        log_info "Processing $crate_file"

        # Backup
        cp "$crate_file" "$crate_file.bak"

        # Update package version
        sed -i "s/^version = .*/version = \"$VERSION\"/" "$crate_file"

        # Update workspace dependencies
        sed -i 's/\({ workspace = true }\)/version = "'"$VERSION"'" \1/' "$crate_file"

        # Verify update
        if grep -q "version = \"$VERSION\"" "$crate_file"; then
            rm -f "$crate_file.bak"
            log_success "Updated $(basename "$crate_file")"
        else
            log_warning "Failed to update $crate_file, restoring backup"
            mv "$crate_file.bak" "$crate_file"
        fi
    done
}

# Function to update Tauri version
update_tauri_version() {
    local tauri_toml="$PROJECT_ROOT/desktop/src-tauri/Cargo.toml"

    if [ ! -f "$tauri_toml" ]; then
        return 0
    fi

    log_info "Updating Tauri configuration"

    if [ -n "$DRY_RUN" ]; then
        echo "Would update Tauri configuration"
        return 0
    fi

    # Backup
    cp "$tauri_toml" "$tauri_toml.bak"

    # Update package version
    sed -i "s/^version = .*/version = \"$VERSION\"/" "$tauri_toml"

    # Verify update
    if grep -q "version = \"$VERSION\"" "$tauri_toml"; then
        rm -f "$tauri_toml.bak"
        log_success "Updated Tauri configuration"
    else
        log_warning "Failed to update Tauri configuration, restoring backup"
        mv "$tauri_toml.bak" "$tauri_toml"
    fi
}

# Function to generate version update report
generate_report() {
    log_info "Generating version update report"

    local report_file="$PROJECT_ROOT/version-update-report.md"

    cat > "$report_file" << EOF
# Version Update Report

**Version:** $VERSION
**Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
${DRY_RUN:+**Mode:** DRY RUN}

## Updated Files

EOF

    if [ -n "$DRY_RUN" ]; then
        echo "This was a dry run. No files were actually modified." >> "$report_file"
    else
        # List all Cargo.toml files with version
        find "$PROJECT_ROOT" -name "Cargo.toml" -type f | while read -r file; do
            echo "- \`${file#$PROJECT_ROOT/}\`" >> "$report_file"
        done >> "$report_file"

        # List all package.json files with version
        find "$PROJECT_ROOT" -name "package.json" -type f | while read -r file; do
            echo "- \`${file#$PROJECT_ROOT/}\`" >> "$report_file"
        done >> "$report_file"
    fi

    cat >> "$report_file" << EOF

## Verification Commands

\`\`\`bash
# Check Cargo workspace version
grep 'version = ' Cargo.toml

# Check crate versions
find crates/ -name Cargo.toml -exec grep -H 'version = ' {} \;

# Check package.json versions
find . -name package.json -exec grep -H 'version' {} \;

# Verify workspace builds
cargo check --workspace
\`\`\`

## Next Steps

1. Review the updated files
2. Run tests to ensure compatibility
3. Commit changes with conventional commit message:
   \`\`\`bash
   git commit -m "chore: bump version to $VERSION"
   \`\`\`
4. Create release tag:
   \`\`\`bash
   git tag v$VERSION
   git push origin v$VERSION
   \`\`\`

EOF

    log_success "Report generated: $report_file"
}

# Main execution
cd "$PROJECT_ROOT"

# Update root files
update_cargo_toml "Cargo.toml"

# Update crate versions
update_crate_versions

# Update package.json files
update_package_json "package.json" 2>/dev/null || true
update_package_json "desktop/package.json"
update_package_json "terraphim_ai_nodejs/package.json"

# Update Tauri
update_tauri_version

# Generate report
generate_report

# Final verification
if [ -z "$DRY_RUN" ]; then
    log_info "Verifying version consistency..."

    # Check that main Cargo.toml has the right version
    if grep -q "version = \"$VERSION\"" "$PROJECT_ROOT/Cargo.toml"; then
        log_success "Version update completed successfully"
    else
        log_error "Version update verification failed"
        exit 1
    fi
else
    log_success "Dry run completed. No files were modified."
fi

log_success "Version update to $VERSION completed!"
echo ""
echo "Next steps:"
echo "1. Review the changes"
echo "2. Run: cargo check --workspace"
echo "3. Commit and push changes"
echo "4. Create release tag: git tag v$VERSION"
