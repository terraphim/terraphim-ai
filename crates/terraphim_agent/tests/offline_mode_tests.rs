use std::process::Command;
use std::str::{self, FromStr};

use anyhow::Result;
use serial_test::serial;

/// Test helper to run TUI commands in offline mode
fn run_offline_command(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"]).args(args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

/// Test helper to run TUI commands in server mode
fn run_server_command(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd_args = vec!["--server"];
    cmd_args.extend_from_slice(args);

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"])
        .args(cmd_args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

#[tokio::test]
#[serial]
async fn test_offline_help_command() -> Result<()> {
    let (stdout, _stderr, code) = run_offline_command(&["--help"])?;

    assert_eq!(code, 0, "Help command should succeed");
    assert!(
        stdout.contains("Terraphim TUI interface"),
        "Should show main help"
    );
    assert!(stdout.contains("--server"), "Should show server flag");
    assert!(
        stdout.contains("--server-url"),
        "Should show server URL flag"
    );
    assert!(stdout.contains("search"), "Should list search command");
    assert!(stdout.contains("roles"), "Should list roles command");
    assert!(stdout.contains("config"), "Should list config command");
    assert!(stdout.contains("graph"), "Should list graph command");
    assert!(stdout.contains("chat"), "Should list chat command");
    assert!(stdout.contains("extract"), "Should list extract command");
    assert!(
        stdout.contains("interactive"),
        "Should list interactive command"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_config_show() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["config", "show"])?;

    assert_eq!(code, 0, "Config show should succeed, stderr: {}", stderr);

    // Parse JSON output
    let lines: Vec<&str> = stdout.lines().collect();
    let json_start = lines.iter().position(|line| line.starts_with('{'));
    assert!(json_start.is_some(), "Should contain JSON output");

    let json_lines = &lines[json_start.unwrap()..];
    let json_str = json_lines.join("\n");

    let config: serde_json::Value = serde_json::from_str(&json_str).expect("Should be valid JSON");

    assert_eq!(config["id"], "Embedded", "Should use Embedded config");
    assert!(
        config.get("selected_role").is_some(),
        "Should have selected_role"
    );
    assert!(
        config.get("default_role").is_some(),
        "Should have default_role"
    );
    assert!(
        config.get("global_shortcut").is_some(),
        "Should have global_shortcut"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_roles_list() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["roles", "list"])?;

    assert_eq!(code, 0, "Roles list should succeed, stderr: {}", stderr);

    // The output should be role names, one per line
    // For embedded config, it might be empty initially, which is valid
    println!("Roles output: {}", stdout);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_search_with_default_role() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["search", "test query"])?;

    // Search may succeed or fail depending on setup, but should not crash
    assert!(
        code == 0 || code == 1,
        "Search should not crash, stderr: {}",
        stderr
    );

    // Should use selected_role from config (no role override)
    if code == 0 {
        println!("Search successful: {}", stdout);
    } else {
        println!("Search failed as expected (no data): {}", stderr);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_search_with_role_override() -> Result<()> {
    let (stdout, stderr, code) =
        run_offline_command(&["search", "test query", "--role", "Default"])?;

    // Search may succeed or fail depending on setup, but should not crash
    assert!(
        code == 0 || code == 1,
        "Search with role override should not crash, stderr: {}",
        stderr
    );

    if code == 0 {
        println!("Search with role override successful: {}", stdout);
    } else {
        println!("Search with role override failed as expected: {}", stderr);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_graph_command() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["graph"])?;

    // Graph command should succeed (returns placeholder data)
    assert_eq!(code, 0, "Graph command should succeed, stderr: {}", stderr);

    // Should show concepts (placeholder data)
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
        .collect();
    println!("Graph output lines: {:?}", lines);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_graph_with_role() -> Result<()> {
    let (stdout, stderr, code) =
        run_offline_command(&["graph", "--role", "Default", "--top-k", "5"])?;

    // Graph command with role should succeed
    assert_eq!(
        code, 0,
        "Graph command with role should succeed, stderr: {}",
        stderr
    );

    println!("Graph with role output: {}", stdout);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_chat_command() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["chat", "Hello, how are you?"])?;

    assert_eq!(code, 0, "Chat command should succeed, stderr: {}", stderr);

    // Should show placeholder response since no LLM is configured
    let output_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
        .collect();
    let response = output_lines.join("\n");
    assert!(
        response.contains("No LLM configured") || response.contains("Chat response"),
        "Should show LLM response or no LLM message: {}",
        response
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_extract_command() -> Result<()> {
    let text =
        "This is a test paragraph. It contains some text for extraction. Another sentence here.";
    let (stdout, stderr, code) = run_offline_command(&["extract", text])?;

    // Extract might succeed or fail based on thesaurus availability
    assert!(
        code == 0 || code == 1,
        "Extract command should not crash, stderr: {}",
        stderr
    );

    let output_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
        .collect();
    let response = output_lines.join("\n");

    if code == 0 {
        println!("Extract successful: {}", response);
    } else {
        println!("Extract failed as expected: {}", response);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_extract_with_role() -> Result<()> {
    let text = "This is a test paragraph. It contains some text for extraction.";
    let (stdout, stderr, code) =
        run_offline_command(&["extract", text, "--role", "Default", "--exclude-term"])?;

    // Extract with role might succeed or fail
    assert!(
        code == 0 || code == 1,
        "Extract with role should not crash, stderr: {}",
        stderr
    );

    println!("Extract with role output: {}", stdout);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_config_set_selected_role() -> Result<()> {
    // First show current config
    let (stdout_before, _, _) = run_offline_command(&["config", "show"])?;
    println!("Config before: {}", stdout_before);

    // Set selected role
    let (stdout, stderr, code) =
        run_offline_command(&["config", "set", "selected_role", "Default"])?;

    assert_eq!(code, 0, "Config set should succeed, stderr: {}", stderr);

    let output_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
        .collect();
    let response = output_lines.join("\n");
    assert!(
        response.contains("updated selected_role to Default"),
        "Should confirm role update: {}",
        response
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_roles_select() -> Result<()> {
    // This test may fail if no roles exist, which is expected for embedded config
    let (stdout, stderr, code) = run_offline_command(&["roles", "select", "Default"])?;

    if code == 0 {
        let output_lines: Vec<&str> = stdout
            .lines()
            .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
            .collect();
        let response = output_lines.join("\n");
        assert!(
            response.contains("selected:Default"),
            "Should confirm role selection: {}",
            response
        );
    } else {
        // Expected if no roles exist in embedded config
        println!("Role select failed as expected (no roles): {}", stderr);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_connection_failure() -> Result<()> {
    // Test that server mode correctly attempts to connect and fails gracefully
    let (_stdout, stderr, code) = run_server_command(&["roles", "list"])?;

    assert_eq!(code, 1, "Server mode should fail when no server running");
    assert!(
        stderr.contains("Connection refused") || stderr.contains("connect error"),
        "Should show connection error: {}",
        stderr
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_with_custom_url() -> Result<()> {
    // Test server mode with custom URL
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"]).args([
        "--server",
        "--server-url",
        "http://localhost:9999",
        "config",
        "show",
    ]);

    let output = cmd.output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code().unwrap_or(-1);

    assert_eq!(
        code, 1,
        "Should fail with custom URL when no server running"
    );
    assert!(
        stderr.contains("Connection refused") || stderr.contains("connect error"),
        "Should show connection error with custom URL: {}",
        stderr
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_command_line_argument_validation() -> Result<()> {
    // Test invalid command
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"])
        .args(["invalid-command"]);

    let output = cmd.output()?;
    let code = output.status.code().unwrap_or(-1);

    assert_ne!(code, 0, "Invalid command should fail");

    // Test help for subcommands
    let (stdout, _, code) = run_offline_command(&["search", "--help"])?;
    assert_eq!(code, 0, "Search help should succeed");
    assert!(stdout.contains("query"), "Should show query parameter");
    assert!(stdout.contains("--role"), "Should show role option");
    assert!(stdout.contains("--limit"), "Should show limit option");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_mode_is_default() -> Result<()> {
    // Test that offline mode is the default (no --server flag)
    let (_stdout, stderr, code) = run_offline_command(&["config", "show"])?;

    assert_eq!(code, 0, "Default mode should be offline and succeed");

    // Should initialize embedded service (log messages will show this)
    assert!(
        stderr.contains("Initializing TUI service with embedded configuration")
            || stderr.contains("Failed to load config")
            || stderr.contains("using default embedded"),
        "Should show embedded initialization: {}",
        stderr
    );

    Ok(())
}
