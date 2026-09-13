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
    absent=()
    for target in "${MANAGED_TARGETS[@]}"; do
        dir="$MANAGED_STAGING/server-managed-packages-$target"
        if [[ -d "$dir" ]] && find "$dir" -maxdepth 1 -type f -print -quit | grep -q .; then
            MANAGED_PRESENT+=("$dir")
        else
            absent+=("$target")
        fi
    done

    if [[ ${#MANAGED_PRESENT[@]} -gt 0 && ${#absent[@]} -gt 0 ]]; then
        printf '::error::managed package matrix incomplete; missing targets: %s (all-or-nothing)\n' "${absent[*]}" >&2
        exit 1
    fi

    for dir in "${MANAGED_PRESENT[@]}"; do
        ls "$dir"/*.deb >/dev/null 2>&1 || {
            echo "::error::managed artifact directory missing DEB output: $dir" >&2
            exit 1
        }
        ls "$dir"/*.rpm >/dev/null 2>&1 || {
            echo "::error::managed artifact directory missing RPM output: $dir" >&2
            exit 1
        }
        ls "$dir"/*.package-sha256sums.txt >/dev/null 2>&1 || {
            echo "::error::managed artifact directory missing package checksum manifest: $dir" >&2
            exit 1
        }
        register_dir "$dir"
    done
fi

# Merge the staged artifacts into the inventory.
if [[ -n "$LEGACY" && -d "$LEGACY" ]]; then
    merge_dir "$LEGACY"
fi
for dir in "${MANAGED_PRESENT[@]}"; do
    merge_dir "$dir"
done

printf 'release inventory assembled: %s files in %s\n' "$(find "$OUTPUT" -maxdepth 1 -type f | wc -l)" "$OUTPUT"
