# Platform-Specific Installation Guide

This guide provides detailed installation instructions for different operating systems and platforms.

## 🐧 Linux

### Debian/Ubuntu (18.04+)

#### Method 1: Package Repository (Recommended)

```bash
# Download and install server
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-server_0.2.3-1_amd64.deb
sudo dpkg -i terraphim-server_0.2.3-1_amd64.deb

# Download and install TUI (optional)
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-tui_0.2.3-1_amd64.deb
sudo dpkg -i terraphim-tui_0.2.3-1_amd64.deb

# Start the server
sudo systemctl start terraphim-server
sudo systemctl enable terraphim-server
```

#### Method 2: Automated Script

```bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.4/install.sh | bash
```

### Red Hat/CentOS/Fedora

#### Method 1: RPM Packages

```bash
# Download and install server
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-server-0.2.3-2.x86_64.rpm
sudo yum localinstall terraphim-server-0.2.3-2.x86_64.rpm

# Download and install TUI (optional)
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-tui-0.2.3-2.x86_64.rpm
sudo yum localinstall terraphim-tui-0.2.3-2.x86_64.rpm

# Start the server
sudo systemctl start terraphim-server
sudo systemctl enable terraphim-server
```

#### Method 2: Using dnf (Fedora)

```bash
sudo dnf install terraphim-server-0.2.3-2.x86_64.rpm
sudo dnf install terraphim-tui-0.2.3-2.x86_64.rpm
```

#### Method 3: Build from Source

```bash
# Install dependencies
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel pkg-config

# Clone and build
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
cargo build --release

# Install binaries
sudo cp target/release/terraphim_server /usr/local/bin/
sudo cp target/release/terraphim-tui /usr/local/bin/
```

### Arch Linux/Manjaro

#### Method 1: Arch Packages

```bash
# Download and install server
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-server-0.2.3-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-server-0.2.3-1-x86_64.pkg.tar.zst

# Download and install TUI (optional)
wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/terraphim-tui-0.2.3-1-x86_64.pkg.tar.zst
sudo pacman -U terraphim-tui-0.2.3-1-x86_64.pkg.tar.zst
```

#### Method 2: AUR (Arch User Repository)

```bash
# If available in AUR
yay -S terraphim-ai-git

# Or build manually
git clone https://aur.archlinux.org/terraphim-ai.git
cd terraphim-ai
makepkg -si
```

#### Method 3: Using Cargo

```bash
cargo install terraphim-ai --git https://github.com/terraphim/terraphim-ai.git
```

### openSUSE

```bash
# Install dependencies
sudo zypper install gcc rust cargo openssl-devel pkg-config

# Clone and build
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release

# Install manually
sudo cp target/release/terraphim_server /usr/local/bin/
sudo cp target/release/teraphim-tui /usr/local/bin/
```

## 🍎 macOS

### Method 1: App Bundle (Easy)

1. Download the macOS app bundle:
   ```bash
   wget https://github.com/terraphim/terraphim-ai/releases/download/v0.2.4/TerraphimServer-0.2.3-macos.tar.gz
   ```

2. Extract and move to Applications:
   ```bash
   tar -xzf TerraphimServer-0.2.3-macos.tar.gz
   mv TerraphimServer.app /Applications/
   ```

3. Launch from Applications folder

### Method 2: Homebrew

```bash
# If available in Homebrew
brew install terraphim-ai

# Or build from source
brew install rust
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release
```

### Method 3: MacPorts

```bash
sudo port install rust cargo
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release
```

### Method 4: Build from Source

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release

# Create symbolic links
ln -s $(pwd)/target/release/terraphim_server /usr/local/bin/
ln -s $(pwd)/target/release/terraphim-tui /usr/local/bin/
```

### First Launch Configuration

When you first launch Terraphim AI on macOS:

1. **Allow the app to run**: System Preferences > Security & Privacy > General
2. **Configuration directories** will be created at:
   - `~/Library/Application Support/Terraphim/config.json`
   - `~/Library/Application Support/Terraphim/data/`

3. **Terminal opens** automatically when launching the app

## 🪟 Windows

### Method 1: Chocolatey (Recommended)

```powershell
# Install Chocolatey if not already installed
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = 'Tls12, Tls11, Tls, Ssl3'; iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# Install Terraphim AI (when available)
choco install terraphim-ai
```

### Method 2: WSL (Windows Subsystem for Linux)

1. **Install WSL**:
   ```powershell
   wsl --install
   ```

2. **Install Ubuntu in WSL**:
   ```powershell
   wsl --install -d Ubuntu
   ```

3. **Follow Linux instructions** inside WSL

### Method 3: WSL2 with GUI

1. **Install WSL2**:
   ```powershell
   wsl --install -d Ubuntu --version 2
   ```

2. **Install WSLg** for GUI applications:
   ```powershell
   # In Ubuntu WSL
   curl -fsSL https://raw.githubusercontent.com/wslutilities/wslu/master/install.sh | bash
   ```

3. **Install Terraphim AI** following Linux instructions

### Method 4: Docker for Windows

1. **Install Docker Desktop** from https://www.docker.com/products/docker-desktop/

2. **Pull and run**:
   ```powershell
   docker run -d `
       --name terraphim-server `
       -p 8000:8000 `
       -v ${HOME}/.config/terraphim:/home/terraphim/.config/terraphim `
       -v ${HOME}/.local/share/terraphim:/home/terraphim/data `
       ghcr.io/terraphim/terraphim-server:v0.2.4
   ```

3. **Access** at http://localhost:8000

### Method 5: MSYS2/MinGW

1. **Install MSYS2** from https://www.msys2.org/

2. **Install dependencies** in MSYS2:
   ```bash
   pacman -S mingw-w64-x86_64-toolchain
   pacman -S mingw-w64-x86_64-openssl
   pacman -S mingw-w64-x86_64-pkg-config
   ```

3. **Build from source** following Linux instructions

### Windows Service Installation

```powershell
# Create service directory
New-Item -ItemType Directory -Path "C:\Terraphim" -Force
New-Item -ItemType Directory -Path "C:\Terraphim\bin" -Force
New-Item -ItemType Directory -Path "C:\Terraphim\config" -Force
New-Item -ItemType Directory -Path "C:\Terraphim\data" -Force

# Create configuration
@'
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "C:\\Users\\%USERNAME%\\Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    }
  ]
}
'@ | Out-File -FilePath "C:\Terraphim\config\config.json" -Encoding UTF8

# Create Windows service
sc create TerraphimServer binPath= "C:\Terraphim\bin\terraphim_server.exe" start= auto displayName= "Terraphim AI Server" description= "Privacy-first AI assistant backend"
sc start TerraphimServer
```

## 🐳 Docker Installation

### Method 1: Quick Start

```bash
# One-command installation
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.4/docker-run.sh | bash
```

### Method 2: Docker Compose

```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Start with Docker Compose
docker-compose up -d

# View logs
docker-compose logs -f terraphim
```

### Method 3: Docker Manual

```bash
# Pull image
docker pull ghcr.io/terraphim/terraphim-server:v0.2.4

# Run container
docker run -d \
  --name terraphim-server \
  -p 8000:8000 \
  -v $(pwd)/config:/home/terraphim/.config/terraphim \
  -v $(pwd)/data:/home/terraphim/data \
  --restart unless-stopped \
  ghcr.io/terraphim/terraphim-server:v0.2.4
```

## 📱 Mobile Devices

### Android (Termux)

```bash
# Install Termux from F-Droid
pkg update && pkg upgrade

# Install dependencies
pkg install rust openssl pkgconfig

# Clone and build
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai
cargo build --release

# Run
./target/release/terrapih_server --config config.json
```

### iOS (iSH Shell)

1. Install iSH Shell from App Store
2. Install dependencies
3. Follow Linux build instructions

## 🔧 Post-Installation Configuration

### Create First Configuration

All platforms create a default configuration at:

- **Linux**: `~/.config/terraphim/config.json`
- **macOS**: `~/Library/Application Support/Terraphim/config.json`
- **Windows**: `%APPDATA%\Terraphim\config.json`

Example configuration:
```json
{
  "name": "Terraphim Engineer",
  "relevance_function": "TerraphimGraph",
  "theme": "spacelab",
  "haystacks": [
    {
      "name": "Local Documents",
      "service": "Ripgrep",
      "location": "~/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst"
      }
    }
  ]
}
```

### Add Data Sources

Edit your configuration to add more haystacks:

```json
{
  "haystacks": [
    {
      "name": "Personal Documents",
      "service": "Ripgrep",
      "location": "~/Documents",
      "extra_parameters": {
        "glob": "*.md,*.txt,*.rst,*.org"
      }
    },
    {
      "name": "Code Repository",
      "service": "Ripgrep",
      "location": "~/Projects",
      "extra_parameters": {
        "glob": "*.rs,*.js,*.ts,*.py,*.go"
      }
    },
    {
      "name": "Knowledge Base",
      "service": "AtomicServer",
      "location": "https://atomic-data.dev",
      "extra_parameters": {}
    }
  ]
}
```

### Verify Installation

```bash
# Check server health
curl http://localhost:8000/health

# Test search
curl -X POST http://localhost:8000/api/documents/search \
  -H "Content-Type: application/json" \
  -d '{"search_term": "test", "limit": 5}'

# Test TUI
terraphim-tui --help
terraphim-tui search "test" --limit 5
```

## 🛠️ Troubleshooting

### Common Issues

#### Port Already in Use
```bash
# Check what's using port 8000
sudo netstat -tlnp | grep :8000

# Kill the process
sudo kill -9 <PID>

# Or use different port
export TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8080"
```

#### Permission Denied
```bash
# Linux/macOS
sudo chown -R $USER:$USER ~/.config/terraphim
sudo chown -R $USER:$USER ~/.local/share/terraphim

# Windows
# Run as Administrator
```

#### Missing Dependencies

**Linux:**
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install build-essential pkg-config libssl-dev

# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel pkgconfig

# Arch Linux
sudo pacman -S base-devel openssl pkgconf
```

**macOS:**
```bash
xcode-select --install
```

#### Build Failures

1. **Update Rust**:
   ```bash
   rustup update
   rustup self update
   ```

2. **Clean build**:
   ```bash
   cargo clean
   cargo build --release
   ```

3. **Check Rust version**:
   ```bash
   rustc --version  # Should be 1.70.0+
   ```

## 📚 Platform-Specific Resources

- [Linux Installation Guide](installation.md#linux)
- [macOS Installation Guide](installation.md#macos)
- [Windows Installation Guide](installation.md#windows)
- [Docker Installation Guide](installation.md#docker)
- [Platform-Specific Issues](troubleshooting.md)

## 🤝 Getting Help

- **GitHub Issues**: [Report bugs](https://github.com/terraphim/terraphim-ai/issues)
- **Discussions**: [Community forum](https://github.com/terraphim/terraphim-ai/discussions)
- **Discord**: [Real-time chat](https://discord.gg/VPJXB6BGuY)
- **Discourse**: [Community discussions](https://terraphim.discourse.group)
