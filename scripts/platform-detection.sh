#!/bin/bash
# Platform Detection Utility for Terraphim AI Installer
# Detects OS, architecture, and other platform-specific information

# Global variables for platform information
OS=""
ARCH=""
PLATFORM=""
INSTALLATION_METHOD=""

# Color output (same as main installer)
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

# Main platform detection function
detect_os_arch() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    # Normalize OS
    case $os in
        linux*)
            OS="linux"
            ;;
        darwin*)
            OS="macos"
            ;;
        cygwin*|mingw*|msys*)
            OS="windows"
            ;;
        freebsd*)
            OS="freebsd"
            ;;
        openbsd*)
            OS="openbsd"
            ;;
        netbsd*)
            OS="netbsd"
            ;;
        *)
            log_error "Unsupported OS: $os"
            return 1
            ;;
    esac

    # Normalize architecture
    case $arch in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7*|armv6*)
            ARCH="armv7"
            ;;
        armv5*)
            ARCH="armv5"
            ;;
        i386|i686)
            ARCH="i386"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            return 1
            ;;
    esac

    # Set platform identifier
    PLATFORM="$OS-$ARCH"

    # Determine installation method
    determine_installation_method

    log_info "Platform detected: $PLATFORM"
    log_info "Installation method: $INSTALLATION_METHOD"

    # Export for use by other scripts
    export OS ARCH PLATFORM INSTALLATION_METHOD

    return 0
}

# Determine the best installation method for the platform
determine_installation_method() {
    case "$PLATFORM" in
        "linux-x86_64"|"linux-aarch64"|"linux-armv7"|"linux-armv5")
            INSTALLATION_METHOD="binary"
            ;;
        "macos-x86_64"|"macos-aarch64")
            INSTALLATION_METHOD="universal-binary"
            ;;
        "windows-x86_64")
            # Check if running in WSL
            if grep -q Microsoft /proc/version 2>/dev/null; then
                INSTALLATION_METHOD="wsl-binary"
            else
                INSTALLATION_METHOD="windows-binary"
            fi
            ;;
        "linux-i386")
            INSTALLATION_METHOD="source"
            ;;
        *)
            INSTALLATION_METHOD="source"
            ;;
    esac
}

# Get platform-specific binary suffix
get_binary_suffix() {
    case "$PLATFORM" in
        "macos-x86_64"|"macos-aarch64")
            echo "universal-apple-darwin"
            ;;
        "windows-"*)
            echo "windows-x86_64.exe"
            ;;
        *)
            echo "${OS}-${ARCH}"
            ;;
    esac
}

# Check if platform has pre-built binaries available
has_prebuilt_binary() {
    local tool=$1
    local version=${2:-"latest"}
    local suffix=$(get_binary_suffix)
    local asset_name="${tool}-${suffix}"

    log_info "Checking for pre-built binary: $asset_name"

    # This would be implemented with actual GitHub API check
    # For now, return true for platforms we know have binaries
    case "$PLATFORM" in
        "linux-x86_64"|"linux-aarch64"|"linux-armv7"|"macos-x86_64"|"macos-aarch64"|"windows-x86_64")
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Get system information for debugging
get_system_info() {
    echo "=== System Information ==="
    echo "OS: $OS"
    echo "Architecture: $ARCH"
    echo "Platform: $PLATFORM"
    echo "Installation Method: $INSTALLATION_METHOD"
    echo "Shell: $SHELL"
    echo "User: $(whoami)"
    echo "Home: $HOME"
    echo "PATH: $PATH"
    echo "========================="
}

# Check if running in container
is_container() {
    # Check for common container indicators
    if [[ -f /.dockerenv ]] || grep -q 'docker\|lxc\|container' /proc/1/cgroup 2>/dev/null; then
        return 0
    fi
    return 1
}

# Check if running in CI environment
is_ci() {
    # Check for common CI environment variables
    if [[ -n "${CI:-}" || -n "${GITHUB_ACTIONS:-}" || -n "${TRAVIS:-}" || -n "${CIRCLECI:-}" ]]; then
        return 0
    fi
    return 0
}

# Get the default installation directory for the platform
get_default_install_dir() {
    case "$OS" in
        "macos")
            echo "/usr/local/bin"
            ;;
        "windows")
            # In WSL, use Windows user's bin directory
            if [[ "$INSTALLATION_METHOD" == "wsl-binary" ]]; then
                echo "/mnt/c/Users/$(powershell.exe -Command 'Write-Host $env:USERNAME' | tr -d '\r')/AppData/Local/Microsoft/WindowsApps"
            else
                echo "$HOME/.local/bin"
            fi
            ;;
        *)
            echo "$HOME/.local/bin"
            ;;
    esac
}

# Check if installation directory requires sudo
requires_sudo() {
    local install_dir=$1

    # Check if directory exists and is writable
    if [[ -d "$install_dir" && -w "$install_dir" ]]; then
        return 1
    fi

    # Check if we can create the directory
    local parent_dir=$(dirname "$install_dir")
    if [[ -w "$parent_dir" ]]; then
        return 1
    fi

    # Requires sudo
    return 0
}

# Validate platform compatibility
validate_platform() {
    case "$OS" in
        "linux"|"macos"|"windows")
            log_info "Supported OS: $OS"
            ;;
        *)
            log_error "Unsupported OS: $OS"
            log_error "Supported OS: Linux, macOS, Windows (WSL)"
            return 1
            ;;
    esac

    case "$ARCH" in
        "x86_64"|"aarch64"|"armv7")
            log_info "Supported architecture: $ARCH"
            ;;
        "i386")
            log_warn "Legacy architecture detected: $ARCH"
            log_warn "Will build from source (slower)"
            ;;
        *)
            log_error "Unsupported architecture: $ARCH"
            log_error "Supported architectures: x86_64, aarch64, armv7"
            return 1
            ;;
    esac

    return 0
}

# Get platform-specific package manager
get_package_manager() {
    case "$OS" in
        "linux")
            if command -v apt-get >/dev/null 2>&1; then
                echo "apt"
            elif command -v yum >/dev/null 2>&1; then
                echo "yum"
            elif command -v dnf >/dev/null 2>&1; then
                echo "dnf"
            elif command -v pacman >/dev/null 2>&1; then
                echo "pacman"
            elif command -v zypper >/dev/null 2>&1; then
                echo "zypper"
            elif command -v apk >/dev/null 2>&1; then
                echo "apk"
            else
                echo "unknown"
            fi
            ;;
        "macos")
            if command -v brew >/dev/null 2>&1; then
                echo "brew"
            else
                echo "none"
            fi
            ;;
        "windows")
            if command -v choco >/dev/null 2>&1; then
                echo "chocolatey"
            elif command -v scoop >/dev/null 2>&1; then
                echo "scoop"
            else
                echo "none"
            fi
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Check for required tools based on platform
check_platform_dependencies() {
    local missing_tools=()

    # Basic tools needed on all platforms
    local basic_tools=("curl" "tar")

    # Platform-specific tools
    case "$OS" in
        "linux"|"macos")
            basic_tools+=("sha256sum")
            ;;
        "windows")
            basic_tools+=("powershell")
            ;;
    esac

    for tool in "${basic_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_tools+=("$tool")
        fi
    done

    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        return 1
    fi

    log_info "All required tools are available"
    return 0
}

# Main function to run full platform detection
main() {
    log_info "Starting platform detection..."

    if ! detect_os_arch; then
        log_error "Platform detection failed"
        return 1
    fi

    if ! validate_platform; then
        log_error "Platform validation failed"
        return 1
    fi

    if ! check_platform_dependencies; then
        log_error "Platform dependency check failed"
        return 1
    fi

    # Additional info
    if is_container; then
        log_info "Running in container environment"
    fi

    if is_ci; then
        log_info "Running in CI environment"
    fi

    local pkg_manager=$(get_package_manager)
    if [[ "$pkg_manager" != "none" && "$pkg_manager" != "unknown" ]]; then
        log_info "Package manager detected: $pkg_manager"
    fi

    echo -e "${GREEN}✓${NC} Platform detection completed successfully"
    get_system_info

    return 0
}

# If script is executed directly, run main
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
