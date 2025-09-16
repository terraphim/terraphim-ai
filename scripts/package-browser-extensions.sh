#!/bin/bash

# Package script for Terraphim Browser Extensions
# Creates distribution-ready packages for Chrome Web Store submission

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BROWSER_EXT_DIR="$PROJECT_ROOT/browser_extensions"
DIST_DIR="$PROJECT_ROOT/dist/browser-extensions"

print_info "📦 Starting browser extensions packaging process"

# Create distribution directory
mkdir -p "$DIST_DIR"

# Function to get extension version from manifest
get_extension_version() {
    local manifest_file="$1"
    if [ -f "$manifest_file" ]; then
        python3 -c "
import json
with open('$manifest_file', 'r') as f:
    manifest = json.load(f)
    print(manifest.get('version', '0.0.0'))
" 2>/dev/null || echo "0.0.0"
    else
        echo "0.0.0"
    fi
}

# Function to create Chrome Web Store package
create_chrome_package() {
    local ext_name="$1"
    local ext_dir="$BROWSER_EXT_DIR/$ext_name"

    print_info "📦 Creating Chrome Web Store package for $ext_name..."

    if [ ! -d "$ext_dir" ]; then
        print_error "Extension directory not found: $ext_dir"
        return 1
    fi

    # Get version from manifest
    local version=$(get_extension_version "$ext_dir/manifest.json")
    local package_name="${ext_name}-v${version}-chrome.zip"
    local package_path="$DIST_DIR/$package_name"

    print_info "Creating package: $package_name"

    # Create temporary directory for clean packaging
    local temp_dir=$(mktemp -d)
    local clean_dir="$temp_dir/$ext_name"

    # Copy extension files to temp directory
    cp -r "$ext_dir" "$clean_dir"

    cd "$clean_dir"

    # Remove files that shouldn't be in the Chrome package
    print_info "Cleaning files for Chrome Web Store submission..."

    # Remove development and build files
    rm -rf .git* 2>/dev/null || true
    rm -rf node_modules 2>/dev/null || true
    rm -rf .vscode 2>/dev/null || true
    rm -rf .idea 2>/dev/null || true
    rm -rf tests 2>/dev/null || true
    rm -rf test 2>/dev/null || true
    rm -rf __tests__ 2>/dev/null || true
    rm -rf coverage 2>/dev/null || true
    rm -rf .nyc_output 2>/dev/null || true
    rm -rf dist 2>/dev/null || true
    rm -rf build 2>/dev/null || true
    rm -rf src 2>/dev/null || true  # Remove source files for WASM, keep only pkg
    rm -rf target 2>/dev/null || true

    # Remove specific files
    rm -f package-lock.json 2>/dev/null || true
    rm -f yarn.lock 2>/dev/null || true
    rm -f .gitignore 2>/dev/null || true
    rm -f .gitmodules 2>/dev/null || true
    rm -f README.md 2>/dev/null || true  # Remove for Chrome store
    rm -f LICENSE* 2>/dev/null || true   # Remove for Chrome store
    rm -f *.log 2>/dev/null || true
    rm -f *.tmp 2>/dev/null || true
    rm -f .DS_Store 2>/dev/null || true
    rm -f Thumbs.db 2>/dev/null || true
    rm -f .appveyor.yml 2>/dev/null || true
    rm -f webpack.config.js 2>/dev/null || true
    rm -f Cargo.* 2>/dev/null || true

    # For TerraphimAIParseExtension, keep only the necessary WASM files
    if [ "$ext_name" = "TerraphimAIParseExtension" ]; then
        if [ -d "wasm" ]; then
            # Keep only the pkg directory and package.json from WASM build
            find wasm -type f ! -path "wasm/pkg/*" -delete 2>/dev/null || true
            find wasm -type d -empty -delete 2>/dev/null || true
        fi
    fi

    # Validate manifest exists and is valid
    if [ ! -f "manifest.json" ]; then
        print_error "manifest.json not found in $ext_name"
        rm -rf "$temp_dir"
        return 1
    fi

    # Validate JSON syntax
    if ! python3 -m json.tool manifest.json > /dev/null 2>&1; then
        print_error "Invalid manifest.json in $ext_name"
        rm -rf "$temp_dir"
        return 1
    fi

    # Check file sizes (Chrome Web Store has limits)
    local total_size=$(du -sb . | cut -f1)
    local max_size=$((50 * 1024 * 1024))  # 50MB limit

    if [ "$total_size" -gt "$max_size" ]; then
        print_error "Extension package too large: $(($total_size / 1024 / 1024))MB > 50MB"
        rm -rf "$temp_dir"
        return 1
    fi

    # Create ZIP package
    cd "$temp_dir"
    if zip -r "$package_path" "$ext_name" -x "*.DS_Store" "*/.*" > /dev/null 2>&1; then
        print_success "Created Chrome package: $package_name"
        print_info "Package size: $(du -h "$package_path" | cut -f1)"
    else
        print_error "Failed to create ZIP package for $ext_name"
        rm -rf "$temp_dir"
        return 1
    fi

    # Clean up temp directory
    rm -rf "$temp_dir"

    cd "$PROJECT_ROOT"
}

# Function to create source code archive for review
create_source_archive() {
    local ext_name="$1"
    local ext_dir="$BROWSER_EXT_DIR/$ext_name"

    print_info "📚 Creating source code archive for $ext_name..."

    if [ ! -d "$ext_dir" ]; then
        print_error "Extension directory not found: $ext_dir"
        return 1
    fi

    # Get version from manifest
    local version=$(get_extension_version "$ext_dir/manifest.json")
    local source_name="${ext_name}-v${version}-source.zip"
    local source_path="$DIST_DIR/$source_name"

    print_info "Creating source archive: $source_name"

    cd "$BROWSER_EXT_DIR"

    # Create source archive including development files
    if zip -r "$source_path" "$ext_name" \
        -x "*/node_modules/*" \
        -x "*/.git/*" \
        -x "*/target/*" \
        -x "*/dist/*" \
        -x "*/build/*" \
        -x "*/coverage/*" \
        -x "*/.nyc_output/*" \
        -x "*.log" \
        -x "*.tmp" \
        -x "*/.DS_Store" \
        -x "*/Thumbs.db" > /dev/null 2>&1; then
        print_success "Created source archive: $source_name"
    else
        print_error "Failed to create source archive for $ext_name"
        return 1
    fi

    cd "$PROJECT_ROOT"
}

# Function to generate release notes
generate_release_notes() {
    local release_notes_file="$DIST_DIR/RELEASE_NOTES.md"

    print_info "📝 Generating release notes..."

    cat > "$release_notes_file" << EOF
# Terraphim Browser Extensions Release

Generated on: $(date '+%Y-%m-%d %H:%M:%S')

## Extensions Included

### TerraphimAIParseExtension
- **Version**: $(get_extension_version "$BROWSER_EXT_DIR/TerraphimAIParseExtension/manifest.json")
- **Purpose**: Uses Knowledge Graph from Terraphim AI to parse text in a tab and replace known text with links to concepts
- **Features**:
  - WASM-based text processing
  - Cloudflare AI integration
  - Side panel interface
  - Context menu integration
  - Configurable wiki link modes

### TerraphimAIContext
- **Version**: $(get_extension_version "$BROWSER_EXT_DIR/TerraphimAIContext/manifest.json")
- **Purpose**: Searches for the selected text in Terraphim AI, Atomic Server or Logseq
- **Features**:
  - Context menu search
  - Multiple backend support
  - Quick text lookup
  - Notification support

## Installation Instructions

### For Chrome Web Store Submission
1. Use the \`*-chrome.zip\` files
2. Upload to Chrome Developer Dashboard
3. Fill out the store listing with appropriate descriptions

### For Development/Testing
1. Extract the \`*-chrome.zip\` file
2. Open Chrome and navigate to \`chrome://extensions/\`
3. Enable "Developer mode"
4. Click "Load unpacked" and select the extracted folder

## Security Notes

- No hardcoded API credentials are included in the packages
- All credentials must be configured through the extension options page
- API keys are stored securely using Chrome's storage.sync API
- Pre-commit hooks ensure no credentials are accidentally committed

## Files Included

$(ls -la "$DIST_DIR" | grep -E '\.(zip|md)$' | awk '{print "- " $9 " (" $5 " bytes)"}')

## Support

For issues or support, please refer to the main Terraphim AI repository:
https://github.com/terraphim/terraphim-ai
EOF

    print_success "Release notes generated: RELEASE_NOTES.md"
}

# Function to validate packages
validate_packages() {
    print_info "🔍 Validating created packages..."

    local validation_failed=false

    for package in "$DIST_DIR"/*-chrome.zip; do
        if [ -f "$package" ]; then
            local package_name=$(basename "$package")
            print_info "Validating $package_name..."

            # Check if zip is valid
            if ! zip -T "$package" > /dev/null 2>&1; then
                print_error "Invalid ZIP file: $package_name"
                validation_failed=true
                continue
            fi

            # Check if manifest.json exists in the zip
            if ! unzip -l "$package" | grep -q "manifest.json"; then
                print_error "No manifest.json found in $package_name"
                validation_failed=true
                continue
            fi

            # Extract and validate manifest
            local temp_manifest=$(mktemp)
            if unzip -p "$package" "*/manifest.json" > "$temp_manifest" 2>/dev/null; then
                if ! python3 -m json.tool "$temp_manifest" > /dev/null 2>&1; then
                    print_error "Invalid manifest.json in $package_name"
                    validation_failed=true
                fi
                rm -f "$temp_manifest"
            else
                print_error "Could not extract manifest.json from $package_name"
                validation_failed=true
            fi

            print_success "$package_name validation passed"
        fi
    done

    if [ "$validation_failed" = true ]; then
        print_error "Package validation failed"
        return 1
    fi

    print_success "All packages validated successfully"
}

# Main packaging function
main() {
    print_info "🎁 Packaging Terraphim Browser Extensions"

    # Check if build was run first
    if [ ! -d "$BROWSER_EXT_DIR/TerraphimAIParseExtension/wasm/pkg" ]; then
        print_warning "WASM package not found. Running build first..."
        if ! "$SCRIPT_DIR/build-browser-extensions.sh"; then
            print_error "Build failed. Cannot package extensions."
            exit 1
        fi
    fi

    # Clean previous packages
    print_info "Cleaning previous packages..."
    rm -rf "$DIST_DIR"
    mkdir -p "$DIST_DIR"

    # Create packages for each extension
    create_chrome_package "TerraphimAIParseExtension"
    create_chrome_package "TerraphimAIContext"

    # Create source archives
    create_source_archive "TerraphimAIParseExtension"
    create_source_archive "TerraphimAIContext"

    # Generate release notes
    generate_release_notes

    # Validate packages
    validate_packages

    print_success "🎉 Browser extensions packaging completed successfully!"
    print_info ""
    print_info "📁 Packages created in: $DIST_DIR"
    print_info "Files created:"
    ls -la "$DIST_DIR" | grep -E '\.(zip|md)$' | awk '{print "  " $9 " (" $5 " bytes)"}'
    print_info ""
    print_info "Next steps:"
    print_info "  1. Test the Chrome packages by loading them as unpacked extensions"
    print_info "  2. Submit *-chrome.zip files to Chrome Web Store"
    print_info "  3. Keep *-source.zip files for review if requested"
    print_info "  4. Refer to RELEASE_NOTES.md for detailed information"
}

# Run main function
main "$@"