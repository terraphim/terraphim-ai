# Installation Guide

Complete guide for installing Terraphim AI across all platforms and use cases.

## 🚀 Quick Install (Choose One Method)

### Option 1: Rust CLI/TUI (Most Powerful)
**Best for**: Power users, developers, researchers, automation

```bash
cargo install terraphim-agent
terraphim-agent --help
```

**Features**: Complete CLI with 14 commands, TUI interface, advanced configuration

### Option 2: Node.js Package (Web Integration)
**Best for**: Web developers, JavaScript projects, real-time search

```bash
npm install @terraphim/autocomplete
# or with Bun
bun add @terraphim/autocomplete
```

**Features**: Native bindings, autocomplete engine, knowledge graph APIs

### Option 3: Python Library (Data Processing)
**Best for**: Data scientists, Python developers, text analysis

```bash
pip install terraphim-automata
```

**Features**: PyO3 bindings, high-performance text processing, fuzzy search

### Option 4: Desktop Application (GUI Users)
**Best for**: Non-technical users, visual interface preference

**Download**: [Latest Release](https://github.com/terraphim/terraphim-ai/releases)

**Features**: Native GUI, system tray, auto-update, cross-platform

---

## 📋 System Requirements

### Minimum Requirements
- **OS**: Linux (Ubuntu 20.04+), macOS (10.15+), Windows (10+)
- **RAM**: 4GB+ (8GB+ recommended for large datasets)
- **Storage**: 500MB for application + 2GB for data (optional)
- **Network**: Internet connection for GitHub/integration features

### Recommended Requirements
- **OS**: Latest stable versions of major distributions
- **RAM**: 16GB+ for optimal performance with large knowledge graphs
- **Storage**: SSD with 10GB+ available space
- **Network**: Broadband connection for large repository cloning

---

## 🔧 Detailed Installation Steps

### Rust Toolchain Setup

#### 1. Install Rust (if not already installed)
```bash
# Official installer
curl --proto '=https://sh.rustup.rs' -sSf | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### 2. Configure Environment
```bash
# Add to PATH (if not already)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Platform-Specific Installation

#### Linux (Ubuntu/Debian)
```bash
# Update package manager
sudo apt update

# Install system dependencies
sudo apt install build-essential pkg-config libssl-dev

# Install Terraphim
cargo install terraphim-agent

# Verify installation
terraphim-agent --version
```

#### macOS
```bash
# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Terraphim
brew install terraphim-agent

# Verify installation
terraphim-agent --version
```

#### Windows
```bash
# Download Rust installer
curl https://sh.rustup.rs -sSf > rustup-init.sh
sh rustup-init.sh

# Install Terraphim
cargo install terraphim-agent

# Add to PATH (PowerShell)
[System.Environment]::SetEnvironmentVariable('PATH', "$env:USERPROFILE\.cargo\bin;$env:PATH", 'User')
```

### Alternative Installation Methods

#### From Source (for Developers)
```bash
# Clone repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build release version
cargo build --release

# Install to local path
cargo install --path .
```

#### Using Package Managers

##### Cargo (Rust Package Manager)
```bash
# Install from crates.io
cargo install terraphim-agent

# Install specific version
cargo install terraphim-agent --version 1.2.3
```

##### NPM (Node.js)
```bash
# Standard installation
npm install @terraphim/autocomplete

# Global installation
npm install -g @terraphim/autocomplete

# Using Yarn
yarn add @terraphim/autocomplete

# Using Bun
bun add @terraphim/autocomplete
```

##### Pip (Python)
```bash
# Standard installation
pip install terraphim-automata

# User-specific installation
pip install --user terraphim-automata

# Using Conda
conda install -c conda-forge terraphim-automata
```

---

## 🔍 Verification & First Run

### Verify Installation
```bash
# Check CLI version
terraphim-agent --version

# Check available commands
terraphim-agent --help

# Run basic functionality test
terraphim-agent search --help
```

### Initial Configuration
```bash
# Create configuration directory
mkdir -p ~/.config/terraphim

# Initialize with defaults
terraphim-agent init

# Verify configuration
cat ~/.config/terraphim/config.toml
```

### First Query Test
```bash
# Test with built-in data
terraphim-agent search "test query"

# Test with your own data (if available)
terraphim-agent search --source local --path "~/Documents" "your query"
```

---

## 🔧 Configuration

### Environment Variables
```bash
# Optional configuration
export TERRAPHIM_LOG_LEVEL=info  # debug, info, warn, error
export TERRAPHIM_CONFIG_PATH=~/.config/terraphim/config.toml
export TERRAPHIM_DATA_PATH=~/Documents/terraphim
```

### Configuration File Setup
Create `~/.config/terraphim/config.toml`:

```toml
[data]
default_data_path = "~/Documents/terraphim"
index_documents = true
cache_size = "1GB"

[search]
default_scorer = "tfidf"  # bm25, tfidf, jaccard
max_results = 20
timeout_seconds = 30

[sources]
local_files_enabled = true
github_enabled = true
team_data_enabled = false

[llm]
provider = "ollama"  # ollama, openrouter, claude
model = "llama3.2:3b"
temperature = 0.7
base_url = "http://localhost:11434"
```

---

## 🚨 Troubleshooting

### Common Installation Issues

#### Permission Denied
```bash
# Fix Rust ownership
sudo chown -R $USER:$(id -gn $USER) ~/.cargo
sudo chmod -R 755 ~/.cargo

# Fix configuration directory
mkdir -p ~/.config/terraphim
chmod 755 ~/.config/terraphim
```

#### PATH Issues
```bash
# Check if cargo is in PATH
which cargo

# Add to current session
export PATH="$HOME/.cargo/bin:$PATH"

# Add to shell profile permanently
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
```

#### Compilation Errors
```bash
# Update Rust toolchain
rustup update
rustup install stable

# Clear cargo cache
cargo clean

# Rebuild
cargo build --release
```

#### Network Issues
```bash
# Test internet connectivity
curl -I https://github.com
curl -I https://crates.io

# Use proxy if needed
export https_proxy=http://proxy.company.com:8080
export http_proxy=http://proxy.company.com:8080
```

---

## 🎯 Next Steps After Installation

### 1. Configure Data Sources
- [Add local documents](user-guide/configuration.md)
- [Connect GitHub repositories](user-guide/team-integration.md)
- [Set up team knowledge bases](user-guide/advanced-usage.md)

### 2. Explore Features
- [Semantic search capabilities](user-guide/advanced-usage.md)
- [AI chat features](user-guide/ai-chat.md)
- [Knowledge graph exploration](user-guide/knowledge-graph.md)

### 3. Integration Examples
- [Node.js integration](developer-guide/nodejs-integration.md)
- [Python data processing](developer-guide/data-processing.md)
- [Desktop application](user-guide/desktop-app.md)

### 4. Advanced Configuration
- [Multiple data sources](user-guide/configuration.md)
- [Custom scorers](user-guide/advanced-usage.md)
- [LLM provider setup](user-guide/ai-setup.md)

---

## 📞 Getting Help

### Documentation
- [Full Documentation](https://docs.terraphim.ai)
- [API Reference](https://docs.terraphim.ai/api)
- [Community Examples](https://github.com/terraphim/terraphim-ai/examples)

### Community Support
- **Discord**: [Join our Community](https://discord.gg/VPJXB6BGuY)
- **GitHub Discussions**: [Start a Discussion](https://github.com/terraphim/terraphim-ai/discussions)
- **GitHub Issues**: [Report an Issue](https://github.com/terraphim/terraphim-ai/issues)

### Professional Support
- **Email**: support@terraphim.ai
- **Documentation**: https://docs.terraphim.ai
- **Status Page**: https://status.terraphim.ai

---

## 🔖 Installation Verification Checklist

- [ ] Rust toolchain installed and verified
- [ ] Terraphim package installed successfully
- [ ] `terraphim-agent --version` shows correct version
- [ ] Configuration file created at expected location
- [ ] Basic search functionality working
- [ ] Help system accessible (`terraphim-agent --help`)
- [ ] Documentation accessible from help command
- [ ] Network connectivity working (for GitHub integration)
- [ ] Permissions properly set on configuration directories

If any of these items fail, consult the [Troubleshooting Guide](troubleshooting.md) or contact community support.

---

*Last Updated: December 20, 2025*
*Version: Terraphim AI v1.3.0*
*Part of: Terraphim AI Documentation Suite*