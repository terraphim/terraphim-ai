#!/bin/bash
# packaging/scripts/build-rpm.sh
# Build RPM packages using cargo-rpm or rpmbuild
# Usage: ./build-rpm.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="$ROOT/target/rpm"

echo "Building RPM packages..."

# Ensure rpmbuild tools are available
if ! command -v rpmbuild &> /dev/null; then
    echo "Warning: rpmbuild not found. Install rpm-build package."
    echo "  Fedora/RHEL: sudo dnf install rpm-build"
    echo "  Debian/Ubuntu: sudo apt install rpm"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build release binaries first
echo "Building release binaries..."
cargo build --release -p terraphim_agent

# Create RPM spec file
SPEC_FILE="$OUTPUT_DIR/terraphim-agent.spec"
VERSION=$(grep '^version' "$ROOT/crates/terraphim_agent/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

cat > "$SPEC_FILE" << EOF
Name:           terraphim-agent
Version:        $VERSION
Release:        1%{?dist}
Summary:        Terraphim AI Agent CLI
License:        Apache-2.0
URL:            https://terraphim.ai
Source0:        terraphim-agent

%description
Terraphim Agent - AI Agent CLI Interface for Terraphim.
Command-line interface with interactive REPL and ASCII graph visualization.
Supports search, configuration management, and data exploration.

%install
mkdir -p %{buildroot}%{_bindir}
install -m 755 %{SOURCE0} %{buildroot}%{_bindir}/terraphim-agent

%files
%{_bindir}/terraphim-agent

%changelog
* $(date "+%a %b %d %Y") Terraphim Contributors <team@terraphim.ai> - $VERSION-1
- Initial package
EOF

# Build the RPM
echo "Building RPM from spec..."
rpmbuild -bb "$SPEC_FILE" \
    --define "_topdir $OUTPUT_DIR/rpmbuild" \
    --define "_sourcedir $ROOT/target/release" \
    2>&1 || {
        echo "RPM build failed. Check that rpmbuild is properly configured."
        exit 1
    }

# Copy RPM to output directory
find "$OUTPUT_DIR/rpmbuild/RPMS" -name "*.rpm" -exec cp {} "$OUTPUT_DIR/" \;

echo ""
echo "Generated .rpm packages:"
find "$OUTPUT_DIR" -maxdepth 1 -name "*.rpm" -type f 2>/dev/null | while read -r rpm; do
    echo "  $(basename "$rpm")"
done

echo ""
echo "RPM packages built successfully!"
