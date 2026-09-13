#!/usr/bin/env bash
# Wrap qualified terraphim_server MUSL binaries into managed DEB/RPM packages.
#
# Fails closed on architecture: the produced DEB must declare Architecture ==
# DEB_ARCH for the requested target and the produced RPM must carry
# ARCH == RPM_ARCH (consumed from the Docker RPM metadata when host rpm
# tooling is unavailable). The optional cargo-deb parity package must carry
# the same DEB_ARCH and the exact payload bytes.

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

FORBIDDEN_MUSL_DEPS_RE='(^|[[:space:],|])((lib)?c6|glibc|gcc-libs|libstdc\+\+|libstdc\+\+6|libgcc|libgcc_s|libgcc-s1)([[:space:],|]|$)'
DEB_LINT_IMAGE="${DEB_LINT_IMAGE:-debian:bookworm-slim}"
RPM_LINT_IMAGE="${RPM_LINT_IMAGE:-fedora:latest}"
RPM_TOOL_IMAGE="${RPM_TOOL_IMAGE:-fedora:latest}"

# The single justified lint error: the qualified MUSL server binary is
# intentionally fully static, and both lintian and rpmlint report exactly
# this fact for package terraphim-server at path usr/bin/terraphim_server:
#   lintian: E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server]
#   rpmlint: terraphim-server.<arch>: E: statically-linked-binary /usr/bin/terraphim_server
LINTIAN_JUSTIFIED_STATIC_E='E: terraphim-server: statically-linked-binary [usr/bin/terraphim_server]'

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

# Fail-closed lint result policy shared by the host and Docker lint paths.
#
# The only tolerated lint error is the exact justified static-MUSL
# diagnostic (LINTIAN_JUSTIFIED_STATIC_E / the rpmlint equivalent for the
# requested RPM_ARCH), and only once. Everything else fails the build:
#   * any other error line, whatever the tag;
#   * a wrong package or a wrong path variant of the static diagnostic;
#   * a malformed variant or a duplicate of the justified line;
#   * a tool, installer or transport failure: any exit status outside the
#     tool's documented clean/errors pair (lintian: 0 clean, 2 errors;
#     rpmlint: 0 clean, 64 errors, 65 promoted-errors);
#   * an inconsistent result: an errors exit without a parseable error
#     line (lintian also exits 2 for an unreadable input), or a clean
#     exit with error lines in the output.
# Warnings remain nonfatal per the established `--fail-on error` lint
# policy. The complete raw transport and lint output is echoed as
# evidence before the verdict.
enforce_lint_policy() {
    local tool="$1"
    local pkg="$2"
    local log="$3"
    local rc="$4"
    local transport="$5"

    echo "----- $tool transport log for $(basename "$pkg") -----"
    cat "$transport" 2>/dev/null || true
    echo "----- $tool raw lint output for $(basename "$pkg") (exit $rc) -----"
    cat "$log" 2>/dev/null || true
    echo "----- end $tool evidence for $(basename "$pkg") -----"

    if [[ ! -f "$log" ]]; then
        echo "$tool produced no lint output file for $pkg" >&2
        exit 1
    fi

    local -a error_rcs=()
    local justified_re=""
    case "$tool" in
        lintian)
            error_rcs=(2)
            ;;
        rpmlint)
            error_rcs=(64 65)
            justified_re="^terraphim-server\\.${RPM_ARCH}: E: statically-linked-binary /usr/bin/terraphim_server$"
            ;;
        *)
            echo "unknown lint tool: $tool" >&2
            exit 1
            ;;
    esac

    local rc_class="invalid"
    if [[ "$rc" -eq 0 ]]; then
        rc_class="clean"
    else
        local candidate
        for candidate in "${error_rcs[@]}"; do
            if [[ "$rc" -eq "$candidate" ]]; then
                rc_class="errors"
            fi
        done
    fi
    if [[ "$rc_class" == "invalid" ]]; then
        printf '%s exited %s for %s: tool/install/transport failure (allowed exits: 0' "$tool" "$rc" "$pkg" >&2
        printf ' %s' "${error_rcs[@]}" >&2
        printf ')\n' >&2
        exit 1
    fi

    if [[ "$tool" == "rpmlint" && ! -s "$log" ]]; then
        echo "rpmlint produced empty lint output for $pkg (a real run always reports a session banner)" >&2
        exit 1
    fi

    local -a e_lines=()
    case "$tool" in
        lintian)
            mapfile -t e_lines < <(grep -E '^E: ' "$log" || true)
            ;;
        rpmlint)
            mapfile -t e_lines < <(grep -E '^[^:[:space:]]+: E: ' "$log" || true)
            ;;
    esac

    local justified=0
    local line ok
    for line in "${e_lines[@]}"; do
        ok=0
        if [[ "$tool" == "lintian" ]]; then
            if [[ "$line" == "$LINTIAN_JUSTIFIED_STATIC_E" ]]; then
                ok=1
            fi
        else
            if [[ "$line" =~ $justified_re ]]; then
                ok=1
            fi
        fi
        if [[ "$ok" -eq 1 ]]; then
            justified=$((justified + 1))
            if [[ "$justified" -gt 1 ]]; then
                echo "$tool reported the justified static-MUSL diagnostic more than once for $pkg:" >&2
                printf '  %s\n' "$line" >&2
                exit 1
            fi
        else
            echo "$tool reported an unjustified error for $pkg (only the exact static-MUSL diagnostic for terraphim-server at usr/bin/terraphim_server is tolerated):" >&2
            printf '  %s\n' "$line" >&2
            exit 1
        fi
    done

    if [[ "$rc_class" == "errors" && "$justified" -eq 0 ]]; then
        echo "$tool exited $rc (errors reported) but no error line could be parsed from the lint output for $pkg" >&2
        exit 1
    fi
    if [[ "$rc_class" == "clean" && "$justified" -ne 0 ]]; then
        echo "$tool exited 0 but the lint output contains error lines for $pkg" >&2
        exit 1
    fi

    echo "lint policy satisfied: $tool accepted $(basename "$pkg") with $justified justified static-MUSL diagnostic(s)"
}

lint_deb() {
    local pkg="$1"
    local log="$WORK_DIR/lintian-$(basename "$pkg").log"
    local transport="$log.transport"
    local rc=0

    : > "$log"
    : > "$transport"

    if command -v lintian >/dev/null 2>&1; then
        # --tag-display-limit 0 keeps lintian from hiding error lines
        # behind its per-tag display cap: the parser must see every E:.
        lintian --fail-on error --tag-display-limit 0 "$pkg" >"$log" 2>&1 || rc=$?
    else
        require_docker_or_fail "DEB linting"
        docker run --rm \
            -v "$(realpath "$pkg"):/pkg.deb:ro" \
            -v "$(realpath "$log"):/lint.log" \
            "$DEB_LINT_IMAGE" sh -euxc '
                export DEBIAN_FRONTEND=noninteractive
                apt-get update
                apt-get install -y --no-install-recommends lintian
                rc=0
                lintian --fail-on error --tag-display-limit 0 /pkg.deb >/lint.log 2>&1 || rc=$?
                exit "$rc"
            ' >"$transport" 2>&1 || rc=$?
    fi

    enforce_lint_policy lintian "$pkg" "$log" "$rc" "$transport"
}

lint_rpm() {
    local pkg="$1"
    # Mount with the real package basename so rpmlint sees a coherent
    # name-version-release.arch.rpm filename.
    local base
    base="$(basename "$pkg")"
    local log="$WORK_DIR/rpmlint-$base.log"
    local transport="$log.transport"
    local rc=0

    : > "$log"
    : > "$transport"

    if command -v rpmlint >/dev/null 2>&1; then
        rpmlint "$pkg" >"$log" 2>&1 || rc=$?
    else
        require_docker_or_fail "RPM linting"
        docker run --rm \
            -v "$(realpath "$pkg"):/$base:ro" \
            -v "$(realpath "$log"):/lint.log" \
            "$RPM_LINT_IMAGE" sh -euxc '
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
                rc=0
                rpmlint "/$1" >/lint.log 2>&1 || rc=$?
                exit "$rc"
            ' sh "$base" >"$transport" 2>&1 || rc=$?
    fi

    enforce_lint_policy rpmlint "$pkg" "$log" "$rc" "$transport"
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

    "$NFPM_BIN" pkg --packager "$format" --config "$config" --target "$PACKAGE_DIR"
}

cleanup_stage() {
    local rc=$?
    trap - EXIT
    if [[ -n "${STAGE_ROOT:-}" && -n "${STAGE_PARENT:-}" &&
        "$(dirname -- "$STAGE_ROOT")" == "$STAGE_PARENT" &&
        "$(basename -- "$STAGE_ROOT")" == .terraphim-server-nfpm.* &&
        ( -e "$STAGE_ROOT" || -L "$STAGE_ROOT" ) ]]; then
        # rm does not dereference a symlink supplied as its command-line
        # operand. The constrained mktemp basename prevents a broad target.
        rm -rf -- "$STAGE_ROOT" || true
    fi
    exit "$rc"
}

require_safe_empty_output_dir() {
    if [[ -L "$OUT_DIR" || ( -e "$OUT_DIR" && ! -d "$OUT_DIR" ) ]]; then
        echo "unsafe output directory (must be a regular directory, not a symlink): $OUT_DIR" >&2
        exit 1
    fi
    if [[ -d "$OUT_DIR" ]] && find "$OUT_DIR" -mindepth 1 -maxdepth 1 -print -quit | grep -q .; then
        echo "output directory must be empty; refusing to delete pre-existing data: $OUT_DIR" >&2
        exit 1
    fi
}

validate_publish_inventory() {
    local expected_deb="$1"
    local expected_rpm="$2"
    local expected_sums="$3"
    local path base count=0

    while IFS= read -r -d '' path; do
        if [[ -L "$path" || ! -f "$path" || ! -s "$path" ]]; then
            echo "staged package output must be a non-empty regular non-symlink file: $path" >&2
            exit 1
        fi
        base="$(basename "$path")"
        if [[ "$base" != "$expected_deb" && "$base" != "$expected_rpm" && "$base" != "$expected_sums" ]]; then
            echo "unexpected staged package output: $path" >&2
            exit 1
        fi
        count=$((count + 1))
    done < <(find "$PACKAGE_DIR" -mindepth 1 -maxdepth 1 -print0)

    [[ "$count" -eq 3 && -f "$PACKAGE_DIR/$expected_deb" &&
        -f "$PACKAGE_DIR/$expected_rpm" && -f "$PACKAGE_DIR/$expected_sums" ]] || {
        echo "staged package inventory is incomplete" >&2
        exit 1
    }
}

verify_deb() {
    local pkg="$1"
    local tmp="$WORK_DIR/deb-extract"

    [[ -f "$pkg" ]] || { echo "missing DEB output: $pkg" >&2; exit 1; }

    # Fail closed unless the produced package declares the architecture that
    # was requested for the target triple.
    local pkg_arch
    pkg_arch="$(dpkg-deb --field "$pkg" Architecture)"
    [[ "$pkg_arch" == "$DEB_ARCH" ]] || {
        echo "DEB arch mismatch expected=$DEB_ARCH actual=$pkg_arch package=$pkg" >&2
        exit 1
    }

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

    # Fail closed unless the produced package carries the architecture that
    # was requested for the target triple. The arch is consumed from the
    # Docker RPM metadata when host rpm tooling produced it, or queried from
    # the host rpm otherwise.
    local pkg_arch
    if [[ -s "$metadata" ]]; then
        pkg_arch="$(sed -n 's/^arch=//p' "$metadata")"
    else
        pkg_arch="$(rpm -qp --qf '%{ARCH}' "$pkg")"
    fi
    [[ "$pkg_arch" == "$RPM_ARCH" ]] || {
        echo "RPM arch mismatch expected=$RPM_ARCH actual=$pkg_arch package=$pkg" >&2
        exit 1
    }

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

    local cargo_arch
    cargo_arch="$(dpkg-deb --field "${matches[0]}" Architecture)"
    [[ "$cargo_arch" == "$DEB_ARCH" ]] || {
        echo "cargo-deb arch mismatch expected=$DEB_ARCH actual=$cargo_arch package=${matches[0]}" >&2
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

main() {
    local VERSION="" TARGET="" BINARY="" OUT_DIR="" CARGO_DEB_DIR=""
    local NFPM_BIN="${NFPM_BIN:-nfpm}"

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

    local ROOT RENDER OUT_PARENT OUT_BASE
    ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
    RENDER="$ROOT/.github/scripts/nfpm/render-server-nfpm.sh"
    OUT_PARENT="$(dirname -- "$OUT_DIR")"
    OUT_BASE="$(basename -- "$OUT_DIR")"
    mkdir -p "$OUT_PARENT"
    OUT_DIR="$OUT_PARENT/$OUT_BASE"
    require_safe_empty_output_dir

    # Stage on the output filesystem so publishing can replace the empty
    # destination with one directory rename after every validation passes.
    STAGE_PARENT="$OUT_PARENT"
    STAGE_ROOT="$(mktemp -d "$STAGE_PARENT/.terraphim-server-nfpm.XXXXXX")"
    WORK_DIR="$STAGE_ROOT/work"
    PACKAGE_DIR="$STAGE_ROOT/packages"
    mkdir -m 0700 "$WORK_DIR" "$PACKAGE_DIR"
    trap cleanup_stage EXIT

    if [[ -z "${SOURCE_DATE_EPOCH:-}" ]]; then
        if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
            export SOURCE_DATE_EPOCH
            SOURCE_DATE_EPOCH="$(git -C "$ROOT" log -1 --format=%ct)"
        else
            echo "SOURCE_DATE_EPOCH is required outside a git worktree" >&2
            exit 1
        fi
    fi

    local EXPECTED_SHA=""
    EXPECTED_SHA="$(sha256sum "$BINARY" | awk '{print $1}')"

    render_and_build deb
    render_and_build rpm

    local DEB RPM DEB_BASE RPM_BASE SUMS_BASE
    DEB_BASE="terraphim-server_${VERSION}-1_${DEB_ARCH}.deb"
    RPM_BASE="terraphim-server-${VERSION}-1.${RPM_ARCH}.rpm"
    SUMS_BASE="terraphim-server-${VERSION}-${TARGET}.package-sha256sums.txt"
    DEB="$PACKAGE_DIR/$DEB_BASE"
    RPM="$PACKAGE_DIR/$RPM_BASE"

    verify_deb "$DEB"
    verify_rpm "$RPM"
    if [[ -n "$CARGO_DEB_DIR" ]]; then
        verify_cargo_deb_parity "$CARGO_DEB_DIR"
    fi

    (cd "$PACKAGE_DIR" && sha256sum "$DEB_BASE" "$RPM_BASE" > "$SUMS_BASE")
    validate_publish_inventory "$DEB_BASE" "$RPM_BASE" "$SUMS_BASE"

    # Recheck immediately before publication. GNU mv -T treats OUT_DIR as the
    # exact destination and atomically replaces an empty directory without
    # traversing a raced symlink or nesting packages inside a raced directory.
    require_safe_empty_output_dir
    mv -T -- "$PACKAGE_DIR" "$OUT_DIR"
    printf 'package payload ok %s %s\n' "$TARGET" "$EXPECTED_SHA"
}

# Tests source this script with TERRAPHIM_BUILD_SERVER_PACKAGES_SOURCED=1 to
# exercise verify_deb/verify_rpm on tampered fixtures without running the
# production pipeline.
if [[ "${TERRAPHIM_BUILD_SERVER_PACKAGES_SOURCED:-0}" != "1" ]]; then
    main "$@"
fi
