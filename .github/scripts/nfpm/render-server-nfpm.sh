#!/usr/bin/env bash
# Render the nFPM descriptor for terraphim_server managed packages.

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage: render-server-nfpm.sh --format deb|rpm --version VERSION --target TRIPLE --binary PATH --output PATH

The target must be a qualified Linux MUSL server binary:
  x86_64-unknown-linux-musl
  aarch64-unknown-linux-musl
EOF
}

FORMAT=""
VERSION=""
TARGET=""
BINARY=""
OUTPUT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format)
            FORMAT="${2:-}"
            shift 2
            ;;
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
        --output)
            OUTPUT="${2:-}"
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

if [[ -z "$FORMAT" || -z "$VERSION" || -z "$TARGET" || -z "$BINARY" || -z "$OUTPUT" ]]; then
    usage
    exit 2
fi

# VERSION is interpolated into the nFPM YAML descriptor below; validate the
# semver grammar before any interpolation (defense in depth, mirroring
# verify-nfpm.sh). An optional v prefix is tolerated and rendered verbatim.
if ! [[ "$VERSION" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "version must be semantic MAJOR.MINOR.PATCH (optional v prefix): $VERSION" >&2
    exit 2
fi

case "$FORMAT" in
    deb)
        RECEIPT_VALUE="dpkg"
        ;;
    rpm)
        RECEIPT_VALUE="rpm"
        ;;
    *)
        echo "unsupported package format: $FORMAT" >&2
        exit 2
        ;;
esac

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
        echo "only qualified MUSL targets are accepted" >&2
        exit 2
        ;;
esac

if [[ ! -f "$BINARY" ]]; then
    echo "missing qualified binary: $BINARY" >&2
    exit 1
fi

case "$FORMAT" in
    deb) ARCH="$DEB_ARCH" ;;
    rpm) ARCH="$RPM_ARCH" ;;
esac

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
LICENSE_FILE="$ROOT/LICENSE-Apache-2.0"
README_FILE="$ROOT/README.md"
if [[ ! -f "$LICENSE_FILE" ]]; then
    echo "missing license file: $LICENSE_FILE" >&2
    exit 1
fi
if [[ ! -f "$README_FILE" ]]; then
    echo "missing readme file: $README_FILE" >&2
    exit 1
fi

if [[ -z "${SOURCE_DATE_EPOCH:-}" ]]; then
    echo "SOURCE_DATE_EPOCH is required to render deterministic package metadata" >&2
    exit 1
fi

OUTPUT_DIR="$(dirname "$OUTPUT")"
mkdir -p "$OUTPUT_DIR"
RECEIPT_FILE="${OUTPUT}.terraphim_server.receipt"
printf '%s\n' "$RECEIPT_VALUE" > "$RECEIPT_FILE"
chmod 0644 "$RECEIPT_FILE"

COPYRIGHT_FILE="${OUTPUT}.copyright"
CHANGELOG_FILE="${OUTPUT}.changelog.Debian"
CHANGELOG_GZ="${CHANGELOG_FILE}.gz"
CHANGELOG_YAML="${OUTPUT}.changelog.yaml"
MANPAGE_FILE="${OUTPUT}.terraphim_server.1"
MANPAGE_GZ="${MANPAGE_FILE}.gz"

# Machine-readable Debian copyright that references the common license instead
# of embedding the full Apache-2.0 text. Upstream name, source and copyright
# holder are taken from the repository metadata (Cargo.toml).
cat > "$COPYRIGHT_FILE" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: terraphim-ai
Upstream-Contact: Terraphim Team <team@terraphim.ai>
Source: https://github.com/terraphim/terraphim-ai

Files: *
Copyright: 2024, Terraphim Contributors
License: Apache-2.0
 On Debian systems, the complete text of the Apache License, version 2.0,
 can be found in /usr/share/common-licenses/Apache-2.0.
EOF

cat > "$CHANGELOG_FILE" <<EOF
terraphim-server (${VERSION}-1) stable; urgency=medium

  * Build managed package from a qualified terraphim_server MUSL binary.

 -- Terraphim Contributors <team@terraphim.ai>  $(date -u -d "@${SOURCE_DATE_EPOCH}" '+%a, %d %b %Y %H:%M:%S +0000')
EOF
gzip -9n -c "$CHANGELOG_FILE" > "$CHANGELOG_GZ"

# chglog YAML consumed by nFPM for the native RPM changelog tags.
cat > "$CHANGELOG_YAML" <<EOF
- semver: ${VERSION}
  date: $(date -u -d "@${SOURCE_DATE_EPOCH}" '+%Y-%m-%dT%H:%M:%SZ')
  packager: Terraphim Contributors <team@terraphim.ai>
  changes:
    - commit: ""
      note: Build managed package from a qualified terraphim_server MUSL binary.
EOF

cat > "$MANPAGE_FILE" <<'EOF'
.TH TERRAPHIM_SERVER 1
.SH NAME
terraphim_server \- Terraphim AI server
.SH SYNOPSIS
.B terraphim_server
.RI [ options ]
.SH DESCRIPTION
Terraphim AI server provides the privacy-first backend for semantic search and
knowledge graph operations.
.SH SEE ALSO
Project documentation is available at https://terraphim.ai.
EOF
gzip -9n -c "$MANPAGE_FILE" > "$MANPAGE_GZ"

touch -d "@${SOURCE_DATE_EPOCH}" \
    "$RECEIPT_FILE" \
    "$COPYRIGHT_FILE" \
    "$CHANGELOG_FILE" \
    "$CHANGELOG_GZ" \
    "$CHANGELOG_YAML" \
    "$MANPAGE_FILE" \
    "$MANPAGE_GZ"
chmod 0644 "$COPYRIGHT_FILE" "$CHANGELOG_GZ" "$MANPAGE_GZ"

# The native RPM changelog tags come from the chglog YAML; the DEB keeps the
# deterministic hand-rendered changelog.Debian.gz instead.
CHANGELOG_CONFIG=""
if [[ "$FORMAT" == "rpm" ]]; then
    CHANGELOG_CONFIG="changelog: ${CHANGELOG_YAML}"
fi

cat > "$OUTPUT" <<EOF
name: terraphim-server
arch: ${ARCH}
platform: linux
version: ${VERSION}
release: "1"
section: utils
priority: optional
maintainer: Terraphim Contributors <team@terraphim.ai>
vendor: Terraphim
homepage: https://terraphim.ai
license: Apache-2.0
${CHANGELOG_CONFIG}
description: |-
  Terraphim AI server.
  Privacy-first AI assistant backend for semantic search and knowledge graphs.
rpm:
  group: Applications/System
  summary: Privacy-first semantic search server
  compression: xz
deb:
  compression: xz
contents:
  - src: ${BINARY}
    dst: /usr/bin/terraphim_server
    type: file
    file_info:
      mode: 0755
  - src: ${RECEIPT_FILE}
    dst: /usr/share/terraphim/package-manager.d/terraphim_server
    type: file
    file_info:
      mode: 0644
  - src: ${LICENSE_FILE}
    dst: /usr/share/doc/terraphim-server/LICENSE
    type: file
    file_info:
      mode: 0644
    packager: deb
  - src: ${LICENSE_FILE}
    dst: /usr/share/licenses/terraphim-server/LICENSE-Apache-2.0
    type: license
    file_info:
      mode: 0644
    packager: rpm
  - src: ${README_FILE}
    dst: /usr/share/doc/terraphim-server/README.md
    type: doc
    file_info:
      mode: 0644
    packager: rpm
  - src: ${COPYRIGHT_FILE}
    dst: /usr/share/doc/terraphim-server/copyright
    type: file
    file_info:
      mode: 0644
    packager: deb
  - src: ${CHANGELOG_GZ}
    dst: /usr/share/doc/terraphim-server/changelog.Debian.gz
    type: file
    file_info:
      mode: 0644
    packager: deb
  - src: ${MANPAGE_GZ}
    dst: /usr/share/man/man1/terraphim_server.1.gz
    type: file
    file_info:
      mode: 0644
    packager: deb
  - src: ${MANPAGE_GZ}
    dst: /usr/share/man/man1/terraphim_server.1.gz
    type: doc
    file_info:
      mode: 0644
    packager: rpm
overrides:
  deb:
    depends: []
  rpm:
    depends: []
EOF

echo "$OUTPUT"
