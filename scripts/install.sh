#!/bin/bash
# Terraphim Universal Installer v2.0.0
#
# Installs the Terraphim client binaries published on the public release
# channel. Resolution, downloads and checksums all come from
# https://downloads.terraphim.ai; nothing is fetched from GitHub Releases.
#
#   curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash
#
# Supported: Linux (x86_64 gnu/musl, aarch64 musl), macOS (x86_64, aarch64,
# universal), Windows via WSL (x86_64).

set -euo pipefail

readonly INSTALLER_VERSION="2.0.0"

# Sibling utilities live next to this script. Under `curl | bash` there is no
# sibling directory, so each is fetched from the same revision as this script
# and verified before it is sourced.
readonly UTILS_REVISION="${UTILS_REVISION:-main}"
readonly UTILS_BASE_URL="${UTILS_BASE_URL:-https://raw.githubusercontent.com/terraphim/terraphim-ai/${UTILS_REVISION}/scripts}"
readonly SOURCE_URL="https://github.com/terraphim/terraphim-ai/blob/${UTILS_REVISION}/scripts/install.sh"
readonly UTILS=("platform-detection.sh" "binary-resolution.sh" "security-verification.sh")

readonly DEFAULT_INSTALL_DIR="$HOME/.local/bin"
readonly SUPPORTED_TOOLS=("terraphim-agent" "terraphim-cli" "terraphim-grep")

INSTALL_DIR="$DEFAULT_INSTALL_DIR"
TOOLS_TO_INSTALL=("terraphim-agent")
VERSION="${VERSION:-latest}"
SKIP_VERIFY="${SKIP_VERIFY:-false}"
VERBOSE="${VERBOSE:-false}"
CURL_UA="terraphim-installer/${INSTALLER_VERSION}"

# Exit codes
readonly EXIT_USAGE=1
readonly EXIT_MANIFEST=2
readonly EXIT_VERSION=3
readonly EXIT_DOWNLOAD=4
readonly EXIT_CHECKSUM=5
readonly EXIT_INSTALL=6

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log_info()    { [[ "$VERBOSE" == "true" ]] && echo -e "${BLUE}i${NC} $*" || true; }
log_warn()    { echo -e "${YELLOW}!${NC} $*"; }
log_error()   { echo -e "${RED}x${NC} $*"; }
log_success() { echo -e "${GREEN}+${NC} $*"; }
log_progress(){ echo -e "${BLUE}>${NC} $*"; }

# Curl wrapper: descriptive User-Agent (the channel rejects generic ones),
# bounded retries, hard timeout.
terraphim_curl() {
    curl --silent --show-error --fail --location \
         --user-agent "$CURL_UA" \
         --retry 3 --retry-delay 1 --max-time 120 \
         "$@"
}

show_banner() {
    cat <<EOF
+---------------------------------------------------------+
|           Terraphim Installer v${INSTALLER_VERSION}                  |
|    Privacy-first AI assistant with semantic search      |
|                                                         |
|    Channel: https://downloads.terraphim.ai               |
+---------------------------------------------------------+
EOF
}

show_help() {
    cat <<EOF
Terraphim Universal Installer

USAGE:
    curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash [OPTIONS]

OPTIONS:
    --install-dir DIR    Installation directory (default: $DEFAULT_INSTALL_DIR)
    --with-cli           Also install terraphim-cli
    --cli-only           Install only terraphim-cli
    --with-grep          Also install terraphim-grep
    --tool NAME          Install a specific tool (repeatable)
    --version VERSION    Require a specific version (default: latest)
    --skip-verify        Skip SHA-256 verification (not recommended)
    --verbose            Enable verbose logging
    --help, -h           Show this help message

EXIT CODES:
    0 success            3 requested version unavailable
    1 usage error        4 download failed
    2 manifest unreachable  5 checksum mismatch
    6 installation failed

EXAMPLES:
    curl -fsSL ... | bash
    curl -fsSL ... | bash --with-cli --with-grep
    curl -fsSL ... | bash --install-dir /usr/local/bin
    curl -fsSL ... | bash --version 1.21.16

Published binaries: ${SUPPORTED_TOOLS[*]}
Not published here: terraphim-server, terraphim-ai (see https://github.com/terraphim/terraphim-ai)
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install-dir)
                [[ -n "${2:-}" ]] || { log_error "--install-dir needs a directory"; exit $EXIT_USAGE; }
                INSTALL_DIR="$2"; shift 2 ;;
            --with-cli)
                TOOLS_TO_INSTALL=("terraphim-agent" "terraphim-cli"); shift ;;
            --cli-only)
                TOOLS_TO_INSTALL=("terraphim-cli"); shift ;;
            --with-grep)
                TOOLS_TO_INSTALL=("terraphim-agent" "terraphim-grep"); shift ;;
            --tool)
                [[ -n "${2:-}" ]] || { log_error "--tool needs a name"; exit $EXIT_USAGE; }
                [[ ${#TOOLS_TO_INSTALL[@]} -eq 1 && "${TOOLS_TO_INSTALL[0]}" == "terraphim-agent" ]] && TOOLS_TO_INSTALL=()
                TOOLS_TO_INSTALL+=("$2"); shift 2 ;;
            --version)
                [[ -n "${2:-}" ]] || { log_error "--version needs a value"; exit $EXIT_USAGE; }
                VERSION="$2"; shift 2 ;;
            --skip-verify)
                SKIP_VERIFY="true"; shift ;;
            --verbose)
                VERBOSE="true"; shift ;;
            --help|-h)
                show_help; exit 0 ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit $EXIT_USAGE ;;
        esac
    done
}

is_supported_tool() {
    local candidate
    for candidate in "${SUPPORTED_TOOLS[@]}"; do
        [[ "$candidate" == "$1" ]] && return 0
    done
    return 1
}

# Load the sibling utilities. Prefer files beside this script (git checkout,
# local testing); otherwise fetch them from the same revision, so a piped
# invocation cannot source a stale or mismatched revision.
load_utils() {
    local utils_dir="" util target sha_expected sha_actual tmp

    if [[ -n "${BASH_SOURCE[0]:-}" && "${BASH_SOURCE[0]}" != "bash" ]]; then
        local candidate
        candidate="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
        [[ -f "$candidate/binary-resolution.sh" ]] && utils_dir="$candidate"
    fi

    if [[ -n "$utils_dir" ]]; then
        log_info "Loading utilities from $utils_dir"
    else
        tmp=$(mktemp -d)
        trap 'rm -rf "${tmp:-}"' EXIT
        log_info "Fetching utilities from ${UTILS_BASE_URL}"
        local expected_list="$tmp/SHA256SUMS"
        if terraphim_curl --output "$expected_list" "${UTILS_BASE_URL}/SHA256SUMS" 2>/dev/null && [[ -s "$expected_list" ]]; then
            log_info "Downloaded SHA256SUMS"
        else
            rm -f "$expected_list"
            expected_list=""
            log_warn "No SHA256SUMS for the utilities; integrity falls back to TLS"
        fi

        for util in "${UTILS[@]}"; do
            target="$tmp/$util"
            if ! terraphim_curl --output "$target" "${UTILS_BASE_URL}/${util}"; then
                log_error "Could not fetch ${util} from ${UTILS_BASE_URL}"
                log_error "Run from a checkout, or set UTILS_REVISION to a released tag."
                exit $EXIT_MANIFEST
            fi
            if [[ -n "$expected_list" ]]; then
                sha_expected=$(grep -E "[[:space:]]${util}\$" "$expected_list" | awk '{print $1}' | head -1 || true)
                if [[ -z "$sha_expected" ]]; then
                    log_error "SHA256SUMS has no entry for ${util}; refusing unpinned helper"
                    exit $EXIT_CHECKSUM
                fi
                sha_actual=$(calculate_sha256 "$target")
                if [[ "$sha_expected" != "$sha_actual" ]]; then
                    log_error "Checksum mismatch for ${util}: expected ${sha_expected}, got ${sha_actual}"
                    exit $EXIT_CHECKSUM
                fi
                log_info "Verified ${util}"
            fi
        done
        utils_dir="$tmp"
    fi

    for util in "${UTILS[@]}"; do
        # shellcheck source=/dev/null
        source "$utils_dir/$util"
    done
}

# SHA-256 of a file, portable across GNU and BSD userlands.
calculate_sha256() {
    local file=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$file" | awk '{print $NF}'
    else
        log_error "No SHA-256 tool available (sha256sum, shasum, or openssl)"
        return 1
    fi
}

check_dependencies() {
    local missing=()
    command -v curl    >/dev/null 2>&1 || missing+=("curl")
    command -v tar     >/dev/null 2>&1 || missing+=("tar")
    command -v python3 >/dev/null 2>&1 || missing+=("python3")
    if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1 && ! command -v openssl >/dev/null 2>&1; then
        missing+=("sha256sum, shasum or openssl")
    fi
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        exit $EXIT_INSTALL
    fi
    log_info "Dependencies satisfied"
}

create_install_directory() {
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_progress "Creating installation directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    if [[ ! -w "$INSTALL_DIR" ]]; then
        log_error "Installation directory is not writable: $INSTALL_DIR"
        log_error "Re-run with sudo, or choose another directory with --install-dir"
        exit $EXIT_INSTALL
    fi
    log_success "Installation directory ready: $INSTALL_DIR"
}

# Download, verify and unpack one tool. Everything happens in a staging
# directory; the destination is only touched once the bytes are verified and
# the archive is proved to contain exactly the expected binary.
install_tool() {
    local tool=$1
    local resolved name url checksum version
    local resolution

    log_progress "Resolving $tool (version: $VERSION)"
    # Capture the exit status before negating the command: `if ! cmd` leaves $?
    # holding the outcome of the negation, not of cmd.
    local status=0
    resolution=$(resolve_best_asset "$tool" "$VERSION" 2>/dev/null) || status=$?
    if [[ $status -ne 0 ]]; then
        if [[ $status -eq 2 ]]; then
            log_error "Version ${VERSION#v} is not available for $tool"
            return $EXIT_VERSION
        fi
        log_error "Could not resolve $tool from the release channel"
        return $EXIT_MANIFEST
    fi

    name=$(echo "$resolution"     | grep '^ASSET_NAME='     | cut -d'=' -f2-)
    url=$(echo "$resolution"      | grep '^ASSET_URL='      | cut -d'=' -f2-)
    checksum=$(echo "$resolution" | grep '^ASSET_CHECKSUM=' | cut -d'=' -f2-)
    version=$(echo "$resolution"  | grep '^ASSET_VERSION='  | cut -d'=' -f2-)

    local staging
    staging=$(mktemp -d)
    # shellcheck disable=SC2064
    trap "rm -rf '$staging'" RETURN

    local archive="$staging/$name"
    log_progress "Downloading $tool ${version}"
    if ! terraphim_curl --output "$archive" "$url"; then
        log_error "Download failed: $url"
        return $EXIT_DOWNLOAD
    fi
    local size
    size=$(wc -c <"$archive" | tr -d ' ')
    log_success "Downloaded $name (${size} bytes)"

    if [[ "$SKIP_VERIFY" == "true" ]]; then
        log_warn "Skipping SHA-256 verification (--skip-verify)"
    elif [[ -z "$checksum" ]]; then
        log_warn "Manifest carried no checksum for $name; install continues unverified"
    else
        local actual
        actual=$(calculate_sha256 "$archive")
        if [[ "$actual" != "$checksum" ]]; then
            log_error "Checksum mismatch for $name"
            log_error "  expected: $checksum"
            log_error "  actual:   $actual"
            return $EXIT_CHECKSUM
        fi
        log_success "SHA-256 verified"
    fi

    # Unpack into an isolated directory and insist on finding exactly the
    # expected binary. A tar or zip that carries something else is a failure,
    # not an installation.
    local unpack="$staging/unpack"
    mkdir -p "$unpack"
    case "$name" in
        *.tar.gz|*.tgz)
            tar -xzf "$archive" -C "$unpack" 2>/dev/null || {
                log_error "Could not extract $name"; return $EXIT_DOWNLOAD; } ;;
        *.zip)
            if command -v unzip >/dev/null 2>&1; then
                unzip -q -o "$archive" -d "$unpack" 2>/dev/null || {
                    log_error "Could not extract $name"; return $EXIT_DOWNLOAD; }
            elif command -v python3 >/dev/null 2>&1; then
                python3 -c 'import sys,zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$archive" "$unpack" || {
                    log_error "Could not extract $name"; return $EXIT_DOWNLOAD; }
            else
                log_error "No unzip tool available for $name"; return $EXIT_DOWNLOAD
            fi ;;
        *)
            log_error "Unsupported archive type: $name"; return $EXIT_DOWNLOAD ;;
    esac

    local binary_name="$tool"
    [[ "$name" == *.zip ]] && binary_name="${tool}.exe"

    local found
    found=$(find "$unpack" -type f -name "$binary_name" -print -quit)
    if [[ -z "$found" ]]; then
        log_error "$name does not contain $binary_name"
        log_error "Archive contents: $(find "$unpack" -type f -printf '%f ' 2>/dev/null)"
        return $EXIT_INSTALL
    fi

    chmod +x "$found"
    mv -f "$found" "$INSTALL_DIR/$binary_name"
    # Friendlier invocation name on platforms where the binary is dotted.
    if [[ "$binary_name" == *.exe ]]; then
        ln -sf "$binary_name" "$INSTALL_DIR/$tool" 2>/dev/null || true
    fi

    log_success "$tool installed to $INSTALL_DIR/$binary_name"
    return 0
}

verify_installation() {
    local tool=$1
    local binary="$INSTALL_DIR/$tool"
    [[ -x "$binary" ]] || binary="$INSTALL_DIR/${tool}.exe"
    if [[ ! -x "$binary" ]]; then
        log_error "$tool binary not found in $INSTALL_DIR"
        return 1
    fi
    local reported
    if reported=$("$binary" --version 2>/dev/null); then
        log_success "$tool --version -> ${reported}"
    else
        log_warn "$tool is present but did not respond to --version"
    fi
    return 0
}

setup_path() {
    local rc line="export PATH=\"\$PATH:$INSTALL_DIR\""
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) log_info "$INSTALL_DIR is already on PATH"; return 0 ;;
    esac

    case "${SHELL:-}" in
        */zsh)  rc="$HOME/.zshrc" ;;
        */bash) rc="$HOME/.bashrc" ;;
        *)      rc="$HOME/.profile" ;;
    esac

    if [[ -f "$rc" ]] && grep -Fq "$line" "$rc" 2>/dev/null; then
        log_info "PATH entry already present in $rc"
    else
        printf '\n# Added by the Terraphim installer\n%s\n' "$line" >>"$rc"
        log_success "Added $INSTALL_DIR to PATH in $rc"
    fi
    log_warn "Restart your shell, or run: export PATH=\"\$PATH:$INSTALL_DIR\""
}

show_completion_message() {
    echo
    echo -e "${BOLD}Terraphim installed${NC}"
    echo
    echo "  Installed to: $INSTALL_DIR"
    for tool in "${TOOLS_TO_INSTALL[@]}"; do
        echo "    - $tool"
    done
    echo
    echo "Next steps:"
    echo "  terraphim-agent --version"
    echo "  terraphim-agent repl"
    echo
    echo "Docs:     ${SOURCE_URL%/scripts/install.sh}"
    echo "Releases: https://github.com/terraphim/terraphim-clients/releases"
}

main() {
    parse_args "$@"
    show_banner

    check_dependencies
    load_utils

    log_progress "Detecting platform"
    detect_os_arch
    export OS ARCH
    log_success "Platform: ${OS}-${ARCH}"

    local tool
    for tool in "${TOOLS_TO_INSTALL[@]}"; do
        if ! is_supported_tool "$tool"; then
            log_error "Unsupported tool: $tool"
            log_error "Published binaries: ${SUPPORTED_TOOLS[*]}"
            exit $EXIT_USAGE
        fi
    done

    create_install_directory

    for tool in "${TOOLS_TO_INSTALL[@]}"; do
        local status=0
        install_tool "$tool" || status=$?
        [[ $status -ne 0 ]] && exit $status
        verify_installation "$tool" || true
    done

    setup_path
    show_completion_message
}

if [[ "${BASH_SOURCE[0]:-}" == "${0}" ]]; then
    main "$@"
fi