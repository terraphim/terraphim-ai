# Terraphim AI v0.2.5-cross-platform - Cross-Platform Release

## 🚀 Quick Installation

### Linux
```bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release-0.2.5-cross-platform/install-linux.sh | bash
```

### macOS
```bash
# macOS binaries are not yet available in this release
# Please build from source or check for a future release
```

### Windows
```powershell
# Windows binaries are not yet available in this release
# Please build from source or check for a future release
```

## 📦 Available Binaries

### Linux ✅
- `x86_64-unknown-linux-gnu`: Standard Linux (glibc) - **AVAILABLE**

### macOS 🔄
- `x86_64-apple-darwin`: Intel Mac - *Coming soon*
- `aarch64-apple-darwin`: Apple Silicon (M1/M2) - *Coming soon*

### Windows 🔄
- `x86_64-pc-windows-msvc`: Windows 64-bit - *Coming soon*

## 🔐 Verification

All files can be verified using the provided `checksums.txt`:

```bash
sha256sum -c checksums.txt
```

## 🐳 Docker

```bash
docker run -d --name terraphim-server -p 8000:8000 ghcr.io/terraphim/terraphim-server:v0.2.5
```

## 📚 Documentation

- Complete documentation: https://docs.terraphim.ai
- GitHub repository: https://github.com/terraphim/terraphim-ai
- Issues and support: https://github.com/terraphim/terraphim-ai/issues

## 🔧 Build from Source

If binaries for your platform are not available, you can build from source:

```bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build the server
cargo build --release --bin terraphim_server

# Build the TUI
cargo build --release --package terraphim_tui

# Binaries will be in target/release/
```

## 📋 Release Notes

### Security Fixes
- **RSA Marvin Attack Vulnerability (RUSTSEC-2023-0071)**: Successfully eliminated by removing SQLite dependency from OpenDAL
- **ed25519-dalek API Migration**: Complete cryptographic API modernization from v1.x to v2.2

### Features
- Cross-platform build infrastructure
- Manual build system for reliable releases
- Comprehensive installation scripts

### Known Limitations
- Cross-compilation for macOS and Windows requires additional toolchain setup
- This release focuses on Linux x86_64 support
- Future releases will include full cross-platform binaries

Built on: Sun Oct 26 2025
Version: 0.2.5-cross-platform