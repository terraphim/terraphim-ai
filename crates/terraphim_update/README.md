# terraphim_update

Auto-update functionality for Terraphim AI binaries.

## Overview

This crate provides a unified interface for self-updating Terraphim AI CLI tools using GitHub Releases as a distribution channel. It includes:

- **Update checking**: Check for available updates from GitHub Releases
- **Binary updating**: Self-update to the latest version
- **Rollback support**: Backup and rollback to previous versions
- **Scheduler**: Background periodic update checks
- **Startup checks**: Non-blocking update checks on application startup

## Features

- Automatic update detection from GitHub Releases
- Safe self-update with signature verification (PGP)
- Backup and rollback support
- Configurable update intervals
- Tokio-based async scheduler for background checks
- Cross-platform support (Linux, macOS, Windows)

## Usage

### Basic Update Check

```rust
use terraphim_update::check_for_updates_auto;

async fn check_updates() {
    let status = check_for_updates_auto("terraphim-cli", "1.0.0").await?;
    println!("{}", status);
    Ok::<(), anyhow::Error>(())
}
```

### Update Binary

```rust
use terraphim_update::update_binary;

async fn update() {
    let status = update_binary("terraphim-cli").await?;
    println!("{}", status);
    Ok::<(), anyhow::Error>(())
}
```

### Startup Check

```rust
use terraphim_update::check_for_updates_startup;

async fn startup() {
    if let Err(e) = check_for_updates_startup("terraphim-agent").await {
        eprintln!("Update check failed: {}", e);
    }
    Ok::<(), anyhow::Error>(())
}
```

### Update Scheduler

```rust
use terraphim_update::start_update_scheduler;

async fn start_scheduler() {
    let handle = start_update_scheduler(
        "terraphim-agent",
        env!("CARGO_PKG_VERSION"),
        Box::new(|update_info| {
            println!("Update available: {} -> {}", update_info.current_version, update_info.latest_version);
        })
    ).await?;

    // Keep scheduler running
    // handle.abort() to stop
    Ok::<(), anyhow::Error>(())
}
```

### Backup and Rollback

```rust
use terraphim_update::{backup_binary, rollback};
use std::path::Path;

// Backup current binary
let backup_path = backup_binary(Path::new("/usr/local/bin/terraphim"), "1.0.0")?;

// Rollback to backup
rollback(&backup_path, Path::new("/usr/local/bin/terraphim"))?;
```

## Configuration

Update behavior can be configured through environment variables or config files:

### Environment Variables

- `TERRAPHIM_AUTO_UPDATE` - Enable/disable auto-update (default: `true`)
- `TERRAPHIM_UPDATE_INTERVAL` - Check interval in seconds (default: `86400` = 24 hours)

### Example Configuration

```bash
export TERRAPHIM_AUTO_UPDATE=true
export TERRAPHIM_UPDATE_INTERVAL=86400
```

## Update Status

The crate returns various update statuses:

- `UpdateStatus::UpToDate(version)` - Already running latest version
- `UpdateStatus::Available { current_version, latest_version }` - Update available but not installed
- `UpdateStatus::Updated { from_version, to_version }` - Successfully updated
- `UpdateStatus::Failed(error)` - Update failed

## Integration with Binaries

### CLI Integration

Add to `Cargo.toml`:

```toml
[dependencies]
terraphim_update = { path = "../terraphim_update", version = "1.0.0" }
```

Add CLI commands:

```rust
enum Command {
    CheckUpdate,
    Update,
    Rollback { version: String },
    // ... other commands
}

async fn handle_check_update() -> Result<serde_json::Value> {
    let status = terraphim_update::check_for_updates_auto(
        "my-binary",
        env!("CARGO_PKG_VERSION")
    ).await?;
    // Convert status to JSON and return
}

async fn handle_update() -> Result<serde_json::Value> {
    let status = terraphim_update::update_binary("my-binary").await?;
    // Convert status to JSON and return
}

async fn handle_rollback(version: &str) -> Result<serde_json::Value> {
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension(format!("bak-{}", version));
    terraphim_update::rollback(&backup_path, &current_exe)?;
    // Return success JSON
}
```

### Agent Integration

Add startup check:

```rust
fn main() -> Result<()> {
    // Check for updates on startup (non-blocking)
    let rt = Runtime::new()?;
    rt.block_on(async {
        if let Err(e) = check_for_updates_startup("terraphim-agent").await {
            eprintln!("Update check failed: {}", e);
        }
    });

    // Start background scheduler
    let scheduler_handle = terraphim_update::start_update_scheduler(
        "terraphim-agent",
        env!("CARGO_PKG_VERSION"),
        Box::new(move |update_info| {
            tracing::info!("Update available: {} -> {}",
                update_info.current_version,
                update_info.latest_version
            );
        })
    ).await?;

    // Run main application
    // ...
}
```

## Rollback Instructions

### List Available Backups

```bash
ls -la /usr/local/bin/terraphim*.bak-*
```

### Rollback to Specific Version

```bash
terraphim-cli rollback 1.0.0
```

Or manually:

```bash
sudo cp /usr/local/bin/terraphim.bak-1.0.0 /usr/local/bin/terraphim
```

## Security

- Updates are downloaded from GitHub Releases over HTTPS
- PGP signature verification is supported (requires manual setup)
- Binary permissions are preserved during updates
- Backup files are created before updating

## Testing

Run tests:

```bash
cargo test -p terraphim_update
```

## License

Apache-2.0
