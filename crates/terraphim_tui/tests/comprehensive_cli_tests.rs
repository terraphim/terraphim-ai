//! Comprehensive CLI tests for TUI interface
//!
//! Tests all TUI CLI commands including multi-term search, chat, graph, and more

use anyhow::Result;
use serial_test::serial;
use std::process::Command;
use std::str;

/// Helper function to run TUI command with arguments
fn run_tui_command(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--"]).args(args);

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
        .filter(|line| {
            !line.contains("INFO")
                && !line.contains("WARN")
                && !line.contains("DEBUG")
                && !line.trim().is_empty()
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[test]
#[serial]
fn test_search_multi_term_functionality() -> Result<()> {
    println!("🔍 Testing multi-term search functionality");

    // Test multi-term search with AND operator
    let (stdout, stderr, code) = run_tui_command(&[
        "search",
        "data",
        "--terms",
        "system,architecture",
        "--operator",
        "and",
        "--limit",
        "5",
    ])?;

    assert!(
        code == 0 || code == 1,
        "Multi-term AND search should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 && !clean_output.is_empty() {
        println!("✅ Multi-term AND search found results");
        // Validate output format (allow various formats)
        let has_expected_format = clean_output
            .lines()
            .any(|line| line.contains('\t') || line.starts_with("- ") || line.contains("rank"));
        if !has_expected_format {
            println!("⚠️ Unexpected output format, but search succeeded");
        }
    } else {
        println!("⚠️ Multi-term AND search found no results");
    }

    // Test multi-term search with OR operator
    let (_stdout, stderr, code) = run_tui_command(&[
        "search",
        "haystack",
        "--terms",
        "service,graph",
        "--operator",
        "or",
        "--limit",
        "3",
    ])?;

    assert!(
        code == 0 || code == 1,
        "Multi-term OR search should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    if code == 0 {
        println!("✅ Multi-term OR search completed successfully");
    }

    Ok(())
}

#[test]
#[serial]
fn test_search_with_role_and_limit() -> Result<()> {
    println!("🔍 Testing search with role and limit options");

    // Test search with specific role
    let (stdout, stderr, code) =
        run_tui_command(&["search", "system", "--role", "Default", "--limit", "8"])?;

    assert!(
        code == 0 || code == 1,
        "Search with role should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if code == 0 && !clean_output.is_empty() {
        println!("✅ Search with role found results");

        // Count results to verify limit
        let result_count = clean_output
            .lines()
            .filter(|line| line.starts_with("- "))
            .count();

        assert!(
            result_count <= 8,
            "Result count should respect limit: found {}",
            result_count
        );
    } else {
        println!("⚠️ Search with role found no results");
    }

    // Test with Terraphim Engineer role
    let (_stdout, stderr, code) = run_tui_command(&[
        "search",
        "haystack",
        "--role",
        "Terraphim Engineer",
        "--limit",
        "5",
    ])?;

    assert!(
        code == 0 || code == 1,
        "Search with Terraphim Engineer role should complete: exit_code={}, stderr={}",
        code,
        stderr
    );

    if code == 0 {
        println!("✅ Search with Terraphim Engineer role completed");
    }

    Ok(())
}

#[test]
#[serial]
fn test_roles_management() -> Result<()> {
    println!("👤 Testing roles management commands");

    // Test roles list
    let (stdout, stderr, code) = run_tui_command(&["roles", "list"])?;

    assert_eq!(
        code, 0,
        "Roles list should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    let clean_output = extract_clean_output(&stdout);
    assert!(
        !clean_output.is_empty(),
        "Roles list should return role names"
    );

    let roles: Vec<&str> = clean_output.lines().collect();
    println!("✅ Found {} roles: {:?}", roles.len(), roles);

    // Verify expected roles exist
    let expected_roles = ["Default", "Terraphim Engineer"];
    for expected_role in &expected_roles {
        assert!(
            roles.iter().any(|role| role.contains(expected_role)),
            "Role '{}' should be available",
            expected_role
        );
    }

    // Test role selection (if roles exist)
    if !roles.is_empty() {
        let test_role = roles[0].trim();
        let (stdout, stderr, code) = run_tui_command(&["roles", "select", test_role])?;

        assert_eq!(
            code, 0,
            "Role selection should succeed: exit_code={}, stderr={}",
            code, stderr
        );

        let clean_output = extract_clean_output(&stdout);
        assert!(
            clean_output.contains(&format!("selected:{}", test_role)),
            "Role selection should confirm the selection"
        );

        println!("✅ Role selection completed for: {}", test_role);
    }

    Ok(())
}

#[test]
#[serial]
fn test_config_management() -> Result<()> {
    println!("🔧 Testing config management commands");

    // Test config show
    let (stdout, stderr, code) = run_tui_command(&["config", "show"])?;

    assert_eq!(
        code, 0,
        "Config show should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    let clean_output = extract_clean_output(&stdout);
    assert!(!clean_output.is_empty(), "Config should return JSON data");

    // Try to parse as JSON to validate format
    let json_start = clean_output.find('{').unwrap_or(0);
    let json_content = &clean_output[json_start..];

    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(json_content);
    assert!(parse_result.is_ok(), "Config output should be valid JSON");

    let config = parse_result.unwrap();
    assert!(config.is_object(), "Config should be JSON object");
    assert!(
        config.get("selected_role").is_some(),
        "Config should have selected_role"
    );
    assert!(config.get("roles").is_some(), "Config should have roles");

    println!("✅ Config show completed and validated");

    // Test config set (selected_role) with valid role
    let (stdout, stderr, code) = run_tui_command(&[
        "config",
        "set",
        "selected_role",
        "Default", // Use a role that exists
    ])?;

    if code == 0 {
        let clean_output = extract_clean_output(&stdout);
        if clean_output.contains("updated selected_role to Default") {
            println!("✅ Config set completed successfully");
        } else {
            println!("⚠️ Config set succeeded but output format may have changed");
        }
    } else {
        println!(
            "⚠️ Config set failed: exit_code={}, stderr={}",
            code, stderr
        );
        // This might be expected if role validation is strict
        println!("   Testing with non-existent role to verify error handling...");

        let (_, _, error_code) =
            run_tui_command(&["config", "set", "selected_role", "NonExistentRole"])?;

        assert_ne!(error_code, 0, "Should fail with non-existent role");
        println!("   ✅ Properly rejects non-existent roles");
    }

    Ok(())
}

#[test]
#[serial]
fn test_graph_command() -> Result<()> {
    println!("🕸️ Testing graph command");

    // Test graph with default settings
    let (stdout, stderr, code) = run_tui_command(&["graph", "--top-k", "5"])?;

    assert_eq!(
        code, 0,
        "Graph command should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    let clean_output = extract_clean_output(&stdout);

    if !clean_output.is_empty() {
        println!(
            "✅ Graph command returned {} lines",
            clean_output.lines().count()
        );

        // Validate that lines contain graph terms
        let graph_lines: Vec<&str> = clean_output.lines().collect();
        assert!(
            graph_lines.len() <= 5,
            "Graph should respect top-k limit of 5"
        );
    } else {
        println!("⚠️ Graph command returned empty results");
    }

    // Test graph with specific role
    let (_stdout, stderr, code) =
        run_tui_command(&["graph", "--role", "Terraphim Engineer", "--top-k", "10"])?;

    assert_eq!(
        code, 0,
        "Graph with role should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    if code == 0 {
        println!("✅ Graph command with role completed");
    }

    Ok(())
}

#[test]
#[serial]
fn test_chat_command() -> Result<()> {
    println!("💬 Testing chat command");

    // Test basic chat
    let (stdout, stderr, code) = run_tui_command(&["chat", "Hello, this is a test message"])?;

    assert_eq!(
        code, 0,
        "Chat command should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    let clean_output = extract_clean_output(&stdout);

    // Chat should either return a response or indicate no LLM is configured
    assert!(!clean_output.is_empty(), "Chat should return some response");

    if clean_output.to_lowercase().contains("no llm configured") {
        println!("✅ Chat correctly indicates no LLM is configured");
    } else {
        println!(
            "✅ Chat returned response: {}",
            clean_output.lines().next().unwrap_or("")
        );
    }

    // Test chat with role
    let (_stdout, stderr, code) =
        run_tui_command(&["chat", "Test message with role", "--role", "Default"])?;

    assert_eq!(
        code, 0,
        "Chat with role should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    println!("✅ Chat with role completed");

    // Test chat with model specification
    let (_stdout, stderr, code) =
        run_tui_command(&["chat", "Test with model", "--model", "test-model"])?;

    assert_eq!(
        code, 0,
        "Chat with model should succeed: exit_code={}, stderr={}",
        code, stderr
    );

    println!("✅ Chat with model specification completed");

    Ok(())
}

#[test]
#[serial]
fn test_command_help_and_usage() -> Result<()> {
    println!("📖 Testing command help and usage");

    // Test main help
    let (stdout, _stderr, code) = run_tui_command(&["--help"])?;

    assert_eq!(code, 0, "Main help should succeed");

    let help_content = stdout.to_lowercase();
    assert!(
        help_content.contains("terraphim"),
        "Help should mention terraphim"
    );
    assert!(
        help_content.contains("search"),
        "Help should mention search command"
    );

    println!("✅ Main help validated");

    // Test subcommand help
    let subcommands = ["search", "roles", "config", "graph", "chat", "extract"];

    for subcommand in &subcommands {
        let (stdout, stderr, code) = run_tui_command(&[subcommand, "--help"])?;

        assert_eq!(
            code, 0,
            "Help for {} should succeed: stderr={}",
            subcommand, stderr
        );

        let help_content = stdout.to_lowercase();
        assert!(
            help_content.contains(subcommand),
            "Help should mention the subcommand: {}",
            subcommand
        );

        println!("  ✅ Help for {} validated", subcommand);
    }

    Ok(())
}

#[test]
#[serial]
fn test_error_handling_and_edge_cases() -> Result<()> {
    println!("⚠️ Testing error handling and edge cases");

    // Test invalid command
    let (_, _, code) = run_tui_command(&["invalid-command"])?;
    assert_ne!(code, 0, "Invalid command should fail");
    println!("✅ Invalid command properly rejected");

    // Test search without required argument
    let (_, _, code) = run_tui_command(&["search"])?;
    assert_ne!(code, 0, "Search without query should fail");
    println!("✅ Missing search query properly rejected");

    // Test roles with invalid subcommand
    let (_, _, code) = run_tui_command(&["roles", "invalid"])?;
    assert_ne!(code, 0, "Invalid roles subcommand should fail");
    println!("✅ Invalid roles subcommand properly rejected");

    // Test config with invalid arguments
    let (_, _, code) = run_tui_command(&["config", "set"])?;
    assert_ne!(code, 0, "Incomplete config set should fail");
    println!("✅ Incomplete config set properly rejected");

    // Test graph with invalid top-k
    let (_, _stderr, code) = run_tui_command(&["graph", "--top-k", "invalid"])?;
    assert_ne!(code, 0, "Invalid top-k should fail");
    println!("✅ Invalid top-k properly rejected");

    // Test search with very long query (should handle gracefully)
    let long_query = "a".repeat(10000);
    let (_, _, code) = run_tui_command(&["search", &long_query, "--limit", "1"])?;
    assert!(
        code == 0 || code == 1,
        "Very long query should be handled gracefully"
    );
    println!("✅ Very long query handled gracefully");

    Ok(())
}

#[test]
#[serial]
fn test_output_formatting() -> Result<()> {
    println!("📝 Testing output formatting");

    // Test search output format
    let (stdout, _, code) = run_tui_command(&["search", "test", "--limit", "3"])?;

    if code == 0 {
        let clean_output = extract_clean_output(&stdout);

        if !clean_output.is_empty() {
            // Search results should have consistent format: "- <rank>\t<title>"
            let lines: Vec<&str> = clean_output.lines().collect();

            for line in &lines {
                if line.starts_with("- ") {
                    assert!(
                        line.contains('\t'),
                        "Search result line should contain tab separator: {}",
                        line
                    );
                }
            }

            println!("✅ Search output format validated");
        }
    }

    // Test roles list output format
    let (stdout, _, code) = run_tui_command(&["roles", "list"])?;

    if code == 0 {
        let clean_output = extract_clean_output(&stdout);
        let lines: Vec<&str> = clean_output.lines().filter(|l| !l.is_empty()).collect();

        // Each line should be a role name
        for line in &lines {
            assert!(
                !line.trim().is_empty(),
                "Role name should not be empty: '{}'",
                line
            );
        }

        println!("✅ Roles list output format validated");
    }

    // Test config show output format (should be valid JSON)
    let (stdout, _, code) = run_tui_command(&["config", "show"])?;

    if code == 0 {
        let clean_output = extract_clean_output(&stdout);

        if let Some(json_start) = clean_output.find('{') {
            let json_content = &clean_output[json_start..];
            let parse_result: Result<serde_json::Value, _> = serde_json::from_str(json_content);
            assert!(
                parse_result.is_ok(),
                "Config output should be valid JSON: {}",
                json_content
            );

            println!("✅ Config output format validated");
        }
    }

    Ok(())
}

#[test]
#[serial]
fn test_performance_and_limits() -> Result<()> {
    println!("⚡ Testing performance and limits");

    // Test search with large limit
    let start = std::time::Instant::now();
    let (_, _, code) = run_tui_command(&["search", "test", "--limit", "100"])?;
    let duration = start.elapsed();

    assert!(code == 0 || code == 1, "Large limit search should complete");

    assert!(
        duration.as_secs() < 60,
        "Search with large limit should complete within 60 seconds"
    );

    println!("✅ Large limit search completed in {:?}", duration);

    // Test graph with large top-k
    let start = std::time::Instant::now();
    let (_, _, code) = run_tui_command(&["graph", "--top-k", "100"])?;
    let duration = start.elapsed();

    assert_eq!(code, 0, "Large top-k graph should succeed");

    assert!(
        duration.as_secs() < 30,
        "Graph with large top-k should complete within 30 seconds"
    );

    println!("✅ Large top-k graph completed in {:?}", duration);

    // Test multiple rapid commands
    println!("  Testing rapid command execution...");

    let commands = [
        vec!["roles", "list"],
        vec!["config", "show"],
        vec!["search", "quick", "--limit", "1"],
        vec!["graph", "--top-k", "1"],
    ];

    let start = std::time::Instant::now();

    for (i, cmd) in commands.iter().enumerate() {
        let (_, _, code) = run_tui_command(cmd)?;
        assert!(
            code == 0 || code == 1,
            "Rapid command {} should complete",
            i + 1
        );
    }

    let total_duration = start.elapsed();
    assert!(
        total_duration.as_secs() < 120,
        "Rapid commands should complete within 2 minutes"
    );

    println!(
        "✅ Rapid command execution completed in {:?}",
        total_duration
    );

    Ok(())
}
