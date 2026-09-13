#!/usr/bin/env bash
# Wrap qualified terraphim_server MUSL binaries into managed DEB/RPM packages.

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: build-server-packages.sh --version VERSION --target TRIPLE --binary PATH --out-dir DIR [--nfpm PATH] [--cargo-deb-dir DIR]

Produces:
  terraphim-server_VERSION-1_amd64.deb
  terraphim-server-VERSION-1.x86_64.rpm
or the arm64/aarch64 equivalents.
EOF
}

VERSION=""
TARGET=""
BINARY=""
OUT_DIR=""
NFPM_BIN="${NFPM_BIN:-nfpm}"
CARGO_DEB_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            VERSION="${2:-}"
            shift 2
            ;;
        --target)
            TARGET="${2:-}"
            shift 2
            ;;
        --binary)
            BINARY="${2:-}"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="${2:-}"
            shift 2
            ;;
        --nfpm)
            NFPM_BIN="${2:-}"
            shift 2
            ;;
        --cargo-deb-dir)
            CARGO_DEB_DIR="${2:-}"
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

if [[ -z "$VERSION" || -z "$TARGET" || -z "$BINARY" || -z "$OUT_DIR" ]]; then
    usage
    exit 2
fi

case "$TARGET" in
    x86_64-unknown-linux-musl)
        DEB_ARCH="amd64"
        RPM_ARCH="x86_64"
        ;;
    aarch64-unknown-linux-musl)
        DEB_ARCH="arm64"
        RPM_ARCH="aarch64"
        ;;
    *)
        echo "unsupported server package target: $TARGET" >&2
        exit 2
        ;;
esac

if [[ ! -f "$BINARY" ]]; then
    echo "missing qualified binary: $BINARY" >&2
    exit 1
fi

if ! command -v "$NFPM_BIN" >/dev/null 2>&1; then
    echo "nFPM is required for managed package production; not found: $NFPM_BIN" >&2
    echo "retain the existing cargo-deb parity path until the nFPM native gates pass" >&2
    exit 127
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
mkdir -p "$OUT_DIR"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-server-nfpm.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
DEB_LINT_IMAGE="${DEB_LINT_IMAGE:-debian:bookworm-slim}"
RPM_LINT_IMAGE="${RPM_LINT_IMAGE:-fedora:latest}"
RPM_TOOL_IMAGE="${RPM_TOOL_IMAGE:-fedora:latest}"

if [[ -z "${SOURCE_DATE_EPOCH:-}" ]]; then
    if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        export SOURCE_DATE_EPOCH
        SOURCE_DATE_EPOCH="$(git -C "$ROOT" log -1 --format=%ct)"
    else
        echo "SOURCE_DATE_EPOCH is required outside a git worktree" >&2
        exit 1
    fi
fi

EXPECTED_SHA="$(sha256sum "$BINARY" | awk '{print $1}')"
FORBIDDEN_MUSL_DEPS_RE='(^|[[:space:],|])((lib)?c6|glibc|gcc-libs|libstdc\+\+|libstdc\+\+6|libgcc|libgcc_s|libgcc-s1)([[:space:],|]|$)'

docker_available() {
    command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

require_docker_or_fail() {
    local purpose="$1"
    docker_available || {
        echo "BLOCKED: Docker is required for $purpose when host tools are unavailable" >&2
        exit 127
    }
}

lint_deb() {
    local pkg="$1"
    if command -v lintian >/dev/null 2>&1; then
        lintian --fail-on error "$pkg"
        return 0
    fi

    require_docker_or_fail "DEB linting"
    docker run --rm -v "$(realpath "$pkg"):/pkg.deb:ro" "$DEB_LINT_IMAGE" sh -euxc '
        export DEBIAN_FRONTEND=noninteractive
        apt-get update
        apt-get install -y --no-install-recommends lintian
        lintian --fail-on error /pkg.deb
    '
}

lint_rpm() {
    local pkg="$1"
    if command -v rpmlint >/dev/null 2>&1; then
        rpmlint "$pkg"
        return 0
    fi

    # Mount with the real package basename so rpmlint sees a coherent
    # name-version-release.arch.rpm filename.
    local base
    base="$(basename "$pkg")"
    require_docker_or_fail "RPM linting"
    docker run --rm -v "$(realpath "$pkg"):/$base:ro" "$RPM_LINT_IMAGE" sh -euxc '
        if ! command -v rpmlint >/dev/null 2>&1; then
            if command -v dnf >/dev/null 2>&1; then
                dnf install -y rpmlint
            elif command -v microdnf >/dev/null 2>&1; then
                microdnf install -y rpmlint
            else
                echo "no RPM package manager available in lint image" >&2
                exit 127
            fi
        fi
        rpmlint "/$1"
    ' sh "$base"
}

docker_rpm_tool() {
    local pkg="$1"
    local extract="$2"
    local expected_sha="$3"
    local metadata="$4"
    local abs_pkg abs_extract abs_metadata

    abs_pkg="$(realpath "$pkg")"
    abs_extract="$(realpath "$extract")"
    abs_metadata="$(realpath "$metadata")"
    require_docker_or_fail "RPM payload and metadata verification"

    docker run --rm \
        -v "$abs_pkg:/pkg.rpm:ro" \
        -v "$abs_extract:/extract" \
        -v "$abs_metadata:/metadata" \
        "$RPM_TOOL_IMAGE" \
        sh -euxc '
            if ! command -v rpm2cpio >/dev/null 2>&1 || ! command -v cpio >/dev/null 2>&1; then
                if command -v dnf >/dev/null 2>&1; then
                    dnf install -y rpm cpio
                elif command -v microdnf >/dev/null 2>&1; then
                    microdnf install -y rpm cpio
                else
                    echo "no RPM package manager available in verification image" >&2
                    exit 127
                fi
            fi
            cd /extract
            rpm2cpio /pkg.rpm | cpio -idmv >/dev/null 2>&1
            actual_sha="$(sha256sum /extract/usr/bin/terraphim_server | awk "{print \$1}")"
            test "$actual_sha" = "$1"
            grep -qx rpm /extract/usr/share/terraphim/package-manager.d/terraphim_server
            {
                printf "arch="
                rpm -qp --qf "%{ARCH}" /pkg.rpm
                printf "\nrequires<<EOF\n"
                rpm -qpR /pkg.rpm || true
                printf "\nEOF\nfile_digest="
                rpm -qp --qf "%{FILEDIGESTALGO}" /pkg.rpm
                printf "\n"
            } > /metadata
            # Container writes are root-owned; keep the host-side cleanup trap
            # able to remove them.
            chmod -R a+rwX /extract /metadata
        ' sh "$expected_sha"
}

render_and_build() {
    local format="$1"
    local config="$WORK_DIR/terraphim-server-$format.yaml"

    "$RENDER" \
        --format "$format" \
        --version "$VERSION" \
        --target "$TARGET" \
        --binary "$BINARY" \
        --output "$config" >/dev/null

    "$NFPM_BIN" pkg --packager "$format" --config "$config" --target "$OUT_DIR"
}

render_and_build deb
render_and_build rpm

DEB="$OUT_DIR/terraphim-server_${VERSION}-1_${DEB_ARCH}.deb"
RPM="$OUT_DIR/terraphim-server-${VERSION}-1.${RPM_ARCH}.rpm"

verify_deb() {
    local pkg="$1"
    local tmp="$WORK_DIR/deb-extract"

    [[ -f "$pkg" ]] || { echo "missing DEB output: $pkg" >&2; exit 1; }
    mkdir -p "$tmp"
    dpkg-deb --extract "$pkg" "$tmp"

    local actual_sha
    actual_sha="$(sha256sum "$tmp/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$actual_sha" == "$EXPECTED_SHA" ]] || {
        echo "DEB payload SHA mismatch expected=$EXPECTED_SHA actual=$actual_sha" >&2
        exit 1
    }

    grep -qx 'dpkg' "$tmp/usr/share/terraphim/package-manager.d/terraphim_server"

    local deps
    deps="$(dpkg-deb --field "$pkg" Depends 2>/dev/null || true)"
    if grep -Eiq "$FORBIDDEN_MUSL_DEPS_RE" <<<"$deps"; then
        echo "MUSL DEB declares forbidden glibc/gcc runtime dependency: $deps" >&2
        exit 1
    fi

    lint_deb "$pkg"
}

verify_rpm() {
    local pkg="$1"
    local tmp="$WORK_DIR/rpm-extract"
    local metadata="$WORK_DIR/rpm.metadata"

    [[ -f "$pkg" ]] || { echo "missing RPM output: $pkg" >&2; exit 1; }
    mkdir -p "$tmp"
    : > "$metadata"

    if command -v rpm2cpio >/dev/null 2>&1 && command -v rpm >/dev/null 2>&1 && command -v cpio >/dev/null 2>&1; then
        (cd "$tmp" && rpm2cpio "$pkg" | cpio -idmv >/dev/null 2>&1)
    else
        docker_rpm_tool "$pkg" "$tmp" "$EXPECTED_SHA" "$metadata"
    fi

    local actual_sha
    actual_sha="$(sha256sum "$tmp/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$actual_sha" == "$EXPECTED_SHA" ]] || {
        echo "RPM payload SHA mismatch expected=$EXPECTED_SHA actual=$actual_sha" >&2
        exit 1
    }

    grep -qx 'rpm' "$tmp/usr/share/terraphim/package-manager.d/terraphim_server"

    local deps
    if [[ -s "$metadata" ]]; then
        deps="$(sed -n '/^requires<<EOF$/,/^EOF$/p' "$metadata" | sed '1d;$d')"
    else
        deps="$(rpm -qpR "$pkg" 2>/dev/null || true)"
    fi
    if grep -Eiq "$FORBIDDEN_MUSL_DEPS_RE" <<<"$deps"; then
        echo "MUSL RPM declares forbidden glibc/gcc runtime dependency: $deps" >&2
        exit 1
    fi

    local pkg_digest_algo
    if [[ -s "$metadata" ]]; then
        pkg_digest_algo="$(sed -n 's/^file_digest=//p' "$metadata")"
    else
        pkg_digest_algo="$(rpm -qp --qf '%{FILEDIGESTALGO}' "$pkg")"
    fi
    [[ "$pkg_digest_algo" == "8" ]] || {
        echo "RPM file digest metadata does not prove SHA-256: FILEDIGESTALGO=$pkg_digest_algo" >&2
        exit 1
    }

    lint_rpm "$pkg"
}

verify_cargo_deb_parity() {
    local cargo_deb_dir="$1"
    local tmp="$WORK_DIR/cargo-deb-extract"
    local matches

    [[ -d "$cargo_deb_dir" ]] || {
        echo "missing cargo-deb artifact directory: $cargo_deb_dir" >&2
        exit 1
    }

    mapfile -t matches < <(find "$cargo_deb_dir" -type f -name 'terraphim-server_*.deb' | sort)
    [[ "${#matches[@]}" -eq 1 ]] || {
        printf 'expected exactly one cargo-deb terraphim-server package, found %s in %s\n' "${#matches[@]}" "$cargo_deb_dir" >&2
        printf '%s\n' "${matches[@]}" >&2
        exit 1
    }

    mkdir -p "$tmp"
    dpkg-deb --extract "${matches[0]}" "$tmp"

    local cargo_sha nfpm_sha
    cargo_sha="$(sha256sum "$tmp/usr/bin/terraphim_server" | awk '{print $1}')"
    nfpm_sha="$(sha256sum "$WORK_DIR/deb-extract/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$cargo_sha" == "$EXPECTED_SHA" ]] || {
        echo "cargo-deb payload SHA mismatch expected=$EXPECTED_SHA actual=$cargo_sha package=${matches[0]}" >&2
        exit 1
    }
    [[ "$cargo_sha" == "$nfpm_sha" ]] || {
        echo "cargo-deb/nFPM payload SHA mismatch cargo=$cargo_sha nfpm=$nfpm_sha" >&2
        exit 1
    }
}

verify_deb "$DEB"
verify_rpm "$RPM"
if [[ -n "$CARGO_DEB_DIR" ]]; then
    verify_cargo_deb_parity "$CARGO_DEB_DIR"
fi

sha256sum "$DEB" "$RPM" > "$OUT_DIR/terraphim-server-${VERSION}-${TARGET}.package-sha256sums.txt"
printf 'package payload ok %s %s\n' "$TARGET" "$EXPECTED_SHA"
