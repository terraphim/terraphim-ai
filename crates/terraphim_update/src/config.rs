//! Configuration types for automatic update functionality
//!
//! This module defines all configuration and state types needed for
//! automatic updates, including UpdateConfig, UpdateInfo, UpdateHistory,
//! and related types.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for automatic updates
///
/// Controls how and when automatic update checks are performed.
///
/// # Example
/// ```no_run
/// use terraphim_update::config::UpdateConfig;
///
/// let config = UpdateConfig::default();
/// println!("Auto-update enabled: {}", config.auto_update_enabled);
/// println!("Check interval: {:?}", config.auto_update_check_interval);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateConfig {
    /// Enable automatic update checking
    ///
    /// When true, the application will automatically check for updates
    /// according to the configured interval. When false, updates must
    /// be triggered manually.
    pub auto_update_enabled: bool,

    /// Interval between update checks
    ///
    /// The application will check for updates at most once per this interval.
    /// Default is 24 hours (daily).
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
///
/// Contains all metadata needed to display and install an update.
///
/// # Example
/// ```no_run
/// use terraphim_update::config::UpdateInfo;
/// use jiff::Timestamp;
///
/// let info = UpdateInfo {
///     version: "1.1.0".to_string(),
///     release_date: Timestamp::now(),
///     notes: "Bug fixes and improvements".to_string(),
///     download_url: "https://example.com/binary".to_string(),
///     signature_url: "https://example.com/binary.sig".to_string(),
///     arch: "x86_64".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateInfo {
    /// Version number of the update
    ///
    /// Format: "X.Y.Z" (e.g., "1.2.3")
    pub version: String,

    /// Release date of the update
    pub release_date: Timestamp,

    /// Release notes or changelog
    ///
    /// May contain markdown-formatted text describing changes in this release.
    pub notes: String,

    /// Download URL for the binary
    ///
    /// URL pointing to the binary file on GitHub Releases.
    pub download_url: String,

    /// PGP signature URL for verification
    ///
    /// URL pointing to the detached PGP signature for the binary.
    /// Used to verify authenticity and integrity of the download.
    pub signature_url: String,

    /// Binary architecture
    ///
    /// Target architecture (e.g., "x86_64", "aarch64").
    pub arch: String,
}

/// Persistent update history state
///
/// Tracks the state of updates over time to avoid redundant checks
/// and maintain backup version information.
///
/// # Example
/// ```no_run
/// use terraphim_update::config::UpdateHistory;
///
/// let history = UpdateHistory::default();
/// println!("Last check: {:?}", history.last_check);
/// println!("Current version: {}", history.current_version);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateHistory {
    /// Last time an update check was performed
    pub last_check: Timestamp,

    /// Version currently installed
    pub current_version: String,

    /// Pending update notification (if user hasn't acted on it)
    ///
    /// When an update is available but not yet installed, this field
    /// stores the update info. After installation or user dismissal,
    /// this field should be cleared.
    pub pending_update: Option<UpdateInfo>,

    /// Backup versions available for rollback
    ///
    /// List of version strings that have been backed up. The newest
    /// backup is at the end of the list.
    pub backup_versions: Vec<String>,

    /// Update check history (last 10 checks)
    ///
    /// Maintains a log of recent update check attempts for debugging
    /// and analytics purposes.
    pub check_history: Vec<UpdateCheckEntry>,
}

impl Default for UpdateHistory {
    fn default() -> Self {
        Self {
            last_check: Timestamp::now(),
            current_version: String::new(),
            pending_update: None,
            backup_versions: Vec::new(),
            check_history: Vec::new(),
        }
    }
}

impl UpdateHistory {
    /// Add a check entry to history
    ///
    /// Maintains at most 10 entries, removing oldest when limit is exceeded.
    ///
    /// # Arguments
    /// * `entry` - Check entry to add
    pub fn add_check_entry(&mut self, entry: UpdateCheckEntry) {
        self.check_history.push(entry);
        // Keep only last 10 entries
        if self.check_history.len() > 10 {
            self.check_history.remove(0);
        }
    }

    /// Add a backup version to the list
    ///
    /// Maintains at most 3 backup versions (configurable).
    ///
    /// # Arguments
    /// * `version` - Version string to add as backup
    /// * `max_backups` - Maximum number of backups to keep (default: 3)
    pub fn add_backup_version(&mut self, version: String, max_backups: usize) {
        self.backup_versions.push(version);
        // Keep only the last N backups
        if self.backup_versions.len() > max_backups {
            self.backup_versions.remove(0);
        }
    }

    /// Get the most recent backup version
    ///
    /// Returns the latest backup version or None if no backups exist.
    pub fn latest_backup(&self) -> Option<&String> {
        self.backup_versions.last()
    }
}

/// Single update check entry
///
/// Records the result of a single update check attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheckEntry {
    /// When the check was performed
    pub timestamp: Timestamp,

    /// Result of the check
    pub result: UpdateCheckResult,
}

/// Result of an update check
///
/// Describes whether an update is available, the system is up to date,
/// or the check failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateCheckResult {
    /// No update available
    ///
    /// The current version is the latest.
    UpToDate,

    /// Update available
    ///
    /// A newer version is available. The `notified` flag indicates
    /// whether the user has been notified about this update.
    UpdateAvailable { version: String, notified: bool },

    /// Update check failed
    ///
    /// The check could not be completed due to network or other errors.
    CheckFailed { error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_config_default() {
        let config = UpdateConfig::default();
        assert!(config.auto_update_enabled, "Default should be enabled");
        assert_eq!(
            config.auto_update_check_interval,
            Duration::from_secs(86400),
            "Default should be 24 hours"
        );
    }

    #[test]
    fn test_update_config_serialize() {
        let config = UpdateConfig {
            auto_update_enabled: false,
            auto_update_check_interval: Duration::from_secs(3600), // 1 hour
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: UpdateConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_update_info_serialization() {
        let info = UpdateInfo {
            version: "1.2.3".to_string(),
            release_date: Timestamp::now(),
            notes: "Test release".to_string(),
            download_url: "https://example.com/binary".to_string(),
            signature_url: "https://example.com/binary.sig".to_string(),
            arch: "x86_64".to_string(),
        };

        let serialized = serde_json::to_string(&info).unwrap();
        let deserialized: UpdateInfo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(info.version, deserialized.version);
        assert_eq!(info.download_url, deserialized.download_url);
        assert_eq!(info.signature_url, deserialized.signature_url);
        assert_eq!(info.arch, deserialized.arch);
    }

    #[test]
    fn test_update_history_default() {
        let history = UpdateHistory::default();
        assert_eq!(history.current_version, String::new());
        assert!(history.pending_update.is_none());
        assert!(history.backup_versions.is_empty());
        assert!(history.check_history.is_empty());
    }

    #[test]
    fn test_update_history_add_entry() {
        let mut history = UpdateHistory::default();
        let entry = UpdateCheckEntry {
            timestamp: Timestamp::now(),
            result: UpdateCheckResult::UpToDate,
        };

        history.add_check_entry(entry);
        assert_eq!(history.check_history.len(), 1);
    }

    #[test]
    fn test_update_history_limit_entries() {
        let mut history = UpdateHistory::default();

        // Add 15 entries (should be limited to 10)
        for _i in 0..15 {
            history.add_check_entry(UpdateCheckEntry {
                timestamp: Timestamp::now(),
                result: UpdateCheckResult::UpToDate,
            });
        }

        assert_eq!(
            history.check_history.len(),
            10,
            "Should keep only last 10 entries"
        );
    }

    #[test]
    fn test_update_history_add_backup() {
        let mut history = UpdateHistory::default();
        let max_backups = 3;

        // Add 5 backups (should be limited to 3)
        for i in 0..5 {
            history.add_backup_version(format!("1.0.{}", i), max_backups);
        }

        assert_eq!(
            history.backup_versions.len(),
            3,
            "Should keep only 3 backups"
        );
        assert_eq!(
            history.backup_versions,
            vec![
                "1.0.2".to_string(),
                "1.0.3".to_string(),
                "1.0.4".to_string()
            ]
        );
    }

    #[test]
    fn test_update_history_latest_backup() {
        let mut history = UpdateHistory::default();
        history.add_backup_version("1.0.0".to_string(), 3);
        history.add_backup_version("1.0.1".to_string(), 3);

        assert_eq!(history.latest_backup(), Some(&"1.0.1".to_string()));
    }

    #[test]
    fn test_update_check_entry_serialization() {
        let entry = UpdateCheckEntry {
            timestamp: Timestamp::now(),
            result: UpdateCheckResult::UpdateAvailable {
                version: "1.1.0".to_string(),
                notified: false,
            },
        };

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: UpdateCheckEntry = serde_json::from_str(&serialized).unwrap();

        match deserialized.result {
            UpdateCheckResult::UpdateAvailable { version, notified } => {
                assert_eq!(version, "1.1.0");
                assert!(!notified);
            }
            _ => panic!("Expected UpdateAvailable variant"),
        }
    }

    #[test]
    fn test_update_check_result_variants() {
        let up_to_date = UpdateCheckResult::UpToDate;
        let update_available = UpdateCheckResult::UpdateAvailable {
            version: "1.2.0".to_string(),
            notified: true,
        };
        let check_failed = UpdateCheckResult::CheckFailed {
            error: "Network error".to_string(),
        };

        // Test that variants are distinct
        assert_ne!(up_to_date, update_available);
        assert_ne!(up_to_date, check_failed);
        assert_ne!(update_available, check_failed);

        // Test serialization for each variant
        for result in [up_to_date, update_available, check_failed] {
            let serialized = serde_json::to_string(&result).unwrap();
            let deserialized: UpdateCheckResult = serde_json::from_str(&serialized).unwrap();
            assert_eq!(result, deserialized);
        }
    }
}
