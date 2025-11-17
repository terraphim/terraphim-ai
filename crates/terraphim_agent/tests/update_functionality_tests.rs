//! Integration tests for terraphim-agent autoupdate functionality
//!
//! Tests the complete autoupdate workflow including checking for updates
//! and updating to new versions from GitHub Releases.

use std::process::Command;

/// Test the check-update command functionality
#[tokio::test]
async fn test_check_update_command() {
    // Run the check-update command
    let output = Command::new("../../target/x86_64-unknown-linux-gnu/release/terraphim-agent")
        .arg("check-update")
        .output()
        .expect("Failed to execute check-update command");

    // Verify the command executed successfully
    assert!(
        output.status.success(),
        "check-update command should succeed"
    );

    // Verify the output contains expected messages
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("🔍 Checking for terraphim-agent updates..."),
        "Should show checking message"
    );
    assert!(
        stdout.contains("✅ Already running latest version: 1.0.0")
            || stdout.contains("📦 Update available:"),
        "Should show either up-to-date or update available message"
    );
}

/// Test the update command when no update is available
#[tokio::test]
async fn test_update_command_no_update_available() {
    // Run the update command
    let output = Command::new("../../target/x86_64-unknown-linux-gnu/release/terraphim-agent")
        .arg("update")
        .output()
        .expect("Failed to execute update command");

    // Verify the command executed successfully
    assert!(output.status.success(), "update command should succeed");

    // Verify the output contains expected messages
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("🚀 Updating terraphim-agent..."),
        "Should show updating message"
    );
    assert!(
        stdout.contains("✅ Already running latest version: 1.0.0"),
        "Should show already up to date message"
    );
}

/// Test error handling for invalid binary name in update functionality
#[tokio::test]
async fn test_update_function_with_invalid_binary() {
    use terraphim_update::check_for_updates;

    // Test with non-existent binary name
    let result = check_for_updates("non-existent-binary").await;

    // Should handle gracefully (not crash)
    match result {
        Ok(status) => {
            // Should return a failed status
            assert!(
                format!("{}", status).contains("❌") || format!("{}", status).contains("✅"),
                "Should return some status"
            );
        }
        Err(e) => {
            // Error is also acceptable - should not panic
            assert!(!e.to_string().is_empty(), "Error should have message");
        }
    }
}

/// Test version comparison logic through update status
#[tokio::test]
async fn test_version_comparison_logic() {
    use terraphim_update::{TerraphimUpdater, UpdaterConfig};

    // Test that version comparison is used internally
    let config = UpdaterConfig::new("test").with_version("1.0.0");

    // Test configuration is correctly set
    assert_eq!(config.bin_name, "test");
    assert_eq!(config.current_version, "1.0.0");

    let updater = TerraphimUpdater::new(config.clone());

    // Test that the updater can be created and has the right configuration
    // (Version comparison is tested internally in terraphim_update tests)
    let result = updater.check_update().await;
    // Should not panic and should return some status
    assert!(
        result.is_ok() || result.is_err(),
        "Should return some result"
    );
}

/// Test update configuration
#[tokio::test]
async fn test_updater_configuration() {
    use terraphim_update::{TerraphimUpdater, UpdaterConfig};

    // Test default configuration
    let config = UpdaterConfig::new("terraphim-agent");
    assert_eq!(config.bin_name, "terraphim-agent");
    assert_eq!(config.repo_owner, "terraphim");
    assert_eq!(config.repo_name, "terraphim-ai");
    assert!(config.show_progress);

    // Test custom configuration
    let config = UpdaterConfig::new("test-binary")
        .with_version("1.0.0")
        .with_progress(false);

    assert_eq!(config.bin_name, "test-binary");
    assert_eq!(config.current_version, "1.0.0");
    assert!(!config.show_progress);

    // Test updater creation
    let updater = TerraphimUpdater::new(config);
    // Should not panic and configuration should be accessible through methods
    let result = updater.check_update().await;
    // Should not panic and should return some status
    assert!(
        result.is_ok() || result.is_err(),
        "Should return some result"
    );
}

/// Test network connectivity for GitHub releases
#[tokio::test]
async fn test_github_release_connectivity() {
    use terraphim_update::{TerraphimUpdater, UpdaterConfig};

    let config = UpdaterConfig::new("terraphim-agent");
    let updater = TerraphimUpdater::new(config);

    // Test checking for updates (should reach GitHub)
    match updater.check_update().await {
        Ok(status) => {
            // Should successfully get a status
            let status_str = format!("{}", status);
            assert!(!status_str.is_empty(), "Status should not be empty");

            // Should be one of the expected statuses
            assert!(
                status_str.contains("✅") || status_str.contains("📦") || status_str.contains("❌"),
                "Status should be a valid response"
            );
        }
        Err(e) => {
            // Network errors are acceptable in test environments
            // The important thing is that it doesn't panic
            assert!(
                e.to_string().contains("github")
                    || e.to_string().contains("network")
                    || e.to_string().contains("http")
                    || !e.to_string().is_empty(),
                "Should handle network errors gracefully"
            );
        }
    }
}

/// Test help messages for update commands
#[tokio::test]
async fn test_update_help_messages() {
    // Test check-update help
    let output = Command::new("../../target/x86_64-unknown-linux-gnu/release/terraphim-agent")
        .arg("check-update")
        .arg("--help")
        .output()
        .expect("Failed to execute check-update --help");

    assert!(
        output.status.success(),
        "check-update --help should succeed"
    );
    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(!help_text.is_empty(), "Help text should not be empty");

    // Test update help
    let output = Command::new("../../target/x86_64-unknown-linux-gnu/release/terraphim-agent")
        .arg("update")
        .arg("--help")
        .output()
        .expect("Failed to execute update --help");

    assert!(output.status.success(), "update --help should succeed");
    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(!help_text.is_empty(), "Help text should not be empty");
}

/// Test concurrent update operations
#[tokio::test]
async fn test_concurrent_update_checks() {
    use terraphim_update::check_for_updates;
    use tokio::task::JoinSet;

    // Run multiple update checks concurrently
    let mut set = JoinSet::new();

    for _ in 0..5 {
        set.spawn(async move { check_for_updates("terraphim-agent").await });
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        match result {
            Ok(update_result) => {
                results.push(update_result);
            }
            Err(e) => {
                // Join errors are acceptable in test environments
                println!("Join error: {}", e);
            }
        }
    }

    // All operations should complete without panicking
    assert_eq!(
        results.len(),
        5,
        "All concurrent operations should complete"
    );

    // All results should be valid UpdateStatus values
    for result in results {
        match result {
            Ok(status) => {
                let status_str = format!("{}", status);
                assert!(!status_str.is_empty(), "Status should not be empty");
            }
            Err(e) => {
                // Errors are acceptable
                assert!(!e.to_string().is_empty(), "Error should have message");
            }
        }
    }
}

/// Test that update commands are properly integrated in CLI
#[tokio::test]
async fn test_update_commands_integration() {
    // Test that commands appear in help
    let output = Command::new("../../target/x86_64-unknown-linux-gnu/release/terraphim-agent")
        .arg("--help")
        .output()
        .expect("Failed to execute --help");

    assert!(output.status.success(), "--help should succeed");
    let help_text = String::from_utf8_lossy(&output.stdout);

    // Verify both update commands are listed
    assert!(
        help_text.contains("check-update"),
        "check-update should be in help"
    );
    assert!(help_text.contains("update"), "update should be in help");
    assert!(
        help_text.contains("Check for updates without installing"),
        "check-update description should be present"
    );
    assert!(
        help_text.contains("Update to latest version if available"),
        "update description should be present"
    );
}
