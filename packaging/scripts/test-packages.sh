#!/bin/bash
# packaging/scripts/test-packages.sh
# Comprehensive package testing across multiple Linux distributions

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RELEASE_DIR="$ROOT/release-artifacts"

echo "🧪 Testing all packages across Linux distributions..."
echo ""

# Function to test package installation
test_package() {
    local pkg_type="$1"
    local pkg_file="$2"
    local test_distro="$3"
    
    echo "Testing $pkg_type on $test_distro..."
    
    # Docker test
    if docker run --rm \
        -v "$ROOT:/workspace:ro" \
        -v "$RELEASE_DIR:/packages:ro" \
        "$test_distro" \
        /workspace/test-install.sh "$pkg_type"; then
        echo "✅ $pkg_type on $test_distro: PASSED"
        return 0
    else
        echo "❌ $pkg_type on $test_distro: FAILED"
        return 1
    fi
}

# Test results
RESULTS=()

echo "🐧 Testing DEB packages on Ubuntu..."
if [[ -f "$RELEASE_DIR/terraphim-server_1.0.0-1_amd64.deb" ]]; then
    if test_package "DEB" "terraphim-server_1.0.0-1_amd64.deb" "ubuntu:22.04"; then
        RESULTS+=("Ubuntu 22.04 DEB: ✅")
    else
        RESULTS+=("Ubuntu 22.04 DEB: ❌")
    fi
else
    echo "⚠️ No DEB files found for testing"
fi

echo "🐧 Testing RPM packages on Fedora..."
if [[ -f "$RELEASE_DIR/terraphim-server-1.0.0-1.x86_64.rpm" ]]; then
    if test_package "RPM" "terraphim-server-1.0.0-1.x86_64.rpm" "fedora:39"; then
        RESULTS+=("Fedora 39 RPM: ✅")
    else
        RESULTS+=("Fedora 39 RPM: ❌")
    fi
else
    echo "⚠️ No RPM files found for testing"
fi

echo "🐧 Testing Arch packages..."
if [[ -f "$RELEASE_DIR/terraphim-server-1.0.0-1-x86_64.pkg.tar.zst" ]]; then
    if test_package "Arch" "terraphim-server-1.0.0-1-x86_64.pkg.tar.zst" "archlinux:latest"; then
        RESULTS+=("Arch Linux: ✅")
    else
        RESULTS+=("Arch Linux: ❌")
    fi
else
    echo "⚠️ No Arch packages found for testing"
fi

echo "🐧 Testing AppImage..."
if [[ -f "$RELEASE_DIR/terraphim-desktop_1.0.0_amd64.AppImage" ]]; then
    if test_package "AppImage" "terraphim-desktop_1.0.0_amd64.AppImage" "ubuntu:24.04"; then
        RESULTS+=("AppImage Ubuntu 24.04: ✅")
    else
        RESULTS+=("AppImage Ubuntu 24.04: ❌")
    fi
else
    echo "⚠️ No AppImage files found for testing"
fi

# Display results
echo ""
echo "====================================================================="
echo "📋 Test Results Summary"
echo "====================================================================="

for result in "${RESULTS[@]}"; do
    echo "  $result"
done

# Success rate
total=${#RESULTS[@]}
passed=$(printf '%s\n' "${RESULTS[@]}" | grep -c "✅" || echo "0")
success_rate=$(( passed * 100 / total ))

echo ""
echo "📊 Success Rate: $success_rate% ($passed/$total tests passed)"

if [[ $success_rate -ge 80 ]]; then
    echo "🎉 Package testing: EXCELLENT"
elif [[ $success_rate -ge 60 ]]; then
    echo "✅ Package testing: GOOD"
elif [[ $success_rate -ge 40 ]]; then
    echo "⚠️ Package testing: NEEDS IMPROVEMENT"
else
    echo "❌ Package testing: CRITICAL ISSUES"
fi

echo "====================================================================="