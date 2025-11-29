//! Shared auto-update functionality for Terraphim AI binaries
//!
//! This crate provides a unified interface for self-updating Terraphim AI CLI tools
//! using GitHub Releases as the distribution channel.

use anyhow::Result;
use self_update::cargo_crate_version;
use std::fmt;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
