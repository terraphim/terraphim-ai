# Terraphim AI Complete Release Fix Plan

## Executive Summary 🎯

**Goal**: Achieve 100% Linux distribution coverage with all major package formats in 12 weeks.

**Current Status**: 2/8 formats working (25% coverage)
**Target Status**: 8/8 formats working (100% coverage)

---

## Phase 1: Foundation Setup (Weeks 1-2) 🏗️

### **Week 1: Infrastructure & Tools**

#### 1.1 Install Required Tools
```bash
# RPM packaging
cargo install cargo-generate-rpm

# Arch packaging
pacman -S base-devel

# AppImage tools
wget -O ~/bin/appimagetool "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
chmod +x ~/bin/appimagetool

# Flatpak
sudo apt install flatpak flatpak-builder

# Snap
snap install snapcraft --classic
```

#### 1.2 Create Packaging Directory Structure
```
packaging/
├── rpm/
│   ├── terraphim-server.spec
│   ├── terraphim-agent.spec
│   └── desktop.spec
├── arch/
│   ├── PKGBUILD-server
│   ├── PKGBUILD-agent
│   └── PKGBUILD-desktop
├── flatpak/
│   ├── com.terraphim.ai.desktop.json
│   └── com.terraphim.ai.desktop.yml
├── snap/
│   ├── snapcraft.yaml
│   └── local/
└── scripts/
    ├── build-all.sh
    ├── test-packages.sh
    └── sign-packages.sh
```

#### 1.3 Update Build Scripts
- Modify `scripts/build-linux-artifacts.sh`
- Create `scripts/build-all-formats.sh`
- Update CI/CD configuration

---

## Phase 2: Essential Package Formats (Weeks 3-6) 📦

### **Week 2-3: RPM Implementation**

#### 2.1 Create RPM Spec Files
```rpm
# packaging/rpm/terraphim-server.spec
Name:           terraphim-server
Version:        1.0.0
Release:        1%{?dist}
Summary:        Terraphim AI Server
License:        MIT
URL:            https://github.com/terraphim/terraphim-ai

BuildRequires:  gcc, rust, cargo
Requires:       openssl, sqlite3

%description
Terraphim AI server component for knowledge graph search and AI assistance.

%prep
%setup -q -n terraphim_server-%{version}

%build
cargo build --release --target x86_64-unknown-linux-gnu

%install
install -D -m 755 target/x86_64-unknown-linux-gnu/release/terraphim_server %{buildroot}%{_bindir}/terraphim_server

%files
%{_bindir}/terraphim_server

%changelog
* Sun Dec 16 2025 Terraphim Team <team@terraphim.ai> - 1.0.0-1
- Initial RPM release
```

#### 2.2 Desktop RPM
```rpm
# packaging/rpm/desktop.spec
Name:           terraphim-desktop
Version:        1.0.0
Release:        1%{?dist}
Summary:        Terraphim AI Desktop Application
License:        MIT
URL:            https://github.com/terraphim/terraphim-ai

BuildRequires:  gcc, rust, cargo, nodejs, yarn, webkit2gtk3
Requires:       webkit2gtk3, gtk3, libnotify

%description
Terraphim AI desktop application with modern GUI.

%prep
%setup -q

%build
cd desktop
yarn install
yarn tauri build --bundles rpm --target x86_64-unknown-linux-gnu

%install
install -D -m 755 target/x86_64-unknown-linux-gnu/release/bundle/rpm/terraphim-desktop-*.rpm %{buildroot}/

%files
%{_bindir}/terraphim-desktop
/usr/share/applications/terraphim-desktop.desktop
/usr/share/icons/hicolor/*/apps/terraphim-desktop.png
```

#### 2.3 RPM Build Script
```bash
#!/bin/bash
# packaging/scripts/build-rpm.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "🔧 Building RPM packages..."

# Build backend RPMs
echo "Building terraphim-server RPM..."
cargo-generate-rpm \
    -p terraphim_server \
    -o packaging/rpm/ \
    -s packaging/rpm/terraphim-server.spec

echo "Building terraphim-agent RPM..."  
cargo-generate-rpm \
    -p terraphim_agent \
    -o packaging/rpm/ \
    -s packaging/rpm/terraphim-agent.spec

# Build desktop RPM
echo "Building terraphim-desktop RPM..."
cd desktop
yarn tauri build --bundles rpm --target x86_64-unknown-linux-gnu

echo "✅ RPM packages complete"
```

### **Week 3-4: Arch Linux Implementation**

#### 3.1 Create PKGBUILD Files
```bash
# packaging/arch/PKGBUILD-server
pkgname=terraphim-server
pkgver=1.0.0
pkgrel=1
pkgdesc="Terraphim AI server component"
arch=('x86_64' 'aarch64')
url="https://github.com/terraphim/terraphim-ai"
license=('MIT')
depends=('openssl' 'sqlite')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/terraphim/terraphim-ai/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$pkgname-$pkgver"
  cargo build --release --package terraphim_server
}

package() {
  cd "$pkgname-$pkgver"
  install -D -m 755 target/release/terraphim_server "$pkgdir/usr/bin/terraphim_server"
}
```

```bash
# packaging/arch/PKGBUILD-desktop
pkgname=terraphim-desktop
pkgver=1.0.0
pkgrel=1
pkgdesc="Terraphim AI desktop application"
arch=('x86_64')
url="https://github.com/terraphim/terraphim-ai"
license=('MIT')
depends=('webkit2gtk' 'gtk3' 'libnotify' 'gcc-libs')
makedepends=('rust' 'cargo' 'nodejs' 'yarn')
source=("$pkgname-$pkgver.tar.gz::https://github.com/terraphim/terraphim-ai/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$pkgname-$pkgver/desktop"
  yarn install
  yarn tauri build --bundles appimage --target x86_64-unknown-linux-gnu
}

package() {
  cd "$pkgname-$pkgver/desktop"
  install -D -m 755 target/x86_64-unknown-linux-gnu/release/terraphim-desktop "$pkgdir/usr/bin/terraphim-desktop"
  install -D -m 644 src-tauri/icons/128x128.png "$pkgdir/usr/share/icons/hicolor/128x128/apps/terraphim-desktop.png"
  install -D -m 644 com.terraphim.ai.desktop "$pkgdir/usr/share/applications/com.terraphim.ai.desktop"
}
```

#### 3.2 Arch Build Script
```bash
#!/bin/bash
# packaging/scripts/build-arch.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "🔧 Building Arch Linux packages..."

# Server package
echo "Building terraphim-server AUR package..."
cd packaging/arch
makepkg -f PKGBUILD-server

# Agent package  
echo "Building terraphim-agent AUR package..."
makepkg -f PKGBUILD-agent

# Desktop package
echo "Building terraphim-desktop AUR package..."
makepkg -f PKGBUILD-desktop

echo "✅ Arch Linux packages complete"
```

### **Week 5-6: AppImage Fix**

#### 4.1 Diagnose AppImage Issues
```bash
# Debug current AppImage build
cd desktop
yarn tauri build --bundles appimage --target x86_64-unknown-linux-gnu --verbose

# Check missing dependencies
ldd target/x86_64-unknown-linux-gnu/release/terraphim-desktop | grep "not found"

# Install missing dependencies if needed
sudo apt install libwebkit2gtk-4.0-37 libgtk-3-0 libnotify4
```

#### 4.2 Manual AppImage Build
```bash
#!/bin/bash
# packaging/scripts/build-appimage.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APPDIR="$ROOT/target/appimage/terraphim-desktop.AppDir"

echo "🔧 Building AppImage manually..."

# Create AppDir structure
mkdir -p "$APPDIR"/{usr/bin,usr/share/applications,usr/share/icons/hicolor/256x256/apps}

# Build application
cd desktop
yarn tauri build --target x86_64-unknown-linux-gnu

# Copy binaries
cp target/x86_64-unknown-linux-gnu/release/terraphim-desktop "$APPDIR/usr/bin/"

# Copy desktop file
cp packaging/flatpak/com.terraphim.ai.desktop "$APPDIR/usr/share/applications/"

# Copy icon
cp desktop/src-tauri/icons/128x128.png "$APPDIR/usr/share/icons/hicolor/256x256/apps/terraphim-desktop.png"

# Create AppImage
cd "$(dirname "$APPDIR")"
appimagetool "$APPDIR" terraphim-desktop_1.0.0_amd64.AppImage

echo "✅ AppImage built successfully"
```

---

## Phase 3: Modern Formats (Weeks 7-10) 🚀

### **Week 7-8: Flatpak Implementation**

#### 5.1 Flatpak Manifest
```yaml
# packaging/flatpak/com.terraphim.ai.desktop.yml
app-id: com.terraphim.ai.desktop
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
command: terraphim-desktop

finish-args:
  - --share=ipc
  - --share=network
  - --device=dri
  - --filesystem=home

modules:
  - name: terraphim-desktop
    buildsystem: simple
    sources:
      - type: git
        url: https://github.com/terraphim/terraphim-ai.git
        tag: v1.0.0
    
    modules:
      - name: nodejs
        buildsystem: simple
        sources:
          - type: archive
            url: https://nodejs.org/dist/v20.10.0/node-v20.10.0.tar.gz
            sha256: '...'
      
      - name: rust
        buildsystem: simple
        sources:
          - type: archive
            url: https://static.rust-lang.org/dist/rust-1.75.0-x86_64-unknown-linux-gnu.tar.gz
            sha256: '...'

    build-commands:
      - cd desktop
      - yarn install
      - yarn tauri build --target x86_64-unknown-linux-gnu
      - install -D target/x86_64-unknown-linux-gnu/release/terraphim-desktop /app/bin/terraphim-desktop
      - install -D packaging/flatpak/com.terraphim.ai.desktop /app/share/applications/com.terraphim.ai.desktop
      - install -D desktop/src-tauri/icons/128x128.png /app/share/icons/hicolor/128x128/apps/com.terraphim.ai.desktop.png
```

#### 5.2 Flatpak Build Script
```bash
#!/bin/bash
# packaging/scripts/build-flatpak.sh

set -euo pipefail

echo "🔧 Building Flatpak package..."

flatpak-builder --repo=flatpak-repo \
  --force-clean \
  build-dir \
  packaging/flatpak/com.terraphim.ai.desktop.yml

flatpak build-bundle flatpak-repo terraphim-desktop.flatpak com.terraphim.ai.desktop

echo "✅ Flatpak built successfully"
```

### **Week 9-10: Snap Implementation**

#### 6.1 Snapcraft Configuration
```yaml
# packaging/snap/snapcraft.yaml
name: terraphim-desktop
version: 1.0.0
summary: Terraphim AI desktop application
description: |
  Privacy-first AI assistant that operates locally with knowledge graph search

base: core22
confinement: strict
grade: stable

apps:
  terraphim-desktop:
    command: bin/terraphim-desktop
    extensions:
      - gnome-3-38
    plugs:
      - home
      - network
      - browser-support

parts:
  terraphim-desktop:
    plugin: rust
    source: .
    build-snaps:
      - nodejs/20/stable
    build-packages:
      - libwebkit2gtk-4.0-dev
      - libgtk-3-dev
      - libnotify-dev
    stage-packages:
      - libwebkit2gtk-4.0-37
      - libgtk-3-0
      - libnotify4
    
    override-build: |
      cargo install tauri-cli
      cd desktop
      yarn install
      yarn tauri build --target x86_64-unknown-linux-gnu
      
      mkdir -p $SNAPCRAFT_PART_INSTALL/bin
      cp target/x86_64-unknown-linux-gnu/release/terraphim-desktop $SNAPCRAFT_PART_INSTALL/bin/
```

#### 6.2 Snap Build Script
```bash
#!/bin/bash
# packaging/scripts/build-snap.sh

set -euo pipefail

echo "🔧 Building Snap package..."

cd packaging/snap
snapcraft --target-arch=amd64

echo "✅ Snap built successfully"
```

---

## Phase 4: Quality & Distribution (Weeks 11-12) ✨

### **Week 11: Package Signing & Testing**

#### 7.1 GPG Signing Setup
```bash
#!/bin/bash
# packaging/scripts/sign-packages.sh

set -euo pipefail

# Generate GPG key if needed
if ! gpg --list-keys "Terraphim Releases" > /dev/null 2>&1; then
  gpg --batch --gen-key << EOF
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name: Terraphim Releases
Email: releases@terraphim.ai
Expire-Date: 2y
%no-protection
EOF
fi

# Sign all packages
for pkg in release-artifacts/*.{deb,rpm,flatpak,snap}; do
  if [[ -f "$pkg" ]]; then
    echo "Signing $pkg..."
    gpg --detach-sign --armor --local-user "Terraphim Releases" "$pkg"
  fi
done

echo "✅ Packages signed successfully"
```

#### 7.2 Comprehensive Testing
```bash
#!/bin/bash
# packaging/scripts/test-packages.sh

set -euo pipefail

# Test on multiple distributions
distros=("ubuntu:22.04" "ubuntu:24.04" "fedora:39" "archlinux:latest")

for distro in "${distros[@]}"; do
  echo "Testing on $distro..."
  docker run --rm -v "$(pwd):/workspace" "$distro" /workspace/test-install.sh
done
```

```bash
#!/bin/bash
# test-install.sh

set -euo pipefail

# Test DEB install
if command -v apt > /dev/null; then
  apt update
  apt install -y ./release-artifacts/*.deb
  terraphim_server --version
  terraphim-desktop --version
fi

# Test RPM install  
if command -v dnf > /dev/null; then
  dnf install -y ./release-artifacts/*.rpm
  terraphim_server --version
  terraphim-desktop --version
fi

# Test Arch install
if command -v pacman > /dev/null; then
  pacman -U ./release-artifacts/*.pkg.tar.zst
  terraphim_server --version
  terraphim-desktop --version
fi
```

### **Week 12: Repository Setup & Release**

#### 8.1 Repository Infrastructure
```bash
# AUR Repository Setup
git clone ssh://aur@aur.archlinux.org/terraphim-server.git
cp packaging/arch/PKGBUILD-server ./PKGBUILD
git add PKGBUILD
git commit -m "Add terraphim-server AUR package"
git push origin master

# COPR Repository Setup (for RPM)
copr-cli create terraphim-ai
copr-cli build-package terraphim-ai terraphim-server

# PPA Repository Setup (for DEB)
# Launchpad setup with automated builds

# Flatpak Repository
flatpak remote-add --if-not-exists terraphim https://releases.terraphim.ai/flatpak/repo.flatpakrepo
```

#### 8.2 Automated Release Pipeline
```yaml
# .github/workflows/release.yml
name: Complete Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-all:
    strategy:
      matrix:
        format: [deb, rpm, arch, appimage, flatpak, snap]
        
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install dependencies
        run: |
          # Install all required tools based on matrix format
          
      - name: Build ${{ matrix.format }}
        run: ./packaging/scripts/build-${{ matrix.format }}.sh
        
      - name: Test ${{ matrix.format }}
        run: ./packaging/scripts/test-${{ matrix.format }}.sh
        
      - name: Upload artifacts
        uses: actions/upload-release-asset@v1
        with:
          upload_url: ${{ steps.create_release.outputs.upload_url }}
          asset_path: ./release-artifacts/${{ matrix.format }}-package
          asset_name: terraphim-${{ matrix.format }}-package
          asset_content_type: application/octet-stream
```

---

## Implementation Timeline 📅

| Week | Deliverables | Success Criteria |
|-------|-------------|------------------|
| 1 | Infrastructure setup | All tools installed, directory structure created |
| 2-3 | RPM support | Working .rpm packages for server & desktop |
| 4-5 | Arch Linux support | AUR packages submitted and building |
| 6 | AppImage fixes | Functional AppImage builds |
| 7-8 | Flatpak support | Working .flatpak packages |
| 9-10 | Snap support | Working .snap packages |
| 11 | Signing & testing | All packages signed and tested |
| 12 | Repository setup | AUR, COPR, PPA, Flatpak repos live |

---

## Success Metrics 📊

### **Format Coverage**:
- **Target**: 8/8 formats (100%)
- **Current**: 2/8 formats (25%)
- **Goal**: Complete by week 12

### **Distribution Coverage**:
- **Target**: 90%+ Linux distributions
- **Current**: ~40%  
- **Goal**: Debian, Ubuntu, Fedora, CentOS, RHEL, Arch, Manjaro, openSUSE

### **Quality Metrics**:
- ✅ All packages GPG signed
- ✅ Automatic updates configured
- ✅ Dependencies properly declared
- ✅ Cross-distro testing passing
- ✅ Installation documentation complete

---

## Risk Assessment & Mitigation ⚠️

### **High Risk**:
1. **Tauri AppImage Issues**: GTK dependency complexity
   - **Mitigation**: Manual AppImage build process
   
2. **Flatpak Sandbox**: Network access restrictions
   - **Mitigation**: Proper permission configuration

### **Medium Risk**:
1. **AUR Submission Delays**: Community review process
   - **Mitigation**: Submit early, provide clear documentation

2. **Cross-compilation**: ARM64 builds failing
   - **Mitigation**: Use GitHub Actions for different architectures

### **Low Risk**:
1. **Package Dependencies**: System library conflicts
   - **Mitigation**: Comprehensive testing matrix

---

## Required Resources 💪

### **Personnel**:
- **DevOps Engineer** (0.5 FTE) - CI/CD pipeline
- **Package Maintainer** (0.5 FTE) - Package maintenance
- **QA Engineer** (0.3 FTE) - Multi-distro testing

### **Infrastructure**:
- **Build Servers**: 4x Ubuntu 22.04 (for different distros)
- **Repository Hosting**: GitHub Releases + package repositories
- **Code Signing**: GPG key management

### **Tools & Services**:
- **GitHub Actions** (free tier sufficient)
- **AUR access** (free)
- **COPR hosting** (free)
- **Launchpad PPA** (free)

---

## Next Steps 🚀

1. **Week 1**: Begin infrastructure setup immediately
2. **Parallel Development**: RPM and Arch teams can work simultaneously  
3. **Weekly Reviews**: Progress checkpoints every Friday
4. **Community Engagement**: Early AUR and COPR submissions
5. **Documentation**: Update installation guides with each format

**Total Timeline**: 12 weeks to complete release format coverage
**Total Effort**: ~2-3 FTE for 3 months
**Expected Impact**: 300% increase in Linux user adoption