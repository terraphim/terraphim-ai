# Terraphim Agent Auto-Update System

Complete guide to the auto-update functionality built into terraphim-agent CLI.

## Overview

Terraphim-agent includes a sophisticated auto-update system that seamlessly keeps your installation current with the latest releases from GitHub. The system is designed to be secure, user-friendly, and reliable.

## Features

- **🚀 Automatic Updates**: Binary replacement without manual intervention
- **📊 Progress Tracking**: Real-time download progress with status indicators
- **🔒 Secure Verification**: GitHub Releases integration ensures authenticated updates
- **🌐 Cross-Platform**: Works on Linux, macOS, and Windows
- **📋 Version Intelligence**: Smart version comparison and update availability detection
- **⚡ Async-Safe**: Designed to work seamlessly with async Rust applications
- **🛡️ Error Handling**: Graceful degradation and detailed error reporting

## Quick Start

```bash
# Check if updates are available
terraphim-agent check-update

# Update to latest version
terraphim-agent update

# Get help for update commands
terraphim-agent check-update --help
terraphim-agent update --help
```

## Commands Reference

### `check-update`
Checks for available updates without installing them.

```bash
terraphim-agent check-update
```

**Output Examples:**
- ✅ **Up-to-date**: `✅ Already running latest version: 1.0.0`
- 📦 **Update Available**: `📦 Update available: 1.0.0 → 1.0.1`
- ❌ **Error**: `❌ Update failed: Network error - Connection refused`

### `update`
Checks for updates and installs them if available.

```bash
terraphim-agent update
```

**Output Examples:**
- 🚀 **Success**: `🚀 Updated from 1.0.0 to 1.0.1`
- ✅ **No Update**: `✅ Already running latest version: 1.0.0`
- ❌ **Error**: `❌ Update failed: Permission denied`

## Technical Architecture

### Update Source
- **Repository**: `terraphim/terraphim-ai`
- **Platform**: GitHub Releases
- **Authentication**: Secure GitHub API integration

### Implementation Details
- **Core Library**: `self_update` crate
- **Architecture**: `tokio::task::spawn_blocking` for async compatibility
- **Version Comparison**: Semantic versioning with intelligent parsing
- **Binary Verification**: GitHub release signature verification

### Runtime Safety
The system uses `tokio::task::spawn_blocking` to isolate the potentially blocking `self_update` operations from the async runtime, preventing conflicts like:

```
Cannot drop a runtime in a context where blocking is not allowed
```

## Update Process

1. **Version Detection**: Current version extracted from binary metadata
2. **Release Query**: Query GitHub Releases API for latest version
3. **Version Comparison**: Compare current vs latest using semantic versioning
4. **Download**: Fetch release binary for current platform and architecture
5. **Verification**: Validate binary integrity and GitHub release authenticity
6. **Installation**: Replace current binary with new version
7. **Cleanup**: Remove temporary files and update status

## Status Messages

| Status | Icon | Message | Meaning |
|--------|------|---------|---------|
| Checking | 🔍 | `🔍 Checking for terraphim-agent updates...` | Querying GitHub Releases |
| Up-to-date | ✅ | `✅ Already running latest version: X.Y.Z` | No updates needed |
| Available | 📦 | `📦 Update available: X.Y.Z → A.B.C` | Update is ready to install |
| Updated | 🚀 | `🚀 Updated from X.Y.Z to A.B.C` | Successfully updated |
| Failed | ❌ | `❌ Update failed: [error details]` | Update process failed |

## Troubleshooting

### Common Issues

#### Network Connectivity
**Error**: `Update failed: Network error - Connection refused`
**Solution**: Check internet connection and GitHub accessibility
```bash
curl -I https://api.github.com/repos/terraphim/terraphim-ai/releases/latest
```

#### Permission Denied
**Error**: `Update failed: Permission denied`
**Solution**: Ensure you have write permissions to the binary location
```bash
# For system-wide installation
sudo terraphim-agent update

# For user installation
chmod +w $(which terraphim-agent)
terraphim-agent update
```

#### Binary Not Found
**Error**: `Failed to execute update command: No such file or directory`
**Solution**: Verify terraphim-agent is in your PATH
```bash
which terraphim-agent
echo $PATH
```

#### GitHub Rate Limiting
**Error**: `Update failed: API rate limit exceeded`
**Solution**: Wait for rate limit reset (typically 1 hour) or try again later

### Debug Mode

Enable verbose logging for troubleshooting:

```bash
RUST_LOG=debug terraphim-agent check-update
RUST_LOG=debug terraphim-agent update
```

### Manual Installation

If auto-update fails, you can manually install:

```bash
# Download latest release
curl -L https://github.com/terraphim/terraphim-ai/releases/latest/download/terraphim-agent-linux-x64 -o terraphim-agent

# Make executable
chmod +x terraphim-agent

# Replace binary (system-wide)
sudo mv terraphim-agent /usr/local/bin/

# Or replace binary (user)
mv terraphim-agent ~/.local/bin/
```

## Security Considerations

- **Source Verification**: Updates only come from official GitHub Releases
- **Binary Integrity**: Release assets are verified during download
- **No Arbitrary Execution**: Only pre-built binaries are installed
- **Transparent Process**: All operations are logged and visible
- **User Control**: Updates are opt-in, no automatic background updates

## Integration Examples

### CI/CD Pipeline
```bash
#!/bin/bash
# Update terraphim-agent before running tests
echo "🔄 Updating terraphim-agent..."
if terraphim-agent update; then
    echo "✅ terraphim-agent updated successfully"
else
    echo "⚠️  terraphim-agent update failed, using current version"
fi

# Run tests with latest version
terraphim-agent --version
```

### Systemd Service
```ini
[Unit]
Description=Terraphim Agent Update
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/terraphim-agent update
User=terraphim
Group=terraphim

[Install]
WantedBy=multi-user.target
```

### Cron Job
```bash
# Weekly update check (Sundays at 2 AM)
0 2 * * 0 /usr/local/bin/terraphim-agent check-update >> /var/log/terraphim-updates.log
```

## API Reference (for developers)

The auto-update functionality is available as a Rust crate:

```rust
use terraphim_update::{TerraphimUpdater, UpdaterConfig};

// Create updater configuration
let config = UpdaterConfig::new("terraphim-agent")
    .with_version("1.0.0")
    .with_progress(true);

// Create updater instance
let updater = TerraphimUpdater::new(config);

// Check for updates
let status = updater.check_update().await?;
println!("Update status: {}", status);

// Update if available
let status = updater.update().await?;
println!("Update result: {}", status);
```

## Development

### Testing Auto-Update Functionality

```bash
# Run integration tests
cargo test -p terraphim_agent --test update_functionality_tests --features repl-full --release

# Test with debug binary
cargo build -p terraphim_agent --features repl-full
./target/debug/terraphim-agent check-update
```

### Mock Updates (Development)

For testing without actual releases, you can:

1. Create test releases in a fork
2. Use environment variables to override repository
3. Modify version strings for testing

## Contributing

When contributing to the auto-update system:

1. Test both `check-update` and `update` commands
2. Verify cross-platform compatibility
3. Add integration tests for new features
4. Update documentation for API changes
5. Test network error scenarios

## Support

- **Issues**: [GitHub Issues](https://github.com/terraphim/terraphim-ai/issues)
- **Discussions**: [GitHub Discussions](https://github.com/terraphim/terraphim-ai/discussions)
- **Discord**: [Terraphim Discord](https://discord.gg/VPJXB6BGuY)