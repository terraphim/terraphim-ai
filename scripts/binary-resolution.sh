#!/bin/bash
# Binary Resolution Engine for Terraphim AI Installer
# Resolves the best binary asset for a given tool, version, and platform

# Configuration (loaded from main installer or defaults)
GITHUB_API_BASE="${GITHUB_API_BASE:-https://api.github.com/repos/terraphim/terraphim-ai}"
GITHUB_RELEASES="${GITHUB_RELEASES:-https://github.com/terraphim/terraphim-ai/releases/download}"
DEFAULT_VERSION="${DEFAULT_VERSION:-latest}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

# Get the latest release version from GitHub API
get_latest_version() {
    log_info "Fetching latest release version..."

    local api_response
    local version

    # Try to get latest release
    api_response=$(curl -s "${GITHUB_API_BASE}/releases/latest" 2>/dev/null)

    if [[ $? -ne 0 || -z "$api_response" ]]; then
        log_error "Failed to fetch latest release from GitHub API"
        return 1
    fi

    # Extract tag name
    version=$(echo "$api_response" | grep '"tag_name":' | sed -E 's/.*"tag_name":\s*"([^"]*).*/\1/')

    if [[ -z "$version" ]]; then
        log_error "Could not extract version from GitHub API response"
        return 1
    fi

    # Remove 'v' prefix if present
    version=${version#v}

    log_success "Latest version: $version"
    echo "$version"
}

# Get a specific version from GitHub API
get_version_info() {
    local version=$1

    log_info "Fetching info for version: $version"

    local api_response
    local version_tag="v${version#v}"

    # Get release info
    api_response=$(curl -s "${GITHUB_API_BASE}/releases/tags/$version_tag" 2>/dev/null)

    if [[ $? -ne 0 || -z "$api_response" ]]; then
        log_error "Failed to fetch version $version from GitHub API"
        return 1
    fi

    echo "$api_response"
}

# List all available assets for a release
list_release_assets() {
    local version=$1

    log_info "Listing assets for version: $version"

    local api_response
    api_response=$(get_version_info "$version")

    if [[ $? -ne 0 ]]; then
        return 1
    fi

    # Extract asset names
    echo "$api_response" | grep '"name":' | sed -E 's/.*"name":\s*"([^"]*).*/\1/' | sort
}

# Generate possible asset names for a tool on current platform
generate_asset_names() {
    local tool=$1
    local os=${OS:-"$(uname -s | tr '[:upper:]' '[:lower:]')"}
    local arch=${ARCH:-"$(uname -m)"}

    # Normalize OS and arch
    case $os in
        linux*) os="linux" ;;
        darwin*) os="macos" ;;
        cygwin*|mingw*|msys*) os="windows" ;;
    esac

    case $arch in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        armv7*|armv6*) arch="armv7" ;;
    esac

    local assets=()

    # Priority order for asset names
    if [[ "$os" == "macos" ]]; then
        # macOS universal binaries first
        assets+=("${tool}-universal-apple-darwin")
        assets+=("${tool}-macos-universal")
        # Then architecture-specific
        assets+=("${tool}-macos-${arch}")
        assets+=("${tool}-darwin-${arch}")
    elif [[ "$os" == "windows" ]]; then
        # Windows executables
        assets+=("${tool}-windows-${arch}.exe")
        assets+=("${tool}-${os}-${arch}.exe")
        assets+=("${tool}-${arch}-pc-windows-msvc.exe")
    else
        # Linux and other Unix-like
        assets+=("${tool}-${os}-${arch}")
        assets+=("${tool}-${os}-${arch}-musl")
        assets+=("${tool}-${arch}-unknown-linux-gnu")
    fi

    # Generic fallbacks
    assets+=("${tool}-${arch}")
    assets+=("${tool}")

    # Print all possible names (highest priority first)
    printf '%s\n' "${assets[@]}"
}

# Check if an asset exists in a release
asset_exists() {
    local asset_name=$1
    local version=$2

    log_info "Checking if asset exists: $asset_name"

    local api_response
    local version_tag="v${version#v}"

    # Get release info
    api_response=$(curl -s "${GITHUB_API_BASE}/releases/tags/$version_tag" 2>/dev/null)

    if [[ $? -ne 0 || -z "$api_response" ]]; then
        log_warn "Failed to get release info for $version"
        return 1
    fi

    # Check if asset exists in the release
    if echo "$api_response" | grep -q "\"name\":\s*\"$asset_name\""; then
        log_success "Asset found: $asset_name"
        return 0
    else
        log_info "Asset not found: $asset_name"
        return 1
    fi
}

# Get download URL for an asset
get_asset_url() {
    local asset_name=$1
    local version=$2

    local version_tag="v${version#v}"
    echo "${GITHUB_RELEASES}/$version_tag/$asset_name"
}

# Get checksum for an asset (if available)
get_asset_checksum() {
    local asset_name=$1
    local version=$2

    # Look for checksum file
    local checksum_file="checksums.txt"
    local checksum_url="${GITHUB_RELEASES}/v${version#v}/$checksum_file"

    log_info "Fetching checksums for verification..."

    local checksums
    checksums=$(curl -s "$checksum_url" 2>/dev/null)

    if [[ $? -ne 0 || -z "$checksums" ]]; then
        log_warn "No checksum file found for version $version"
        return 1
    fi

    # Extract checksum for the specific asset
    local checksum
    checksum=$(echo "$checksums" | grep "$asset_name" | head -1 | awk '{print $1}')

    if [[ -n "$checksum" ]]; then
        echo "$checksum"
        return 0
    else
        log_warn "No checksum found for $asset_name"
        return 1
    fi
}

# Resolve the best asset for a tool and version
resolve_best_asset() {
    local tool=$1
    local version=${2:-"$DEFAULT_VERSION"}

    log_info "Resolving best asset for $tool (version: $version)"

    # Get version if 'latest'
    if [[ "$version" == "latest" ]]; then
        version=$(get_latest_version)
        if [[ $? -ne 0 ]]; then
            log_error "Failed to get latest version"
            return 1
        fi
    fi

    log_info "Resolved version: $version"

    # Generate possible asset names
    local asset_names
    readarray -t asset_names < <(generate_asset_names "$tool")

    log_info "Trying asset names in priority order:"
    for name in "${asset_names[@]}"; do
        log_info "  - $name"
    done

    # Try each asset name
    for asset_name in "${asset_names[@]}"; do
        if asset_exists "$asset_name" "$version"; then
            local asset_url
            asset_url=$(get_asset_url "$asset_name" "$version")

            log_success "Resolved asset: $asset_name"
            log_info "Download URL: $asset_url"

            # Get checksum if available
            local checksum
            checksum=$(get_asset_checksum "$asset_name" "$version" 2>/dev/null || true)

            if [[ -n "$checksum" ]]; then
                log_info "Checksum: $checksum"
            fi

            # Output in a format that can be easily parsed
            echo "ASSET_NAME=$asset_name"
            echo "ASSET_URL=$asset_url"
            [[ -n "$checksum" ]] && echo "ASSET_CHECKSUM=$checksum"
            echo "ASSET_VERSION=$version"

            return 0
        fi
    done

    # No binary found, recommend source compilation
    log_warn "No pre-built binary found for $tool on this platform"
    log_warn "Will need to build from source"

    echo "ASSET_NAME=source"
    echo "ASSET_URL=source"
    echo "ASSET_CHECKSUM="
    echo "ASSET_VERSION=$version"

    return 1
}

# Resolve binary URL (simplified function for main installer compatibility)
resolve_binary_url() {
    local tool=$1
    local version=${2:-"$DEFAULT_VERSION"}

    log_info "Resolving binary URL for $tool (version: $version)"

    # Parse the output of resolve_best_asset
    local resolution_output
    resolution_output=$(resolve_best_asset "$tool" "$version")

    if [[ $? -eq 0 ]]; then
        local asset_url
        asset_url=$(echo "$resolution_output" | grep "^ASSET_URL=" | cut -d'=' -f2-)
        echo "$asset_url"
    else
        echo "source"
    fi
}

# Get asset size for progress reporting
get_asset_size() {
    local asset_url=$1

    log_info "Getting asset size for: $asset_url"

    # Use HEAD request to get content-length
    local size
    size=$(curl -s -I "$asset_url" | grep -i "content-length" | cut -d' ' -f2- | tr -d '\r\n')

    if [[ -n "$size" && "$size" =~ ^[0-9]+$ ]]; then
        echo "$size"
        return 0
    else
        echo "0"
        return 1
    fi
}

# Verify that an asset is suitable for the current platform
verify_asset_compatibility() {
    local asset_name=$1
    local os=${OS:-"$(uname -s | tr '[:upper:]' '[:lower:]')"}
    local arch=${ARCH:-"$(uname -m)"}

    log_info "Verifying asset compatibility: $asset_name"

    # Check OS compatibility
    local os_compatible=false
    case $os in
        linux*)
            if [[ "$asset_name" =~ linux ]]; then
                os_compatible=true
            fi
            ;;
        darwin*)
            if [[ "$asset_name" =~ (darwin|macos) ]]; then
                os_compatible=true
            fi
            ;;
        cygwin*|mingw*|msys*)
            if [[ "$asset_name" =~ windows ]] || [[ "$asset_name" =~ \.exe$ ]]; then
                os_compatible=true
            fi
            ;;
    esac

    # Check architecture compatibility
    local arch_compatible=false
    case $arch in
        x86_64|amd64)
            if [[ "$asset_name" =~ (x86_64|amd64|x64) ]]; then
                arch_compatible=true
            fi
            ;;
        aarch64|arm64)
            if [[ "$asset_name" =~ (aarch64|arm64|arm) ]]; then
                arch_compatible=true
            fi
            ;;
        armv7*)
            if [[ "$asset_name" =~ armv7 ]]; then
                arch_compatible=true
            fi
            ;;
    esac

    if [[ "$os_compatible" == true && "$arch_compatible" == true ]]; then
        log_success "Asset is compatible with current platform"
        return 0
    else
        log_error "Asset is not compatible with current platform"
        log_error "OS compatible: $os_compatible, Arch compatible: $arch_compatible"
        return 1
    fi
}

# Main function for testing
main() {
    local tool=${1:-"terraphim-agent"}
    local version=${2:-"latest"}

    echo "=== Binary Resolution Test ==="
    echo "Tool: $tool"
    echo "Version: $version"
    echo "==========================="

    resolve_best_asset "$tool" "$version"

    echo
    echo "Testing compatibility check..."
    if verify_asset_compatibility "terraphim-agent-linux-x86_64"; then
        echo "Compatibility check passed"
    else
        echo "Compatibility check failed"
    fi
}

# If script is executed directly, run main
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
