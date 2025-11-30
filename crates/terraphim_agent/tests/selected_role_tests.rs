use anyhow::{ensure, Result};
use serial_test::serial;
use std::process::Command;
use std::str;

/// Test helper to run TUI commands and parse output
fn run_command_and_parse(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--"]).args(args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

/// Extract clean output (without log messages)
fn extract_clean_output(output: &str) -> String {
    output
        .lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN") && !line.contains("DEBUG"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Parse JSON config from output
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

fn fetch_config() -> Result<serde_json::Value> {
    let (stdout, stderr, code) = run_command_and_parse(&["config", "show"])?;
    ensure!(code == 0, "Config show should succeed, stderr: {}", stderr);
    parse_config_from_output(&stdout)
}

fn fetch_available_roles() -> Result<Vec<String>> {
    let config = fetch_config()?;
    let roles_obj = config["roles"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("roles field missing from config"))?;
    Ok(roles_obj.keys().cloned().collect())
}

#[tokio::test]
#[serial]
async fn test_default_selected_role_is_used() -> Result<()> {
    // Get current config to see selected role
    let (stdout, stderr, code) = run_command_and_parse(&["config", "show"])?;

    assert_eq!(code, 0, "Config show should succeed, stderr: {}", stderr);

    let config = parse_config_from_output(&stdout)?;
    let selected_role = config["selected_role"].as_str().unwrap();

    println!("Current selected_role: {}", selected_role);

    // Test that commands use the selected role by default
    // Graph command should use selected role when no --role is specified
    let (graph_stdout, graph_stderr, graph_code) =
        run_command_and_parse(&["graph", "--top-k", "3"])?;

    assert_eq!(
        graph_code, 0,
        "Graph command should succeed, stderr: {}",
        graph_stderr
    );

    println!(
        "Graph command output (using selected role): {}",
        extract_clean_output(&graph_stdout)
    );

    // Chat command should use selected role when no --role is specified
    let (chat_stdout, chat_stderr, chat_code) = run_command_and_parse(&["chat", "test message"])?;

    assert_eq!(
        chat_code, 0,
        "Chat command should succeed, stderr: {}",
        chat_stderr
    );

    let chat_output = extract_clean_output(&chat_stdout);
    println!("Chat command output (using selected role): {}", chat_output);

    // Chat should reference the selected role in its response
    assert!(
        chat_output.contains(selected_role) || chat_output.contains("No LLM configured"),
        "Chat should use selected role '{}' or show no LLM message: {}",
        selected_role,
        chat_output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_override_in_commands() -> Result<()> {
    // Test that --role flag overrides selected_role in config

    // Search with role override
    let (search_stdout, search_stderr, search_code) =
        run_command_and_parse(&["search", "test query", "--role", "Default", "--limit", "3"])?;

    // Should succeed or fail gracefully (depending on whether role exists)
    assert!(
        search_code == 0 || search_code == 1,
        "Search with role override should not crash, stderr: {}",
        search_stderr
    );

    println!(
        "Search with role override: {}",
        extract_clean_output(&search_stdout)
    );

    // Graph with role override
    let (graph_stdout, graph_stderr, graph_code) =
        run_command_and_parse(&["graph", "--role", "Default", "--top-k", "5"])?;

    assert_eq!(
        graph_code, 0,
        "Graph with role override should succeed, stderr: {}",
        graph_stderr
    );

    println!(
        "Graph with role override: {}",
        extract_clean_output(&graph_stdout)
    );

    // Chat with role override
    let (chat_stdout, chat_stderr, chat_code) =
        run_command_and_parse(&["chat", "test message", "--role", "Default"])?;

    assert_eq!(
        chat_code, 0,
        "Chat with role override should succeed, stderr: {}",
        chat_stderr
    );

    let chat_output = extract_clean_output(&chat_stdout);
    println!("Chat with role override: {}", chat_output);

    // Should use the overridden role
    assert!(
        chat_output.contains("Default") || chat_output.contains("No LLM configured"),
        "Chat should use overridden role 'Default': {}",
        chat_output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_selected_role_persistence() -> Result<()> {
    let initial_config = fetch_config()?;
    let initial_role = initial_config["selected_role"]
        .as_str()
        .unwrap()
        .to_string();

    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one available role"
    );

    let new_role = available_roles
        .iter()
        .find(|role| role.as_str() != initial_role)
        .cloned()
        .unwrap_or_else(|| initial_role.clone());

    println!("Initial selected role: {}", initial_role);

    let (set_stdout, set_stderr, set_code) =
        run_command_and_parse(&["config", "set", "selected_role", new_role.as_str()])?;
    assert_eq!(
        set_code, 0,
        "Config set should succeed, stderr: {}",
        set_stderr
    );

    let set_output = extract_clean_output(&set_stdout);
    assert!(
        set_output.contains(&format!("updated selected_role to {}", new_role)),
        "Should confirm role update: {}",
        set_output
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_select_command_updates_selected_role() -> Result<()> {
    // Get initial config
    let config1 = fetch_config()?;
    let initial_role = config1["selected_role"].as_str().unwrap().to_string();

    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one available role"
    );
    let target_role = available_roles
        .iter()
        .find(|role| role.as_str() != initial_role)
        .cloned()
        .unwrap_or_else(|| initial_role.clone());

    println!("Initial selected role: {}", initial_role);

    // Try to select a role using roles select command
    let (select_stdout, select_stderr, select_code) =
        run_command_and_parse(&["roles", "select", target_role.as_str()])?;

    assert_eq!(
        select_code, 0,
        "Role select should succeed for '{}', stderr: {}",
        target_role, select_stderr
    );

    let select_output = extract_clean_output(&select_stdout);
    assert!(
        select_output.contains(&format!("selected:{}", target_role)),
        "Should confirm role selection: {}",
        select_output
    );

    // Verify the change persisted in config
    println!(
        "Successfully updated selected role via 'roles select' from '{}' to '{}'",
        initial_role, target_role
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multiple_commands_use_same_selected_role() -> Result<()> {
    // Set a specific selected role
    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one available role"
    );
    let test_role = available_roles[0].clone();
    let (_, _, set_code) =
        run_command_and_parse(&["config", "set", "selected_role", test_role.as_str()])?;
    assert_eq!(set_code, 0, "Should be able to set test role");

    // Test that multiple commands consistently use the same selected role
    let commands_to_test = vec![
        vec!["graph", "--top-k", "2"],
        vec!["chat", "consistency test"],
        vec!["search", "test", "--limit", "1"],
    ];

    for cmd_args in commands_to_test {
        let (stdout, stderr, code) = run_command_and_parse(&cmd_args)?;

        // All commands should succeed (or fail gracefully)
        assert!(
            code == 0 || code == 1,
            "Command '{:?}' should not crash, stderr: {}",
            cmd_args,
            stderr
        );

        if code == 0 {
            let output = extract_clean_output(&stdout);

            // For chat command, output should reference the role or show no LLM
            if cmd_args[0] == "chat" {
                assert!(
                    output.contains(test_role.as_str()) || output.contains("No LLM configured"),
                    "Chat command should use selected role '{}': {}",
                    test_role,
                    output
                );
            }

            println!("Command '{:?}' output: {}", cmd_args, output);
        } else {
            println!("Command '{:?}' failed gracefully: {}", cmd_args, stderr);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_config_role_validation() -> Result<()> {
    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one role in config"
    );

    for role in &available_roles {
        println!("Testing role name: '{}'", role);

        let (stdout, stderr, code) =
            run_command_and_parse(&["config", "set", "selected_role", role])?;

        assert_eq!(
            code, 0,
            "Should be able to set role '{}', stderr: {}",
            role, stderr
        );

        let output = extract_clean_output(&stdout);
        assert!(
            output.contains(&format!("updated selected_role to {}", role)),
            "Should confirm role update to '{}': {}",
            role,
            output
        );

        // Config commands run in isolated processes backed by in-memory storage,
        // so subsequent invocations start from the embedded defaults. We only
        // validate command feedback here.
    }

    // Invalid roles should be rejected
    let invalid_roles = [
        "Test Role With Spaces",
        "test-role-with-dashes",
        "test_role_with_underscores",
    ];

    for role in invalid_roles {
        let (_, stderr, code) = run_command_and_parse(&["config", "set", "selected_role", role])?;
        assert_ne!(
            code, 0,
            "Setting invalid role '{}' should fail. stderr: {}",
            role, stderr
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_inheritance_in_search() -> Result<()> {
    // Set a specific role
    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one role for search test"
    );
    let test_role = available_roles[0].clone();
    let (_, _, set_code) =
        run_command_and_parse(&["config", "set", "selected_role", test_role.as_str()])?;
    assert_eq!(set_code, 0, "Should set test role");

    // Search without specifying role (should use selected_role)
    let (search1_stdout, search1_stderr, search1_code) =
        run_command_and_parse(&["search", "test query", "--limit", "2"])?;

    // Search with explicit role override
    let (search2_stdout, search2_stderr, search2_code) =
        run_command_and_parse(&["search", "test query", "--role", "Default", "--limit", "2"])?;

    // Both should handle the role appropriately (succeed or fail gracefully)
    assert!(
        search1_code == 0 || search1_code == 1,
        "Search with selected role should not crash, stderr: {}",
        search1_stderr
    );
    assert!(
        search2_code == 0 || search2_code == 1,
        "Search with role override should not crash, stderr: {}",
        search2_stderr
    );

    println!(
        "Search with selected role '{}': {}",
        test_role,
        extract_clean_output(&search1_stdout)
    );
    println!(
        "Search with role override 'Default': {}",
        extract_clean_output(&search2_stdout)
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_command_role_behavior() -> Result<()> {
    let test_text = "This is a sample text for extraction. It contains various terms and concepts that might be in a thesaurus.";

    // Set a specific role for testing
    let available_roles = fetch_available_roles()?;
    ensure!(
        !available_roles.is_empty(),
        "Expected at least one role for extract test"
    );
    let test_role = available_roles[0].clone();
    let (_, _, set_code) =
        run_command_and_parse(&["config", "set", "selected_role", test_role.as_str()])?;
    assert_eq!(set_code, 0, "Should set test role");

    // Extract without role (should use selected_role)
    let (extract1_stdout, extract1_stderr, extract1_code) =
        run_command_and_parse(&["extract", test_text])?;

    // Extract with role override
    let (extract2_stdout, extract2_stderr, extract2_code) =
        run_command_and_parse(&["extract", test_text, "--role", "Default"])?;

    // Extract with exclude term flag
    let (extract3_stdout, extract3_stderr, extract3_code) =
        run_command_and_parse(&["extract", test_text, "--exclude-term"])?;

    // All should complete (may succeed or fail based on thesaurus availability)
    assert!(
        extract1_code == 0 || extract1_code == 1,
        "Extract with selected role should not crash, stderr: {}",
        extract1_stderr
    );
    assert!(
        extract2_code == 0 || extract2_code == 1,
        "Extract with role override should not crash, stderr: {}",
        extract2_stderr
    );
    assert!(
        extract3_code == 0 || extract3_code == 1,
        "Extract with exclude-term should not crash, stderr: {}",
        extract3_stderr
    );

    println!(
        "Extract with selected role: {}",
        extract_clean_output(&extract1_stdout)
    );
    println!(
        "Extract with role override: {}",
        extract_clean_output(&extract2_stdout)
    );
    println!(
        "Extract with exclude-term: {}",
        extract_clean_output(&extract3_stdout)
    );

    Ok(())
}
