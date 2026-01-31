//! Backup and rollback functionality for safe updates
//!
//! This module provides robust backup and rollback capabilities to ensure
//! failed updates can be safely reverted. It includes:
//! - Automatic backup rotation (keeps last N backups)
//! - Backup integrity validation
//! - Versioned rollback support
//! - Graceful error handling for missing/corrupt backups

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Default number of backups to keep
pub const DEFAULT_MAX_BACKUPS: usize = 3;

/// Represents a single backup with metadata
#[derive(Debug, Clone)]
pub struct Backup {
    /// Path to the backup file
    pub path: PathBuf,
    /// Version string for this backup
    pub version: String,
    /// Timestamp when backup was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// SHA256 hash for integrity verification
    pub checksum: String,
}

impl Backup {
    /// Create a new backup instance
    fn new(path: PathBuf, version: String, checksum: String) -> Self {
        Self {
            path,
            version,
            timestamp: chrono::Utc::now(),
            checksum,
        }
    }

    /// Calculate SHA256 checksum of the backup file
    pub fn calculate_checksum(&self) -> Result<String> {
        let contents = fs::read(&self.path)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Verify that the backup file is intact
    pub fn verify_integrity(&self) -> Result<bool> {
        if !self.path.exists() {
            return Ok(false);
        }

        let current_checksum = self.calculate_checksum()?;
        Ok(current_checksum == self.checksum)
    }
}

/// Manages backup lifecycle including rotation and validation
#[derive(Debug)]
pub struct BackupManager {
    /// Directory where backups are stored
    backup_dir: PathBuf,
    /// Maximum number of backups to keep
    max_backups: usize,
    /// Map of version to backup metadata
    backups: HashMap<String, Backup>,
}

impl BackupManager {
    /// Create a new backup manager
    ///
    /// # Arguments
    /// * `backup_dir` - Directory to store backups in
    /// * `max_backups` - Maximum number of backups to keep (default: 3)
    ///
    /// # Example
    /// ```no_run
    /// use terraphim_update::rollback::BackupManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = BackupManager::new(
    ///     PathBuf::from("/var/lib/terraphim/backups"),
    ///     3
    /// );
    /// ```
    pub fn new(backup_dir: PathBuf, max_backups: usize) -> Result<Self> {
        if max_backups == 0 {
            return Err(anyhow!("max_backups must be at least 1"));
        }

        fs::create_dir_all(&backup_dir)?;

        let mut manager = Self {
            backup_dir,
            max_backups,
            backups: HashMap::new(),
        };

        manager.load_existing_backups()?;
        Ok(manager)
    }

    /// Load existing backups from the backup directory
    fn load_existing_backups(&mut self) -> Result<()> {
        if !self.backup_dir.exists() {
            return Ok(());
        }

        debug!("Loading existing backups from {:?}", self.backup_dir);

        let entries = fs::read_dir(&self.backup_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("bak") {
                let filename = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| anyhow!("Invalid backup filename"))?;

                if let Some(version) = filename.strip_prefix("backup-") {
                    let backup = Backup::new(path.clone(), version.to_string(), String::new());
                    self.backups.insert(version.to_string(), backup);
                    debug!("Loaded backup for version {}", version);
                }
            }
        }

        Ok(())
    }

    /// Create a backup of the specified binary
    ///
    /// # Arguments
    /// * `binary_path` - Path to the binary to backup
    /// * `version` - Version string for this backup
    ///
    /// # Returns
    /// * `Ok(Backup)` - The created backup
    /// * `Err(anyhow::Error)` - Error if backup creation fails
    ///
    /// # Example
    /// ```no_run
    /// # use terraphim_update::rollback::BackupManager;
    /// # use std::path::PathBuf;
    /// # let mut manager = BackupManager::new(PathBuf::from("/tmp/backups"), 3).unwrap();
    /// let backup = manager.create_backup(
    ///     &PathBuf::from("/usr/local/bin/terraphim"),
    ///     "1.0.0"
    /// ).unwrap();
    /// ```
    pub fn create_backup(&mut self, binary_path: &Path, version: &str) -> Result<Backup> {
        info!("Creating backup for {:?} version {}", binary_path, version);

        if !binary_path.exists() {
            return Err(anyhow!("Binary not found at {:?}", binary_path));
        }

        let backup_filename = format!("backup-{}.bak", version);
        let backup_path = self.backup_dir.join(&backup_filename);

        fs::copy(binary_path, &backup_path)?;

        let checksum = Self::calculate_file_checksum(&backup_path)?;
        let backup = Backup::new(backup_path.clone(), version.to_string(), checksum);

        self.backups.insert(version.to_string(), backup.clone());

        info!("Backup created: {:?}", backup_path);

        Ok(backup)
    }

    /// Calculate SHA256 checksum of a file
    fn calculate_file_checksum(path: &Path) -> Result<String> {
        let contents = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Rotate backups to maintain the maximum count
    ///
    /// Removes the oldest backups if the count exceeds max_backups.
    ///
    /// # Returns
    /// * `Ok(Vec<String>)` - List of versions that were removed
    /// * `Err(anyhow::Error)` - Error if rotation fails
    pub fn rotate_backups(&mut self) -> Result<Vec<String>> {
        let mut removed = Vec::new();

        if self.backups.len() <= self.max_backups {
            debug!(
                "Backup rotation not needed ({} backups)",
                self.backups.len()
            );
            return Ok(removed);
        }

        info!(
            "Rotating backups: {} backups, max {}",
            self.backups.len(),
            self.max_backups
        );

        let mut backups: Vec<_> = self.backups.values().collect();

        backups.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let num_to_remove = backups.len().saturating_sub(self.max_backups);

        let to_remove: Vec<_> = backups
            .into_iter()
            .take(num_to_remove)
            .map(|backup| (backup.path.clone(), backup.version.clone()))
            .collect();

        for (path, version) in to_remove.iter() {
            info!("Removing old backup: {:?}", path);

            if let Err(e) = fs::remove_file(path) {
                warn!("Failed to remove old backup {:?}: {}", path, e);
            } else {
                self.backups.remove(version);
                removed.push(version.clone());
            }
        }

        Ok(removed)
    }

    /// Get a backup by version
    ///
    /// # Arguments
    /// * `version` - Version string to look up
    ///
    /// # Returns
    /// * `Ok(Backup)` - The backup if found
    /// * `Err(anyhow::Error)` - Error if not found or corrupt
    pub fn get_backup(&self, version: &str) -> Result<Backup> {
        let backup = self
            .backups
            .get(version)
            .ok_or_else(|| anyhow!("Backup not found for version {}", version))?;

        if !backup.path.exists() {
            return Err(anyhow!("Backup file missing for version {}", version));
        }

        Ok(backup.clone())
    }

    /// Get all available backup versions
    ///
    /// Returns a list of version strings sorted by timestamp (newest first).
    ///
    /// # Returns
    /// * `Vec<String>` - Sorted list of version strings
    pub fn list_backups(&self) -> Vec<String> {
        let mut backups: Vec<_> = self.backups.values().collect();

        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        backups.iter().map(|b| b.version.clone()).collect()
    }

    /// Rollback to a specific version
    ///
    /// # Arguments
    /// * `version` - Version to rollback to
    /// * `target_path` - Path where to restore the binary
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(anyhow::Error)` - Error if rollback fails
    ///
    /// # Example
    /// ```no_run
    /// # use terraphim_update::rollback::BackupManager;
    /// # use std::path::PathBuf;
    /// # let manager = BackupManager::new(PathBuf::from("/tmp/backups"), 3).unwrap();
    /// manager.rollback_to_version(
    ///     "1.0.0",
    ///     &PathBuf::from("/usr/local/bin/terraphim")
    /// ).unwrap();
    /// ```
    pub fn rollback_to_version(&self, version: &str, target_path: &Path) -> Result<()> {
        info!("Rolling back to version {} at {:?}", version, target_path);

        let backup = self.get_backup(version)?;

        if !backup.verify_integrity()? {
            error!("Backup integrity check failed for version {}", version);
            return Err(anyhow!("Backup file corrupt for version {}", version));
        }

        if !target_path.parent().map(|p| p.exists()).unwrap_or(true) {
            fs::create_dir_all(target_path.parent().unwrap())?;
        }

        fs::copy(&backup.path, target_path)?;

        info!("Rollback to version {} completed successfully", version);
        Ok(())
    }

    /// Rollback to the most recent backup
    ///
    /// # Arguments
    /// * `target_path` - Path where to restore the binary
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(anyhow::Error)` - Error if rollback fails
    pub fn rollback_to_latest(&self, target_path: &Path) -> Result<()> {
        let versions = self.list_backups();

        if versions.is_empty() {
            return Err(anyhow!("No backups available for rollback"));
        }

        let latest_version = &versions[0];
        self.rollback_to_version(latest_version, target_path)
    }

    /// Delete a specific backup
    ///
    /// # Arguments
    /// * `version` - Version to delete
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(anyhow::Error)` - Error if deletion fails
    pub fn delete_backup(&mut self, version: &str) -> Result<()> {
        if let Some(backup) = self.backups.remove(version) {
            if backup.path.exists() {
                fs::remove_file(&backup.path)?;
                info!("Deleted backup for version {}", version);
            }
        }

        Ok(())
    }

    /// Clean up all backups
    ///
    /// Removes all backup files and clears the backup registry.
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(anyhow::Error)` - Error if cleanup fails
    pub fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up all backups in {:?}", self.backup_dir);

        for backup in self.backups.values() {
            if backup.path.exists() {
                if let Err(e) = fs::remove_file(&backup.path) {
                    warn!("Failed to remove backup {:?}: {}", backup.path, e);
                }
            }
        }

        self.backups.clear();
        Ok(())
    }

    /// Get the number of stored backups
    pub fn backup_count(&self) -> usize {
        self.backups.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_binary(dir: &Path, version: &str) -> PathBuf {
        let binary_path = dir.join("terraphim");
        fs::write(&binary_path, format!("binary version {}", version)).unwrap();
        binary_path
    }

    #[test]
    fn test_backup_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let manager = BackupManager::new(backup_dir.clone(), 3).unwrap();

        assert_eq!(manager.max_backups, 3);
        assert!(manager.backup_dir == backup_dir);
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_backup_manager_zero_max_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let result = BackupManager::new(backup_dir, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir.clone(), 3).unwrap();

        let backup = manager.create_backup(&binary_path, "1.0.0").unwrap();

        assert!(backup.path.exists());
        assert!(backup.path.starts_with(&backup_dir));
        assert!(backup.path.to_string_lossy().contains("backup-1.0.0"));
        assert_eq!(backup.version, "1.0.0");
        assert!(!backup.checksum.is_empty());
        assert_eq!(manager.backup_count(), 1);
    }

    #[test]
    fn test_create_backup_nonexistent_binary() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();

        let result = manager.create_backup(Path::new("/nonexistent/binary"), "1.0.0");

        assert!(result.is_err());
    }

    #[test]
    fn test_backup_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();

        for i in 0..5 {
            let binary_path = create_test_binary(&binary_dir, &format!("1.0.{}", i));
            manager
                .create_backup(&binary_path, &format!("1.0.{}", i))
                .unwrap();
        }

        assert_eq!(manager.backup_count(), 5);

        let removed = manager.rotate_backups().unwrap();

        assert_eq!(removed.len(), 2);
        assert_eq!(manager.backup_count(), 3);

        let versions = manager.list_backups();
        assert_eq!(versions.len(), 3);
        assert!(versions.contains(&"1.0.2".to_string()));
        assert!(versions.contains(&"1.0.3".to_string()));
        assert!(versions.contains(&"1.0.4".to_string()));
    }

    #[test]
    fn test_backup_rotation_not_needed() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();

        for i in 0..2 {
            let binary_path = create_test_binary(&binary_dir, &format!("1.0.{}", i));
            manager
                .create_backup(&binary_path, &format!("1.0.{}", i))
                .unwrap();
        }

        let removed = manager.rotate_backups().unwrap();

        assert_eq!(removed.len(), 0);
        assert_eq!(manager.backup_count(), 2);
    }

    #[test]
    fn test_get_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        manager.create_backup(&binary_path, "1.0.0").unwrap();

        let backup = manager.get_backup("1.0.0").unwrap();

        assert_eq!(backup.version, "1.0.0");
        assert!(backup.path.exists());
    }

    #[test]
    fn test_get_backup_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let manager = BackupManager::new(backup_dir, 3).unwrap();

        let result = manager.get_backup("1.0.0");

        assert!(result.is_err());
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();

        for i in 0..3 {
            let binary_path = create_test_binary(&binary_dir, &format!("1.0.{}", i));
            manager
                .create_backup(&binary_path, &format!("1.0.{}", i))
                .unwrap();
        }

        let versions = manager.list_backups();

        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0], "1.0.2");
        assert_eq!(versions[1], "1.0.1");
        assert_eq!(versions[2], "1.0.0");
    }

    #[test]
    fn test_rollback_to_version() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");
        let target_path = binary_dir.join("target");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        manager.create_backup(&binary_path, "1.0.0").unwrap();

        manager.rollback_to_version("1.0.0", &target_path).unwrap();

        assert!(target_path.exists());
        let content = fs::read_to_string(&target_path).unwrap();
        assert_eq!(content, "binary version 1.0.0");
    }

    #[test]
    fn test_rollback_to_version_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let target_path = binary_dir.join("target");

        let manager = BackupManager::new(backup_dir, 3).unwrap();

        let result = manager.rollback_to_version("1.0.0", &target_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_rollback_to_latest() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let target_path = binary_dir.join("target");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();

        for i in 0..3 {
            let binary_path = create_test_binary(&binary_dir, &format!("1.0.{}", i));
            manager
                .create_backup(&binary_path, &format!("1.0.{}", i))
                .unwrap();
        }

        manager.rollback_to_latest(&target_path).unwrap();

        assert!(target_path.exists());
        let content = fs::read_to_string(&target_path).unwrap();
        assert_eq!(content, "binary version 1.0.2");
    }

    #[test]
    fn test_rollback_to_latest_no_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let target_path = binary_dir.join("target");

        let manager = BackupManager::new(backup_dir, 3).unwrap();

        let result = manager.rollback_to_latest(&target_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        manager.create_backup(&binary_path, "1.0.0").unwrap();

        assert_eq!(manager.backup_count(), 1);

        manager.delete_backup("1.0.0").unwrap();

        assert_eq!(manager.backup_count(), 0);
    }

    #[test]
    fn test_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();

        let mut manager = BackupManager::new(backup_dir.clone(), 3).unwrap();

        for i in 0..3 {
            let binary_path = create_test_binary(&binary_dir, &format!("1.0.{}", i));
            manager
                .create_backup(&binary_path, &format!("1.0.{}", i))
                .unwrap();
        }

        assert_eq!(manager.backup_count(), 3);

        manager.cleanup().unwrap();

        assert_eq!(manager.backup_count(), 0);

        let backup_files: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(backup_files.len(), 0);
    }

    #[test]
    fn test_backup_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        let backup = manager.create_backup(&binary_path, "1.0.0").unwrap();

        let is_valid = backup.verify_integrity().unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_backup_integrity_corrupt() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        let backup = manager.create_backup(&binary_path, "1.0.0").unwrap();

        fs::write(&backup.path, "corrupted data").unwrap();

        let is_valid = backup.verify_integrity().unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_backup_integrity_missing() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let binary_dir = temp_dir.path().join("binaries");

        fs::create_dir(&binary_dir).unwrap();
        let binary_path = create_test_binary(&binary_dir, "1.0.0");

        let mut manager = BackupManager::new(backup_dir, 3).unwrap();
        let backup = manager.create_backup(&binary_path, "1.0.0").unwrap();

        fs::remove_file(&backup.path).unwrap();

        let is_valid = backup.verify_integrity().unwrap();
        assert!(!is_valid);
    }
}
