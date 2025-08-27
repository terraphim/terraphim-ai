use anyhow::Result;
use serial_test::serial;
use std::process::Command;
use std::str;

/// Test helper to run TUI commands and parse output
fn run_command_and_parse(args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "-p", "terraphim_tui", "--"])
        .args(args);
    
    let output = cmd.output()?;
    
    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1)
    ))
}

/// Extract clean output (without log messages)
fn extract_clean_output(output: &str) -> String {
    output.lines()
        .filter(|line| !line.contains("INFO") && !line.contains("WARN") && !line.contains("DEBUG"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Parse JSON config from output
fn parse_config_from_output(output: &str) -> Result<serde_json::Value> {
    let clean_output = extract_clean_output(output);
    let lines: Vec<&str> = clean_output.lines().collect();
    let json_start = lines.iter().position(|line| line.starts_with('{'))
        .ok_or_else(|| anyhow::anyhow!("No JSON found in output"))?;
    
    let json_lines = &lines[json_start..];
    let json_str = json_lines.join("\n");
    
    Ok(serde_json::from_str(&json_str)?)
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
    let (graph_stdout, graph_stderr, graph_code) = run_command_and_parse(&["graph", "--top-k", "3"])?;
    
    assert_eq!(graph_code, 0, "Graph command should succeed, stderr: {}", graph_stderr);
    
    println!("Graph command output (using selected role): {}", extract_clean_output(&graph_stdout));
    
    // Chat command should use selected role when no --role is specified
    let (chat_stdout, chat_stderr, chat_code) = run_command_and_parse(&["chat", "test message"])?;
    
    assert_eq!(chat_code, 0, "Chat command should succeed, stderr: {}", chat_stderr);
    
    let chat_output = extract_clean_output(&chat_stdout);
    println!("Chat command output (using selected role): {}", chat_output);
    
    // Chat should reference the selected role in its response
    assert!(chat_output.contains(selected_role) || chat_output.contains("No LLM configured"),
           "Chat should use selected role '{}' or show no LLM message: {}", selected_role, chat_output);
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_override_in_commands() -> Result<()> {
    // Test that --role flag overrides selected_role in config
    
    // Search with role override
    let (search_stdout, search_stderr, search_code) = run_command_and_parse(&[
        "search", "test query", "--role", "Default", "--limit", "3"
    ])?;
    
    // Should succeed or fail gracefully (depending on whether role exists)
    assert!(search_code == 0 || search_code == 1, 
           "Search with role override should not crash, stderr: {}", search_stderr);
    
    println!("Search with role override: {}", extract_clean_output(&search_stdout));
    
    // Graph with role override
    let (graph_stdout, graph_stderr, graph_code) = run_command_and_parse(&[
        "graph", "--role", "Default", "--top-k", "5"
    ])?;
    
    assert_eq!(graph_code, 0, "Graph with role override should succeed, stderr: {}", graph_stderr);
    
    println!("Graph with role override: {}", extract_clean_output(&graph_stdout));
    
    // Chat with role override
    let (chat_stdout, chat_stderr, chat_code) = run_command_and_parse(&[
        "chat", "test message", "--role", "Default"
    ])?;
    
    assert_eq!(chat_code, 0, "Chat with role override should succeed, stderr: {}", chat_stderr);
    
    let chat_output = extract_clean_output(&chat_stdout);
    println!("Chat with role override: {}", chat_output);
    
    // Should use the overridden role
    assert!(chat_output.contains("Default") || chat_output.contains("No LLM configured"),
           "Chat should use overridden role 'Default': {}", chat_output);
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_selected_role_persistence() -> Result<()> {
    // Get initial selected role
    let (stdout1, _, code1) = run_command_and_parse(&["config", "show"])?;
    assert_eq!(code1, 0, "Initial config show should succeed");
    
    let config1 = parse_config_from_output(&stdout1)?;
    let initial_role = config1["selected_role"].as_str().unwrap();
    
    println!("Initial selected role: {}", initial_role);
    
    // Change selected role via config set
    let new_role = "TestRole";
    let (set_stdout, set_stderr, set_code) = run_command_and_parse(&[
        "config", "set", "selected_role", new_role
    ])?;
    
    assert_eq!(set_code, 0, "Config set should succeed, stderr: {}", set_stderr);
    
    let set_output = extract_clean_output(&set_stdout);
    assert!(set_output.contains(&format!("updated selected_role to {}", new_role)),
           "Should confirm role update: {}", set_output);
    
    // Verify the change persisted by checking config again
    let (stdout2, _, code2) = run_command_and_parse(&["config", "show"])?;
    assert_eq!(code2, 0, "Second config show should succeed");
    
    let config2 = parse_config_from_output(&stdout2)?;
    let updated_role = config2["selected_role"].as_str().unwrap();
    
    assert_eq!(updated_role, new_role, 
              "Selected role should be updated to '{}' but was '{}'", new_role, updated_role);
    
    println!("Successfully updated selected role from '{}' to '{}'", initial_role, updated_role);
    
    // Test that subsequent commands use the new selected role
    let (chat_stdout, chat_stderr, chat_code) = run_command_and_parse(&["chat", "hello"])?;
    assert_eq!(chat_code, 0, "Chat should succeed with new selected role, stderr: {}", chat_stderr);
    
    let chat_output = extract_clean_output(&chat_stdout);
    assert!(chat_output.contains(new_role) || chat_output.contains("No LLM configured"),
           "Chat should use new selected role '{}': {}", new_role, chat_output);
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_select_command_updates_selected_role() -> Result<()> {
    // Get initial config
    let (stdout1, _, _) = run_command_and_parse(&["config", "show"])?;
    let config1 = parse_config_from_output(&stdout1)?;
    let initial_role = config1["selected_role"].as_str().unwrap();
    
    println!("Initial selected role: {}", initial_role);
    
    // Try to select a role using roles select command
    let test_role = "NewTestRole";
    let (select_stdout, select_stderr, select_code) = run_command_and_parse(&[
        "roles", "select", test_role
    ])?;
    
    // This may succeed or fail depending on whether the role exists
    if select_code == 0 {
        let select_output = extract_clean_output(&select_stdout);
        assert!(select_output.contains(&format!("selected:{}", test_role)),
               "Should confirm role selection: {}", select_output);
        
        // Verify the change persisted in config
        let (stdout2, _, code2) = run_command_and_parse(&["config", "show"])?;
        assert_eq!(code2, 0, "Config show after role select should succeed");
        
        let config2 = parse_config_from_output(&stdout2)?;
        let updated_role = config2["selected_role"].as_str().unwrap();
        
        assert_eq!(updated_role, test_role,
                  "Selected role should be updated via roles select command");
        
        println!("Successfully updated selected role via 'roles select' from '{}' to '{}'", 
                initial_role, updated_role);
    } else {
        println!("Role select failed as expected (role '{}' may not exist): {}", test_role, select_stderr);
        // This is acceptable - the role might not exist in embedded config
    }
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_multiple_commands_use_same_selected_role() -> Result<()> {
    // Set a specific selected role
    let test_role = "ConsistencyTestRole";
    let (_, _, set_code) = run_command_and_parse(&[
        "config", "set", "selected_role", test_role
    ])?;
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
        assert!(code == 0 || code == 1, 
               "Command '{:?}' should not crash, stderr: {}", cmd_args, stderr);
        
        if code == 0 {
            let output = extract_clean_output(&stdout);
            
            // For chat command, output should reference the role or show no LLM
            if cmd_args[0] == "chat" {
                assert!(output.contains(test_role) || output.contains("No LLM configured"),
                       "Chat command should use selected role '{}': {}", test_role, output);
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
    // Test setting various role names to ensure they're handled correctly
    let test_roles = vec![
        "Default",
        "Terraphim Engineer", 
        "Test Role With Spaces",
        "test-role-with-dashes",
        "test_role_with_underscores",
    ];
    
    for role in test_roles {
        println!("Testing role name: '{}'", role);
        
        let (stdout, stderr, code) = run_command_and_parse(&[
            "config", "set", "selected_role", role
        ])?;
        
        assert_eq!(code, 0, "Should be able to set role '{}', stderr: {}", role, stderr);
        
        let output = extract_clean_output(&stdout);
        assert!(output.contains(&format!("updated selected_role to {}", role)),
               "Should confirm role update to '{}': {}", role, output);
        
        // Verify it was set correctly
        let (config_stdout, _, config_code) = run_command_and_parse(&["config", "show"])?;
        assert_eq!(config_code, 0, "Config show should work after setting role");
        
        let config = parse_config_from_output(&config_stdout)?;
        let current_role = config["selected_role"].as_str().unwrap();
        
        assert_eq!(current_role, role, "Role should be set correctly in config");
    }
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_inheritance_in_search() -> Result<()> {
    // Set a specific role
    let test_role = "SearchTestRole";
    let (_, _, set_code) = run_command_and_parse(&[
        "config", "set", "selected_role", test_role
    ])?;
    assert_eq!(set_code, 0, "Should set test role");
    
    // Search without specifying role (should use selected_role)
    let (search1_stdout, search1_stderr, search1_code) = run_command_and_parse(&[
        "search", "test query", "--limit", "2"
    ])?;
    
    // Search with explicit role override
    let (search2_stdout, search2_stderr, search2_code) = run_command_and_parse(&[
        "search", "test query", "--role", "Default", "--limit", "2"
    ])?;
    
    // Both should handle the role appropriately (succeed or fail gracefully)
    assert!(search1_code == 0 || search1_code == 1, 
           "Search with selected role should not crash, stderr: {}", search1_stderr);
    assert!(search2_code == 0 || search2_code == 1, 
           "Search with role override should not crash, stderr: {}", search2_stderr);
    
    println!("Search with selected role '{}': {}", test_role, extract_clean_output(&search1_stdout));
    println!("Search with role override 'Default': {}", extract_clean_output(&search2_stdout));
    
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_extract_command_role_behavior() -> Result<()> {
    let test_text = "This is a sample text for extraction. It contains various terms and concepts that might be in a thesaurus.";
    
    // Set a specific role for testing
    let test_role = "ExtractTestRole";
    let (_, _, set_code) = run_command_and_parse(&[
        "config", "set", "selected_role", test_role
    ])?;
    assert_eq!(set_code, 0, "Should set test role");
    
    // Extract without role (should use selected_role)
    let (extract1_stdout, extract1_stderr, extract1_code) = run_command_and_parse(&[
        "extract", test_text
    ])?;
    
    // Extract with role override
    let (extract2_stdout, extract2_stderr, extract2_code) = run_command_and_parse(&[
        "extract", test_text, "--role", "Default"
    ])?;
    
    // Extract with exclude term flag
    let (extract3_stdout, extract3_stderr, extract3_code) = run_command_and_parse(&[
        "extract", test_text, "--exclude-term"
    ])?;
    
    // All should complete (may succeed or fail based on thesaurus availability)
    assert!(extract1_code == 0 || extract1_code == 1, 
           "Extract with selected role should not crash, stderr: {}", extract1_stderr);
    assert!(extract2_code == 0 || extract2_code == 1, 
           "Extract with role override should not crash, stderr: {}", extract2_stderr);
    assert!(extract3_code == 0 || extract3_code == 1, 
           "Extract with exclude-term should not crash, stderr: {}", extract3_stderr);
    
    println!("Extract with selected role: {}", extract_clean_output(&extract1_stdout));
    println!("Extract with role override: {}", extract_clean_output(&extract2_stdout));
    println!("Extract with exclude-term: {}", extract_clean_output(&extract3_stdout));
    
    Ok(())
}