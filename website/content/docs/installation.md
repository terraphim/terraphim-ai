+++
title = "Installation"
description = "Install Terraphim AI on Linux, macOS, or Windows using your preferred method"
date = 2026-01-27
+++

# Installation

Choose installation method that best suits your needs and platform.

## Quick Install (Recommended)

The universal installer automatically detects your platform and installs the appropriate version.

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/scripts/install.sh | bash
\`\`\`

## Package Managers

### Homebrew (macOS/Linux)

Homebrew provides signed and notarized binaries for macOS and Linux.

\`\`\`bash
# Add Terraphim tap
brew tap terraphim/terraphim

# Install server
brew install terraphim-server

# Install TUI/REPL
brew install terraphim-agent
\`\`\`

### Cargo (Rust)

Install using Cargo, Rust's package manager.

\`\`\`bash
# Install REPL with interactive TUI (11 commands)
cargo install terraphim-repl

# Install CLI for automation (8 commands)
cargo install terraphim-cli
\`\`\`

### npm (Node.js)

Install the autocomplete package with knowledge graph support.

\`\`\`bash
npm install @terraphim/autocomplete
\`\`\`

### PyPI (Python)

Install the high-performance text processing library.

\`\`\`bash
pip install terraphim-automata
\`\`\`

## Platform-Specific Guides

### Linux

#### Binary Download

Download the latest release from GitHub:

\`\`\`bash
wget https://github.com/terraphim/terraphim-ai/releases/latest/download/terraphim_server-linux-x86_64.tar.gz
tar -xzf terraphim_server-linux-x86_64.tar.gz
sudo mv terraphim_server /usr/local/bin/
\`\`\`

#### Build from Source

\`\`\`bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build the workspace
cargo build --workspace --release

# Install (optional)
sudo cp target/release/terraphim_server /usr/local/bin/
sudo cp target/release/terraphim-agent /usr/local/bin/
\`\`\`

### macOS

#### Binary Download

\`\`\`bash
# Download using Homebrew (recommended)
brew install terraphim-server terraphim-agent

# Or download manually
curl -L https://github.com/terraphim/terraphim-ai/releases/latest/download/terraphim_server-darwin-x86_64.tar.gz -o terraphim_server.tar.gz
tar -xzf terraphim_server.tar.gz
sudo mv terraphim_server /usr/local/bin/
\`\`\`

#### Build from Source

Requires Xcode command line tools.

\`\`\`bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build the workspace
cargo build --workspace --release

# Install (optional)
sudo cp target/release/terraphim_server /usr/local/bin/
sudo cp target/release/terraphim-agent /usr/local/bin/
\`\`\`

### Windows

#### Binary Download

Download the latest release from GitHub and extract to a directory in your PATH.

- [Download for Windows x64](https://github.com/terraphim/terraphim-ai/releases/latest)

#### Build from Source

Requires [Rust for Windows](https://rustup.rs/).

\`\`\`powershell
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Build the workspace
cargo build --workspace --release

# The binaries will be in target\\release\\
\`\`\`

## Docker

Run Terraphim in a Docker container.

\`\`\`bash
# Pull the latest image
docker pull terraphim/terraphim-ai:latest

# Run the server
docker run -p 8080:8080 terraphim/terraphim-ai:latest
\`\`\`

## Verification

After installation, verify that Terraphim is working:

\`\`\`bash
# Check version
terraphim-server --version
terraphim-agent --version

# Start the server
terraphim-server

# In another terminal, use the REPL
terraphim-repl
\`\`\`

## Troubleshooting

### Permission Denied

If you get a permission denied error, make the binary executable:

\`\`\`bash
chmod +x /usr/local/bin/terraphim_server
chmod +x /usr/local/bin/terraphim-agent
\`\`\`

### Command Not Found

Ensure that the installation directory is in your PATH:

\`\`\`bash
# For bash
echo 'export PATH=$PATH:/usr/local/bin' >> ~/.bashrc
source ~/.bashrc

# For zsh
echo 'export PATH=$PATH:/usr/local/bin' >> ~/.zshrc
source ~/.zshrc
\`\`\`

### Rust Version Issues

Ensure that you have a recent Rust version:

\`\`\`bash
rustc --version  # Should be 1.70.0 or later
rustup update stable
\`\`\`

## Next Steps

- [Quickstart Guide](/docs/quickstart) - Get up and running in 5 minutes
- [Full Documentation](https://docs.terraphim.ai) - Comprehensive user guide
- [Configuration Guide](/docs/terraphim_config) - Customize Terraphim to your needs
- [Community](https://discord.gg/VPJXB6BGuY) - Join our Discord for support
