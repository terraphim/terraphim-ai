# Homebrew Formula for Terraphim AI

This page provides instructions for installing Terraphim AI using Homebrew on macOS and Linux.

## Installation

### Option 1: Install from Tap (Recommended)

Once published to a Homebrew tap, you can install with:

```bash
# Add the Terraphim AI tap
brew tap terraphim/terraphim-ai

# Install Terraphim AI
brew install terraphim-ai
```

### Option 2: Install from Local Formula

For development or testing, you can install directly from the formula file:

```bash
# Clone the repository
git clone https://github.com/terraphim/terraphim-ai.git
cd terraphim-ai

# Install from local formula
brew install --build-from-source ./terraphim-ai.rb
```

## What Gets Installed

The Homebrew formula installs the following components:

### Binaries
- **Server**: `terraphim_server` command-line tool
- **TUI**: `terraphim-agent` terminal user interface
- **Desktop App**: "Terraphim Desktop.app" (macOS only)

### Configuration
- Default configuration files in `/usr/local/etc/terraphim-ai/`
- Includes role configurations for different user profiles

### Documentation
- README and documentation files in `/usr/local/share/doc/terraphim-ai/`

## Usage

### Server Mode
```bash
# Start the server
terraphim_server

# Start with custom configuration
terraphim_server --config /path/to/config.json

# View help
terraphim_server --help
```

### Terminal UI (TUI)
```bash
# Start the interactive terminal interface
terraphim-agent

# Use REPL mode with full features
terraphim-agent --features repl-full

# View available commands
terraphim-agent --help
```

### Desktop App (macOS)
```bash
# Launch desktop application
open "/Applications/Terraphim Desktop.app"
```

### Service Management
```bash
# Start as background service
brew services start terraphim-ai

# Stop the service
brew services stop terraphim-ai

# View service status
brew services list | grep terraphim-ai
```

## Configuration

Default configuration files are installed in `/usr/local/etc/terraphim-ai/`:
- `terraphim_engineer_config.json` - Engineering role configuration
- `system_operator_config.json` - System operator role configuration

You can customize these files or create your own configurations.

## Logs

When running as a service, logs are written to:
- **Standard output**: `/usr/local/var/log/terraphim-ai.log`
- **Error output**: `/usr/local/var/log/terraphim-ai-error.log`

## Uninstalling

```bash
# Stop the service (if running)
brew services stop terraphim-ai

# Uninstall the package
brew uninstall terraphim-ai

# Remove the tap (optional)
brew untap terraphim/terraphim-ai
```

## Homebrew Formula

The complete Homebrew formula (`terraphim-ai.rb`):

```ruby
class TerraphimAi < Formula
  desc "Privacy-first AI assistant with semantic search and knowledge graphs"
  homepage "https://github.com/terraphim/terraphim-ai"
  url "https://github.com/terraphim/terraphim-ai/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Will need to be updated with actual SHA256
  license "MIT"
  head "https://github.com/terraphim/terraphim-ai.git", branch: "main"

  depends_on "rust" => :build
  depends_on "node" => :build
  depends_on "yarn" => :build
  depends_on "pkg-config" => :build
  depends_on "openssl@3"
  depends_on "sqlite"

  def install
    # Build the Rust components
    system "cargo", "build", "--release", "--bin", "terraphim_server"

    # Build the desktop app (if on macOS)
    if OS.mac?
      cd "desktop" do
        system "yarn", "install"
        system "yarn", "run", "tauri", "build"
      end

      # Install the desktop app bundle
      app_bundle = "target/release/bundle/macos/Terraphim Desktop.app"
      if File.exist?(app_bundle)
        prefix.install app_bundle => "Terraphim Desktop.app"
      end
    end

    # Install the server binary
    bin.install "target/release/terraphim_server"

    # Install configuration files
    (etc/"terraphim-ai").mkpath
    (etc/"terraphim-ai").install Dir["terraphim_server/default/*.json"]

    # Install documentation
    doc.install "README.md"
    doc.install "docs" if Dir.exist?("docs")
  end

  def caveats
    <<~EOS
      Terraphim AI has been installed with the following components:

      1. Server: Run with `terraphim_server`
      2. Desktop App: Available in Applications folder (macOS only)

      Default configuration files are located in:
        #{etc}/terraphim-ai/

      For first-time setup:
        1. Run `terraphim_server --help` to see available options
        2. Configuration files can be customized in #{etc}/terraphim-ai/
        3. The desktop app will create its own config on first run

      Documentation is available in:
        #{doc}/
    EOS
  end

  service do
    run opt_bin/"terraphim_server"
    keep_alive true
    error_log_path var/"log/terraphim-ai-error.log"
    log_path var/"log/terraphim-ai.log"
    working_dir HOMEBREW_PREFIX
  end

  test do
    # Test that the server binary was installed and shows version info
    system "#{bin}/terraphim_server", "--version"

    # Test that config files were installed
    assert_predicate etc/"terraphim-ai", :exist?

    # Test basic functionality by checking the help output
    help_output = shell_output("#{bin}/terraphim_server --help")
    assert_match "Terraphim AI Server", help_output
  end
end
```

## Requirements

### Build Dependencies
- Rust (latest stable)
- Node.js (v16 or later)
- Yarn package manager
- pkg-config

### Runtime Dependencies
- OpenSSL 3.x
- SQLite 3.x

## Troubleshooting

### Build Issues

1. **Rust not found**: Ensure Rust is installed via `rustup` or Homebrew
2. **Node.js version**: Requires Node.js v16 or later
3. **Missing dependencies**: Run `brew doctor` to check for system issues

### Runtime Issues

1. **Config not found**: Check `/usr/local/etc/terraphim-ai/` for configuration files
2. **Permission errors**: Ensure proper file permissions for log directory
3. **Port conflicts**: Default server port may conflict with other services

### Getting Help

- Check the logs in `/usr/local/var/log/terraphim-ai*.log`
- Run `terraphim_server --help` for command-line options
- Refer to the main documentation in `/usr/local/share/doc/terraphim-ai/`

## Publishing to Homebrew

To publish this formula to Homebrew:

1. **Create a release** on GitHub with proper version tagging
2. **Calculate SHA256** of the release archive
3. **Update the formula** with the correct URL and SHA256
4. **Submit to homebrew-core** or create your own tap

For detailed instructions on creating Homebrew formulae, see the [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook).
