#!/usr/bin/env bash
# Verify the exact nFPM binary and its canonical semantic version.

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: verify-nfpm.sh --binary PATH --version VERSION --sha256 SHA256

VERSION may have an optional leading "v"; comparison is made against the
normalized semantic version, for example v2.47.0 and 2.47.0 both compare as
2.47.0.
EOF
}

BINARY=""
VERSION=""
EXPECTED_SHA256=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --binary)
            BINARY="${2:-}"
            shift 2
            ;;
        --version)
            VERSION="${2:-}"
            shift 2
            ;;
        --sha256)
            EXPECTED_SHA256="${2:-}"
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

if [[ -z "$BINARY" || -z "$VERSION" || -z "$EXPECTED_SHA256" ]]; then
    usage
    exit 2
fi

if [[ ! -x "$BINARY" ]]; then
    echo "nFPM binary is missing or not executable: $BINARY" >&2
    exit 1
fi

NORMALIZED_VERSION="${VERSION#v}"
if ! [[ "$NORMALIZED_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "nFPM version must be semantic MAJOR.MINOR.PATCH: $VERSION" >&2
    exit 2
fi

if ! [[ "$EXPECTED_SHA256" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "nFPM SHA-256 must be 64 hexadecimal characters" >&2
    exit 2
fi

printf '%s  %s\n' "$EXPECTED_SHA256" "$BINARY" | sha256sum -c -

VERSION_OUTPUT="$("$BINARY" --version)"
ACTUAL_VERSION="$(
    grep -Eo 'v?[0-9]+\.[0-9]+\.[0-9]+' <<<"$VERSION_OUTPUT" |
        head -n 1 |
        sed 's/^v//'
)"

if [[ -z "$ACTUAL_VERSION" ]]; then
    echo "nFPM --version did not contain a semantic version" >&2
    printf '%s\n' "$VERSION_OUTPUT" >&2
    exit 1
fi

if [[ "$ACTUAL_VERSION" != "$NORMALIZED_VERSION" ]]; then
    echo "nFPM version mismatch expected=$NORMALIZED_VERSION actual=$ACTUAL_VERSION" >&2
    printf '%s\n' "$VERSION_OUTPUT" >&2
    exit 1
fi

printf 'nFPM binary verified: %s sha256=%s version=%s\n' "$BINARY" "$EXPECTED_SHA256" "$NORMALIZED_VERSION"
