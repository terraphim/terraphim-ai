#!/usr/bin/env bash
# Assemble the GitHub release asset inventory from staged artifact downloads.
#
# Enforces the managed-package release contract:
#   * all-or-nothing: server-managed-packages-<target> artifacts must be
#     present for every matrix target, or none are added to the inventory.
#   * duplicate basenames across the merged binary inventory, the legacy
#     host-native cargo-deb packages and the managed DEB/RPM outputs fail
#     closed instead of silently clobbering merged downloads (notably the
#     legacy terraphim-server_<version>-1_amd64.deb collides by basename with
#     the managed amd64 DEB while carrying different payload bytes).
#
# Requires bash 4+ (associative arrays).

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: assemble-release-inventory.sh --output DIR [--legacy DIR] [--managed-staging DIR] [--managed-target TRIPLE ...]

Merges staged release artifacts into DIR (which must already exist and hold
the merged binary artifacts). Optional stages are only merged when the gate
below passes.
EOF
}

OUTPUT=""
LEGACY=""
MANAGED_STAGING=""
MANAGED_TARGETS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output)
            OUTPUT="${2:-}"
            shift 2
            ;;
        --legacy)
            LEGACY="${2:-}"
            shift 2
            ;;
        --managed-staging)
            MANAGED_STAGING="${2:-}"
            shift 2
            ;;
        --managed-target)
            MANAGED_TARGETS+=("${2:-}")
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage
            exit 2
            ;;
    esac
done

[[ -n "$OUTPUT" && -d "$OUTPUT" ]] || {
    echo "assemble-release-inventory: --output DIR must exist" >&2
    exit 2
}
if [[ ${#MANAGED_TARGETS[@]} -gt 0 && -z "$MANAGED_STAGING" ]]; then
    echo "assemble-release-inventory: --managed-target requires --managed-staging" >&2
    exit 2
fi

declare -A SEEN=()

register_file() {
    local path="$1"
    local base
    base="$(basename "$path")"
    if [[ -n "${SEEN[$base]:-}" ]]; then
        echo "::error::duplicate release asset basename: $base (${SEEN[$base]} and $path)" >&2
        exit 1
    fi
    SEEN["$base"]="$path"
}

register_dir() {
    local dir="$1"
    local f
    while IFS= read -r -d '' f; do
        register_file "$f"
    done < <(find "$dir" -maxdepth 1 -type f -print0)
}

merge_dir() {
    local dir="$1"
    local f
    while IFS= read -r -d '' f; do
        cp -f "$f" "$OUTPUT/"
    done < <(find "$dir" -maxdepth 1 -type f -print0)
}

MANAGED_VERSION=""
MANAGED_FILES=()

validate_managed_dir() {
    local dir="$1"
    local target="$2"
    local deb_arch rpm_arch
    case "$target" in
        x86_64-unknown-linux-musl)
            deb_arch="amd64"
            rpm_arch="x86_64"
            ;;
        aarch64-unknown-linux-musl)
            deb_arch="arm64"
            rpm_arch="aarch64"
            ;;
        *)
            echo "::error::unsupported managed package target: $target" >&2
            exit 1
            ;;
    esac

    local -a entries=()
    local path base version=""
    while IFS= read -r -d '' path; do
        if [[ -L "$path" || ! -f "$path" ]]; then
            echo "::error::managed artifact must be a regular non-symlink file: $path" >&2
            exit 1
        fi
        if [[ ! -s "$path" ]]; then
            echo "::error::managed artifact must not be zero-length: $path" >&2
            exit 1
        fi
        entries+=("$path")
        base="$(basename "$path")"
        if [[ "$base" == terraphim-server-*"-$target.package-sha256sums.txt" ]]; then
            if [[ -n "$version" ]]; then
                echo "::error::unexpected managed artifact (duplicate checksum manifest): $path" >&2
                exit 1
            fi
            version="${base#terraphim-server-}"
            version="${version%-$target.package-sha256sums.txt}"
        fi
    done < <(find "$dir" -mindepth 1 -maxdepth 1 -print0)

    if [[ -z "$version" ]]; then
        echo "::error::managed artifact directory missing package checksum manifest: $dir" >&2
        exit 1
    fi

    local expected_deb="terraphim-server_${version}-1_${deb_arch}.deb"
    local expected_rpm="terraphim-server-${version}-1.${rpm_arch}.rpm"
    local expected_sums="terraphim-server-${version}-${target}.package-sha256sums.txt"
    declare -A actual=()
    for path in "${entries[@]}"; do
        base="$(basename "$path")"
        actual["$base"]="$path"
        if [[ "$base" != "$expected_deb" && "$base" != "$expected_rpm" && "$base" != "$expected_sums" ]]; then
            echo "::error::unexpected managed artifact (stale-version, wrong-target, or extra): $path" >&2
            exit 1
        fi
    done

    [[ -n "${actual[$expected_deb]:-}" ]] || {
        echo "::error::managed artifact directory missing DEB output: $dir" >&2
        exit 1
    }
    [[ -n "${actual[$expected_rpm]:-}" ]] || {
        echo "::error::managed artifact directory missing RPM output: $dir" >&2
        exit 1
    }
    [[ -n "${actual[$expected_sums]:-}" ]] || {
        echo "::error::managed artifact directory missing package checksum manifest: $dir" >&2
        exit 1
    }
    [[ "${#entries[@]}" -eq 3 ]] || {
        echo "::error::managed artifact directory must contain exactly three files: $dir" >&2
        exit 1
    }

    if [[ -n "$MANAGED_VERSION" && "$MANAGED_VERSION" != "$version" ]]; then
        echo "::error::managed package matrix contains stale-version mismatch: expected=$MANAGED_VERSION actual=$version target=$target" >&2
        exit 1
    fi
    MANAGED_VERSION="$version"

    register_file "${actual[$expected_deb]}"
    register_file "${actual[$expected_rpm]}"
    register_file "${actual[$expected_sums]}"
    MANAGED_FILES+=(
        "${actual[$expected_deb]}"
        "${actual[$expected_rpm]}"
        "${actual[$expected_sums]}"
    )
}

# 1. Inventory already merged into the output (binary artifacts).
register_dir "$OUTPUT"

# 2. Legacy host-native cargo-deb packages (optional stage).
if [[ -n "$LEGACY" && -d "$LEGACY" ]]; then
    register_dir "$LEGACY"
fi

# 3. Managed DEB/RPM matrix artifacts: all-or-nothing gate.
MANAGED_PRESENT=()
if [[ -n "$MANAGED_STAGING" && ! -d "$MANAGED_STAGING" ]]; then
    # The workflow only downloads managed artifacts when the managed package
    # job succeeded; a missing staging directory means the stage is absent.
    echo "NOTE: managed staging directory absent (job skipped): $MANAGED_STAGING" >&2
    MANAGED_STAGING=""
fi
if [[ -n "$MANAGED_STAGING" ]]; then
    while IFS= read -r -d '' path; do
        base="$(basename "$path")"
        case "$base" in
            server-managed-packages-x86_64-unknown-linux-musl|server-managed-packages-aarch64-unknown-linux-musl)
                if [[ -L "$path" || ! -d "$path" ]]; then
                    echo "::error::unexpected managed staging entry (expected a regular target directory): $path" >&2
                    exit 1
                fi
                ;;
            *)
                echo "::error::unexpected managed staging entry: $path" >&2
                exit 1
                ;;
        esac
    done < <(find "$MANAGED_STAGING" -mindepth 1 -maxdepth 1 -print0)

    absent=()
    for target in "${MANAGED_TARGETS[@]}"; do
        dir="$MANAGED_STAGING/server-managed-packages-$target"
        if [[ -L "$dir" || ( -e "$dir" && ! -d "$dir" ) ]]; then
            echo "::error::managed artifact target must be a regular directory: $dir" >&2
            exit 1
        fi
        if [[ -d "$dir" ]] && find "$dir" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
            MANAGED_PRESENT+=("$dir")
        else
            absent+=("$target")
        fi
    done

    if [[ ${#MANAGED_PRESENT[@]} -gt 0 && ${#absent[@]} -gt 0 ]]; then
        printf '::error::managed package matrix incomplete; missing targets: %s (all-or-nothing)\n' "${absent[*]}" >&2
        exit 1
    fi

    for target in "${MANAGED_TARGETS[@]}"; do
        dir="$MANAGED_STAGING/server-managed-packages-$target"
        if [[ -d "$dir" ]] && find "$dir" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
            validate_managed_dir "$dir" "$target"
        fi
    done
fi

# Merge the staged artifacts into the inventory.
for path in "${MANAGED_FILES[@]}"; do
    if [[ -L "$path" || ! -f "$path" || ! -s "$path" ]]; then
        echo "::error::validated managed artifact changed before merge: $path" >&2
        exit 1
    fi
done
if [[ -n "$LEGACY" && -d "$LEGACY" ]]; then
    merge_dir "$LEGACY"
fi
for path in "${MANAGED_FILES[@]}"; do
    cp -f "$path" "$OUTPUT/"
done

printf 'release inventory assembled: %s files in %s\n' "$(find "$OUTPUT" -maxdepth 1 -type f | wc -l)" "$OUTPUT"
