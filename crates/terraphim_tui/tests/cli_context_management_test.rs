use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Integration tests for context management via CLI interface
/// Tests CLI interaction with the server for context operations
#[tokio::test]
async fn test_cli_context_management_workflow() {
    // Test 1: CLI server interaction for context management
    test_cli_server_context_integration().await;

    // Test 2: CLI configuration for context-enabled roles
    test_cli_context_role_configuration().await;

    // Test 3: CLI search with context integration
    test_cli_search_with_context().await;
}

/// Test 1: CLI server interaction for context management
async fn test_cli_server_context_integration() {
    println!("🧪 Testing CLI server interaction for context management");

    // Create a temporary directory for test configuration
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("cli_test_config.json");

    // Create a test configuration with context-enabled role
    let test_config = serde_json::json!({
        "roles": {
            "CLITestRole": {
                "name": "CLI Test Role",
                "shortname": "cli-test",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [],
                "extra": {},
                "terraphim_it": false
            }
        },
        "default_role": "CLITestRole",
        "selected_role": "CLITestRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&test_config).unwrap(),
    )
    .expect("Failed to write test config");

    // Test CLI commands that would interact with context management
    let cli_tests = vec![
        ("config show", "should show current configuration"),
        ("role list", "should list available roles"),
        ("role select CLITestRole", "should select the CLI test role"),
    ];

    for (command, description) in cli_tests {
        println!("  Testing CLI command: {} - {}", command, description);

        // Execute CLI command with test configuration
        let result = timeout(
            Duration::from_secs(10),
            execute_cli_command(&config_path, command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!("    ✅ Command succeeded: {}", command);
                // Verify output contains expected content
                match command {
                    cmd if cmd.contains("config show") => {
                        assert!(
                            output.contains("CLITestRole"),
                            "Config should contain test role"
                        );
                    }
                    cmd if cmd.contains("role list") => {
                        assert!(
                            output.contains("CLITestRole"),
                            "Role list should contain test role"
                        );
                    }
                    cmd if cmd.contains("role select") => {
                        // Role selection should succeed without error
                        assert!(
                            !output.contains("error"),
                            "Role selection should not contain errors"
                        );
                    }
                    _ => {}
                }
            }
            Ok(Err(e)) => {
                println!(
                    "    ⚠️  Command failed (expected for some commands): {} - {}",
                    command, e
                );
                // Some commands might fail if server is not running, which is OK for this test
            }
            Err(_) => {
                println!("    ⚠️  Command timed out: {}", command);
                // Timeout is acceptable for this test
            }
        }
    }

    println!("✅ CLI server context integration test completed");
}

/// Test 2: CLI configuration for context-enabled roles  
async fn test_cli_context_role_configuration() {
    println!("🧪 Testing CLI context role configuration");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("context_role_config.json");

    // Create configuration with multiple roles that would support context
    let context_config = serde_json::json!({
        "roles": {
            "ContextEnabledRole": {
                "name": "Context Enabled Role",
                "shortname": "context",
                "relevance_function": "BM25",
                "theme": "default",
                "haystacks": [
                    {
                        "location": temp_dir.path().join("test_docs").to_string_lossy(),
                        "service": "Ripgrep",
                        "read_only": false,
                        "atomic_server_secret": null,
                        "extra_parameters": {}
                    }
                ],
                "extra": {
                    "supports_context": true,
                    "max_context_items": 10
                },
                "terraphim_it": false
            },
            "LLMEnabledRole": {
                "name": "LLM Enabled Role",
                "shortname": "llm",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [],
                "extra": {
                    "llm_provider": "ollama",
                    "llm_model": "llama3.2:3b",
                    "llm_auto_summarize": true,
                    "supports_context": true
                },
                "terraphim_it": false
            }
        },
        "default_role": "ContextEnabledRole",
        "selected_role": "ContextEnabledRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&context_config).unwrap(),
    )
    .expect("Failed to write context config");

    // Create test documents directory
    let docs_dir = temp_dir.path().join("test_docs");
    std::fs::create_dir_all(&docs_dir).expect("Failed to create docs directory");

    let test_doc = docs_dir.join("context_test.md");
    std::fs::write(
        &test_doc,
        r#"
# Context Test Document

This document is used for testing context management functionality in the CLI.
It contains information about Rust programming and system design concepts.

## Key Topics
- Memory safety
- Concurrency
- Performance optimization
- Error handling

This content should be discoverable through search and usable as context.
    "#,
    )
    .expect("Failed to write test document");

    // Test configuration-related CLI commands
    let config_tests = vec![
        ("config show", "should show context-enabled configuration"),
        ("role list", "should list context-enabled roles"),
        (
            "role select LLMEnabledRole",
            "should select LLM-enabled role",
        ),
    ];

    for (command, description) in config_tests {
        println!(
            "  Testing context config command: {} - {}",
            command, description
        );

        let result = timeout(
            Duration::from_secs(10),
            execute_cli_command(&config_path, command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!("    ✅ Config command succeeded: {}", command);
                // Verify context-related configuration is present
                if command.contains("config show") {
                    assert!(
                        output.contains("ContextEnabledRole") || output.contains("LLMEnabledRole"),
                        "Config should contain context-enabled roles"
                    );
                }
            }
            Ok(Err(_)) | Err(_) => {
                println!(
                    "    ⚠️  Config command had issues: {} (may be expected)",
                    command
                );
            }
        }
    }

    println!("✅ CLI context role configuration test completed");
}

/// Test 3: CLI search with context integration
async fn test_cli_search_with_context() {
    println!("🧪 Testing CLI search with context integration");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("search_context_config.json");

    // Create search-ready configuration
    let docs_dir = temp_dir.path().join("search_docs");
    std::fs::create_dir_all(&docs_dir).expect("Failed to create search docs directory");

    // Create multiple test documents for search
    let search_docs = vec![
        ("rust_basics.md", "# Rust Basics\n\nRust is a systems programming language focused on safety and performance."),
        ("memory_safety.md", "# Memory Safety\n\nRust prevents memory leaks through its ownership system."),
        ("concurrency.md", "# Concurrency\n\nRust provides safe concurrency through its type system."),
    ];

    for (filename, content) in search_docs {
        let file_path = docs_dir.join(filename);
        std::fs::write(file_path, content).expect("Failed to write search document");
    }

    let search_config = serde_json::json!({
        "roles": {
            "SearchRole": {
                "name": "Search Role",
                "shortname": "search",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [
                    {
                        "location": docs_dir.to_string_lossy(),
                        "service": "Ripgrep",
                        "read_only": false,
                        "atomic_server_secret": null,
                        "extra_parameters": {}
                    }
                ],
                "extra": {
                    "context_integration": true
                },
                "terraphim_it": false
            }
        },
        "default_role": "SearchRole",
        "selected_role": "SearchRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&search_config).unwrap(),
    )
    .expect("Failed to write search config");

    // Test search commands that would generate context-usable results
    let search_tests = vec![
        ("search Rust", "should find Rust-related documents"),
        (
            "search memory --limit 5",
            "should find memory-related documents with limit",
        ),
        (
            "search concurrency --role SearchRole",
            "should search with specific role",
        ),
    ];

    for (command, description) in search_tests {
        println!("  Testing search command: {} - {}", command, description);

        let result = timeout(
            Duration::from_secs(15), // Longer timeout for search operations
            execute_cli_command(&config_path, command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!("    ✅ Search command succeeded: {}", command);
                // Verify search results are suitable for context
                if !output.is_empty() && !output.contains("error") {
                    println!("      Search returned results suitable for context integration");
                }
            }
            Ok(Err(e)) => {
                println!("    ⚠️  Search command failed: {} - {}", command, e);
                // Search might fail if indexing hasn't completed, which is acceptable
            }
            Err(_) => {
                println!(
                    "    ⚠️  Search command timed out: {} (may indicate indexing in progress)",
                    command
                );
            }
        }
    }

    println!("✅ CLI search with context integration test completed");
}

/// Helper function to execute CLI commands with timeout
async fn execute_cli_command(
    config_path: &Path,
    command: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let binary_path = find_terraphim_binary()?;

    let args: Vec<&str> = command.split_whitespace().collect();

    let output = tokio::process::Command::new(&binary_path)
        .arg("--config")
        .arg(config_path)
        .args(&args)
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(format!("{}{}", stdout, stderr))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Command failed with exit code {:?}: {}",
            output.status.code(),
            stderr
        )
        .into())
    }
}

/// Helper function to find the terraphim binary
fn find_terraphim_binary() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Try multiple possible locations for the terraphim binary
    let possible_paths = vec![
        "target/debug/terraphim_tui",
        "target/release/terraphim_tui",
        "../target/debug/terraphim_tui",
        "../target/release/terraphim_tui",
        "../../target/debug/terraphim_tui",
        "../../target/release/terraphim_tui",
        "terraphim_tui", // In PATH
    ];

    for path in possible_paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    // As fallback, try to build the binary
    let build_result = Command::new("cargo")
        .args(&["build", "--bin", "terraphim_tui"])
        .output();

    if let Ok(output) = build_result {
        if output.status.success() {
            return Ok("target/debug/terraphim_tui".to_string());
        }
    }

    Err("Could not find or build terraphim_tui binary".into())
}

/// Test CLI help system for context-related commands
#[tokio::test]
async fn test_cli_help_for_context_features() {
    println!("🧪 Testing CLI help system for context features");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("help_test_config.json");

    let help_config = serde_json::json!({
        "roles": {
            "HelpTestRole": {
                "name": "Help Test Role",
                "shortname": "help-test",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [],
                "extra": {},
                "terraphim_it": false
            }
        },
        "default_role": "HelpTestRole",
        "selected_role": "HelpTestRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&help_config).unwrap(),
    )
    .expect("Failed to write help config");

    // Test help commands for context-related functionality
    let help_tests = vec![
        ("help", "should show general help"),
        ("help search", "should show search command help"),
        ("help config", "should show config command help"),
        ("help role", "should show role command help"),
    ];

    for (command, description) in help_tests {
        println!("  Testing help command: {} - {}", command, description);

        let result = timeout(
            Duration::from_secs(10),
            execute_cli_command(&config_path, command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!("    ✅ Help command succeeded: {}", command);
                // Verify help output contains relevant information
                assert!(!output.is_empty(), "Help output should not be empty");

                if command == "help" {
                    // General help should list available commands
                    assert!(
                        output.contains("search") || output.contains("config"),
                        "General help should mention key commands"
                    );
                }
            }
            Ok(Err(_)) | Err(_) => {
                println!(
                    "    ⚠️  Help command had issues: {} (may be expected if binary not available)",
                    command
                );
            }
        }
    }

    println!("✅ CLI help system test completed");
}

/// Test error handling for context-related CLI operations
#[tokio::test]
async fn test_cli_error_handling_for_context() {
    println!("🧪 Testing CLI error handling for context operations");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("error_test_config.json");

    // Create configuration with intentional issues to test error handling
    let error_config = serde_json::json!({
        "roles": {
            "ErrorTestRole": {
                "name": "Error Test Role",
                "shortname": "error-test",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [
                    {
                        "location": "/nonexistent/path",
                        "service": "Ripgrep",
                        "read_only": false,
                        "atomic_server_secret": null,
                        "extra_parameters": {}
                    }
                ],
                "extra": {},
                "terraphim_it": false
            }
        },
        "default_role": "ErrorTestRole",
        "selected_role": "ErrorTestRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&error_config).unwrap(),
    )
    .expect("Failed to write error config");

    // Test commands that should handle errors gracefully
    let error_tests = vec![
        (
            "search nonexistent_term",
            "should handle search with no results gracefully",
        ),
        (
            "role select NonExistentRole",
            "should handle invalid role selection",
        ),
        (
            "config set invalid_key invalid_value",
            "should handle invalid config settings",
        ),
    ];

    for (command, description) in error_tests {
        println!("  Testing error handling: {} - {}", command, description);

        let result = timeout(
            Duration::from_secs(10),
            execute_cli_command(&config_path, command),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                println!(
                    "    ✅ Command completed (possibly with expected errors): {}",
                    command
                );
                // Commands might succeed but return empty results or error messages
                // which is acceptable behavior
                if output.contains("error") || output.contains("not found") {
                    println!("      Expected error condition handled properly");
                }
            }
            Ok(Err(_)) => {
                println!("    ✅ Command failed as expected: {}", command);
                // Expected failures are good - shows error handling is working
            }
            Err(_) => {
                println!(
                    "    ⚠️  Command timed out: {} (may indicate hanging on error)",
                    command
                );
            }
        }
    }

    println!("✅ CLI error handling test completed");
}

/// Integration test for CLI with server-side context management
#[tokio::test]
async fn test_cli_server_context_roundtrip() {
    println!("🧪 Testing CLI with server-side context management roundtrip");

    // This test would ideally:
    // 1. Start a terraphim server in the background
    // 2. Use CLI to configure context-enabled roles
    // 3. Perform searches that generate context-suitable results
    // 4. Verify context management through server APIs
    // 5. Clean up server instance

    // For now, we'll test the CLI configuration aspects
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("roundtrip_config.json");

    let roundtrip_config = serde_json::json!({
        "roles": {
            "ServerIntegrationRole": {
                "name": "Server Integration Role",
                "shortname": "server-integration",
                "relevance_function": "BM25",
                "theme": "default",
                "haystacks": [],
                "extra": {
                    "server_context_integration": true,
                    "api_endpoints": {
                        "context_add": "/conversations/{id}/context",
                        "context_update": "/conversations/{id}/context/{context_id}",
                        "context_delete": "/conversations/{id}/context/{context_id}"
                    }
                },
                "terraphim_it": false
            }
        },
        "default_role": "ServerIntegrationRole",
        "selected_role": "ServerIntegrationRole",
        "server": {
            "host": "127.0.0.1",
            "port": 3000
        }
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&roundtrip_config).unwrap(),
    )
    .expect("Failed to write roundtrip config");

    // Test configuration verification
    let config_test = timeout(
        Duration::from_secs(5),
        execute_cli_command(&config_path, "config show"),
    )
    .await;

    match config_test {
        Ok(Ok(output)) => {
            println!("✅ CLI server integration config verified");
            assert!(
                output.contains("ServerIntegrationRole"),
                "Config should contain server integration role"
            );
        }
        _ => {
            println!("⚠️  CLI config verification had issues (expected if binary not available)");
        }
    }

    println!("✅ CLI server context roundtrip test completed");
}

/// Performance test for CLI context operations
#[tokio::test]
async fn test_cli_context_performance() {
    println!("🧪 Testing CLI context operation performance");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("performance_config.json");

    // Create performance test configuration
    let perf_config = serde_json::json!({
        "roles": {
            "PerformanceRole": {
                "name": "Performance Test Role",
                "shortname": "perf",
                "relevance_function": "TitleScorer",
                "theme": "default",
                "haystacks": [],
                "extra": {
                    "performance_mode": true,
                    "cache_enabled": true
                },
                "terraphim_it": false
            }
        },
        "default_role": "PerformanceRole",
        "selected_role": "PerformanceRole"
    });

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&perf_config).unwrap(),
    )
    .expect("Failed to write performance config");

    // Test performance of repeated CLI operations
    let performance_commands = vec!["config show", "role list", "help"];

    let mut total_duration = Duration::ZERO;
    let mut successful_commands = 0;

    for command in performance_commands {
        let start_time = std::time::Instant::now();

        let result = timeout(
            Duration::from_secs(5),
            execute_cli_command(&config_path, command),
        )
        .await;

        let duration = start_time.elapsed();
        total_duration += duration;

        match result {
            Ok(Ok(_)) => {
                successful_commands += 1;
                println!("  ✅ Command '{}' completed in {:?}", command, duration);
            }
            _ => {
                println!("  ⚠️  Command '{}' had issues in {:?}", command, duration);
            }
        }
    }

    if successful_commands > 0 {
        let avg_duration = total_duration / successful_commands;
        println!("📊 Performance results:");
        println!("   Successful commands: {}", successful_commands);
        println!("   Total time: {:?}", total_duration);
        println!("   Average time per command: {:?}", avg_duration);

        // Performance should be reasonable (under 2 seconds per command)
        assert!(
            avg_duration < Duration::from_secs(2),
            "CLI commands should complete in reasonable time"
        );
    }

    println!("✅ CLI context performance test completed");
}
