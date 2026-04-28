use std::process::Command;
use std::str;

use anyhow::Result;
use serial_test::serial;

mod support;
use support::cli_test_env::apply_hermetic_env;

/// Test helper to run TUI commands in offline mode
fn run_offline_command(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--"]).args(args);
    apply_hermetic_env(&mut cmd)?;

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
    cmd.args(["run", "-p", "terraphim_agent", "--features", "server", "--"])
        .args(cmd_args);
    apply_hermetic_env(&mut cmd)?;

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
        stdout.contains("server-backed fullscreen TUI"),
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
async fn test_non_tty_usage_clarifies_mode_contract() -> Result<()> {
    let (stdout, _stderr, code) = run_offline_command(&[])?;

    assert_eq!(code, 0, "Non-TTY usage should exit cleanly");
    assert!(stdout.contains("Interactive Modes (requires TTY)"));
    assert!(stdout.contains("fullscreen TUI (requires running server)"));
    assert!(stdout.contains("repl"));
    assert!(stdout.contains("offline-capable by default"));

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

    assert!(
        config["id"] == "Embedded" || config["id"] == "Server",
        "Should load a valid config id for offline mode"
    );
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

    // Chat command may return exit code 1 if no LLM is configured, which is valid
    assert!(
        code == 0 || code == 1,
        "Chat command should not crash, stderr: {}",
        stderr
    );

    // Check for appropriate output - either LLM response or "no LLM configured" message
    let output_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN"))
        .collect();
    let response = output_lines.join("\n");

    // Also check stderr for "No LLM configured" since error messages go there
    if code == 0 {
        println!("Chat successful: {}", response);
    } else {
        // Exit code 1 is expected when no LLM is configured
        assert!(
            stderr.contains("No LLM configured") || response.contains("No LLM configured"),
            "Should show no LLM configured message: stdout={}, stderr={}",
            response,
            stderr
        );
        println!("Chat correctly indicated no LLM configured");
    }

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
    // Test that server mode gracefully handles no server running.
    // The CLI may fall back to offline mode (exit 0) or fail (exit 1).
    let (_stdout, stderr, code) = run_server_command(&["roles", "list"])?;

    let graceful_fallback = code == 0;
    let connection_error =
        stderr.contains("Connection refused") || stderr.contains("connect error");
    assert!(
        graceful_fallback || connection_error,
        "Server mode should either fall back gracefully or report connection error: exit={code}, stderr={stderr}",
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_with_custom_url() -> Result<()> {
    // Test server mode with custom URL
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--features", "server", "--"])
        .args([
            "--server",
            "--server-url",
            "http://localhost:9999",
            "config",
            "show",
        ]);
    apply_hermetic_env(&mut cmd)?;

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
    apply_hermetic_env(&mut cmd)?;

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
        stderr.contains("Initializing TUI service")
            || stderr.contains("Failed to load config")
            || stderr.contains("using default embedded")
            || stderr.contains("using embedded defaults"),
        "Should show embedded initialization: {}",
        stderr
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_forgiving_parser_auto_correction() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["serach", "test query"])?;

    // Should auto-correct and either succeed or fail gracefully (not crash)
    assert!(
        code == 0 || code == 1,
        "Auto-corrected command should not crash, stderr: {}",
        stderr
    );

    // Should print correction notification to stderr
    assert!(
        stderr.contains("auto-corrected"),
        "Should notify about auto-correction in stderr: {}",
        stderr
    );

    // Should produce search-like output (results or "No results" / empty indicator)
    let output = stdout.to_string();
    assert!(
        !output.contains("unrecognized subcommand") && !output.contains("error: unrecognized"),
        "Should not show clap unknown-subcommand error: {}",
        output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_forgiving_parser_alias_expansion() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["q", "test query"])?;

    // Should expand alias and either succeed or fail gracefully
    assert!(
        code == 0 || code == 1,
        "Alias-expanded command should not crash, stderr: {}",
        stderr
    );

    // Should print expansion notification to stderr
    assert!(
        stderr.contains("expanded") || stderr.contains("auto-corrected"),
        "Should notify about alias expansion in stderr: {}",
        stderr
    );

    // Should not show clap unknown-subcommand error
    let output = stdout.to_string();
    assert!(
        !output.contains("unrecognized subcommand") && !output.contains("error: unrecognized"),
        "Should not show clap unknown-subcommand error: {}",
        output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_forgiving_parser_case_insensitive() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["SEARCH", "test query"])?;

    // Should match case-insensitively and either succeed or fail gracefully
    assert!(
        code == 0 || code == 1,
        "Case-insensitive command should not crash, stderr: {}",
        stderr
    );

    // Should not show clap unknown-subcommand error
    let output = stdout.to_string();
    assert!(
        !output.contains("unrecognized subcommand") && !output.contains("error: unrecognized"),
        "Should not show clap unknown-subcommand error: {}",
        output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_robot_capabilities_json() -> Result<()> {
    let (stdout, stderr, code) =
        run_offline_command(&["robot", "capabilities", "--format", "json"])?;

    assert_eq!(
        code, 0,
        "robot capabilities should succeed, stderr: {}",
        stderr
    );

    // Should output valid JSON
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert!(json.get("name").is_some(), "Should have name field");
    assert!(json.get("version").is_some(), "Should have version field");
    assert!(json.get("commands").is_some(), "Should have commands field");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_robot_schemas_json() -> Result<()> {
    let (stdout, stderr, code) =
        run_offline_command(&["robot", "schemas", "search", "--format", "json"])?;

    assert_eq!(
        code, 0,
        "robot schemas search should succeed, stderr: {}",
        stderr
    );

    // Should output valid JSON schema
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert!(json.get("name").is_some(), "Should have name field");
    assert!(
        json.get("arguments").is_some(),
        "Should have arguments field"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_robot_examples_text() -> Result<()> {
    let (stdout, stderr, code) = run_offline_command(&["robot", "examples", "search"])?;

    assert_eq!(
        code, 0,
        "robot examples search should succeed, stderr: {}",
        stderr
    );

    // Should output readable text (not error)
    assert!(
        !stdout.starts_with("error:") && !stdout.starts_with("Error:"),
        "Should not show error: {}",
        stdout
    );
    assert!(!stdout.is_empty(), "Should have output");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_robot_capabilities_all_formats() -> Result<()> {
    for format in ["json", "table", "minimal"] {
        let (stdout, stderr, code) =
            run_offline_command(&["robot", "capabilities", "--format", format])?;
        assert_eq!(
            code, 0,
            "robot capabilities --format {} should succeed, stderr: {}",
            format, stderr
        );
        assert!(
            !stdout.is_empty(),
            "Should have output for format {}",
            format
        );
    }

    Ok(())
}
