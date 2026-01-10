//! Shared auto-update functionality for Terraphim AI binaries
//!
//! This crate provides a unified interface for self-updating Terraphim AI CLI tools
//! using GitHub Releases as a distribution channel.

pub mod config;
pub mod downloader;
pub mod notification;
pub mod platform;
pub mod rollback;
pub mod scheduler;
pub mod signature;
pub mod state;

use anyhow::Result;
use self_update::cargo_crate_version;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// Represents the status of an update operation
#[derive(Debug, Clone)]
pub enum UpdateStatus {
    /// No update available - already running latest version
    UpToDate(String),
    /// Update available and successfully installed
    Updated {
        from_version: String,
        to_version: String,
    },
    /// Update available but not installed
    Available {
        current_version: String,
        latest_version: String,
    },
    /// Update failed with error
    Failed(String),
}

/// Compare two version strings to determine if the first is newer than the second
/// Static version that can be called from blocking contexts
fn is_newer_version_static(version1: &str, version2: &str) -> bool {
    // Simple version comparison - in production you might want to use semver crate
    let v1_parts: Vec<u32> = version1
        .trim_start_matches('v')
        .split('.')
        .take(3)
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    let v2_parts: Vec<u32> = version2
        .trim_start_matches('v')
        .split('.')
        .take(3)
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    // Pad with zeros if needed
    let v1 = [
        v1_parts.first().copied().unwrap_or(0),
        v1_parts.get(1).copied().unwrap_or(0),
        v1_parts.get(2).copied().unwrap_or(0),
    ];

    let v2 = [
        v2_parts.first().copied().unwrap_or(0),
        v2_parts.get(1).copied().unwrap_or(0),
        v2_parts.get(2).copied().unwrap_or(0),
    ];

    v1 > v2
}

impl fmt::Display for UpdateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateStatus::UpToDate(version) => {
                write!(f, "✅ Already running latest version: {}", version)
            }
            UpdateStatus::Updated {
                from_version,
                to_version,
            } => {
                write!(f, "🚀 Updated from {} to {}", from_version, to_version)
            }
            UpdateStatus::Available {
                current_version,
                latest_version,
            } => {
                write!(
                    f,
                    "📦 Update available: {} → {}",
                    current_version, latest_version
                )
            }
            UpdateStatus::Failed(error) => {
                write!(f, "❌ Update failed: {}", error)
            }
        }
    }
}

/// Configuration for the updater
#[derive(Debug, Clone)]
pub struct UpdaterConfig {
    /// Name of the binary (e.g., "terraphim_server")
    pub bin_name: String,
    /// GitHub repository owner (e.g., "terraphim")
    pub repo_owner: String,
    /// GitHub repository name (e.g., "terraphim-ai")
    pub repo_name: String,
    /// Current version of the binary
    pub current_version: String,
    /// Whether to show download progress
    pub show_progress: bool,
}

impl UpdaterConfig {
    /// Create a new updater config for Terraphim AI binaries
    pub fn new(bin_name: impl Into<String>) -> Self {
        Self {
            bin_name: bin_name.into(),
            repo_owner: "terraphim".to_string(),
            repo_name: "terraphim-ai".to_string(),
            current_version: cargo_crate_version!().to_string(),
            show_progress: true,
        }
    }

    /// Set a custom current version (useful for testing)
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.current_version = version.into();
        self
    }

    /// Enable or disable progress display
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }
}

/// Updater client for Terraphim AI binaries
pub struct TerraphimUpdater {
    config: UpdaterConfig,
}

impl TerraphimUpdater {
    /// Create a new updater instance
    pub fn new(config: UpdaterConfig) -> Self {
        Self { config }
    }

    /// Check if an update is available without installing
    pub async fn check_update(&self) -> Result<UpdateStatus> {
        info!(
            "Checking for updates: {} v{}",
            self.config.bin_name, self.config.current_version
        );

        // Clone data for the blocking task
        let repo_owner = self.config.repo_owner.clone();
        let repo_name = self.config.repo_name.clone();
        let bin_name = self.config.bin_name.clone();
        let current_version = self.config.current_version.clone();
        let show_progress = self.config.show_progress;

        // Move self_update operations to a blocking task to avoid runtime conflicts
        let result = tokio::task::spawn_blocking(move || {
            // Check if update is available
            match self_update::backends::github::Update::configure()
                .repo_owner(&repo_owner)
                .repo_name(&repo_name)
                .bin_name(&bin_name)
                .current_version(&current_version)
                .show_download_progress(show_progress)
                .build()
            {
                Ok(updater) => {
                    // This will check without updating
                    match updater.get_latest_release() {
                        Ok(release) => {
                            let latest_version = release.version.clone();

                            // Simple version comparison
                            if is_newer_version_static(&latest_version, &current_version) {
                                Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::Available {
                                    current_version,
                                    latest_version,
                                })
                            } else {
                                Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::UpToDate(
                                    current_version,
                                ))
                            }
                        }
                        Err(e) => Ok(UpdateStatus::Failed(format!("Check failed: {}", e))),
                    }
                }
                Err(e) => Ok(UpdateStatus::Failed(format!("Configuration error: {}", e))),
            }
        })
        .await;

        match result {
            Ok(update_result) => {
                match update_result {
                    Ok(status) => {
                        // Log the result for debugging
                        match &status {
                            UpdateStatus::Available {
                                current_version,
                                latest_version,
                            } => {
                                info!(
                                    "Update available: {} -> {}",
                                    current_version, latest_version
                                );
                            }
                            UpdateStatus::UpToDate(version) => {
                                info!("Already up to date: {}", version);
                            }
                            UpdateStatus::Updated {
                                from_version,
                                to_version,
                            } => {
                                info!(
                                    "Successfully updated from {} to {}",
                                    from_version, to_version
                                );
                            }
                            UpdateStatus::Failed(error) => {
                                error!("Update check failed: {}", error);
                            }
                        }
                        Ok(status)
                    }
                    Err(e) => {
                        error!("Blocking task failed: {}", e);
                        Ok(UpdateStatus::Failed(format!("Blocking task error: {}", e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                Ok(UpdateStatus::Failed(format!("Task spawn error: {}", e)))
            }
        }
    }

    /// Update the binary to the latest version
    pub async fn update(&self) -> Result<UpdateStatus> {
        info!(
            "Updating {} from version {}",
            self.config.bin_name, self.config.current_version
        );

        // Clone data for the blocking task
        let repo_owner = self.config.repo_owner.clone();
        let repo_name = self.config.repo_name.clone();
        let bin_name = self.config.bin_name.clone();
        let current_version = self.config.current_version.clone();
        let show_progress = self.config.show_progress;

        // Move self_update operations to a blocking task to avoid runtime conflicts
        let result = tokio::task::spawn_blocking(move || {
            match self_update::backends::github::Update::configure()
                .repo_owner(&repo_owner)
                .repo_name(&repo_name)
                .bin_name(&bin_name)
                .current_version(&current_version)
                .show_download_progress(show_progress)
                .build()
            {
                Ok(updater) => match updater.update() {
                    Ok(status) => match status {
                        self_update::Status::UpToDate(version) => {
                            Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::UpToDate(version))
                        }
                        self_update::Status::Updated(version) => {
                            Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::Updated {
                                from_version: current_version,
                                to_version: version,
                            })
                        }
                    },
                    Err(e) => Ok(UpdateStatus::Failed(format!("Update failed: {}", e))),
                },
                Err(e) => Ok(UpdateStatus::Failed(format!("Configuration error: {}", e))),
            }
        })
        .await;

        match result {
            Ok(update_result) => {
                match update_result {
                    Ok(status) => {
                        // Log the result for debugging
                        match &status {
                            UpdateStatus::Updated {
                                from_version,
                                to_version,
                            } => {
                                info!(
                                    "Successfully updated from {} to {}",
                                    from_version, to_version
                                );
                            }
                            UpdateStatus::UpToDate(version) => {
                                info!("Already up to date: {}", version);
                            }
                            UpdateStatus::Available {
                                current_version,
                                latest_version,
                            } => {
                                info!(
                                    "Update available: {} -> {}",
                                    current_version, latest_version
                                );
                            }
                            UpdateStatus::Failed(error) => {
                                error!("Update failed: {}", error);
                            }
                        }
                        Ok(status)
                    }
                    Err(e) => {
                        error!("Blocking task failed: {}", e);
                        Ok(UpdateStatus::Failed(format!("Blocking task error: {}", e)))
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn blocking task: {}", e);
                Ok(UpdateStatus::Failed(format!("Task spawn error: {}", e)))
            }
        }
    }

    /// Check for update and install if available
    pub async fn check_and_update(&self) -> Result<UpdateStatus> {
        match self.check_update().await? {
            UpdateStatus::Available {
                current_version,
                latest_version,
            } => {
                info!(
                    "Update available: {} → {}, installing...",
                    current_version, latest_version
                );
                self.update().await
            }
            status => Ok(status),
        }
    }

    /// Compare two version strings to determine if the first is newer than the second
    #[allow(dead_code)]
    fn is_newer_version(&self, version1: &str, version2: &str) -> Result<bool> {
        // Simple version comparison - in production you might want to use semver crate
        let v1_parts: Vec<u32> = version1
            .trim_start_matches('v')
            .split('.')
            .take(3)
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        let v2_parts: Vec<u32> = version2
            .trim_start_matches('v')
            .split('.')
            .take(3)
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        // Pad with zeros if needed
        let v1 = [
            v1_parts.first().copied().unwrap_or(0),
            v1_parts.get(1).copied().unwrap_or(0),
            v1_parts.get(2).copied().unwrap_or(0),
        ];

        let v2 = [
            v2_parts.first().copied().unwrap_or(0),
            v2_parts.get(1).copied().unwrap_or(0),
            v2_parts.get(2).copied().unwrap_or(0),
        ];

        Ok(v1 > v2)
    }
}

/// Convenience function to create an updater and check for updates
pub async fn check_for_updates(bin_name: impl Into<String>) -> Result<UpdateStatus> {
    let config = UpdaterConfig::new(bin_name);
    let updater = TerraphimUpdater::new(config);
    updater.check_update().await
}

/// Convenience function to create an updater and install updates
pub async fn update_binary(bin_name: impl Into<String>) -> Result<UpdateStatus> {
    let config = UpdaterConfig::new(bin_name);
    let updater = TerraphimUpdater::new(config);
    updater.check_and_update().await
}

/// Convenience function with progress disabled (useful for automated environments)
pub async fn update_binary_silent(bin_name: impl Into<String>) -> Result<UpdateStatus> {
    let config = UpdaterConfig::new(bin_name).with_progress(false);
    let updater = TerraphimUpdater::new(config);
    updater.check_and_update().await
}

/// Check for updates automatically using self_update backend
///
/// This is a simplified function that leverages self_update's GitHub backend
/// to check for available updates without installing them.
///
/// # Arguments
/// * `bin_name` - Name of the binary (e.g., "terraphim")
/// * `current_version` - Current version of the binary (e.g., "1.0.0")
///
/// # Returns
/// * `Ok(UpdateStatus)` - Status indicating if an update is available
/// * `Err(anyhow::Error)` - Error if the check fails
///
/// # Example
/// ```no_run
/// use terraphim_update::check_for_updates_auto;
///
/// async {
///     let status = check_for_updates_auto("terraphim", "1.0.0").await?;
///     println!("Update status: {}", status);
///     Ok::<(), anyhow::Error>(())
/// };
/// ```
pub async fn check_for_updates_auto(bin_name: &str, current_version: &str) -> Result<UpdateStatus> {
    info!("Checking for updates: {} v{}", bin_name, current_version);

    let bin_name = bin_name.to_string();
    let current_version = current_version.to_string();

    let result =
        tokio::task::spawn_blocking(
            move || match self_update::backends::github::Update::configure()
                .repo_owner("terraphim")
                .repo_name("terraphim-ai")
                .bin_name(&bin_name)
                .current_version(&current_version)
                .build()
            {
                Ok(updater) => match updater.get_latest_release() {
                    Ok(release) => {
                        let latest_version = release.version.clone();

                        if is_newer_version_static(&latest_version, &current_version) {
                            Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::Available {
                                current_version,
                                latest_version,
                            })
                        } else {
                            Ok::<UpdateStatus, anyhow::Error>(UpdateStatus::UpToDate(
                                current_version,
                            ))
                        }
                    }
                    Err(e) => Ok(UpdateStatus::Failed(format!("Check failed: {}", e))),
                },
                Err(e) => Ok(UpdateStatus::Failed(format!("Configuration error: {}", e))),
            },
        )
        .await;

    match result {
        Ok(update_result) => update_result,
        Err(e) => {
            error!("Failed to spawn blocking task: {}", e);
            Ok(UpdateStatus::Failed(format!("Task spawn error: {}", e)))
        }
    }
}

/// Check for updates on application startup
///
/// This function performs a non-blocking update check on startup
/// and logs a warning if the check fails (doesn't interrupt startup).
///
/// # Arguments
/// * `bin_name` - Name of the binary (e.g., "terraphim-agent")
///
/// # Returns
/// * `Ok(UpdateStatus)` - Status of update check
/// * `Err(anyhow::Error)` - Error if check fails
///
/// # Example
/// ```no_run
/// use terraphim_update::check_for_updates_startup;
///
/// async {
///     if let Err(e) = check_for_updates_startup("terraphim-agent").await {
///         eprintln!("Update check failed: {}", e);
///     }
///     Ok::<(), anyhow::Error>(())
/// };
/// ```
pub async fn check_for_updates_startup(bin_name: &str) -> Result<UpdateStatus> {
    let current_version = env!("CARGO_PKG_VERSION");
    check_for_updates_auto(bin_name, current_version).await
}

/// Start the update scheduler
///
/// This function starts a background task that periodically checks for updates
/// and sends notifications through a callback when updates are available.
///
/// # Arguments
/// * `bin_name` - Name of the binary (e.g., "terraphim-agent")
/// * `current_version` - Current version of the binary
/// * `callback` - Function to call when an update is available
///
/// # Returns
/// * `Ok(JoinHandle<()>)` - Handle to the scheduler task (can be used to abort)
/// * `Err(anyhow::Error)` - Error if scheduler fails to start
///
/// # Example
/// ```no_run
/// use terraphim_update::start_update_scheduler;
///
/// async {
///     let handle = start_update_scheduler(
///         "terraphim-agent",
///         "1.0.0",
///         Box::new(|update_info| {
///             println!("Update available: {}", update_info.latest_version);
///         })
///     ).await?;
///     # Ok::<(), anyhow::Error>(())
/// };
/// ```
pub async fn start_update_scheduler(
    bin_name: &str,
    current_version: &str,
    callback: Box<dyn Fn(UpdateAvailableInfo) + Send + Sync>,
) -> Result<tokio::task::JoinHandle<()>> {
    use crate::config::UpdateConfig;
    use crate::scheduler::{UpdateCheckResult, UpdateScheduler};
    use std::sync::Arc;

    let config = UpdateConfig::default();

    let bin_name_clone = bin_name.to_string();
    let current_version_clone = current_version.to_string();

    let check_fn = Arc::new(move || -> anyhow::Result<UpdateCheckResult> {
        let status = {
            let bin_name = bin_name_clone.clone();
            let current_version = current_version_clone.clone();

            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async { check_for_updates_auto(&bin_name, &current_version).await })
            })
        }?;

        match status {
            UpdateStatus::Available {
                current_version,
                latest_version,
            } => Ok(UpdateCheckResult::UpdateAvailable {
                current_version,
                latest_version,
            }),
            UpdateStatus::UpToDate(_) => Ok(UpdateCheckResult::UpToDate),
            UpdateStatus::Failed(error) => Ok(UpdateCheckResult::Failed { error }),
            _ => Ok(UpdateCheckResult::UpToDate),
        }
    });

    let mut scheduler = UpdateScheduler::new(Arc::new(config), check_fn);
    let mut receiver = scheduler.create_notification_channel()?;

    scheduler.start().await?;

    let callback = Arc::new(callback);

    let handle = tokio::spawn(async move {
        while let Some(notification) = receiver.recv().await {
            match notification {
                crate::scheduler::UpdateNotification::UpdateAvailable {
                    current_version,
                    latest_version,
                } => {
                    callback(UpdateAvailableInfo {
                        current_version: current_version.clone(),
                        latest_version: latest_version.clone(),
                    });
                }
                crate::scheduler::UpdateNotification::CheckFailed { error } => {
                    tracing::warn!("Update check failed: {}", error);
                }
                crate::scheduler::UpdateNotification::Stopped => {
                    break;
                }
            }
        }
    });

    Ok(handle)
}

/// Information about an available update (for callback)
#[derive(Debug, Clone)]
pub struct UpdateAvailableInfo {
    pub current_version: String,
    pub latest_version: String,
}

/// Backup the current binary with a version suffix
///
/// Creates a backup of the binary before updating, allowing rollback
/// if the update fails.
///
/// # Arguments
/// * `binary_path` - Path to the binary to backup
/// * `version` - Version string to use in backup filename
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the backup file
/// * `Err(anyhow::Error)` - Error if backup fails
///
/// # Example
/// ```no_run
/// use terraphim_update::backup_binary;
/// use std::path::Path;
///
/// let backup = backup_binary(Path::new("/usr/local/bin/terraphim"), "1.0.0")?;
/// println!("Backup created at: {:?}", backup);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn backup_binary(binary_path: &Path, version: &str) -> Result<PathBuf> {
    info!(
        "Backing up binary at {:?} with version {}",
        binary_path, version
    );

    if !binary_path.exists() {
        anyhow::bail!("Binary not found at {:?}", binary_path);
    }

    let backup_path = binary_path.with_extension(format!("bak-{}", version));

    fs::copy(binary_path, &backup_path)?;

    info!("Backup created at {:?}", backup_path);
    Ok(backup_path)
}

/// Rollback to a previous version from backup
///
/// Restores a backed-up binary to the original location.
///
/// # Arguments
/// * `backup_path` - Path to the backup file
/// * `target_path` - Path where to restore the binary
///
/// # Returns
/// * `Ok(())` - Success
/// * `Err(anyhow::Error)` - Error if rollback fails
///
/// # Example
/// ```no_run
/// use terraphim_update::rollback;
/// use std::path::Path;
///
/// rollback(
///     Path::new("/usr/local/bin/terraphim.bak-1.0.0"),
///     Path::new("/usr/local/bin/terraphim")
/// )?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn rollback(backup_path: &Path, target_path: &Path) -> Result<()> {
    info!("Rolling back from {:?} to {:?}", backup_path, target_path);

    if !backup_path.exists() {
        anyhow::bail!("Backup not found at {:?}", backup_path);
    }

    fs::copy(backup_path, target_path)?;

    info!("Rollback completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_version_comparison() {
        let config = UpdaterConfig::new("test");
        let updater = TerraphimUpdater::new(config);

        // Test basic version comparisons
        assert!(updater.is_newer_version("1.1.0", "1.0.0").unwrap());
        assert!(updater.is_newer_version("2.0.0", "1.9.9").unwrap());
        assert!(updater.is_newer_version("1.0.1", "1.0.0").unwrap());

        // Test equal versions
        assert!(!updater.is_newer_version("1.0.0", "1.0.0").unwrap());

        // Test older versions
        assert!(!updater.is_newer_version("1.0.0", "1.1.0").unwrap());
        assert!(!updater.is_newer_version("1.9.9", "2.0.0").unwrap());

        // Test with v prefix
        assert!(updater.is_newer_version("v1.1.0", "v1.0.0").unwrap());
        assert!(updater.is_newer_version("1.1.0", "v1.0.0").unwrap());
        assert!(updater.is_newer_version("v1.1.0", "1.0.0").unwrap());
    }

    #[tokio::test]
    async fn test_updater_config() {
        let config = UpdaterConfig::new("test-binary")
            .with_version("1.0.0")
            .with_progress(false);

        assert_eq!(config.bin_name, "test-binary");
        assert_eq!(config.current_version, "1.0.0");
        assert!(!config.show_progress);
        assert_eq!(config.repo_owner, "terraphim");
        assert_eq!(config.repo_name, "terraphim-ai");
    }

    #[test]
    fn test_backup_binary() {
        // Create a temporary file to simulate a binary
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test binary content").unwrap();

        let binary_path = temp_file.path();
        let version = "1.0.0";

        let backup_path = backup_binary(binary_path, version).unwrap();

        // Verify backup was created
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains("bak-1.0.0"));

        // Verify backup has same content
        let original_content = fs::read_to_string(binary_path).unwrap();
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(original_content, backup_content);

        // Clean up backup
        fs::remove_file(&backup_path).unwrap();
    }

    #[test]
    fn test_backup_binary_nonexistent() {
        let nonexistent_path = Path::new("/nonexistent/path/to/binary");

        let result = backup_binary(nonexistent_path, "1.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_rollback() {
        // Create a temporary file to simulate a backup
        let mut backup_file = NamedTempFile::new().unwrap();
        writeln!(backup_file, "backup content").unwrap();

        let backup_path = backup_file.path();

        // Create target path
        let mut target_file = NamedTempFile::new().unwrap();
        writeln!(target_file, "original content").unwrap();
        let target_path = target_file.path();

        // Perform rollback
        rollback(backup_path, target_path).unwrap();

        // Verify target now has backup content
        let target_content = fs::read_to_string(target_path).unwrap();
        assert_eq!(target_content, "backup content\n");
    }

    #[test]
    fn test_rollback_nonexistent() {
        let nonexistent_backup = Path::new("/nonexistent/backup.bak");
        let temp_file = NamedTempFile::new().unwrap();
        let target_path = temp_file.path();

        let result = rollback(nonexistent_backup, target_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_backup_and_rollback_roundtrip() {
        // Create original binary
        let mut original_file = NamedTempFile::new().unwrap();
        writeln!(original_file, "original binary v1.0.0").unwrap();
        let original_path = original_file.path();

        // Create backup
        let backup_path = backup_binary(original_path, "1.0.0").unwrap();

        // Modify original (simulate update)
        fs::write(original_path, "updated binary v1.1.0").unwrap();

        // Verify original changed
        assert_eq!(
            fs::read_to_string(original_path).unwrap(),
            "updated binary v1.1.0"
        );

        // Rollback
        rollback(&backup_path, original_path).unwrap();

        // Verify original restored
        assert_eq!(
            fs::read_to_string(original_path).unwrap(),
            "original binary v1.0.0\n"
        );

        // Clean up backup
        fs::remove_file(&backup_path).unwrap();
    }

    #[tokio::test]
    async fn test_check_for_updates_auto() {
        // This test will make actual API calls to GitHub
        // It's useful for manual testing but may be flaky in CI
        let status = check_for_updates_auto("terraphim", "0.0.1").await;

        match status {
            Ok(UpdateStatus::Available {
                current_version,
                latest_version,
            }) => {
                assert_eq!(current_version, "0.0.1");
                assert_ne!(current_version, latest_version);
            }
            Ok(UpdateStatus::UpToDate(version)) => {
                assert_eq!(version, "0.0.1");
            }
            Ok(UpdateStatus::Failed(_)) => {
                // This is acceptable if GitHub API is unavailable
            }
            _ => {}
        }
    }

    #[test]
    fn test_is_newer_version_static() {
        // Test basic comparisons
        assert!(is_newer_version_static("2.0.0", "1.0.0"));
        assert!(is_newer_version_static("1.1.0", "1.0.0"));
        assert!(is_newer_version_static("1.0.1", "1.0.0"));

        // Test equal versions
        assert!(!is_newer_version_static("1.0.0", "1.0.0"));

        // Test older versions
        assert!(!is_newer_version_static("1.0.0", "2.0.0"));
        assert!(!is_newer_version_static("1.0.0", "1.1.0"));

        // Test with v prefix
        assert!(is_newer_version_static("v2.0.0", "v1.0.0"));
        assert!(!is_newer_version_static("v1.0.0", "v2.0.0"));
    }

    #[test]
    fn test_update_status_display() {
        let up_to_date = UpdateStatus::UpToDate("1.0.0".to_string());
        assert!(up_to_date.to_string().contains("1.0.0"));

        let updated = UpdateStatus::Updated {
            from_version: "1.0.0".to_string(),
            to_version: "2.0.0".to_string(),
        };
        assert!(updated.to_string().contains("1.0.0"));
        assert!(updated.to_string().contains("2.0.0"));

        let available = UpdateStatus::Available {
            current_version: "1.0.0".to_string(),
            latest_version: "2.0.0".to_string(),
        };
        assert!(available.to_string().contains("1.0.0"));
        assert!(available.to_string().contains("2.0.0"));

        let failed = UpdateStatus::Failed("test error".to_string());
        assert!(failed.to_string().contains("test error"));
    }
}
