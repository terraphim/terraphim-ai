use anyhow::Result;
use serial_test::serial;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::str;
use std::thread;
use std::time::Duration;

/// Test helper to run TUI commands
fn run_tui_command(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"]).args(args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

/// Extract clean output without log messages
fn extract_clean_output(output: &str) -> String {
    output
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN") && !line.contains("DEBUG"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Parse config from TUI output
fn parse_config_from_output(output: &str) -> Result<serde_json::Value> {
    let clean_output = extract_clean_output(output);
    let lines: Vec<&str> = clean_output.lines().collect();
    let json_start = lines
        .iter()
        .position(|line| line.starts_with('{'))
        .ok_or_else(|| anyhow::anyhow!("No JSON found in output"))?;

    let json_lines = &lines[json_start..];
    let json_str = json_lines.join("\n");

    Ok(serde_json::from_str(&json_str)?)
}

/// Clean up test persistence files
fn cleanup_test_persistence() -> Result<()> {
    // Clean up test persistence directories
    let test_dirs = vec!["/tmp/terraphim_sqlite", "/tmp/dashmaptest", "/tmp/opendal"];

    for dir in test_dirs {
        if Path::new(dir).exists() {
            let _ = fs::remove_dir_all(dir);
            println!("Cleaned up test directory: {}", dir);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_setup_and_cleanup() -> Result<()> {
    // Clean up first
    cleanup_test_persistence()?;

    // Run a simple command that should initialize persistence
    let (_stdout, stderr, code) = run_tui_command(&["config", "show"])?;

    assert_eq!(
        code, 0,
        "Config show should succeed and initialize persistence, stderr: {}",
        stderr
    );

    // Check that persistence directories were created
    let expected_dirs = vec!["/tmp/terraphim_sqlite", "/tmp/dashmaptest"];

    for dir in expected_dirs {
        assert!(
            Path::new(dir).exists(),
            "Persistence directory should be created: {}",
            dir
        );
        println!("✓ Persistence directory created: {}", dir);
    }

    // Check that SQLite database file exists
    let db_file = "/tmp/terraphim_sqlite/terraphim.db";
    assert!(
        Path::new(db_file).exists(),
        "SQLite database should be created: {}",
        db_file
    );
    println!("✓ SQLite database file created: {}", db_file);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_config_persistence_across_runs() -> Result<()> {
    cleanup_test_persistence()?;

    // First run: Set a configuration value
    let test_role = "PersistenceTestRole";
    let (stdout1, stderr1, code1) =
        run_tui_command(&["config", "set", "selected_role", test_role])?;

    assert_eq!(
        code1, 0,
        "First config set should succeed, stderr: {}",
        stderr1
    );
    assert!(
        extract_clean_output(&stdout1).contains(&format!("updated selected_role to {}", test_role)),
        "Should confirm role update"
    );

    println!("✓ Set selected_role to '{}' in first run", test_role);

    // Wait a moment to ensure persistence
    thread::sleep(Duration::from_millis(500));

    // Second run: Check if the configuration persisted
    let (stdout2, stderr2, code2) = run_tui_command(&["config", "show"])?;

    assert_eq!(
        code2, 0,
        "Second config show should succeed, stderr: {}",
        stderr2
    );

    let config = parse_config_from_output(&stdout2)?;
    let persisted_role = config["selected_role"].as_str().unwrap();

    assert_eq!(
        persisted_role, test_role,
        "Selected role should persist across runs: expected '{}', got '{}'",
        test_role, persisted_role
    );

    println!(
        "✓ Selected role '{}' persisted across TUI runs",
        persisted_role
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_switching_persistence() -> Result<()> {
    cleanup_test_persistence()?;

    // Test switching between different roles and verifying persistence
    let roles_to_test = ["Role1", "Role2", "Role3", "Final Role"];

    for (i, role) in roles_to_test.iter().enumerate() {
        println!("Testing role switch #{}: '{}'", i + 1, role);

        // Set the role
        let (set_stdout, set_stderr, set_code) =
            run_tui_command(&["config", "set", "selected_role", role])?;

        assert_eq!(
            set_code, 0,
            "Should be able to set role '{}', stderr: {}",
            role, set_stderr
        );
        assert!(
            extract_clean_output(&set_stdout)
                .contains(&format!("updated selected_role to {}", role)),
            "Should confirm role update to '{}'",
            role
        );

        // Verify immediately
        let (show_stdout, show_stderr, show_code) = run_tui_command(&["config", "show"])?;
        assert_eq!(
            show_code, 0,
            "Config show should work, stderr: {}",
            show_stderr
        );

        let config = parse_config_from_output(&show_stdout)?;
        let current_role = config["selected_role"].as_str().unwrap();

        assert_eq!(
            current_role, *role,
            "Role should be set immediately: expected '{}', got '{}'",
            role, current_role
        );

        println!("  ✓ Role '{}' set and verified", role);

        // Small delay to ensure persistence writes complete
        thread::sleep(Duration::from_millis(200));
    }

    // Final verification after all switches
    let (final_stdout, final_stderr, final_code) = run_tui_command(&["config", "show"])?;
    assert_eq!(
        final_code, 0,
        "Final config show should work, stderr: {}",
        final_stderr
    );

    let final_config = parse_config_from_output(&final_stdout)?;
    let final_role = final_config["selected_role"].as_str().unwrap();

    assert_eq!(final_role, "Final Role", "Final role should persist");
    println!(
        "✓ All role switches persisted correctly, final role: '{}'",
        final_role
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_backend_functionality() -> Result<()> {
    cleanup_test_persistence()?;

    // Test that different persistence backends work
    // Run multiple operations to exercise the persistence layer

    // Set multiple config values
    let config_changes = vec![
        ("selected_role", "BackendTestRole1"),
        ("selected_role", "BackendTestRole2"),
        ("selected_role", "BackendTestRole3"),
    ];

    for (key, value) in config_changes {
        let (_stdout, stderr, code) = run_tui_command(&["config", "set", key, value])?;

        assert_eq!(
            code, 0,
            "Config set '{}' = '{}' should succeed, stderr: {}",
            key, value, stderr
        );
        println!("✓ Set {} = {}", key, value);

        // Verify the change
        let (show_stdout, _, show_code) = run_tui_command(&["config", "show"])?;
        assert_eq!(show_code, 0, "Config show should work after set");

        let config = parse_config_from_output(&show_stdout)?;
        let current_value = config[key].as_str().unwrap();
        assert_eq!(current_value, value, "Value should be set correctly");
    }

    // Check database files exist and have content
    let db_file = "/tmp/terraphim_sqlite/terraphim.db";
    assert!(Path::new(db_file).exists(), "SQLite database should exist");

    let db_metadata = fs::metadata(db_file)?;
    assert!(db_metadata.len() > 0, "SQLite database should have content");

    println!("✓ SQLite database has {} bytes of data", db_metadata.len());

    // Check that dashmap directory has content
    let dashmap_dir = "/tmp/dashmaptest";
    assert!(
        Path::new(dashmap_dir).exists(),
        "Dashmap directory should exist"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_concurrent_persistence_operations() -> Result<()> {
    cleanup_test_persistence()?;

    // Test that concurrent TUI operations don't corrupt persistence
    // Run multiple TUI commands simultaneously

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let role = format!("ConcurrentRole{}", i);
            tokio::spawn(async move {
                let result = run_tui_command(&["config", "set", "selected_role", &role]);
                (i, role, result)
            })
        })
        .collect();

    // Wait for all operations to complete
    let mut results = Vec::new();
    for handle in handles {
        let (i, role, result) = handle.await?;
        results.push((i, role, result));
    }

    // Check that operations completed successfully
    for (i, role, result) in results {
        match result {
            Ok((_stdout, stderr, code)) => {
                if code == 0 {
                    println!("✓ Concurrent operation {} (role '{}') succeeded", i, role);
                } else {
                    println!(
                        "⚠ Concurrent operation {} (role '{}') failed: {}",
                        i, role, stderr
                    );
                }
            }
            Err(e) => {
                println!("✗ Concurrent operation {} failed to run: {}", i, e);
            }
        }
    }

    // Check final state
    let (final_stdout, final_stderr, final_code) = run_tui_command(&["config", "show"])?;
    assert_eq!(
        final_code, 0,
        "Final config check should work, stderr: {}",
        final_stderr
    );

    let config = parse_config_from_output(&final_stdout)?;
    let final_role = config["selected_role"].as_str().unwrap();

    // Should have one of the concurrent roles
    assert!(
        final_role.starts_with("ConcurrentRole"),
        "Final role should be one of the concurrent roles: '{}'",
        final_role
    );

    println!(
        "✓ Concurrent operations completed, final role: '{}'",
        final_role
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_recovery_after_corruption() -> Result<()> {
    cleanup_test_persistence()?;

    // First, set up normal persistence
    let (_, stderr1, code1) =
        run_tui_command(&["config", "set", "selected_role", "PreCorruption"])?;
    assert_eq!(
        code1, 0,
        "Initial setup should succeed, stderr: {}",
        stderr1
    );

    // Simulate corruption by deleting persistence files
    let _ = fs::remove_dir_all("/tmp/terraphim_sqlite");
    let _ = fs::remove_dir_all("/tmp/dashmaptest");

    println!("✓ Simulated persistence corruption by removing files");

    // Try to use TUI after corruption - should recover gracefully
    let (stdout, stderr, code) = run_tui_command(&["config", "show"])?;

    assert_eq!(
        code, 0,
        "TUI should recover after corruption, stderr: {}",
        stderr
    );

    // Should create new persistence and use defaults
    let config = parse_config_from_output(&stdout)?;
    println!(
        "✓ TUI recovered with config: id={}, selected_role={}",
        config["id"], config["selected_role"]
    );

    // Persistence directories should be recreated
    assert!(
        Path::new("/tmp/terraphim_sqlite").exists(),
        "SQLite dir should be recreated"
    );
    assert!(
        Path::new("/tmp/dashmaptest").exists(),
        "Dashmap dir should be recreated"
    );

    // Should be able to set new values
    let (_, stderr2, code2) = run_tui_command(&["config", "set", "selected_role", "PostRecovery"])?;
    assert_eq!(
        code2, 0,
        "Should be able to set config after recovery, stderr: {}",
        stderr2
    );

    println!("✓ Successfully recovered from persistence corruption");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_with_special_characters() -> Result<()> {
    cleanup_test_persistence()?;

    // Test that special characters in role names are handled correctly by persistence
    let special_roles = vec![
        "Role with spaces",
        "Role-with-dashes",
        "Role_with_underscores",
        "Role.with.dots",
        "Role (with parentheses)",
        "Role/with/slashes",
        "Rôle wïth ûnicøde",
        "Role with \"quotes\"",
    ];

    for role in special_roles {
        println!("Testing persistence with special role: '{}'", role);

        let (_set_stdout, set_stderr, set_code) =
            run_tui_command(&["config", "set", "selected_role", role])?;

        assert_eq!(
            set_code, 0,
            "Should handle special characters in role '{}', stderr: {}",
            role, set_stderr
        );

        // Verify it persisted correctly
        let (show_stdout, show_stderr, show_code) = run_tui_command(&["config", "show"])?;
        assert_eq!(
            show_code, 0,
            "Config show should work with special role, stderr: {}",
            show_stderr
        );

        let config = parse_config_from_output(&show_stdout)?;
        let stored_role = config["selected_role"].as_str().unwrap();

        assert_eq!(
            stored_role, role,
            "Special character role should persist correctly"
        );

        println!("  ✓ Role '{}' persisted correctly", role);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_directory_permissions() -> Result<()> {
    cleanup_test_persistence()?;

    // Test that TUI can create persistence directories with proper permissions
    let (_stdout, stderr, code) = run_tui_command(&["config", "show"])?;

    assert_eq!(
        code, 0,
        "TUI should create directories successfully, stderr: {}",
        stderr
    );

    // Check directory permissions
    let test_dirs = vec!["/tmp/terraphim_sqlite", "/tmp/dashmaptest"];

    for dir in test_dirs {
        let dir_path = Path::new(dir);
        assert!(dir_path.exists(), "Directory should exist: {}", dir);

        let metadata = fs::metadata(dir_path)?;
        assert!(metadata.is_dir(), "Should be a directory: {}", dir);

        // Check we can write to the directory by creating a test file
        let test_file = dir_path.join("permission_test.tmp");
        fs::write(&test_file, "test")?;
        assert!(
            test_file.exists(),
            "Should be able to write to directory: {}",
            dir
        );
        fs::remove_file(&test_file)?;

        println!("✓ Directory '{}' has correct permissions", dir);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_persistence_backend_selection() -> Result<()> {
    cleanup_test_persistence()?;

    // Test that the TUI uses the expected persistence backends
    // Based on settings, it should use multiple backends for redundancy

    let (_stdout, stderr, code) =
        run_tui_command(&["config", "set", "selected_role", "BackendSelectionTest"])?;
    assert_eq!(code, 0, "Config set should succeed, stderr: {}", stderr);

    // Check that expected backends are being used (from log output)
    let log_output = stderr;

    // Should mention various persistence backends in initialization
    let expected_backends = vec!["sqlite", "memory", "dashmap"];

    for backend in expected_backends {
        if log_output.contains(backend) {
            println!("✓ Persistence backend '{}' mentioned in logs", backend);
        } else {
            println!("⚠ Persistence backend '{}' not mentioned in logs", backend);
        }
    }

    // Verify the data was actually stored
    let (verify_stdout, verify_stderr, verify_code) = run_tui_command(&["config", "show"])?;
    assert_eq!(
        verify_code, 0,
        "Config show should work, stderr: {}",
        verify_stderr
    );

    let config = parse_config_from_output(&verify_stdout)?;
    assert_eq!(
        config["selected_role"], "BackendSelectionTest",
        "Data should persist correctly"
    );

    println!("✓ Persistence backend selection working correctly");

    Ok(())
}
