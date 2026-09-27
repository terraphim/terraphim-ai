#!/bin/bash
# Binary Resolution Engine for the Terraphim installer.
#
# Resolution is manifest-driven. The public release channel publishes one
# manifest per binary at:
#
#   https://downloads.terraphim.ai/<binary>/stable-v2.json
#   https://downloads.terraphim.ai/<binary>/stable.json
#
# stable-v2.json is a strict object-valued manifest:
#
#   { "version", "released_at", "notes_url",
#     "assets": { "<target>": {"path","sha256","size"} } }
#
# stable.json is the legacy pointer: identical shape except that each asset is
# a bare repository-relative path string with no digest. It is read only when
# the strict manifest is unavailable, and a release resolved that way reports
# an empty checksum so the caller runs unverified rather than failing.
#
# Resolution never consults GitHub Releases. The v1 release line's GitHub
# assets are version-less bare binaries published under terraphim/terraphim-ai;
# the current line publishes version-and-target archives through the channel
# below. Keeping one home for resolution removes that mismatch entirely.

# Configuration (overridable by the caller; see scripts/install.sh)
TERRAPHIM_CHANNEL_BASE="${TERRAPHIM_CHANNEL_BASE:-https://downloads.terraphim.ai}"
TERRAPHIM_RELEASES_REPO="${TERRAPHIM_RELEASES_REPO:-terraphim/terraphim-clients}"
DEFAULT_VERSION="${DEFAULT_VERSION:-latest}"

# The channel serves every client binary; anything else is a caller error.
TERRAPHIM_SUPPORTED_BINARIES=("terraphim-agent" "terraphim-cli" "terraphim-grep")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}i${NC} $*"; }
log_warn() { echo -e "${YELLOW}!${NC} $*"; }
log_error() { echo -e "${RED}x${NC} $*"; }
log_success() { echo -e "${GREEN}+${NC} $*"; }

# Map uname output onto the target triples the channel publishes.
normalise_os() {
    case "${1:-$(uname -s)}" in
        Linux|linux*)              echo "linux" ;;
        Darwin|darwin*)            echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*|cygwin*|mingw*|msys*|windows*) echo "windows" ;;
        *)                         echo "unknown" ;;
    esac
}

normalise_arch() {
    case "${1:-$(uname -m)}" in
        x86_64|amd64)      echo "x86_64" ;;
        aarch64|arm64)     echo "aarch64" ;;
        armv7*|armv6*|arm) echo "armv7" ;;
        *)                 echo "unknown" ;;
    esac
}

# Order matters: the first target present in the manifest wins.
# x86_64 macOS prefers the architecture-specific archive over the universal
# one because it is roughly half the download.
generate_target_candidates() {
    local os arch
    os=$(normalise_os)
    arch=$(normalise_arch)

    case "$os" in
        macos)
            case "$arch" in
                aarch64) echo "aarch64-apple-darwin"; echo "universal-apple-darwin" ;;
                x86_64)  echo "x86_64-apple-darwin";  echo "universal-apple-darwin" ;;
            esac
            ;;
        linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu";  echo "x86_64-unknown-linux-musl" ;;
                aarch64) echo "aarch64-unknown-linux-musl" ;;
            esac
            ;;
        windows)
            case "$arch" in
                x86_64) echo "x86_64-pc-windows-msvc" ;;
            esac
            ;;
    esac
}

is_supported_binary() {
    local candidate
    for candidate in "${TERRAPHIM_SUPPORTED_BINARIES[@]}"; do
        [[ "$candidate" == "$1" ]] && return 0
    done
    return 1
}

# Fetch a manifest into a caller-owned file. Returns non-zero on any failure so
# the caller can fall back without inspecting partial output.
fetch_manifest() {
    local binary=$1 version=$2 destination=$3
    local url="${TERRAPHIM_CHANNEL_BASE}/${binary}/stable-v2.json"

    # Version selection is a manifest lookup: the channel only ever serves the
    # current stable release, so asking for anything else is a hard error
    # rather than a silent substitution.
    log_info "Reading manifest: $url"

    if ! curl --silent --show-error --fail --location \
              --retry 2 --retry-delay 1 --max-time 30 \
              --output "$destination" "$url" 2>/dev/null; then
        return 1
    fi
    [[ -s "$destination" ]] || return 1

    if [[ "$version" != "latest" ]]; then
        local manifest_version
        manifest_version=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["version"])' "$destination" 2>/dev/null || true)
        if [[ -z "$manifest_version" ]]; then
            return 1
        fi
        if [[ "$manifest_version" != "${version#v}" ]]; then
            log_error "Channel serves ${binary} ${manifest_version}, not ${version#v}"
            log_error "Requested versions are not archived; see https://github.com/${TERRAPHIM_RELEASES_REPO}/releases"
            return 2
        fi
    fi

    return 0
}

# Read a single field out of a manifest. Never trusts the file's shape.
manifest_field() {
    local file=$1 expression=$2
    python3 -c 'import json,sys
try:
    data = json.load(open(sys.argv[1]))
except Exception:
    raise SystemExit(1)
value = eval(sys.argv[2], {"__builtins__": {}}, {"data": data})
print("" if value is None else value)' "$file" "$expression" 2>/dev/null
}

# Resolve the best asset for the current platform.
#
# Prints four ASSET_* lines for the caller to read, and a human-readable
# summary on stderr. Exit codes:
#   0  resolved against the strict manifest (checksum available)
#   0  resolved against the legacy manifest (ASSET_CHECKSUM empty)
#   1  channel unreachable, or no published asset matches this platform
#   2  the requested version is not the one the channel serves
resolve_best_asset() {
    local binary=$1
    local version=${2:-"$DEFAULT_VERSION"}

    if ! is_supported_binary "$binary"; then
        log_error "Unknown binary: $binary"
        log_error "Published binaries: ${TERRAPHIM_SUPPORTED_BINARIES[*]}"
        return 2
    fi

    local tmpdir
    tmpdir=$(mktemp -d) || return 1
    trap 'rm -rf "$tmpdir"' RETURN

    local strict="$tmpdir/stable-v2.json"
    local legacy="$tmpdir/stable.json"
    local manifest="" source_kind=""

    local status=0
    fetch_manifest "$binary" "$version" "$strict" || status=$?
    if [[ $status -eq 2 ]]; then
        return 2
    fi

    if [[ $status -eq 0 ]]; then
        manifest="$strict"
        source_kind="strict"
    else
        log_warn "Strict manifest unavailable; falling back to the legacy pointer"
        local legacy_url="${TERRAPHIM_CHANNEL_BASE}/${binary}/stable.json"
        if ! curl --silent --show-error --fail --location \
                  --retry 2 --retry-delay 1 --max-time 30 \
                  --output "$legacy" "$legacy_url" 2>/dev/null || [[ ! -s "$legacy" ]]; then
            log_error "No manifest for ${binary} at ${TERRAPHIM_CHANNEL_BASE}/${binary}/"
            return 1
        fi
        manifest="$legacy"
        source_kind="legacy"

        if [[ "$version" != "latest" ]]; then
            local legacy_version
            legacy_version=$(manifest_field "$legacy" 'data["version"]')
            if [[ "$legacy_version" != "${version#v}" ]]; then
                log_error "Channel serves ${binary} ${legacy_version}, not ${version#v}"
                return 2
            fi
        fi
    fi

    local resolved_version
    resolved_version=$(manifest_field "$manifest" 'data["version"]')
    if [[ -z "$resolved_version" ]]; then
        log_error "Manifest for ${binary} has no version field"
        return 1
    fi

    local target=""
    while IFS= read -r candidate; do
        [[ -n "$candidate" ]] || continue
        if [[ "$(manifest_field "$manifest" "data['assets'].get('$candidate')")" != "" ]]; then
            target="$candidate"
            break
        fi
    done < <(generate_target_candidates)

    if [[ -z "$target" ]]; then
        log_error "No published ${binary} archive matches $(normalise_os)/$(normalise_arch)"
        log_error "Published targets: $(manifest_field "$manifest" "' '.join(sorted(data['assets']))")"
        return 1
    fi

    local asset_path checksum=""
    if [[ "$source_kind" == "strict" ]]; then
        asset_path=$(manifest_field "$manifest" "data['assets']['$target']['path']")
        checksum=$(manifest_field "$manifest" "data['assets']['$target']['sha256']")
    else
        asset_path=$(manifest_field "$manifest" "data['assets']['$target']")
    fi

    if [[ -z "$asset_path" || "$asset_path" == /* || "$asset_path" == *".."* ]]; then
        log_error "Manifest for ${binary} carries an unsafe asset path: ${asset_path}"
        return 1
    fi

    target_url="${TERRAPHIM_CHANNEL_BASE}/${asset_path}"
    asset_name=$(basename "$asset_path")

    log_success "Resolved ${binary} ${resolved_version} for ${target}"
    log_info "Manifest: ${source_kind}"
    log_info "Download: ${target_url}"
    [[ -n "$checksum" ]] && log_info "SHA-256:  ${checksum}"

    echo "ASSET_NAME=${asset_name}"
    echo "ASSET_URL=${target_url}"
    echo "ASSET_CHECKSUM=${checksum}"
    echo "ASSET_VERSION=${resolved_version}"
    echo "ASSET_TARGET=${target}"
    echo "ASSET_MANIFEST=${source_kind}"

    return 0
}

# Installer-compatible shim: emit only the URL.
resolve_binary_url() {
    local binary=$1
    local version=${2:-"$DEFAULT_VERSION"}

    local output
    output=$(resolve_best_asset "$binary" "$version" 2>/dev/null) || {
        echo "source"
        return 1
    }
    echo "$output" | grep '^ASSET_URL=' | cut -d'=' -f2-
}

# Freshness of the channel's pointer, for diagnostics and acceptance runs.
channel_status() {
    local binary=${1:-terraphim-agent}
    local tmpdir
    tmpdir=$(mktemp -d) || return 1
    trap 'rm -rf "$tmpdir"' RETURN

    local manifest="$tmpdir/stable-v2.json"
    if ! fetch_manifest "$binary" latest "$manifest"; then
        log_error "Channel manifest unreachable for ${binary}"
        return 1
    fi

    log_info "Binary:    ${binary}"
    log_info "Version:   $(manifest_field "$manifest" 'data["version"]')"
    log_info "Released:  $(manifest_field "$manifest" 'data["released_at"]')"
    log_info "Targets:   $(manifest_field "$manifest" "' '.join(sorted(data['assets']))")"
}

main() {
    local binary=${1:-"terraphim-agent"}
    local version=${2:-"latest"}

    echo "=== Terraphim Binary Resolution ==="
    echo "Binary:  $binary"
    echo "Version: $version"
    echo "Channel: $TERRAPHIM_CHANNEL_BASE"
    echo "==================================="

    resolve_best_asset "$binary" "$version"
    local status=$?

    echo
    if [[ $status -eq 0 ]]; then
        log_success "Resolution succeeded"
    else
        log_error "Resolution failed (exit ${status})"
    fi
    return $status
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi