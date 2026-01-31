//! State persistence for update history
//!
//! This module handles saving and loading update history to disk,
//! providing persistent state management for the auto-update system.

use anyhow::{Context, Result};
use serde_json;
use std::fs;
use std::path::PathBuf;

/// Get the default path for update history storage
///
/// Returns `~/.config/terraphim/update_history.json` on Unix systems
/// and appropriate location on other platforms.
pub fn get_history_path() -> Result<PathBuf> {
    let mut config_dir = dirs::config_dir().context("Could not determine config directory")?;

    config_dir.push("terraphim");
    fs::create_dir_all(&config_dir).context("Could not create terraphim config directory")?;

    Ok(config_dir.join("update_history.json"))
}

/// Save update history to disk
///
/// # Arguments
/// * `history` - The UpdateHistory to save
///
/// # Returns
/// * `Ok(())` - Successfully saved
/// * `Err(anyhow::Error)` - Error during save
///
/// # Example
/// ```no_run
/// use terraphim_update::config::UpdateHistory;
/// use terraphim_update::state::save_update_history;
///
/// let history = UpdateHistory::default();
/// save_update_history(&history)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn save_update_history(history: &crate::config::UpdateHistory) -> Result<()> {
    let path = get_history_path()?;

    let serialized =
        serde_json::to_string_pretty(history).context("Failed to serialize update history")?;

    fs::write(&path, serialized)
        .context(format!("Failed to write update history to {:?}", path))?;

    tracing::debug!("Update history saved to {:?}", path);
    Ok(())
}

/// Load update history from disk
///
/// Returns a default UpdateHistory if the file doesn't exist.
///
/// # Returns
/// * `Ok(UpdateHistory)` - Loaded or default history
/// * `Err(anyhow::Error)` - Error during load
///
/// # Example
/// ```no_run
/// use terraphim_update::state::load_update_history;
///
/// let history = load_update_history()?;
/// println!("Current version: {}", history.current_version);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn load_update_history() -> Result<crate::config::UpdateHistory> {
    let path = get_history_path()?;

    if !path.exists() {
        tracing::debug!("Update history file not found, using defaults");
        return Ok(crate::config::UpdateHistory::default());
    }

    let content = fs::read_to_string(&path)
        .context(format!("Failed to read update history from {:?}", path))?;

    let history: crate::config::UpdateHistory =
        serde_json::from_str(&content).context("Failed to deserialize update history")?;

    tracing::debug!("Update history loaded from {:?}", path);
    Ok(history)
}

/// Delete update history file
///
/// # Returns
/// * `Ok(())` - Successfully deleted or didn't exist
/// * `Err(anyhow::Error)` - Error during deletion
pub fn delete_update_history() -> Result<()> {
    let path = get_history_path()?;

    if path.exists() {
        fs::remove_file(&path).context(format!("Failed to delete update history at {:?}", path))?;
        tracing::debug!("Update history deleted from {:?}", path);
    } else {
        tracing::debug!("Update history file not found, nothing to delete");
    }

    Ok(())
}

/// Check if update history file exists
///
/// # Returns
/// * `true` - File exists
/// * `false` - File doesn't exist
pub fn history_exists() -> bool {
    match get_history_path() {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{UpdateCheckEntry, UpdateCheckResult, UpdateHistory};
    use serial_test::serial;
    use tempfile::TempDir;

    /// Helper to create a temporary config directory for testing
    fn setup_temp_config_dir() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let xdg_config_home = temp_dir.path().join(".config");
        // SAFETY: This test runs in isolation before spawning other threads
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("USERPROFILE", temp_dir.path());
            std::env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
            std::env::set_var("APPDATA", temp_dir.path());
            std::env::set_var("LOCALAPPDATA", temp_dir.path());
        }
        let config_path = temp_dir.path().join("terraphim");
        fs::create_dir_all(&config_path).expect("Failed to create config dir");
        temp_dir
    }

    #[test]
    #[serial]
    fn test_save_and_load_history() {
        let _temp_dir = setup_temp_config_dir();
        // Clean up first
        let _ = delete_update_history();

        let original = UpdateHistory {
            current_version: "1.0.0".to_string(),
            ..Default::default()
        };

        save_update_history(&original).expect("Failed to save history");
        assert!(history_exists(), "History file should exist after save");

        let loaded = load_update_history().expect("Failed to load history");
        assert_eq!(loaded.current_version, "1.0.0");
    }

    #[test]
    #[serial]
    fn test_load_default_when_missing() {
        let _temp_dir = setup_temp_config_dir();
        // Clean up any existing history file first
        let _ = delete_update_history();

        // Now test loading when file doesn't exist
        let history = load_update_history().expect("Should load default");
        assert_eq!(history.current_version, String::new());
        assert!(history.backup_versions.is_empty());
    }

    #[test]
    #[serial]
    fn test_history_with_check_entries() {
        let _temp_dir = setup_temp_config_dir();
        let _ = delete_update_history();

        let mut history = UpdateHistory {
            current_version: "1.0.0".to_string(),
            ..Default::default()
        };

        let entry = UpdateCheckEntry {
            timestamp: jiff::Timestamp::now(),
            result: UpdateCheckResult::UpToDate,
        };
        history.add_check_entry(entry);

        save_update_history(&history).expect("Failed to save history");

        let loaded = load_update_history().expect("Failed to load history");
        assert_eq!(loaded.check_history.len(), 1);
    }

    #[test]
    #[serial]
    fn test_history_with_pending_update() {
        let _temp_dir = setup_temp_config_dir();
        let _ = delete_update_history();

        let info = crate::config::UpdateInfo {
            version: "1.1.0".to_string(),
            release_date: jiff::Timestamp::now(),
            notes: "Test release".to_string(),
            download_url: "https://example.com/binary".to_string(),
            signature_url: "https://example.com/binary.sig".to_string(),
            arch: "x86_64".to_string(),
        };

        let history = UpdateHistory {
            current_version: "1.0.0".to_string(),
            pending_update: Some(info),
            ..Default::default()
        };

        save_update_history(&history).expect("Failed to save history");

        let loaded = load_update_history().expect("Failed to load history");
        assert!(loaded.pending_update.is_some());
        assert_eq!(loaded.pending_update.unwrap().version, "1.1.0");
    }

    #[test]
    #[serial]
    fn test_history_with_backups() {
        let _temp_dir = setup_temp_config_dir();
        let _ = delete_update_history();

        let mut history = UpdateHistory {
            current_version: "1.0.0".to_string(),
            ..Default::default()
        };
        history.add_backup_version("0.9.0".to_string(), 3);
        history.add_backup_version("0.8.0".to_string(), 3);

        save_update_history(&history).expect("Failed to save history");

        let loaded = load_update_history().expect("Failed to load history");
        assert_eq!(loaded.backup_versions.len(), 2);
        assert_eq!(loaded.backup_versions[0], "0.9.0");
        assert_eq!(loaded.backup_versions[1], "0.8.0");
    }

    #[test]
    #[serial]
    fn test_delete_history() {
        let _temp_dir = setup_temp_config_dir();
        let _ = delete_update_history();

        let history = UpdateHistory {
            current_version: "1.0.0".to_string(),
            ..Default::default()
        };

        save_update_history(&history).expect("Failed to save history");
        assert!(history_exists(), "History should exist");

        delete_update_history().expect("Failed to delete history");
        assert!(!history_exists(), "History should not exist after delete");
    }

    #[test]
    #[serial]
    fn test_delete_nonexistent_history() {
        let _temp_dir = setup_temp_config_dir();
        let result = delete_update_history();
        assert!(
            result.is_ok(),
            "Deleting non-existent history should succeed"
        );
    }

    #[test]
    #[serial]
    fn test_invalid_json_fails_gracefully() {
        let _temp_dir = setup_temp_config_dir();

        let path = get_history_path().expect("Failed to get history path");
        fs::write(&path, "invalid json").expect("Failed to write invalid json");

        let result = load_update_history();
        assert!(result.is_err(), "Should fail to load invalid JSON");
    }

    #[test]
    #[serial]
    fn test_schema_migration_compatibility() {
        let _temp_dir = setup_temp_config_dir();

        // Create a history file with only current_version (simulating old schema)
        let old_schema = r#"{
            "last_check": "2024-01-01T00:00:00Z",
            "current_version": "1.0.0",
            "pending_update": null,
            "backup_versions": [],
            "check_history": []
        }"#;

        let path = get_history_path().expect("Failed to get history path");
        fs::write(&path, old_schema).expect("Failed to write old schema");

        let history = load_update_history().expect("Should load old schema");
        assert_eq!(history.current_version, "1.0.0");
        assert!(history.pending_update.is_none());
    }

    #[test]
    #[serial]
    fn test_get_history_path() {
        let _temp_dir = setup_temp_config_dir();
        let path = get_history_path();
        assert!(path.is_ok(), "Should get history path");

        let path = path.unwrap();
        assert!(
            path.ends_with("update_history.json"),
            "Should have correct filename"
        );
        assert!(
            path.to_string_lossy().contains("terraphim"),
            "Should contain terraphim directory"
        );
    }
}
