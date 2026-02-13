# Terraphim AI Release System Map

## Overview

This document provides a comprehensive mapping of the Terraphim AI release system, including all components, platforms, package formats, and distribution channels.

## Release Components Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Terraphim AI Release System                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐   │
│  │   Server    │  │     TUI     │  │      Desktop App        │   │
│  │ Component   │  │ Component   │  │    (Tauri-based)       │   │
│  │             │  │             │  │                         │   │
│  │ • Core API   │  │ • CLI       │  │ • GUI Interface        │   │
│  │ • Indexing   │  │ • Terminal  │  │ • Auto-updater         │   │
│  │ • Search     │  │ • Sessions  │  │ • System Integration   │   │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘   │
│         │                │                      │              │
│         └────────────────┼──────────────────────┘              │
│                          │                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Docker Container Images                    │   │
│  │                                                         │   │
│  │ • Multi-architecture (amd64, arm64, arm/v7)            │   │
│  │ • Ubuntu base variants (20.04, 22.04)                  │   │
│  │ • Registry: GHCR, Docker Hub                           │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Platform Distribution Matrix

### Linux Platform Support
```
Linux
├── x86_64 (Intel/AMD)
│   ├── Server Binary
│   ├── TUI Binary
│   ├── Desktop App (AppImage)
│   ├── Debian Package (.deb)
│   ├── RPM Package (.rpm)
│   └── Docker Image (amd64)
├── aarch64 (ARM64)
│   ├── Server Binary
│   ├── TUI Binary
│   ├── Debian Package (.deb)
│   ├── Arch Package (.tar.zst)
│   └── Docker Image (arm64)
└── armv7 (ARM32)
    ├── Server Binary
    ├── TUI Binary
    ├── Debian Package (.deb)
    └── Docker Image (arm/v7)
```

### macOS Platform Support
```
macOS
├── x86_64 (Intel)
│   ├── Server Binary
│   ├── TUI Binary
│   ├── Desktop App (.dmg)
│   ├── Archive Package (.tar.gz)
│   └── Docker Image (amd64)
└── aarch64 (Apple Silicon)
    ├── Server Binary
    ├── TUI Binary
    ├── Desktop App (.dmg)
    ├── Archive Package (.tar.gz)
    └── Docker Image (arm64)
```

### Windows Platform Support
```
Windows x86_64
├── Server Binary (.exe)
├── TUI Binary (.exe)
├── Desktop App (.msi/.exe)
└── Installer (.exe)
```

## Package Format Mapping

### Binary Distributions
| Component | Format | Linux x86_64 | Linux aarch64 | Linux armv7 | macOS x86_64 | macOS aarch64 | Windows |
|-----------|--------|--------------|---------------|-------------|--------------|---------------|----------|
| Server    | Binary | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| TUI       | Binary | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

### System Package Distributions
| Component | Format | Linux x86_64 | Linux aarch64 | Linux armv7 | macOS x86_64 | macOS aarch64 | Windows |
|-----------|--------|--------------|---------------|-------------|--------------|---------------|----------|
| Server    | .deb   | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Server    | .rpm   | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Server    | .tar.zst| ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Server    | .tar.gz| ❌ | ❌ | ❌ | ✅ | ✅ | ❌ |
| TUI       | .deb   | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| TUI       | .rpm   | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| TUI       | .tar.zst| ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| TUI       | .tar.gz| ❌ | ❌ | ❌ | ✅ | ✅ | ❌ |

### Desktop Application Distributions
| Platform | Format | Status | Notes |
|----------|--------|--------|-------|
| Linux x86_64 | AppImage | ✅ | Portable, no installation required |
| Linux x86_64 | .deb | ✅ | Integration with package managers |
| macOS x86_64 | .dmg | ✅ | Standard macOS installer |
| macOS aarch64 | .dmg | ✅ | Universal binary support |
| Windows x86_64 | .msi | ✅ | Windows Installer |
| Windows x86_64 | .exe | ✅ | NSIS installer |

## Distribution Channels

### Primary Distribution
```
GitHub Releases (https://github.com/terraphim/terraphim-ai/releases)
├── Versioned Releases
│   ├── Source Code (tarball)
│   ├── Server Binaries (all platforms)
│   ├── TUI Binaries (all platforms)
│   ├── Desktop Applications (all platforms)
│   ├── System Packages (.deb, .rpm, .tar.zst)
│   ├── Docker Images (multi-arch)
│   ├── Installation Scripts
│   ├── Checksums (SHA256)
│   └── Release Notes
└── Latest Release
    └── Same structure as versioned releases
```

### Package Manager Distribution
```
Homebrew (macOS/Linux)
├── terraphim/terraphim-ai/terraphim-ai tap
├── Automatic formula updates
└── Dependency management

APT Repositories (Debian/Ubuntu)
├── Server package
├── TUI package
└── Desktop package (future)

YUM Repositories (RHEL/CentOS/Fedora)
├── Server package
├── TUI package
└── Desktop package (future)

AUR (Arch Linux)
├── terraphim-server package
├── terraphim-agent package
└── terraphim-desktop package (future)
```

### Container Registry Distribution
```
GitHub Container Registry (ghcr.io/terraphim)
├── terraphim-server:latest
├── terraphim-server:v{version}
├── terraphim-server:ubuntu-20.04
├── terraphim-server:ubuntu-22.04
└── Multi-architecture manifests
    ├── amd64
    ├── arm64
    └── arm/v7

Docker Hub (docker.io/terraphim)
├── terraphim-server:latest
├── terraphim-server:v{version}
└── Same multi-architecture support
```

## Build and Release Flow

### GitHub Actions Workflow
```
Tag Push (v*, component-v*)
├── Build Binaries (Matrix)
│   ├── Linux (x86_64, aarch64, armv7)
│   ├── macOS (x86_64, aarch64)
│   └── Windows (x86_64)
├── Build System Packages
│   ├── Debian (.deb)
│   ├── RPM (.rpm)
│   └── Arch (.tar.zst)
├── Build Desktop Applications
│   ├── Linux (AppImage, .deb)
│   ├── macOS (.dmg)
│   └── Windows (.msi, .exe)
├── Build Docker Images
│   ├── Multi-architecture builds
│   └── Push to registries
├── Create GitHub Release
│   ├── Upload all artifacts
│   ├── Generate checksums
│   └── Publish release notes
└── Update Package Managers
    ├── Homebrew formula
    ├── AUR packages
    └── Repository metadata
```

## Installation Methods

### Quick Installation
```bash
# One-line installation (curl | bash)
curl -fsSL https://github.com/terraphim/terraphim-ai/releases/latest/download/install.sh | sh

# Docker run
docker run -d --name terraphim ghcr.io/terraphim/terraphim-server:latest

# Homebrew
brew install terraphim/terraphim-ai/terraphim-ai
```

### Platform-Specific Installation
```
Ubuntu/Debian
├── sudo dpkg -i terraphim-server_*.deb
├── sudo apt-get install -f  # Fix dependencies
└── sudo systemctl enable terraphim-server

RHEL/CentOS/Fedora
├── sudo rpm -i terraphim-server-*.rpm
└── sudo systemctl enable terraphim-server

Arch Linux
├── yay -S terraphim-server
├── pacman -U terraphim-server-*.pkg.tar.zst
└── sudo systemctl enable terraphim-server

macOS
├── brew install terraphim/terraphim-ai/terraphim-ai
├── Download and open .dmg file
└── Drag to Applications folder

Windows
├── Download and run .msi installer
├── Follow installation wizard
└── Auto-start option available
```

## File Structure and Naming Conventions

### Binary Naming
```
terraphim_server-{target}
├── terraphim_server-x86_64-unknown-linux-gnu
├── terraphim_server-x86_64-unknown-linux-musl
├── terraphim_server-aarch64-unknown-linux-musl
├── terraphim_server-armv7-unknown-linux-musleabihf
├── terraphim_server-x86_64-apple-darwin
├── terraphim_server-aarch64-apple-darwin
└── terraphim_server-x86_64-pc-windows-msvc.exe

terraphim-agent-{target}
├── Same target variants as server
└── Binary name includes .exe on Windows
```

### Package Naming
```
Debian Packages (.deb)
├── terraphim-server_{version}-1_amd64.deb
├── terraphim-server_{version}-1_arm64.deb
├── terraphim-agent_{version}-1_amd64.deb
├── terraphim-agent_{version}-1_arm64.deb
├── terraphim-ai-desktop_{version}-1_amd64.deb
└── terraphim-ai-desktop_{version}-1_arm64.deb

RPM Packages (.rpm)
├── terraphim-server-{version}-1.x86_64.rpm
├── terraphim-agent-{version}-1.x86_64.rpm
└── terraphim-ai-desktop-{version}-1.x86_64.rpm

Arch Packages (.tar.zst)
├── terraphim-server-{version}-1-x86_64.pkg.tar.zst
├── terraphim-server-{version}-1-aarch64.pkg.tar.zst
├── terraphim-agent-{version}-1-x86_64.pkg.tar.zst
└── terraphim-agent-{version}-1-aarch64.pkg.tar.zst
```

## Validation Checkpoints

### Pre-Release Validation
1. **Build Success**: All matrix builds complete successfully
2. **Binary Verification**: Executables are valid for target platforms
3. **Package Integrity**: System packages install without conflicts
4. **Desktop Functionality**: GUI applications launch and function
5. **Container Testing**: Docker images run on all architectures

### Post-Release Validation
1. **Download Availability**: All artifacts accessible from GitHub
2. **Checksum Verification**: SHA256 hashes match published values
3. **Installation Testing**: Clean installations work on all platforms
4. **Update Testing**: In-place updates preserve user data
5. **Integration Testing**: Components communicate correctly

This system map provides the foundation for understanding the complexity of the Terraphim AI release process and identifying critical validation points across the entire distribution ecosystem.