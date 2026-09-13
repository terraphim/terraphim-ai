#!/usr/bin/env bash
# Native gate for terraphim_server nFPM packages.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
BUILD="$ROOT/.github/scripts/nfpm/build-server-packages.sh"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/terraphim-server-nfpm-native.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

VERSION_OLD="${VERSION_OLD:-0.0.1}"
VERSION_NEW="${VERSION_NEW:-0.0.2}"
TARGET="${TARGET:-x86_64-unknown-linux-musl}"
NFPM_BIN="${NFPM_BIN:-nfpm}"
REQUIRE_INSTALL="${REQUIRE_INSTALL:-1}"
DEB_LIFECYCLE_IMAGE="${DEB_LIFECYCLE_IMAGE:-debian:bookworm-slim}"
RPM_LIFECYCLE_IMAGE="${RPM_LIFECYCLE_IMAGE:-fedora:latest}"

case "$TARGET" in
    x86_64-unknown-linux-musl)
        DEB_ARCH="amd64"
        RPM_ARCH="x86_64"
        NATIVE_MACHINE="x86_64"
        ;;
    aarch64-unknown-linux-musl)
        DEB_ARCH="arm64"
        RPM_ARCH="aarch64"
        NATIVE_MACHINE="aarch64"
        ;;
    *)
        echo "unsupported target for native gate: $TARGET" >&2
        exit 2
        ;;
esac

docker_available() {
    command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1
}

is_native_target() {
    [[ "$(uname -m)" == "$NATIVE_MACHINE" ]]
}

require_install_path() {
    local package_type="$1"
    if [[ "$REQUIRE_INSTALL" == "0" ]]; then
        echo "SKIP: $package_type install/upgrade/remove gate disabled by REQUIRE_INSTALL=0"
        return 1
    fi

    echo "BLOCKED: $package_type install/upgrade/remove gate requires host root or Docker for native target $TARGET" >&2
    exit 127
}

# The fixture must be a real dynamically-linked executable, not a shell script
# (rpmlint E: no-binary) and not a static one (lintian E:
# statically-linked-binary). Compilation is deterministic for a fixed
# compiler/flag set, and the production qualified-byte checks still compare
# exact SHA-256 of whatever input binary is provided.
make_binary() {
    local path="$1"
    local version="$2"
    local cc_bin=""
    local candidate
    for candidate in "${CC:-}" cc gcc clang; do
        if [[ -n "$candidate" ]] && command -v "$candidate" >/dev/null 2>&1; then
            cc_bin="$candidate"
            break
        fi
    done
    [[ -n "$cc_bin" ]] || {
        echo "BLOCKED: a C compiler (CC/cc/gcc/clang) is required to build the native fixture binary" >&2
        exit 127
    }

    mkdir -p "$(dirname "$path")"
    local src="$path.fixture.c"
    cat > "$src" <<'EOF'
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc > 1 && strcmp(argv[1], "--version") == 0) {
        printf("terraphim_server %s\n", VERSION);
        return 0;
    }
    fprintf(stderr, "usage: terraphim_server --version\n");
    return 64;
}
EOF
    "$cc_bin" -O2 -s -DVERSION="\"$version\"" -o "$path" "$src"
    rm -f "$src"
    chmod 0755 "$path"
}

inspect_deb() {
    local deb="$1"
    local binary="$2"
    local expected_sha actual_sha deps arch extract

    extract="$TMP/extract-deb-$(basename "$deb")"
    expected_sha="$(sha256sum "$binary" | awk '{print $1}')"
    mkdir -p "$extract"
    dpkg-deb --extract "$deb" "$extract"
    actual_sha="$(sha256sum "$extract/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$actual_sha" == "$expected_sha" ]] || {
        echo "DEB payload SHA mismatch expected=$expected_sha actual=$actual_sha" >&2
        exit 1
    }
    grep -qx 'dpkg' "$extract/usr/share/terraphim/package-manager.d/terraphim_server"
    arch="$(dpkg-deb --field "$deb" Architecture)"
    [[ "$arch" == "$DEB_ARCH" ]] || {
        echo "DEB arch mismatch expected=$DEB_ARCH actual=$arch" >&2
        exit 1
    }
    deps="$(dpkg-deb --field "$deb" Depends 2>/dev/null || true)"
    if grep -Eiq '(^|[[:space:],|])((lib)?c6|glibc|gcc-libs|libstdc\+\+|libstdc\+\+6|libgcc|libgcc_s|libgcc-s1)([[:space:],|]|$)' <<<"$deps"; then
        echo "MUSL DEB declares forbidden dependency: $deps" >&2
        exit 1
    fi
}

inspect_rpm() {
    local rpm_pkg="$1"
    local binary="$2"
    local expected_sha actual_sha deps arch extract metadata

    extract="$TMP/extract-rpm-$(basename "$rpm_pkg")"
    metadata="$extract.metadata"
    expected_sha="$(sha256sum "$binary" | awk '{print $1}')"
    mkdir -p "$extract"

    if command -v rpm2cpio >/dev/null 2>&1 && command -v rpm >/dev/null 2>&1 && command -v cpio >/dev/null 2>&1; then
        (cd "$extract" && rpm2cpio "$rpm_pkg" | cpio -idmv >/dev/null 2>&1)
        {
            printf 'arch='
            rpm -qp --qf '%{ARCH}' "$rpm_pkg"
            printf '\nrequires<<EOF\n'
            rpm -qpR "$rpm_pkg" 2>/dev/null || true
            printf '\nEOF\nfile_digest='
            rpm -qp --qf '%{FILEDIGESTALGO}' "$rpm_pkg"
            printf '\n'
        } > "$metadata"
    elif docker_available; then
        : > "$metadata"
        docker run --rm \
            -v "$(realpath "$rpm_pkg"):/pkg.rpm:ro" \
            -v "$(realpath "$extract"):/extract" \
            -v "$(realpath "$metadata"):/metadata" \
            "$RPM_LIFECYCLE_IMAGE" \
            sh -euxc '
                if ! command -v rpm2cpio >/dev/null 2>&1 || ! command -v cpio >/dev/null 2>&1; then
                    if command -v dnf >/dev/null 2>&1; then
                        dnf install -y rpm cpio
                    elif command -v microdnf >/dev/null 2>&1; then
                        microdnf install -y rpm cpio
                    else
                        echo "no RPM package manager available in inspection image" >&2
                        exit 127
                    fi
                fi
                cd /extract
                rpm2cpio /pkg.rpm | cpio -idmv >/dev/null 2>&1
                {
                    printf "arch="
                    rpm -qp --qf "%{ARCH}" /pkg.rpm
                    printf "\nrequires<<EOF\n"
                    rpm -qpR /pkg.rpm || true
                    printf "\nEOF\nfile_digest="
                    rpm -qp --qf "%{FILEDIGESTALGO}" /pkg.rpm
                    printf "\n"
                } > /metadata
                # Container writes are root-owned; keep the host-side cleanup
                # trap able to remove them.
                chmod -R a+rwX /extract /metadata
            '
    else
        echo "BLOCKED: RPM inspection requires host rpm/rpm2cpio/cpio or Docker" >&2
        exit 127
    fi

    actual_sha="$(sha256sum "$extract/usr/bin/terraphim_server" | awk '{print $1}')"
    [[ "$actual_sha" == "$expected_sha" ]] || {
        echo "RPM payload SHA mismatch expected=$expected_sha actual=$actual_sha" >&2
        exit 1
    }
    grep -qx 'rpm' "$extract/usr/share/terraphim/package-manager.d/terraphim_server"
    arch="$(sed -n 's/^arch=//p' "$metadata")"
    [[ "$arch" == "$RPM_ARCH" ]] || {
        echo "RPM arch mismatch expected=$RPM_ARCH actual=$arch" >&2
        exit 1
    }
    deps="$(sed -n '/^requires<<EOF$/,/^EOF$/p' "$metadata" | sed '1d;$d')"
    if grep -Eiq '(^|[[:space:],|])((lib)?c6|glibc|gcc-libs|libstdc\+\+|libstdc\+\+6|libgcc|libgcc_s|libgcc-s1)([[:space:],|]|$)' <<<"$deps"; then
        echo "MUSL RPM declares forbidden dependency: $deps" >&2
        exit 1
    fi

    local file_digest
    file_digest="$(sed -n 's/^file_digest=//p' "$metadata")"
    [[ "$file_digest" == "8" ]] || {
        echo "RPM file digest metadata does not prove SHA-256: FILEDIGESTALGO=$file_digest" >&2
        exit 1
    }
}

install_upgrade_remove_deb_host() {
    local old_deb="$1"
    local new_deb="$2"

    dpkg -i "$old_deb"
    dpkg-query -S /usr/bin/terraphim_server >/dev/null
    [[ "$(stat -c '%U:%G %a' /usr/bin/terraphim_server)" == "root:root 755" ]]
    grep -qx 'dpkg' /usr/share/terraphim/package-manager.d/terraphim_server
    dpkg -i "$new_deb"
    terraphim_server --version | grep -F "$VERSION_NEW"
    dpkg -r terraphim-server
    test ! -e /usr/share/terraphim/package-manager.d/terraphim_server
}

install_upgrade_remove_deb_docker() {
    local old_deb="$1"
    local new_deb="$2"

    docker run --rm \
        -v "$(realpath "$old_deb"):/old.deb:ro" \
        -v "$(realpath "$new_deb"):/new.deb:ro" \
        -e VERSION_NEW="$VERSION_NEW" \
        "$DEB_LIFECYCLE_IMAGE" \
        sh -euxc '
            dpkg -i /old.deb
            dpkg-query -S /usr/bin/terraphim_server >/dev/null
            test "$(stat -c "%U:%G %a" /usr/bin/terraphim_server)" = "root:root 755"
            grep -qx dpkg /usr/share/terraphim/package-manager.d/terraphim_server
            dpkg -i /new.deb
            terraphim_server --version | grep -F "$VERSION_NEW"
            dpkg -r terraphim-server
            test ! -e /usr/share/terraphim/package-manager.d/terraphim_server
        '
}

install_upgrade_remove_rpm_host() {
    local old_rpm="$1"
    local new_rpm="$2"

    rpm -Uvh "$old_rpm"
    rpm -qf /usr/bin/terraphim_server >/dev/null
    [[ "$(stat -c '%U:%G %a' /usr/bin/terraphim_server)" == "root:root 755" ]]
    grep -qx 'rpm' /usr/share/terraphim/package-manager.d/terraphim_server
    rpm -Uvh "$new_rpm"
    terraphim_server --version | grep -F "$VERSION_NEW"
    rpm -e terraphim-server
    test ! -e /usr/share/terraphim/package-manager.d/terraphim_server
}

install_upgrade_remove_rpm_docker() {
    local old_rpm="$1"
    local new_rpm="$2"

    docker run --rm \
        -v "$(realpath "$old_rpm"):/old.rpm:ro" \
        -v "$(realpath "$new_rpm"):/new.rpm:ro" \
        -e VERSION_NEW="$VERSION_NEW" \
        "$RPM_LIFECYCLE_IMAGE" \
        sh -euxc '
            if ! command -v rpm >/dev/null 2>&1; then
                if command -v dnf >/dev/null 2>&1; then
                    dnf install -y rpm
                elif command -v microdnf >/dev/null 2>&1; then
                    microdnf install -y rpm
                else
                    echo "no RPM package manager available in lifecycle image" >&2
                    exit 127
                fi
            fi
            rpm -Uvh /old.rpm
            rpm -qf /usr/bin/terraphim_server >/dev/null
            test "$(stat -c "%U:%G %a" /usr/bin/terraphim_server)" = "root:root 755"
            grep -qx rpm /usr/share/terraphim/package-manager.d/terraphim_server
            rpm -Uvh /new.rpm
            terraphim_server --version | grep -F "$VERSION_NEW"
            rpm -e terraphim-server
            test ! -e /usr/share/terraphim/package-manager.d/terraphim_server
        '
}

install_upgrade_remove_deb() {
    local old_deb="$1"
    local new_deb="$2"

    if ! is_native_target; then
        echo "QUALIFIED: $TARGET DEB byte/metadata/lint checks passed; install lifecycle is native-only"
        return 0
    fi

    if [[ "$(id -u)" -eq 0 ]] && command -v dpkg >/dev/null 2>&1; then
        install_upgrade_remove_deb_host "$old_deb" "$new_deb"
    elif docker_available; then
        install_upgrade_remove_deb_docker "$old_deb" "$new_deb"
    else
        require_install_path "DEB"
    fi
}

install_upgrade_remove_rpm() {
    local old_rpm="$1"
    local new_rpm="$2"

    if ! is_native_target; then
        echo "QUALIFIED: $TARGET RPM byte/metadata/lint checks passed; install lifecycle is native-only"
        return 0
    fi

    if [[ "$(id -u)" -eq 0 ]] && command -v rpm >/dev/null 2>&1; then
        install_upgrade_remove_rpm_host "$old_rpm" "$new_rpm"
    elif docker_available; then
        install_upgrade_remove_rpm_docker "$old_rpm" "$new_rpm"
    else
        require_install_path "RPM"
    fi
}

command -v "$NFPM_BIN" >/dev/null 2>&1 || {
    echo "BLOCKED: nFPM is required for native gate: $NFPM_BIN" >&2
    exit 127
}

export SOURCE_DATE_EPOCH=1700000000
OLD_BIN="$TMP/v-old/terraphim_server"
NEW_BIN="$TMP/v-new/terraphim_server"
make_binary "$OLD_BIN" "$VERSION_OLD"
make_binary "$NEW_BIN" "$VERSION_NEW"

"$BUILD" --version "$VERSION_OLD" --target "$TARGET" --binary "$OLD_BIN" --out-dir "$TMP/out-old" --nfpm "$NFPM_BIN"
"$BUILD" --version "$VERSION_NEW" --target "$TARGET" --binary "$NEW_BIN" --out-dir "$TMP/out-new-a" --nfpm "$NFPM_BIN"
"$BUILD" --version "$VERSION_NEW" --target "$TARGET" --binary "$NEW_BIN" --out-dir "$TMP/out-new-b" --nfpm "$NFPM_BIN"

OLD_DEB="$TMP/out-old/terraphim-server_${VERSION_OLD}-1_${DEB_ARCH}.deb"
NEW_DEB_A="$TMP/out-new-a/terraphim-server_${VERSION_NEW}-1_${DEB_ARCH}.deb"
NEW_DEB_B="$TMP/out-new-b/terraphim-server_${VERSION_NEW}-1_${DEB_ARCH}.deb"
OLD_RPM="$TMP/out-old/terraphim-server-${VERSION_OLD}-1.${RPM_ARCH}.rpm"
NEW_RPM_A="$TMP/out-new-a/terraphim-server-${VERSION_NEW}-1.${RPM_ARCH}.rpm"
NEW_RPM_B="$TMP/out-new-b/terraphim-server-${VERSION_NEW}-1.${RPM_ARCH}.rpm"

inspect_deb "$OLD_DEB" "$OLD_BIN"
inspect_deb "$NEW_DEB_A" "$NEW_BIN"
inspect_rpm "$OLD_RPM" "$OLD_BIN"
inspect_rpm "$NEW_RPM_A" "$NEW_BIN"

cmp "$NEW_DEB_A" "$NEW_DEB_B"
cmp "$NEW_RPM_A" "$NEW_RPM_B"

install_upgrade_remove_deb "$OLD_DEB" "$NEW_DEB_A"
install_upgrade_remove_rpm "$OLD_RPM" "$NEW_RPM_A"

echo "server nFPM native gate passed for $TARGET"
