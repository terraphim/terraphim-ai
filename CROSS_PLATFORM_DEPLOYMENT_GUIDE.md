# Terraphim AI v0.2.5 - Cross-Platform Deployment Guide

## 🚀 Overview

This guide provides comprehensive deployment instructions for Terraphim AI v0.2.5 across all supported platforms. The release includes security fixes and cross-platform binaries for Windows, macOS, and Linux.

## 📦 Available Release Artifacts

### Core Binaries
- **terraphim_server** - Main HTTP API server with semantic search
- **terraphim-tui** - Terminal User Interface with interactive REPL
- **terraphim-desktop** - Native desktop application (Tauri-based)

### Package Formats
- **Windows**: `.exe`, `.msi` installers
- **macOS**: `.dmg` disk images, `.app` bundles
- **Linux**: `.AppImage`, `.deb`, `.rpm`, `.tar.gz`
- **Docker**: Multi-architecture container images

---

## 🖥️ Windows Deployment

### Option 1: Installer (Recommended)
```powershell
# Download and run MSI installer
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-server-0.2.5-x86_64-pc-windows-msvc.msi
Start-Process msiexec -ArgumentList "/i terraphim-server-0.2.5-x86_64-pc-windows-msvc.msi /quiet"

# For desktop application
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-desktop-0.2.5-x86_64-pc-windows-msvc.msi
Start-Process msiexec -ArgumentList "/i terraphim-desktop-0.2.5-x86_64-pc-windows-msvc.msi /quiet"
```

### Option 2: Portable Binary
```powershell
# Download portable binaries
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim_server-0.2.5-x86_64-pc-windows-msvc.exe
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-x86_64-pc-windows-msvc.exe

# Run directly
.\terraphim_server-0.2.5-x86_64-pc-windows-msvc.exe --help
.\terraphim-tui-0.2.5-x86_64-pc-windows-msvc.exe --help
```

### Option 3: PowerShell Installation Script
```powershell
# Automated installation
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install-windows.ps1" -OutFile "install-windows.ps1"
.\install-windows.ps1 -Version "v0.2.5-cross-platform"
```

### Windows Prerequisites
- **Windows 10/11** (x64)
- **Visual C++ Redistributable** (included in installers)
- **.NET Framework 4.7.2** (for desktop app)
- **PowerShell 5.1+** (for installation scripts)

---

## 🍎 macOS Deployment

### Option 1: DMG Installer (Recommended)
```bash
# Download and mount DMG
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/Terraphim-Desktop-0.2.5.dmg
hdiutil attach Terraphim-Desktop-0.2.5.dmg

# Copy to Applications
sudo cp -r "/Volumes/Terraphim Desktop/Terraphim Desktop.app" /Applications/
hdiutil detach "/Volumes/Terraphim Desktop"
```

### Option 2: Homebrew
```bash
# Install via Homebrew tap
brew install terraphim/terraphim-ai/terraphim-ai

# Or install specific version
brew install terraphim/terraphim-ai/terraphim-ai@0.2.5
```

### Option 3: Binary Installation
```bash
# Download binaries
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim_server-0.2.5-x86_64-apple-darwin
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-x86_64-apple-darwin

# Make executable and install
chmod +x terraphim_server-0.2.5-x86_64-apple-darwin terraphim-tui-0.2.5-x86_64-apple-darwin
sudo mv terraphim_server-0.2.5-x86_64-apple-darwin /usr/local/bin/terraphim_server
sudo mv terraphim-tui-0.2.5-x86_64-apple-darwin /usr/local/bin/terraphim-tui
```

### Option 4: Apple Silicon (M1/M2)
```bash
# Download ARM64 binaries
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim_server-0.2.5-aarch64-apple-darwin
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-aarch64-apple-darwin

# Install
chmod +x terraphim_server-0.2.5-aarch64-apple-darwin terraphim-tui-0.2.5-aarch64-apple-darwin
sudo mv terraphim_server-0.2.5-aarch64-apple-darwin /usr/local/bin/terraphim_server
sudo mv terraphim-tui-0.2.5-aarch64-apple-darwin /usr/local/bin/terraphim-tui
```

### macOS Prerequisites
- **macOS 10.15+** (Catalina or newer)
- **Xcode Command Line Tools**: `xcode-select --install`
- **Homebrew** (optional, for package management)

---

## 🐧 Linux Deployment

### Option 1: AppImage (Universal Linux)
```bash
# Download and run AppImage
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/Terraphim-Desktop-0.2.5.AppImage
chmod +x Terraphim-Desktop-0.2.5.AppImage
./Terraphim-Desktop-0.2.5.AppImage
```

### Option 2: Debian/Ubuntu
```bash
# Download and install .deb package
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-server_0.2.5_amd64.deb
sudo dpkg -i terraphim-server_0.2.5_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed

# For TUI
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui_0.2.5_amd64.deb
sudo dpkg -i terraphim-tui_0.2.5_amd64.deb
```

### Option 3: RedHat/Fedora/CentOS
```bash
# Download and install .rpm package
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-server-0.2.5-1.x86_64.rpm
sudo rpm -i terraphim-server-0.2.5-1.x86_64.rpm

# For TUI
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-1.x86_64.rpm
sudo rpm -i terraphim-tui-0.2.5-1.x86_64.rpm
```

### Option 4: Arch Linux
```bash
# Download and install with pacman
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-server-0.2.5-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-server-0.2.5-1-x86_64.pkg.tar.zst

# For TUI
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-tui-0.2.5-1-x86_64.pkg.tar.zst
```

### Option 5: Generic Binary
```bash
# Download binaries
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim_server-0.2.5-x86_64-unknown-linux-gnu
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.5-cross-platform/terraphim-tui-0.2.5-x86_64-unknown-linux-gnu

# Install
chmod +x terraphim_server-0.2.5-x86_64-unknown-linux-gnu terraphim-tui-0.2.5-x86_64-unknown-linux-gnu
sudo mv terraphim_server-0.2.5-x86_64-unknown-linux-gnu /usr/local/bin/terraphim_server
sudo mv terraphim-tui-0.2.5-x86_64-unknown-linux-gnu /usr/local/bin/terraphim-tui
```

### Linux Prerequisites
- **glibc 2.17+** (most modern distributions)
- **OpenSSL 1.1.1+** or **LibreSSL**
- **pkg-config** (for building from source)
- **GTK+3** (for desktop application)

---

## 🐳 Docker Deployment

### Option 1: Quick Start
```bash
# Pull and run latest image
docker run -d \
  --name terraphim-server \
  -p 8000:8000 \
  -v ~/.terraphim:/app/data \
  ghcr.io/terraphim/terraphim-server:v0.2.5-cross-platform
```

### Option 2: Docker Compose
```yaml
# docker-compose.yml
version: '3.8'
services:
  terraphim-server:
    image: ghcr.io/terraphim/terraphim-server:v0.2.5-cross-platform
    container_name: terraphim-server
    ports:
      - "8000:8000"
    volumes:
      - ~/.terraphim:/app/data
      - ~/.terraphim/kg:/app/kg
    environment:
      - RUST_LOG=info
      - TERAPHIM_DATA_DIR=/app/data
    restart: unless-stopped

  terraphim-desktop:
    image: ghcr.io/terraphim/terraphim-desktop:v0.2.5-cross-platform
    container_name: terraphim-desktop
    volumes:
      - /tmp/.X11-unix:/tmp/.X11-unix:rw
      - ~/.terraphim:/app/data
    environment:
      - DISPLAY=${DISPLAY}
    network_mode: host
    restart: unless-stopped
```

### Option 3: Multi-Architecture
```bash
# Supports AMD64, ARM64, ARMv7
docker run --platform linux/amd64 ghcr.io/terraphim/terraphim-server:v0.2.5-cross-platform
docker run --platform linux/arm64 ghcr.io/terraphim/terraphim-server:v0.2.5-cross-platform
docker run --platform linux/arm/v7 ghcr.io/terraphim/terraphim-server:v0.2.5-cross-platform
```

---

## ⚙️ Configuration

### Environment Variables
```bash
# Server configuration
export TERAPHIM_SERVER_HOST=127.0.0.1
export TERAPHIM_SERVER_PORT=8000
export TERAPHIM_DATA_DIR=~/.terraphim
export RUST_LOG=info

# Database backend selection
export TERAPHIM_DATABASE_BACKEND=rocksdb  # rocksdb, redis, memory, dashmap

# LLM configuration (optional)
export TERAPHIM_LLM_API_KEY=your_api_key
export TERAPHIM_LLM_MODEL=gpt-4
export TERAPHIM_LLM_ENABLED=true
```

### Configuration Files
```bash
# Create configuration directory
mkdir -p ~/.terraphim

# Server configuration
cat > ~/.terraphim/server.toml << EOF
[server]
host = "127.0.0.1"
port = 8000
data_dir = "~/.terraphim"

[database]
backend = "rocksdb"
path = "~/.terraphim/data"

[llm]
enabled = false
api_key = ""
model = "gpt-4"
EOF
```

---

## 🔧 Verification

### Test Server Installation
```bash
# Start server
terraphim_server --config ~/.terraphim/server.toml

# Test API endpoint
curl http://localhost:8000/api/health
curl http://localhost:8000/api/version
```

### Test TUI Installation
```bash
# Start TUI
terraphim-tui --help
terraphim-tui --config ~/.terraphim/config.toml
```

### Test Desktop Application
```bash
# Linux
./Terraphim-Desktop-0.2.5.AppImage

# macOS
open /Applications/Terraphim\ Desktop.app

# Windows
Start-Process "C:\Program Files\Terraphim\Terraphim Desktop.exe"
```

---

## 🚀 Production Deployment

### Systemd Service (Linux)
```ini
# /etc/systemd/system/terraphim-server.service
[Unit]
Description=Terraphim AI Server
After=network.target

[Service]
Type=simple
User=terraphim
Group=terraphim
WorkingDirectory=/opt/terraphim
ExecStart=/opt/terraphim/terraphim_server --config /etc/terraphim/server.toml
Restart=always
RestartSec=5
Environment=TERAPHIM_DATA_DIR=/var/lib/terraphim

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable terraphim-server
sudo systemctl start terraphim-server
sudo systemctl status terraphim-server
```

### Nginx Reverse Proxy
```nginx
# /etc/nginx/sites-available/terraphim
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

---

## 🔍 Troubleshooting

### Common Issues

#### Windows
- **"DLL not found"**: Install Visual C++ Redistributable
- **"Access denied"**: Run PowerShell as Administrator
- **"Firewall blocking"**: Add exception for terraphim_server.exe

#### macOS
- **"App can't be opened":** `xattr -d com.apple.quarantine Terraphim\ Desktop.app`
- **"Command not found"**: Add `/usr/local/bin` to PATH
- **"Permission denied"**: `chmod +x` the binary

#### Linux
- **"libssl not found"**: `sudo apt-get install libssl-dev` (Ubuntu/Debian)
- **"GTK not found"**: `sudo apt-get install libgtk-3-dev` (Ubuntu/Debian)
- **"Permission denied"**: Use `sudo` or add user to appropriate groups

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug terraphim_server --config ~/.terraphim/server.toml

# Verbose TUI output
RUST_LOG=debug terraphim-tui --verbose
```

### Log Locations
- **Linux/macOS**: `~/.terraphim/logs/`
- **Windows**: `%APPDATA%\Terraphim\logs\`
- **Docker**: Container logs (`docker logs terraphim-server`)

---

## 📚 Additional Resources

### Documentation
- **Complete API Reference**: https://docs.terraphim.ai
- **Configuration Guide**: https://docs.terraphim.ai/configuration
- **Security Guide**: https://docs.terraphim.ai/security

### Community
- **GitHub Issues**: https://github.com/terraphim/terraphim-ai/issues
- **Discussions**: https://github.com/terraphim/terraphim-ai/discussions
- **Discord**: https://discord.gg/terraphim

### Support
- **Security Reports**: security@terraphim.ai
- **General Support**: support@terraphim.ai
- **Enterprise**: enterprise@terraphim.ai

---

## 🎯 Next Steps

1. **Choose your deployment method** based on platform and requirements
2. **Verify installation** using the verification steps
3. **Configure** according to your use case
4. **Monitor** using logs and health checks
5. **Scale** using Docker or systemd services as needed

---

**Release Version**: v0.2.5-cross-platform
**Security Status**: ✅ All vulnerabilities resolved
**Last Updated**: October 26, 2025

For the most up-to-date information, visit: https://github.com/terraphim/terraphim-ai/releases