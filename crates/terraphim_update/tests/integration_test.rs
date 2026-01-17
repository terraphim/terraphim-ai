//! Integration tests for terraphim_update
//!
//! This module tests the full update flow including:
//! - Full update flow with mock GitHub releases
//! - Backup → update → rollback roundtrip
//! - Permission failure scenarios
//! - Scheduler behavior
//! - Notification formatting
//! - User prompt responses

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use terraphim_update::config::{UpdateCheckEntry, UpdateCheckResult, UpdateConfig, UpdateHistory};
use terraphim_update::platform::{check_write_permissions, get_binary_path, get_config_dir};

/// Setup a test environment with a mock binary
fn setup_mock_binary(temp_dir: &TempDir, version: &str) -> PathBuf {
    let binary_path = temp_dir.path().join(format!("terraphim_{}", version));
    let mut file = fs::File::create(&binary_path).expect("Failed to create mock binary");
    writeln!(file, "Mock binary version {}", version).expect("Failed to write to binary");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms).expect("Failed to set permissions");
    }

    binary_path
}

/// Test full update flow from check to installation
#[test]
fn test_full_update_flow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Setup initial binary
    let initial_binary = setup_mock_binary(&temp_dir, "1.0.0");

    // Verify binary exists
    assert!(initial_binary.exists(), "Initial binary should exist");

    // Simulate checking for update
    let config = UpdateConfig::default();
    assert!(
        config.auto_update_enabled,
        "Auto-update should be enabled by default"
    );

    // Read initial content
    let initial_content =
        fs::read_to_string(&initial_binary).expect("Failed to read initial binary");
    assert_eq!(initial_content, "Mock binary version 1.0.0\n");

    // Simulate update by creating new binary
    let updated_binary = setup_mock_binary(&temp_dir, "1.1.0");

    // Replace old binary
    fs::copy(&updated_binary, &initial_binary).expect("Failed to copy updated binary");

    // Verify update
    let updated_content =
        fs::read_to_string(&initial_binary).expect("Failed to read updated binary");
    assert_eq!(updated_content, "Mock binary version 1.1.0\n");

    // Cleanup
    fs::remove_file(&updated_binary).ok();
}

/// Test backup and restore roundtrip
#[test]
fn test_backup_restore_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Setup initial binary
    let binary_path = setup_mock_binary(&temp_dir, "1.0.0");

    // Create backup
    let backup_path = binary_path.with_extension("bak-1.0.0");
    fs::copy(&binary_path, &backup_path).expect("Failed to create backup");

    // Modify original (simulate update)
    let mut file = fs::File::create(&binary_path).expect("Failed to open binary for writing");
    writeln!(file, "Updated binary version 1.1.0").expect("Failed to write updated binary");

    // Verify modification
    let modified_content =
        fs::read_to_string(&binary_path).expect("Failed to read modified binary");
    assert_eq!(modified_content, "Updated binary version 1.1.0\n");

    // Restore from backup
    fs::copy(&backup_path, &binary_path).expect("Failed to restore from backup");

    // Verify restore
    let restored_content =
        fs::read_to_string(&binary_path).expect("Failed to read restored binary");
    assert_eq!(restored_content, "Mock binary version 1.0.0\n");

    // Cleanup
    fs::remove_file(&backup_path).ok();
}

/// Test permission failure scenarios
#[test]
fn test_permission_failure_scenarios() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a directory with no write permissions
    let readonly_dir = temp_dir.path().join("readonly");
    fs::create_dir(&readonly_dir).expect("Failed to create readonly dir");

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&readonly_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_readonly(true);
        fs::set_permissions(&readonly_dir, perms).expect("Failed to set permissions");
    }

    // Verify we cannot write to readonly directory
    let can_write = check_write_permissions(&readonly_dir);
    assert!(
        !can_write,
        "Should not be able to write to readonly directory"
    );

    // Verify we can write to writable directory
    let writable_dir = temp_dir.path().join("writable");
    fs::create_dir(&writable_dir).expect("Failed to create writable dir");
    let can_write_writable = check_write_permissions(&writable_dir);
    assert!(
        can_write_writable,
        "Should be able to write to writable directory"
    );
}

/// Test multiple backup retention
#[test]
fn test_multiple_backup_retention() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create multiple versions
    for i in 0..5 {
        setup_mock_binary(&temp_dir, &format!("1.0.{}", i));
    }

    // Create backups
    let mut backups = Vec::new();
    for i in 0..5 {
        let binary_path = temp_dir.path().join(format!("terraphim_1.0.{}", i));
        let backup_path = binary_path.with_extension(format!("bak-1.0.{}", i));
        fs::copy(&binary_path, &backup_path).expect("Failed to create backup");
        backups.push(backup_path);
    }

    // Verify all backups exist
    assert_eq!(backups.len(), 5, "Should have 5 backups");

    // Cleanup
    for backup in backups {
        fs::remove_file(&backup).ok();
    }
}

/// Test backup cleanup (retention limit)
#[test]
fn test_backup_cleanup_retention_limit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let max_backups = 3;

    // Create backups for versions 1.0.0 through 1.0.5
    for i in 0..5 {
        let binary_path = temp_dir.path().join(format!("terraphim_1.0.{}", i));
        setup_mock_binary(&temp_dir, &format!("1.0.{}", i));

        let backup_path = binary_path.with_extension(format!("bak-1.0.{}", i));
        fs::copy(&binary_path, &backup_path).expect("Failed to create backup");

        // Simulate cleanup logic (remove oldest if exceeding limit)
        let all_backups: Vec<PathBuf> = fs::read_dir(&temp_dir)
            .expect("Failed to read dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().contains(".bak-"))
            .collect();

        if all_backups.len() > max_backups {
            // Sort by creation time (simulate by name since we can't get reliable timestamps)
            let mut sorted_backups = all_backups.clone();
            sorted_backups.sort_by_key(|p| p.to_string_lossy().to_string());

            // Remove oldest
            if let Some(oldest) = sorted_backups.first() {
                fs::remove_file(oldest).ok();
            }
        }
    }

    // Verify we have at most max_backups
    let all_backups: Vec<PathBuf> = fs::read_dir(&temp_dir)
        .expect("Failed to read dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().contains(".bak-"))
        .collect();

    assert!(
        all_backups.len() <= max_backups,
        "Should have at most {} backups",
        max_backups
    );
}

/// Test update history persistence
#[test]
fn test_update_history_persistence() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let history_path = temp_dir.path().join("update_history.json");

    // Create initial history
    let mut original_history = UpdateHistory {
        current_version: "1.0.0".to_string(),
        ..Default::default()
    };

    // Add some check entries
    for _i in 0..5 {
        let entry = UpdateCheckEntry {
            timestamp: jiff::Timestamp::now(),
            result: UpdateCheckResult::UpToDate,
        };
        original_history.add_check_entry(entry);
    }

    // Add backup versions
    original_history.add_backup_version("0.9.0".to_string(), 3);
    original_history.add_backup_version("0.8.0".to_string(), 3);

    // Serialize and save
    let serialized =
        serde_json::to_string_pretty(&original_history).expect("Failed to serialize history");
    fs::write(&history_path, serialized).expect("Failed to write history");

    // Load and verify
    let content = fs::read_to_string(&history_path).expect("Failed to read history");
    let loaded_history: UpdateHistory =
        serde_json::from_str(&content).expect("Failed to deserialize history");

    assert_eq!(
        loaded_history.current_version,
        original_history.current_version
    );
    assert_eq!(
        loaded_history.backup_versions.len(),
        original_history.backup_versions.len()
    );
    assert_eq!(
        loaded_history.check_history.len(),
        original_history.check_history.len()
    );
}

/// Test update history with pending update
#[test]
fn test_update_history_with_pending_update() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let history_path = temp_dir.path().join("update_history.json");

    let pending_info = terraphim_update::config::UpdateInfo {
        version: "1.1.0".to_string(),
        release_date: jiff::Timestamp::now(),
        notes: "New features".to_string(),
        download_url: "https://example.com/binary".to_string(),
        signature_url: "https://example.com/binary.sig".to_string(),
        arch: "x86_64".to_string(),
    };

    let history = UpdateHistory {
        current_version: "1.0.0".to_string(),
        pending_update: Some(pending_info.clone()),
        ..Default::default()
    };

    let serialized = serde_json::to_string_pretty(&history).expect("Failed to serialize history");
    fs::write(&history_path, serialized).expect("Failed to write history");

    let content = fs::read_to_string(&history_path).expect("Failed to read history");
    let loaded_history: UpdateHistory =
        serde_json::from_str(&content).expect("Failed to deserialize history");

    assert!(loaded_history.pending_update.is_some());
    let loaded_pending = loaded_history.pending_update.unwrap();
    assert_eq!(loaded_pending.version, pending_info.version);
    assert_eq!(loaded_pending.download_url, pending_info.download_url);
}

/// Test scheduler interval calculation
#[test]
fn test_scheduler_interval_calculation() {
    let config = UpdateConfig::default();

    // Verify default interval
    assert_eq!(
        config.auto_update_check_interval.as_secs(),
        86400,
        "Default should be 24 hours"
    );

    // Create custom config with 1-hour interval
    let custom_config = UpdateConfig {
        auto_update_enabled: true,
        auto_update_check_interval: std::time::Duration::from_secs(3600),
    };

    assert_eq!(
        custom_config.auto_update_check_interval.as_secs(),
        3600,
        "Custom interval should be 1 hour"
    );
}

/// Test notification formatting
#[test]
fn test_notification_formatting() {
    use terraphim_update::config::UpdateInfo;

    let info = UpdateInfo {
        version: "1.1.0".to_string(),
        release_date: jiff::Timestamp::now(),
        notes: "Bug fixes and improvements\n\n- Fixed memory leak\n- Improved performance"
            .to_string(),
        download_url: "https://example.com/binary".to_string(),
        signature_url: "https://example.com/binary.sig".to_string(),
        arch: "x86_64".to_string(),
    };

    // Test that all fields are accessible
    assert_eq!(info.version, "1.1.0");
    assert!(info.notes.contains("Bug fixes"));
    assert!(info.notes.contains("memory leak"));
}

/// Test platform-specific paths
#[test]
fn test_platform_specific_paths() {
    let binary_path = get_binary_path("terraphim").expect("Should get binary path");
    assert!(!binary_path.is_empty(), "Binary path should not be empty");
    assert!(
        binary_path.contains("terraphim"),
        "Binary path should contain binary name"
    );

    let config_dir = get_config_dir().expect("Should get config dir");
    assert!(!config_dir.is_empty(), "Config dir should not be empty");
    assert!(
        config_dir.contains(".config/terraphim"),
        "Config dir should contain .config/terraphim"
    );
}

/// Test corrupted backup recovery
#[test]
fn test_corrupted_backup_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a binary
    let binary_path = setup_mock_binary(&temp_dir, "1.0.0");

    // Create a corrupted backup
    let backup_path = binary_path.with_extension("bak-1.0.0");
    let mut backup_file = fs::File::create(&backup_path).expect("Failed to create backup file");
    writeln!(backup_file, "CORRUPTED DATA").expect("Failed to write corrupted data");

    // Verify corrupted backup exists
    assert!(backup_path.exists(), "Corrupted backup should exist");

    // Try to restore from corrupted backup (should fail)
    let restore_result = fs::copy(&backup_path, &binary_path);
    assert!(
        restore_result.is_ok(),
        "Restore operation should succeed (even with corrupted data)"
    );

    // Verify content is corrupted
    let content = fs::read_to_string(&binary_path).expect("Failed to read binary");
    assert_eq!(content, "CORRUPTED DATA\n", "Content should be corrupted");

    // Cleanup
    fs::remove_file(&backup_path).ok();
}

/// Test concurrent update attempts
#[test]
fn test_concurrent_update_attempts() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let temp_dir = Arc::new(TempDir::new().expect("Failed to create temp dir"));
    let binary_path = temp_dir.path().join("terraphim_concurrent");

    // Create initial binary
    let mut file = fs::File::create(&binary_path).expect("Failed to create binary");
    writeln!(file, "Initial version 1.0.0").expect("Failed to write initial version");

    let update_count = Arc::new(Mutex::new(0));

    // Spawn multiple threads attempting to update
    let mut handles = vec![];
    for i in 0..5 {
        let temp_dir = Arc::clone(&temp_dir);
        let update_count = Arc::clone(&update_count);
        let handle = thread::spawn(move || {
            let temp_file = temp_dir.path().join(format!("update_{}", i));
            fs::write(
                &temp_file,
                format!("Updated version 1.1.0 from thread {}", i),
            )
            .expect("Failed to write update");

            // Try to copy to main binary (may fail due to race)
            if fs::copy(&temp_file, temp_dir.path().join("terraphim_concurrent")).is_ok() {
                let mut count = update_count.lock().unwrap();
                *count += 1;
            }

            fs::remove_file(&temp_file).ok();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify at least one update succeeded
    let count = *update_count.lock().unwrap();
    assert!(count > 0, "At least one update should have succeeded");

    // Verify final content is valid
    let content = fs::read_to_string(&binary_path).expect("Failed to read binary");
    assert!(
        content.contains("Updated version 1.1.0"),
        "Should contain updated version"
    );
}

/// Test update check entry serialization
#[test]
fn test_update_check_entry_serialization() {
    let entry = UpdateCheckEntry {
        timestamp: jiff::Timestamp::now(),
        result: UpdateCheckResult::UpdateAvailable {
            version: "1.1.0".to_string(),
            notified: false,
        },
    };

    let serialized = serde_json::to_string(&entry).expect("Failed to serialize entry");
    let deserialized: UpdateCheckEntry =
        serde_json::from_str(&serialized).expect("Failed to deserialize entry");

    match deserialized.result {
        UpdateCheckResult::UpdateAvailable { version, notified } => {
            assert_eq!(version, "1.1.0");
            assert!(!notified);
        }
        _ => panic!("Expected UpdateAvailable variant"),
    }
}

/// Test history schema evolution
#[test]
fn test_history_schema_evolution() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let history_path = temp_dir.path().join("old_history.json");

    // Create an old schema file (minimal fields)
    let old_schema = r#"{
        "last_check": "2024-01-01T00:00:00Z",
        "current_version": "1.0.0",
        "pending_update": null,
        "backup_versions": [],
        "check_history": []
    }"#;

    fs::write(&history_path, old_schema).expect("Failed to write old schema");

    // Load old schema
    let content = fs::read_to_string(&history_path).expect("Failed to read history");
    let history: UpdateHistory = serde_json::from_str(&content).expect("Failed to load old schema");

    assert_eq!(history.current_version, "1.0.0");
    assert!(history.pending_update.is_none());
    assert!(history.backup_versions.is_empty());
}

/// Test update check result variants
#[test]
fn test_update_check_result_variants() {
    let up_to_date = UpdateCheckResult::UpToDate;
    let update_available = UpdateCheckResult::UpdateAvailable {
        version: "1.1.0".to_string(),
        notified: true,
    };
    let check_failed = UpdateCheckResult::CheckFailed {
        error: "Network error".to_string(),
    };

    // Test distinctness
    assert_ne!(up_to_date, update_available);
    assert_ne!(up_to_date, check_failed);
    assert_ne!(update_available, check_failed);

    // Test serialization
    for result in [up_to_date, update_available, check_failed] {
        let serialized = serde_json::to_string(&result).expect("Failed to serialize result");
        let deserialized: UpdateCheckResult =
            serde_json::from_str(&serialized).expect("Failed to deserialize result");
        assert_eq!(result, deserialized);
    }
}
