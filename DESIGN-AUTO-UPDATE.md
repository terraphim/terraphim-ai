# Implementation Plan: Automatic Updates Feature

**Status**: Draft
**Research Doc**: RESEARCH-AUTO-UPDATE.md
**Author**: Design Agent
**Date**: 2025-01-09
**Estimated Effort**: 60-80 hours

## Overview

### Summary

Implement comprehensive automatic update capabilities for both `terraphim-agent` and `terraphim-cli`, enabling users to receive notifications about available updates and optionally install them automatically with full PGP signature verification and rollback support. The feature adds configuration options, background update checking, interactive prompts, and platform-specific notification mechanisms while maintaining backward compatibility with existing manual update commands.

### Approach

**Phase 1 (MVP - Full Implementation):**
1. Add update configuration to `DeviceSettings` and `Config` structs
2. Implement check-on-startup for both binaries
3. Add in-app notification when updates are available
4. Implement auto-update with PGP signature verification
5. Add interactive prompts for user confirmation
6. Add binary backup and rollback support
7. Use tokio intervals for background checks when binaries are running
8. Support Linux and macOS platforms (Windows deferred to Phase 2)

**Phase 2 (Future - tracked in GitHub issue):**
- Desktop notifications (OS-level)
- Scheduled background checks via system schedulers (systemd, launchd)
- Multiple release channels (stable, beta, nightly)
- Update telemetry (with consent)
- Windows platform support

### Scope

**In Scope:**
- Update configuration model (enabled, check interval, auto-install flag)
- Check-on-startup mechanism for `terraphim-agent` and `terraphim-cli`
- In-app notification system for available updates
- Interactive update prompts with user confirmation
- Auto-update with PGP signature verification
- Binary backup and rollback support
- Background update checking using tokio intervals
- Extension of `terraphim_cli` with update commands and rollback command
- Update state persistence (last check time, pending update info)
- Platform-specific update execution logic (Linux, macOS)
- Graceful handling of permissions and network failures
- Silent failure on network errors
- Configuration file management
- Manual update instructions when no write permissions

**Out of Scope:**
- Desktop notifications (deferred to Phase 2)
- System-level schedulers (systemd, launchd, Task Scheduler)
- GUI update prompts
- Delta updates
- Multiple release channels (stable, beta)
- Update telemetry
- Windows platform support (deferred to Phase 2)

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Application                              │
│  ┌──────────────────────┐         ┌──────────────────────┐     │
│  │  terraphim-agent     │         │  terraphim-cli       │     │
│  │  (TUI Application)   │         │  (CLI Interface)     │     │
│  └──────────┬───────────┘         └──────────┬───────────┘     │
│             │                                │                   │
│             │                                │                   │
│  ┌──────────▼────────────────────────────────▼───────────┐     │
│  │           terraphim_update (Update Logic)            │     │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │     │
│  │  │  check_for_  │  │  update_     │  │  schedul-  │ │     │
│  │  │  updates()   │  │  binary()    │  │  er()      │ │     │
│  │  └──────────────┘  └──────────────┘  └────────────┘ │     │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │     │
│  │  │  verify_     │  │  rollback()  │  │  prompt_  │ │     │
│  │  │  signature() │  │              │  │  user()   │ │     │
│  │  └──────────────┘  └──────────────┘  └────────────┘ │     │
│  └──────────┬───────────────────────────────────────────┘     │
│             │                                                   │
│             │                                                   │
│  ┌──────────▼───────────────────────────────────────────┐     │
│  │         terraphim_settings (Device Settings)         │     │
│  │  ┌────────────────────────────────────────────────┐  │     │
│  │  │  UpdateConfig {                               │  │     │
│  │  │    auto_update_enabled: bool,                 │  │     │
│  │  │    auto_update_check_interval: Duration,       │  │     │
│  │  │  }                                             │  │     │
│  │  └────────────────────────────────────────────────┘  │     │
│  └──────────┬───────────────────────────────────────────┘     │
│             │                                                   │
│             │                                                   │
│  ┌──────────▼───────────────────────────────────────────┐     │
│  │         terraphim_config (User Config)               │     │
│  │  ┌────────────────────────────────────────────────┐  │     │
│  │  │  UpdateHistory {                              │  │     │
│  │  │    last_check: DateTime<Utc>,                 │  │     │
│  │  │    last_version: String,                      │  │     │
│  │  │    pending_update: Option<UpdateInfo>,         │  │     │
│  │  │    backup_versions: Vec<String>,              │  │     │
│  │  │  }                                             │  │     │
│  │  └────────────────────────────────────────────────┘  │     │
│  └───────────────────────────────────────────────────────┘     │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐   │
│  │            External Services                            │   │
│  │  ┌──────────────┐         ┌──────────────┐            │   │
│  │  │  GitHub      │         │  Platform     │            │   │
│  │  │  Releases    │         │  Filesystem   │            │   │
│  │  │  API         │         │               │            │   │
│  │  └──────────────┘         └──────────────┘            │   │
│  └────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
```

### Data Flow

**Startup Check Flow:**
```
[Binary Start]
    ↓
[Load Config] → Load DeviceSettings and UpdateHistory
    ↓
[Check if update enabled?]
    ├─ No → [Skip update check]
    └─ Yes → [Check last_check time]
              ↓
          [Time to check?]
              ├─ No → [Schedule next check]
              └─ Yes → [check_for_updates()]
                        ↓
                    [Update available?]
                        ├─ No → [Update last_check time]
                        └─ Yes → [Show in-app notification]
                                 ↓
                             [Update pending_update in config]
```

**Manual Update Command Flow:**
```
[User: terraphim-agent update]
    ↓
[Check for pending update in config]
    ↓
[Check latest version from GitHub]
    ↓
[Update available?]
    ├─ No → [Display "up to date"]
    └─ Yes → [Display interactive prompt: "Update to vX.Y.Z? (y/N)"]
              ↓
          [User confirms?]
              ├─ No → [Skip update]
              └─ Yes → [Check write permissions]
                        ↓
                    [Has write permissions?]
                        ├─ No → [Display manual update instructions]
                        └─ Yes → [Backup current binary as binary.vX.Y.Z]
                                  ↓
                              [Download new binary]
                                  ↓
                              [Verify PGP signature]
                                  ↓
                              [Signature valid?]
                                  ├─ No → [Delete partial download, retry on next check]
                                  └─ Yes → [Replace binary]
                                            ↓
                                        [Update last_version in config]
                                            ↓
                                        [Display success message]
```

**Rollback Command Flow:**
```
[User: terraphim-agent rollback vX.Y.Z]
    ↓
[Check if backup exists: binary.vX.Y.Z]
    ↓
[Backup exists?]
    ├─ No → [Display error: no backup found]
    └─ Yes → [Backup current binary as current.vCURRENT_VERSION]
              ↓
          [Restore binary.vX.Y.Z as binary]
              ↓
          [Update last_version in config]
              ↓
          [Display success message]
```

**Background Check Flow (when binary running):**
```
[Binary Running]
    ↓
[tokio interval timer fires]
    ↓
[check_for_updates() in background task]
    ↓
[Update available?]
    ├─ No → [Continue normal operation]
    └─ Yes → [Queue notification for next UI render]
              ↓
          [Display in-app notification to user]
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| **Check-on-startup + tokio intervals** | Simpler than system schedulers, works across platforms, no root privileges needed | Systemd/launchd/cron - too complex, requires privileges, platform-specific |
| **Configuration in DeviceSettings** | Device-level setting makes sense for update policy (not per-user role) | User-level config - update policy should be system-wide |
| **UpdateHistory separate from DeviceSettings** | Frequently updated data should be separate from static settings | Store in DeviceSettings - would cause frequent config file writes |
| **In-app notifications only (MVP)** | Simpler implementation, works without notification daemon, reduces dependencies | Desktop notifications - requires notify-rust crate, platform-specific daemon |
| **Auto-install enabled in MVP** | Full implementation from the start, users can disable via config | Defer auto-install - would require separate implementation phase |
| **Interactive prompts for updates** | Gives users control while enabling automation | Silent auto-install - could break running sessions without user awareness |
| **PGP signature verification** | Ensures binary integrity and authenticity, prevents supply chain attacks | Checksum-only verification - insufficient for security |
| **Binary backup and rollback** | Allows users to revert problematic updates, reduces risk | No rollback - users stuck with broken updates |
| **Update state persistence** | Allows checking if user already saw notification, prevents spam | No persistence - would show notification on every startup |
| **Graceful degradation (silent network failures)** | Network errors shouldn't interrupt user workflow | Panic or exit - poor user experience |
| **CLI update parity** | Both tools should have same update capabilities | CLI-only manual - creates inconsistent experience |
| **Tokio-based scheduling** | Leverages existing async runtime, cross-platform, well-tested | Custom scheduling logic - unnecessary complexity |
| **Linux + macOS only (Windows deferred)** | self_update crate support uncertain on Windows, requires spike | Include Windows - high risk without testing |
| **Daily check frequency (enabled by default, opt-out)** | Reasonable balance between staying current and not being intrusive | Weekly checks - too infrequent, security patches delayed |
| **Keep old binary after update** | Enables rollback, safety net for failed updates | Delete old binary - no recovery option |
| **Skip to manual instructions on no permissions** | Better user experience than cryptic permission errors | Fail with error - users don't know what to do |
| **Never interrupt user sessions** | Updates should be transparent to active work | Interrupt sessions - disruptive and frustrating |

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_update/src/config.rs` | Update configuration types (UpdateConfig, UpdateHistory) |
| `crates/terraphim_update/src/scheduler.rs` | Tokio-based update scheduling logic |
| `crates/terraphim_update/src/notification.rs` | In-app notification system |
| `crates/terraphim_update/src/state.rs` | Update state persistence and management |
| `crates/terraphim_update/src/verification.rs` | PGP signature verification logic |
| `crates/terraphim_update/src/rollback.rs` | Binary backup and rollback functionality |
| `crates/terraphim_update/src/prompt.rs` | Interactive user prompts for updates |
| `crates/terraphim_update/tests/integration_test.rs` | Integration tests for auto-update flow |
| `crates/terraphim_update/tests/security_test.rs` | Security verification tests for PGP signatures |
| `scripts/test-self-update.sh` | Test script for self-update functionality across platforms |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_update/src/lib.rs` | Export new modules, add public API functions (check_for_updates, update_binary, rollback, verify_signature) |
| `crates/terraphim_update/Cargo.toml` | Add dependencies: tokio, chrono, serde, pgp, base64 |
| `crates/terraphim_settings/src/lib.rs` | Add `UpdateConfig` to `DeviceSettings` struct |
| `crates/terraphim_settings/Cargo.toml` | Add dependency: chrono, serde |
| `crates/terraphim_config/src/lib.rs` | Add `UpdateHistory` to `Config` struct |
| `crates/terraphim_config/Cargo.toml` | Add dependency: chrono |
| `crates/terraphim_agent/src/main.rs` | Add startup check, background scheduler, notification display, interactive prompts |
| `crates/terraphim_cli/src/main.rs` | Add `check-update`, `update`, and `rollback` commands, startup check, interactive prompts |
| `crates/terraphim_cli/Cargo.toml` | Add dependency: terraphim_update |

### Deleted Files

None

## API Design

### Public Types

```rust
/// Configuration for automatic updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable automatic update checking
    pub auto_update_enabled: bool,

    /// Interval between update checks (default: daily)
    pub auto_update_check_interval: Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_update_enabled: true,
            auto_update_check_interval: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Information about an available update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Version number of the update
    pub version: String,

    /// Release date
    pub release_date: DateTime<Utc>,

    /// Release notes or changelog
    pub notes: String,

    /// Download URL for the binary
    pub download_url: String,

    /// PGP signature URL for verification
    pub signature_url: String,

    /// Binary architecture
    pub arch: String,
}

/// Persistent update history state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHistory {
    /// Last time an update check was performed
    pub last_check: DateTime<Utc>,

    /// Version currently installed
    pub current_version: String,

    /// Pending update notification (if user hasn't acted on it)
    pub pending_update: Option<UpdateInfo>,

    /// Backup versions available for rollback
    pub backup_versions: Vec<String>,

    /// Update check history (last 10 checks)
    pub check_history: Vec<UpdateCheckEntry>,
}

/// Single update check entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckEntry {
    /// When the check was performed
    pub timestamp: DateTime<Utc>,

    /// Result of the check
    pub result: UpdateCheckResult,
}

/// Result of an update check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateCheckResult {
    /// No update available
    UpToDate,

    /// Update available
    UpdateAvailable {
        version: String,
        notified: bool,
    },

    /// Check failed
    CheckFailed {
        error: String,
    },
}

/// Status of an update operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStatus {
    /// Binary is up to date
    UpToDate {
        current_version: String,
    },

    /// Update is available
    Available {
        current_version: String,
        latest_version: String,
        info: UpdateInfo,
    },

    /// Update was successful
    Updated {
        old_version: String,
        new_version: String,
    },
}

/// Error types for update operations
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Update check failed: {0}")]
    CheckFailed(String),

    #[error("Update download failed: {0}")]
    DownloadFailed(String),

    #[error("Update installation failed: {0}")]
    InstallationFailed(String),

    #[error("Permission denied: cannot write to installation path")]
    PermissionDenied,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("PGP signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Backup failed: {0}")]
    BackupFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Update timeout after {0:?}")]
    Timeout(Duration),

    #[error("No update available")]
    NoUpdateAvailable,

    #[error("Already up to date: {0}")]
    AlreadyUpToDate(String),

    #[error("User cancelled update")]
    UserCancelled,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Chrono(#[from] chrono::ParseError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

### Public Functions

```rust
/// Check for updates with automatic handling based on configuration
///
/// This function reads the update configuration and performs update checks
/// according to the configured policy. It handles scheduling, notification,
/// and state persistence automatically.
///
/// # Arguments
/// * `binary_name` - Name of the binary to check updates for
/// * `current_version` - Current version of the binary
///
/// # Returns
/// Update status or error
///
/// # Errors
/// Returns `UpdateError::ConfigError` if configuration cannot be loaded
/// Returns `UpdateError::CheckFailed` if update check fails
///
/// # Example
/// ```no_run
/// use terraphim_update::check_for_updates_auto;
///
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let status = check_for_updates_auto("terraphim-agent", env!("CARGO_PKG_VERSION")).await?;
///     println!("{:?}", status);
///     Ok(())
/// }
/// ```
pub async fn check_for_updates_auto(
    binary_name: &str,
    current_version: &str,
) -> Result<UpdateStatus, UpdateError>;

/// Check for updates with manual control
///
/// This function performs an immediate update check regardless of
/// configuration settings. Useful for manual update commands.
///
/// # Arguments
/// * `binary_name` - Name of the binary to check updates for
/// * `current_version` - Current version of the binary
///
/// # Returns
/// Update status or error
///
/// # Example
/// ```no_run
/// use terraphim_update::check_for_updates;
///
/// async fn check_updates() -> Result<(), Box<dyn std::error::Error>> {
///     match check_for_updates("terraphim-agent", "1.0.0").await? {
///         UpdateStatus::Available { latest_version, .. } => {
///             println!("Update available: {}", latest_version);
///         }
///         _ => println!("No update available"),
///     }
///     Ok(())
/// }
/// ```
pub async fn check_for_updates(
    binary_name: &str,
    current_version: &str,
) -> Result<UpdateStatus, UpdateError>;

/// Update binary to latest version
///
/// Downloads and installs the latest version of the binary.
///
/// # Arguments
/// * `binary_name` - Name of the binary to update
/// * `current_version` - Current version of the binary
///
/// # Returns
/// Update status after update attempt
///
/// # Errors
/// Returns `UpdateError::PermissionDenied` if cannot write to installation path
/// Returns `UpdateError::DownloadFailed` if download fails
///
/// # Example
/// ```no_run
/// use terraphim_update::update_binary;
///
/// async fn update() -> Result<(), Box<dyn std::error::Error>> {
///     match update_binary("terraphim-agent", "1.0.0").await? {
///         UpdateStatus::Updated { new_version, .. } => {
///             println!("Updated to {}", new_version);
///         }
///         _ => println!("No update performed"),
///     }
///     Ok(())
/// }
/// ```
pub async fn update_binary(
    binary_name: &str,
    current_version: &str,
) -> Result<UpdateStatus, UpdateError>;

/// Start background update scheduler
///
/// Starts a tokio task that periodically checks for updates
/// according to the configured interval.
///
/// # Arguments
/// * `binary_name` - Name of the binary to check updates for
/// * `current_version` - Current version of the binary
/// * `notification_callback` - Callback function to handle update notifications
///
/// # Returns
/// Handle to the background task (can be used to cancel)
///
/// # Example
/// ```no_run
/// use terraphim_update::start_update_scheduler;
/// use tokio::sync::mpsc;
///
/// async fn start_scheduler() {
///     let (tx, mut rx) = mpsc::channel(10);
///
///     let handle = start_update_scheduler(
///         "terraphim-agent",
///         env!("CARGO_PKG_VERSION"),
///         Box::new(move |info| {
///             let tx = tx.clone();
///             async move {
///                 tx.send(info).await.ok();
///             }
///         })
///     ).await;
///
///     // Handle notifications in main loop
///     while let Some(info) = rx.recv().await {
///         println!("Update available: {}", info.version);
///     }
/// }
/// ```
pub async fn start_update_scheduler(
    binary_name: &'static str,
    current_version: &'static str,
    notification_callback: Box<dyn Fn(UpdateInfo) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
) -> tokio::task::JoinHandle<()>;

/// Check if update should be performed based on configuration
///
/// Determines whether an update check should be performed based on
/// the last check time and configured interval.
///
/// # Arguments
/// * `config` - Update configuration
/// * `history` - Update history
///
/// # Returns
/// true if update check should be performed, false otherwise
///
/// # Example
/// ```no_run
/// use terraphim_update::{UpdateConfig, UpdateHistory, should_check_for_update};
/// use chrono::Utc;
///
/// fn check_if_needed() {
///     let config = UpdateConfig::default();
///     let history = UpdateHistory {
///         last_check: Utc::now() - chrono::Duration::hours(25),
///         current_version: "1.0.0".to_string(),
///         pending_update: None,
///         check_history: vec![],
///     };
///
///     if should_check_for_update(&config, &history) {
///         println!("Time to check for updates");
///     }
/// }
/// ```
pub fn should_check_for_update(config: &UpdateConfig, history: &UpdateHistory) -> bool;

/// Save update history to config file
///
/// Persists the update history to the user's config directory.
///
/// # Arguments
/// * `history` - Update history to save
///
/// # Returns
/// Ok(()) or error
///
/// # Errors
/// Returns `UpdateError::Io` if file cannot be written
///
/// # Example
/// ```no_run
/// use terraphim_update::{UpdateHistory, save_update_history};
///
/// async fn save_history(history: UpdateHistory) -> Result<(), Box<dyn std::error::Error>> {
///     save_update_history(&history).await?;
///     Ok(())
/// }
/// ```
pub async fn save_update_history(history: &UpdateHistory) -> Result<(), UpdateError>;

/// Load update history from config file
///
/// Loads the update history from the user's config directory.
///
/// # Returns
/// Update history or error
///
/// # Errors
/// Returns `UpdateError::Io` if file cannot be read
/// Returns `UpdateError::Json` if file is corrupted
///
/// # Example
/// ```no_run
/// use terraphim_update::load_update_history;
///
/// async fn load_history() -> Result<UpdateHistory, Box<dyn std::error::Error>> {
///     let history = load_update_history().await?;
///     Ok(history)
/// }
/// ```
pub async fn load_update_history() -> Result<UpdateHistory, UpdateError>;

/// Get notification message for update
///
/// Returns a user-friendly message about an available update.
///
/// # Arguments
/// * `info` - Update information
///
/// # Returns
/// Formatted notification message
///
/// # Example
/// ```no_run
/// use terraphim_update::{UpdateInfo, get_update_notification};
///
/// fn show_notification(info: UpdateInfo) {
///     let message = get_update_notification(&info);
///     println!("{}", message);
/// }
/// ```
pub fn get_update_notification(info: &UpdateInfo) -> String;

/// Get update configuration from device settings
///
/// Loads the update configuration from device settings.
///
/// # Returns
/// Update configuration or default if not configured
///
/// # Example
/// ```no_run
/// use terraphim_update::get_update_config;
///
/// async fn get_config() {
///     let config = get_update_config().await;
///     println!("Update enabled: {}", config.auto_update_enabled);
/// }
/// ```
pub async fn get_update_config() -> UpdateConfig;

/// Rollback to a previous version
///
/// Restores a previously backed up binary version.
///
/// # Arguments
/// * `binary_name` - Name of the binary to rollback
/// * `target_version` - Version to rollback to
///
/// # Returns
/// Update status after rollback attempt
///
/// # Errors
/// Returns `UpdateError::RollbackFailed` if backup not found
///
/// # Example
/// ```no_run
/// use terraphim_update::rollback;
///
/// async fn rollback_version() -> Result<(), Box<dyn std::error::Error>> {
///     match rollback("terraphim-agent", "1.0.0").await? {
///         UpdateStatus::Updated { new_version, .. } => {
///             println!("Rolled back to {}", new_version);
///         }
///         _ => println!("Rollback failed"),
///     }
///     Ok(())
/// }
/// ```
pub async fn rollback(
    binary_name: &str,
    target_version: &str,
) -> Result<UpdateStatus, UpdateError>;

/// Verify PGP signature of downloaded binary
///
/// Verifies that the downloaded binary is signed with the project's official key.
///
/// # Arguments
/// * `binary_path` - Path to the downloaded binary
/// * `signature_path` - Path to the PGP signature file
///
/// # Returns
/// true if signature is valid, false otherwise
///
/// # Errors
/// Returns `UpdateError::SignatureVerificationFailed` if verification fails
///
/// # Example
/// ```no_run
/// use terraphim_update::verify_pgp_signature;
///
/// async fn verify_update() -> Result<(), Box<dyn std::error::Error>> {
///     let valid = verify_pgp_signature(
///         "/tmp/terraphim-agent-new",
///         "/tmp/terraphim-agent.sig"
///     ).await?;
///
///     if valid {
///         println!("Signature verified");
///     } else {
///         println!("Signature invalid!");
///     }
///     Ok(())
/// }
/// ```
pub async fn verify_pgp_signature(
    binary_path: &Path,
    signature_path: &Path,
) -> Result<bool, UpdateError>;

/// Prompt user for update confirmation
///
/// Displays an interactive prompt asking the user to confirm update installation.
///
/// # Arguments
/// * `update_info` - Information about the available update
///
/// # Returns
/// true if user confirms, false otherwise
///
/// # Example
/// ```no_run
/// use terraphim_update::prompt_user_for_update;
///
/// async fn ask_user() -> bool {
///     let info = UpdateInfo { ... };
///     prompt_user_for_update(&info).await
/// }
/// ```
pub async fn prompt_user_for_update(update_info: &UpdateInfo) -> bool;

/// Backup current binary before update
///
/// Creates a backup of the current binary with version suffix.
///
/// # Arguments
/// * `binary_path` - Path to the current binary
/// * `version` - Current version to use in backup filename
///
/// # Returns
/// Path to the backup file
///
/// # Example
/// ```no_run
/// use terraphim_update::backup_binary;
///
/// async fn backup() -> Result<(), Box<dyn std::error::Error>> {
///     let backup_path = backup_binary(
///         "/usr/local/bin/terraphim-agent",
///         "1.0.0"
///     ).await?;
///     println!("Backup created at: {:?}", backup_path);
///     Ok(())
/// }
/// ```
pub async fn backup_binary(
    binary_path: &Path,
    version: &str,
) -> Result<PathBuf, UpdateError>;
```

### Error Types

See `UpdateError` enum in Public Types section above.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_update_config_default` | `config.rs` | Verify default configuration values |
| `test_update_config_deserialize` | `config.rs` | Verify config deserialization |
| `test_should_check_for_update_true` | `scheduler.rs` | Verify check needed when interval elapsed |
| `test_should_check_for_update_false` | `scheduler.rs` | Verify no check when interval not elapsed |
| `test_should_check_for_update_disabled` | `scheduler.rs` | Verify no check when disabled |
| `test_update_info_serialization` | `config.rs` | Verify UpdateInfo serialization |
| `test_update_history_add_entry` | `state.rs` | Verify history entry addition |
| `test_update_history_limit_entries` | `state.rs` | Verify history entry limit (10) |
| `test_get_update_notification_format` | `notification.rs` | Verify notification message format |
| `test_permission_denied_error` | `lib.rs` | Verify permission denied error handling |
| `test_pgp_signature_verification_valid` | `verification.rs` | Verify valid PGP signature passes |
| `test_pgp_signature_verification_invalid` | `verification.rs` | Verify invalid PGP signature fails |
| `test_backup_binary_creates_file` | `rollback.rs` | Verify backup creates versioned file |
| `test_rollback_restores_backup` | `rollback.rs` | Verify rollback restores from backup |
| `test_rollback_fails_no_backup` | `rollback.rs` | Verify rollback fails when no backup exists |
| `test_prompt_user_accepts` | `prompt.rs` | Verify prompt returns true on 'y' |
| `test_prompt_user_declines` | `prompt.rs` | Verify prompt returns false on 'n' |
| `test_prompt_user_defaults_no` | `prompt.rs` | Verify prompt returns false on empty input |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_check_for_updates_auto_enabled` | `integration_test.rs` | Full flow with auto-update enabled |
| `test_check_for_updates_auto_disabled` | `integration_test.rs` | Verify no check when disabled |
| `test_check_for_updates_manual` | `integration_test.rs` | Manual update check command flow |
| `test_update_binary_success_with_pgp` | `integration_test.rs` | Successful binary update with PGP verification |
| `test_update_binary_pgp_fail` | `integration_test.rs` | Update fails with bad PGP signature |
| `test_update_binary_permission_denied` | `integration_test.rs` | Update falls back to manual instructions |
| `test_update_with_user_prompt_accept` | `integration_test.rs` | Update proceeds when user accepts |
| `test_update_with_user_prompt_decline` | `integration_test.rs` | Update skipped when user declines |
| `test_background_scheduler_start_stop` | `integration_test.rs` | Scheduler lifecycle |
| `test_background_scheduler_notification` | `integration_test.rs` | Scheduler sends notifications |
| `test_rollback_to_previous_version` | `integration_test.rs` | Successful rollback to backup |
| `test_rollback_creates_backup_of_current` | `integration_test.rs` | Rollback backs up current version |
| `test_rollback_multiple_versions` | `integration_test.rs` | Rollback to any available backup version |
| `test_config_persistence` | `integration_test.rs` | Config saved and loaded correctly |
| `test_history_persistence` | `integration_test.rs` | History saved and loaded correctly |

### Property Tests

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    use super::*;

    proptest! {
        #[test]
        fn update_config_roundtrip(config in any::<UpdateConfig>()) {
            let serialized = serde_json::to_string(&config).unwrap();
            let deserialized: UpdateConfig = serde_json::from_str(&serialized).unwrap();
            prop_assert_eq!(config, deserialized);
        }

        #[test]
        fn update_history_never_panics(history in any::<UpdateHistory>()) {
            // Should never panic when accessing fields
            let _ = history.last_check;
            let _ = history.current_version;
            let _ = &history.check_history;
        }

        #[test]
        fn should_check_never_panics(
            config in any::<UpdateConfig>(),
            history in any::<UpdateHistory>()
        ) {
            // Should never panic regardless of inputs
            let _ = should_check_for_update(&config, &history);
        }

        #[test]
        fn check_interval_positive(interval_secs in 1u64..86400u64) {
            let config = UpdateConfig {
                check_interval: Duration::from_secs(interval_secs),
                ..Default::default()
            };
            prop_assert!(config.check_interval.as_secs() > 0);
        }

        #[test]
        fn notification_format_never_panics(info in any::<UpdateInfo>()) {
            // Should never panic when formatting notification
            let _ = get_update_notification(&info);
        }
    }
}
```

### Security Verification Tests

```rust
#[cfg(test)]
mod security_tests {
    use super::*;

    #[tokio::test]
    async fn test_pgp_signature_valid() {
        let binary_path = PathBuf::from("fixtures/valid-binary");
        let signature_path = PathBuf::from("fixtures/valid-binary.sig");
        let result = verify_pgp_signature(&binary_path, &signature_path).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_pgp_signature_invalid() {
        let binary_path = PathBuf::from("fixtures/valid-binary");
        let signature_path = PathBuf::from("fixtures/invalid-binary.sig");
        let result = verify_pgp_signature(&binary_path, &signature_path).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_pgp_signature_tampered_binary() {
        let binary_path = PathBuf::from("fixtures/tampered-binary");
        let signature_path = PathBuf::from("fixtures/valid-binary.sig");
        let result = verify_pgp_signature(&binary_path, &signature_path).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_update_rejects_unsigned_binary() {
        let update_info = UpdateInfo {
            version: "2.0.0".to_string(),
            download_url: "http://example.com/binary".to_string(),
            signature_url: "".to_string(), // No signature
            ..Default::default()
        };

        let result = update_binary_with_verification("terraphim-agent", "1.0.0", &update_info).await;
        assert!(matches!(result, Err(UpdateError::SignatureVerificationFailed(_))));
    }

    #[tokio::test]
    async fn test_binary_path_validation() {
        let malicious_path = PathBuf::from("/etc/passwd");
        let result = validate_binary_path(&malicious_path);
        assert!(result.is_err());

        let valid_path = PathBuf::from("/usr/local/bin/terraphim-agent");
        let result = validate_binary_path(&valid_path);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_binary_permissions_read_only() {
        let binary_path = PathBuf::from("fixtures/read-only-binary");
        let result = update_binary("terraphim-agent", "1.0.0").await;
        assert!(matches!(result, Err(UpdateError::PermissionDenied)));
    }
}
```

## Implementation Steps

### Step 1: Create Configuration Types
**Files:** `crates/terraphim_update/src/config.rs`
**Description:** Define UpdateConfig (auto_update_enabled, auto_update_check_interval), UpdateInfo (add signature_url), UpdateHistory (add backup_versions), UpdateCheckEntry, UpdateCheckResult
**Tests:** Unit tests for type construction, serialization, deserialization
**Dependencies:** None
**Estimated:** 3 hours

```rust
// Key code to write
pub struct UpdateConfig { auto_update_enabled: bool, auto_update_check_interval: Duration }
pub struct UpdateInfo { signature_url: String, ... }
pub struct UpdateHistory { backup_versions: Vec<String>, ... }
```

### Step 2: Create Error Types
**Files:** `crates/terraphim_update/src/lib.rs` (add UpdateError enum)
**Description:** Define comprehensive error types including PGP verification errors, rollback errors, backup errors
**Tests:** Unit tests for error creation and display
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to write
#[derive(Debug, thiserror::Error)]
pub enum UpdateError { SignatureVerificationFailed, RollbackFailed, BackupFailed, UserCancelled, ... }
```

### Step 3: Implement State Management
**Files:** `crates/terraphim_update/src/state.rs`
**Description:** Implement save_update_history and load_update_history functions, handle backup_versions tracking
**Tests:** Unit tests for save/load, error handling, file I/O
**Dependencies:** Step 1, Step 2
**Estimated:** 3 hours

```rust
// Key code to write
pub async fn save_update_history(history: &UpdateHistory) -> Result<(), UpdateError>
pub async fn load_update_history() -> Result<UpdateHistory, UpdateError>
```

### Step 4: Implement Scheduling Logic
**Files:** `crates/terraphim_update/src/scheduler.rs`
**Description:** Implement should_check_for_update and start_update_scheduler
**Tests:** Unit tests for scheduling logic, interval calculation
**Dependencies:** Step 1
**Estimated:** 4 hours

```rust
// Key code to write
pub fn should_check_for_update(config: &UpdateConfig, history: &UpdateHistory) -> bool
pub async fn start_update_scheduler(...) -> tokio::task::JoinHandle<()>
```

### Step 5: Implement Notification System
**Files:** `crates/terraphim_update/src/notification.rs`
**Description:** Implement get_update_notification function
**Tests:** Unit tests for message formatting, edge cases
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to write
pub fn get_update_notification(info: &UpdateInfo) -> String
```

### Step 6: Implement PGP Verification
**Files:** `crates/terraphim_update/src/verification.rs`, `crates/terraphim_update/Cargo.toml`
**Description:** Implement verify_pgp_signature function using pgp crate
**Tests:** Unit tests for valid/invalid signatures, tampered binaries
**Dependencies:** None
**Estimated:** 5 hours

```rust
// Key code to write
pub async fn verify_pgp_signature(binary_path: &Path, signature_path: &Path) -> Result<bool, UpdateError>
```

### Step 7: Implement Rollback System
**Files:** `crates/terraphim_update/src/rollback.rs`
**Description:** Implement backup_binary and rollback functions, track backup_versions in UpdateHistory
**Tests:** Unit tests for backup creation, restore, error cases
**Dependencies:** Step 2
**Estimated:** 5 hours

```rust
// Key code to write
pub async fn backup_binary(binary_path: &Path, version: &str) -> Result<PathBuf, UpdateError>
pub async fn rollback(binary_name: &str, target_version: &str) -> Result<UpdateStatus, UpdateError>
```

### Step 8: Implement Interactive Prompts
**Files:** `crates/terraphim_update/src/prompt.rs`
**Description:** Implement prompt_user_for_update function, handle user input
**Tests:** Unit tests for accept/decline scenarios, default behavior
**Dependencies:** None
**Estimated:** 3 hours

```rust
// Key code to write
pub async fn prompt_user_for_update(update_info: &UpdateInfo) -> bool
```

### Step 9: Integrate with terraphim_settings
**Files:** `crates/terraphim_settings/src/lib.rs`, `crates/terraphim_settings/Cargo.toml`
**Description:** Add UpdateConfig to DeviceSettings struct
**Tests:** Unit tests for config loading, serialization
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to change
pub struct DeviceSettings {
    pub update_config: UpdateConfig,
    // ... existing fields
}
```

### Step 10: Integrate with terraphim_config
**Files:** `crates/terraphim_config/src/lib.rs`, `crates/terraphim_config/Cargo.toml`
**Description:** Add UpdateHistory to Config struct
**Tests:** Unit tests for history loading, serialization
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to change
pub struct Config {
    pub update_history: UpdateHistory,
    // ... existing fields
}
```

### Step 11: Extend terraphim_update crate API
**Files:** `crates/terraphim_update/src/lib.rs`, `crates/terraphim_update/Cargo.toml`
**Description:** Export new modules, implement check_for_updates_auto, get_update_config, rollback, verify_pgp_signature, prompt_user_for_update, backup_binary
**Tests:** Integration tests for API functions
**Dependencies:** Step 1-8
**Estimated:** 5 hours

```rust
// Key code to add
pub mod config;
pub mod scheduler;
pub mod notification;
pub mod state;
pub mod verification;
pub mod rollback;
pub mod prompt;

pub async fn check_for_updates_auto(...) -> Result<UpdateStatus, UpdateError>
pub async fn get_update_config() -> UpdateConfig
pub async fn rollback(...) -> Result<UpdateStatus, UpdateError>
pub async fn verify_pgp_signature(...) -> Result<bool, UpdateError>
pub async fn prompt_user_for_update(...) -> bool
pub async fn backup_binary(...) -> Result<PathBuf, UpdateError>
```

### Step 12: Add Update Commands to terraphim_cli
**Files:** `crates/terraphim_cli/src/main.rs`, `crates/terraphim_cli/Cargo.toml`
**Description:** Add CheckUpdate, Update, and Rollback commands, implement startup check, add interactive prompts, PGP verification, binary backup
**Tests:** Integration tests for command flow
**Dependencies:** Step 11
**Estimated:** 6 hours

```rust
// Key code to add
enum Command {
    CheckUpdate,
    Update,
    Rollback { version: String },
    // ... existing commands
}

async fn handle_check_update() -> Result<(), Box<dyn std::error::Error>>
async fn handle_update() -> Result<(), Box<dyn std::error::Error>>
async fn handle_rollback(version: String) -> Result<(), Box<dyn std::error::Error>>
async fn perform_startup_check() -> Result<(), Box<dyn std::error::Error>>
```

### Step 13: Integrate Startup Check in terraphim_agent
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add startup check, integrate with notification system, add interactive prompts
**Tests:** Integration tests for startup flow
**Dependencies:** Step 11
**Estimated:** 4 hours

```rust
// Key code to add
async fn perform_startup_check() -> Result<(), Box<dyn std::error::Error>>
```

### Step 14: Add Background Scheduler to terraphim_agent
**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Start background update scheduler with notification callback, ensure non-interruptive behavior
**Tests:** Integration tests for background scheduler
**Dependencies:** Step 4, Step 11
**Estimated:** 4 hours

```rust
// Key code to add
let scheduler_handle = start_update_scheduler(
    "terraphim-agent",
    env!("CARGO_PKG_VERSION"),
    Box::new(|info| {
        // Notification callback - silent, queue notification for next render
    })
).await;
```

### Step 15: Implement Platform-Specific Logic (Linux/macOS)
**Files:** `crates/terraphim_update/src/platform.rs` (new file)
**Description:** Handle Linux/macOS binary paths, permissions, manual update instructions fallback
**Tests:** Unit tests for platform detection, path resolution
**Dependencies:** Step 11
**Estimated:** 4 hours

```rust
// Key code to write
pub fn get_binary_path(binary_name: &str) -> PathBuf
pub fn check_write_permissions(path: &Path) -> Result<(), UpdateError>
pub fn show_manual_update_instructions(update_info: &UpdateInfo) -> String
```

### Step 16: Add Comprehensive Integration Tests
**Files:** `crates/terraphim_update/tests/integration_test.rs`
**Description:** Add integration tests for all flows including PGP verification, rollback, prompts, permission handling
**Tests:** All integration tests
**Dependencies:** All previous steps
**Estimated:** 6 hours

```rust
// Key tests to add
#[tokio::test]
async fn test_check_for_updates_auto_enabled() { ... }
#[tokio::test]
async fn test_update_binary_success_with_pgp() { ... }
#[tokio::test]
async fn test_rollback_to_previous_version() { ... }
#[tokio::test]
async fn test_update_with_user_prompt_accept() { ... }
```

### Step 17: Add Security Verification Tests
**Files:** `crates/terraphim_update/tests/security_test.rs`
**Description:** Add security tests for PGP signature verification, binary validation, permission handling
**Tests:** All security tests
**Dependencies:** Step 6, Step 7, Step 15
**Estimated:** 5 hours

```rust
// Key tests to add
#[tokio::test]
async fn test_pgp_signature_valid() { ... }
#[tokio::test]
async fn test_pgp_signature_tampered_binary() { ... }
#[tokio::test]
async fn test_binary_path_validation() { ... }
```

### Step 18: Documentation and Examples
**Files:** Inline documentation, README updates
**Description:** Add comprehensive doc comments, usage examples, update policies
**Tests:** Doc tests
**Dependencies:** All previous steps
**Estimated:** 3 hours

```rust
// Add doc comments to all public APIs
/// Check for updates with automatic handling based on configuration
/// ... (detailed documentation)
```

### Step 19: Cross-Platform Testing Script (Linux/macOS)
**Files:** `scripts/test-self-update.sh`
**Description:** Create test script for validating self-update on Linux and macOS
**Tests:** Manual testing on Linux and macOS
**Dependencies:** All previous steps
**Estimated:** 4 hours

```bash
#!/bin/bash
# Test self-update on current platform (Linux/macOS)
./scripts/test-self-update.sh
```

### Step 20: Final Integration and Polish
**Files:** All modified files
**Description:** Code review, lint fixes, error handling improvements, ensure silent network failures, non-interruptive behavior
**Tests:** All tests pass, clippy clean
**Dependencies:** All previous steps
**Estimated:** 4 hours

```bash
cargo clippy --workspace
cargo test --workspace
```

### Step 21: Create GitHub Issue for Phase 2 Features
**Files:** GitHub issue
**Description:** Document and track Phase 2 features: desktop notifications, system schedulers, telemetry, multiple release channels, Windows support
**Tests:** N/A
**Dependencies:** All previous steps
**Estimated:** 1 hour

```bash
gh issue create --title "Phase 2: Advanced Auto-Update Features" --body "$(cat <<'EOF'
## Features to Implement
- Desktop notifications (OS-level)
- System schedulers (systemd, launchd)
- Update telemetry (with consent)
- Multiple release channels (stable, beta, nightly)
- Windows platform support
EOF
)"
```

## Rollback Plan

### Built-in Rollback Support

The system includes comprehensive rollback support that allows users to revert to any previously installed version:

1. **Command-line rollback:**
    ```bash
    # Rollback to specific version
    terraphim-cli rollback 1.2.3

    # View available backup versions
    terraphim-cli check-update --list-backups
    ```

2. **Automatic binary backups:**
    - Before each update, current binary is backed up with version suffix: `binary.vX.Y.Z`
    - Backups tracked in UpdateHistory.backup_versions
    - Backups retained indefinitely (user can manually delete)
    - Located in same directory as binary

3. **Rollback process:**
    - Backs up current version before restoring
    - Restores target backup as active binary
    - Updates last_version in config
    - Success/failure messages to user

### If Issues Discovered During Rollout

1. **Disable auto-update via configuration:**
    ```bash
    # In device settings file
    update_config.auto_update_enabled = false
    ```

2. **Graceful degradation:**
    - If auto-update fails, log error but continue normal operation
    - Silent failure on network errors (no user interruption)
    - Existing manual update commands remain functional
    - Users can still update manually via `terraphim-agent update`

3. **Binary rollback via built-in system:**
    - Automatic backups created on every update
    - Users can rollback via `terraphim-cli rollback <version>`
    - If update fails, delete partial download, retry on next check
    - PGP verification prevents installing corrupted/malicious binaries

4. **Configuration rollback:**
    - Previous configuration files are backed up before modification
    - Automatic rollback to previous version if new config fails to load

5. **Feature flag:**
    - Can disable entire feature by commenting out startup check code
    - No code deployment needed for disable

6. **Data cleanup:**
    - Update history files are non-essential, can be deleted without impact
    - No database migrations to roll back
    - Backup binaries can be manually deleted if no longer needed

7. **Emergency stop:**
    - Kill background scheduler task if it causes issues
    - No persistent services to stop
    - Updates never interrupt user sessions

## Migration

### Configuration Changes

**No database migrations** - all state is in configuration files.

**Configuration file updates:**

1. **DeviceSettings file** (e.g., `~/.config/terraphim/device-settings.json`):
    ```json
    {
      "update_config": {
        "auto_update_enabled": true,
        "auto_update_check_interval": 86400
      },
      // ... existing fields
    }
    ```

2. **User config file** (e.g., `~/.config/terraphim/config.json`):
    ```json
    {
      "update_history": {
        "last_check": "2025-01-09T12:00:00Z",
        "current_version": "1.0.0",
        "pending_update": null,
        "backup_versions": [],
        "check_history": []
      },
      // ... existing fields
    }
    ```

### Migration Strategy

**Backward compatibility:**
- Old configuration files will be automatically upgraded with default values
- Missing fields will be populated with defaults from `UpdateConfig::default()`
- No manual migration required

**Configuration upgrade logic:**
```rust
// In load_update_history()
fn ensure_update_history_exists(config: &mut Config) {
    if config.update_history.is_empty() {
        config.update_history = UpdateHistory {
            last_check: Utc::now(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            pending_update: None,
            check_history: vec![],
        };
    }
}
```

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `tokio` | 1.35+ (already in workspace) | Async runtime for scheduler |
| `chrono` | 0.4+ (already in workspace) | DateTime handling for timestamps |
| `serde` | 1.0+ (already in workspace) | Serialization of config/history |
| `proptest` | 1.4+ (dev dependency) | Property-based testing |
| `pgp` | 0.13+ | PGP signature verification for update security |
| `base64` | 0.22+ | Base64 encoding for PGP signatures |
| `notify-rust` | 4.10+ (optional, future) | Desktop notifications (Phase 2) |

### Dependency Updates

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `terraphim_settings` | current | current | Add UpdateConfig dependency |
| `terraphim_config` | current | current | Add UpdateHistory dependency |
| `self_update` | 0.42 | 0.42 | No change, existing dependency |

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Update check latency | < 2 seconds | Benchmark with real GitHub API |
| Binary download time | < 30 seconds (10MB binary) | Benchmark on typical connection |
| Config file load/save | < 10ms | Benchmark I/O operations |
| Background scheduler CPU | < 1% when idle | Profile background task |
| Memory overhead | < 50MB | Profiling with valgrind |
| Startup overhead (check) | < 100ms | Benchmark startup time |

### Benchmarks to Add

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn bench_check_for_updates_latency() {
        let start = Instant::now();
        let _ = check_for_updates("terraphim-agent", "1.0.0").await;
        let duration = start.elapsed();
        println!("Update check latency: {:?}", duration);
        assert!(duration.as_secs() < 2);
    }

    #[tokio::test]
    async fn bench_config_save_load() {
        let history = UpdateHistory::default();

        let start = Instant::now();
        save_update_history(&history).await.unwrap();
        let save_duration = start.elapsed();

        let start = Instant::now();
        load_update_history().await.unwrap();
        let load_duration = start.elapsed();

        println!("Config save: {:?}, load: {:?}", save_duration, load_duration);
        assert!(save_duration.as_millis() < 10);
        assert!(load_duration.as_millis() < 10);
    }

    #[tokio::test]
    async fn bench_scheduler_interval_accuracy() {
        let config = UpdateConfig {
            check_interval: Duration::from_secs(1),
            ..Default::default()
        };

        let mut checks = 0;
        let start = Instant::now();

        let _ = start_update_scheduler("terraphim-agent", "1.0.0", Box::new(move |_| {
            checks += 1;
            Box::pin(async {})
        })).await;

        tokio::time::sleep(Duration::from_secs(5)).await;
        let duration = start.elapsed();

        println!("{} checks in {:?}", checks, duration);
        // Should be approximately 5 checks (one per second)
    }
}
```

### Performance Optimization Strategies

1. **Lazy loading:** Only load update history when needed (e.g., on startup or before check)
2. **Debouncing:** Avoid multiple rapid update checks
3. **Caching:** Cache GitHub API responses (use ETag headers)
4. **Async I/O:** All file operations are async to avoid blocking
5. **Background tasks:** Update checks run in background tokio tasks, don't block UI
6. **Rate limiting:** Respect GitHub API limits (60 requests/hour for unauthenticated)

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Define default update check frequency | Resolved: Daily | - |
| Approve opt-in vs. opt-out policy | Resolved: Opt-out (enabled by default) | - |
| Approve auto-install policy | Resolved: Auto-install enabled with prompts | - |
| Test on real Linux/macOS systems | Pending | QA |
| Security review of PGP verification | Pending | Security team |
| Write user documentation for auto-update feature | Pending | Technical writer |
| Generate and publish PGP key for signing releases | Pending | Release manager |

## Phase 2: Advanced Features (Tracking in GitHub Issue)

The following features are deferred to Phase 2 and will be tracked in a separate GitHub issue created during Step 21:

1. **Desktop notifications (OS-level)**
   - Linux: libnotify integration
   - macOS: NotificationCenter integration
   - Windows: Toast notifications
   - Requires: notify-rust crate evaluation and testing

2. **System schedulers**
   - Linux: systemd user units
   - macOS: launchd agents
   - Windows: Task Scheduler
   - Enables background checks even when app not running

3. **Update telemetry (with consent)**
   - Track update success/failure rates
   - Monitor update adoption
   - Identify problematic releases
   - Requires: Privacy policy review and user consent

4. **Multiple release channels**
   - Stable: Default channel, tested releases
   - Beta: Pre-release testing
   - Nightly: Latest development builds
   - Allows users to choose update channel

5. **Windows platform support**
   - Requires spike on self_update crate Windows support
   - UAC prompt handling
   - Windows-specific paths and permissions
   - Windows Toast notifications

**Note:** All Phase 2 features will be prioritized and scheduled based on user feedback and business needs after Phase 1 completion and user deployment.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Open items resolved or deferred
- [ ] Stakeholder approval received
- [ ] Ready to proceed to Phase 3 (Implementation)

## Appendix

### Platform-Specific Considerations

**Linux:**
- Binary installation paths: `/usr/local/bin` (system), `~/.local/bin` (user)
- Permissions: System installs require sudo, user installs do not
- When no write permissions: Display manual update instructions (download URL, install instructions)
- Desktop notifications: libnotify (Phase 2)
- Config location: `~/.config/terraphim/`
- Backup location: Same directory as binary (e.g., `/usr/local/bin/terraphim-agent.v1.0.0`)

**macOS:**
- Binary installation paths: `/usr/local/bin` (Homebrew), `/usr/local/bin` (manual)
- Permissions: Similar to Linux
- When no write permissions: Display manual update instructions
- Desktop notifications: NotificationCenter (Phase 2)
- Config location: `~/Library/Application Support/terraphim/`
- Backup location: Same directory as binary

**Windows (Phase 2):**
- Binary installation paths: `C:\Program Files\terraphim\` (system), `%USERPROFILE%\.local\bin` (user)
- Permissions: UAC prompts for system installs
- Desktop notifications: Windows Toast notifications (Phase 2)
- Config location: `%APPDATA%\terraphim\`
- **Unknown:** self_update crate Windows support - requires spike

### Security Considerations

**MVP security (Phase 1):**
- Verify PGP signatures of downloaded binaries
- Validate binary path before replacement
- Prevent directory traversal attacks
- Log all update attempts
- HTTPS-only downloads enforced
- Pinning GitHub repository owner/name
- Delete partial downloads on failure
- Silent failure on network errors (no credential leaks)
- Never interrupt user sessions (prevents session hijacking)

**Enhanced security (Phase 2 - out of scope):**
- Hardened PGP key distribution
- Multi-signature verification
- Rate limiting update checks
- Update telemetry with consent

### User Experience Considerations

**MVP UX (Phase 1):**
- Check-on-startup: Show notification if update available (silent check, only notify)
- Daily check frequency: Maximum once per day, enabled by default with opt-out
- In-app notification: Clear message with version info and release notes
- Interactive prompt: Ask user before installing update ("Update to vX.Y.Z? (y/N)")
- Auto-install: Proceed with automatic download and installation after user confirmation
- PGP verification: Automatic, transparent to user, failed verification aborts update
- Binary backup: Automatic, transparent to user, enables rollback
- Rollback support: Easy command-line rollback to any previous version
- No write permissions: Display clear manual update instructions
- No interruption: Updates never interrupt active sessions
- Silent network failures: Network errors don't show popups or interrupt workflow
- Clear error messages: Explain what went wrong and how to fix
- Graceful degradation: System continues normal operation even if updates fail

**Enhanced UX (Phase 2 - out of scope):**
- Desktop notifications: OS-level notifications
- System schedulers: Background checks even when app not running
- Update progress bar: Show download/install progress
- One-click update: Simplify update command
- Update telemetry (with consent): Track success rates
- Multiple release channels: Choose stable, beta, or nightly

### Testing Strategy Summary

**Unit tests:**
- Type construction and serialization
- Scheduling logic (should_check_for_update)
- State management (save/load)
- Notification formatting
- Error handling
- PGP signature verification
- Binary backup and restore
- Interactive prompts
- Platform-specific path resolution

**Integration tests:**
- Full update flow (check → prompt → backup → download → verify → install)
- Configuration persistence
- Background scheduler lifecycle
- Error recovery scenarios
- Cross-platform behavior (Linux, macOS)
- Rollback functionality
- Permission handling and fallback to manual instructions
- User prompt acceptance/decline scenarios

**Security verification tests:**
- Valid PGP signature verification
- Invalid PGP signature rejection
- Tampered binary detection
- Binary path validation
- Permission checks
- Malicious input handling

**Property tests:**
- Config serialization roundtrip
- History state invariants
- Notification formatting robustness
- Backup version tracking

**Manual testing:**
- Real GitHub API interaction
- Actual binary replacement
- Permission handling on Linux/macOS
- Network failure scenarios (silent failure verification)
- User interruption prevention verification
- Rollback to previous versions

**CI/CD:**
- Run all tests on every commit
- Mock GitHub API for CI tests
- Test on Linux and macOS
- Lint with clippy
- Format with cargo fmt
