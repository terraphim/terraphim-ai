use anyhow::Result;
use serial_test::serial;
use std::process::{Child, Command, Stdio};
use std::str;
use std::thread;
use std::time::Duration;
use tokio::time::timeout;

/// Test helper to start a real terraphim server for testing
async fn start_test_server() -> Result<(Child, String)> {
    // Find an available port
    let port = portpicker::pick_unused_port().expect("Failed to find unused port");
    let server_url = format!("http://localhost:{}", port);

    println!("Starting test server on {}", server_url);

    // Start the server with terraphim engineer config
    let mut server = Command::new("cargo")
        .args([
            "run",
            "-p",
            "terraphim_server",
            "--",
            "--config",
            "terraphim_server/default/terraphim_engineer_config.json",
        ])
        .env("TERRAPHIM_SERVER_PORT", port.to_string())
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for server to be ready
    let client = reqwest::Client::new();
    let health_url = format!("{}/health", server_url);

    println!("Waiting for server to be ready at {}", health_url);

    // Try to connect for up to 30 seconds
    for attempt in 1..=30 {
        thread::sleep(Duration::from_secs(1));

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                println!("Server ready after {} seconds", attempt);
                return Ok((server, server_url));
            }
            Ok(_) => println!("Server responding but not healthy (attempt {})", attempt),
            Err(_) => println!("Server not responding yet (attempt {})", attempt),
        }

        // Check if server process is still running
        match server.try_wait() {
            Ok(Some(status)) => {
                return Err(anyhow::anyhow!(
                    "Server exited early with status: {}",
                    status
                ));
            }
            Ok(None) => {} // Still running
            Err(e) => return Err(anyhow::anyhow!("Error checking server status: {}", e)),
        }
    }

    // Kill server if we couldn't connect
    let _ = server.kill();
    Err(anyhow::anyhow!(
        "Server failed to become ready within 30 seconds"
    ))
}

/// Test helper to run TUI commands against a real server
fn run_server_command(server_url: &str, args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd_args = vec!["--server", "--server-url", server_url];
    cmd_args.extend_from_slice(args);

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--"])
        .args(&cmd_args);

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    ))
}

#[tokio::test]
#[serial]
async fn test_server_mode_config_show() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Test config show with real server
    let (stdout, stderr, code) = run_server_command(&server_url, &["config", "show"])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode config show should succeed, stderr: {}",
        stderr
    );

    // Parse JSON output
    let lines: Vec<&str> = stdout.lines().collect();
    let json_start = lines.iter().position(|line| line.starts_with('{'));
    assert!(json_start.is_some(), "Should contain JSON output");

    let json_lines = &lines[json_start.unwrap()..];
    let json_str = json_lines.join("\n");

    let config: serde_json::Value = serde_json::from_str(&json_str).expect("Should be valid JSON");

    assert_eq!(config["id"], "Server", "Should use Server config");
    assert!(
        config.get("selected_role").is_some(),
        "Should have selected_role"
    );
    assert_eq!(
        config["selected_role"], "Terraphim Engineer",
        "Should have Terraphim Engineer as selected role"
    );

    println!("Server config: {}", json_str);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_roles_list() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Test roles list with real server
    let (stdout, stderr, code) = run_server_command(&server_url, &["roles", "list"])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode roles list should succeed, stderr: {}",
        stderr
    );

    // Should have roles from terraphim engineer config
    let roles: Vec<&str> = stdout.lines().collect();
    println!("Available roles: {:?}", roles);

    // Terraphim engineer config should have at least these roles
    assert!(
        roles
            .iter()
            .any(|r| r.contains("Terraphim Engineer") || r.contains("Engineer")),
        "Should have Terraphim Engineer role: {:?}",
        roles
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_search_with_selected_role() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Give server time to index documents
    thread::sleep(Duration::from_secs(3));

    // Test search using selected role (should be Terraphim Engineer)
    let (stdout, stderr, code) =
        run_server_command(&server_url, &["search", "rust programming", "--limit", "5"])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode search should succeed, stderr: {}",
        stderr
    );

    println!("Search results: {}", stdout);

    // Should have some results or at least not error
    // Results format: "- <rank>\t<title>"
    let result_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    println!("Found {} search results", result_lines.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_search_with_role_override() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Give server time to index documents
    thread::sleep(Duration::from_secs(2));

    // Test search with role override
    let (stdout, stderr, code) = run_server_command(
        &server_url,
        &["search", "test", "--role", "Default", "--limit", "3"],
    )?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    // Search may succeed or fail depending on whether Default role exists
    assert!(
        code == 0 || code == 1,
        "Search with role override should not crash, stderr: {}",
        stderr
    );

    if code == 0 {
        println!("Search with role override successful: {}", stdout);
    } else {
        println!(
            "Search with role override failed (role may not exist): {}",
            stderr
        );
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_roles_select() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // First get available roles
    let (roles_stdout, _, roles_code) = run_server_command(&server_url, &["roles", "list"])?;
    assert_eq!(roles_code, 0, "Should be able to list roles");

    let roles: Vec<&str> = roles_stdout.lines().collect();
    if roles.is_empty() {
        println!("No roles available for selection test");
        let _ = server.kill();
        return Ok(());
    }

    let first_role = roles[0].trim();
    println!("Selecting first available role: {}", first_role);

    // Test role selection
    let (stdout, stderr, code) = run_server_command(&server_url, &["roles", "select", first_role])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode role select should succeed, stderr: {}",
        stderr
    );
    assert!(
        stdout.contains(&format!("selected:{}", first_role)),
        "Should confirm role selection: {}",
        stdout
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_graph_command() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Give server time to build knowledge graph
    thread::sleep(Duration::from_secs(5));

    // Test graph command with real server
    let (stdout, stderr, code) = run_server_command(&server_url, &["graph", "--top-k", "10"])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode graph should succeed, stderr: {}",
        stderr
    );

    println!("Graph concepts: {}", stdout);

    // Should show some concepts
    let concept_lines: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    println!("Found {} graph concepts", concept_lines.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_chat_command() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Test chat command with real server
    let (stdout, stderr, code) = run_server_command(&server_url, &["chat", "Hello, how are you?"])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode chat should succeed, stderr: {}",
        stderr
    );

    println!("Chat response: {}", stdout);

    // Should have some response (even if no LLM configured)
    assert!(!stdout.trim().is_empty(), "Should have some chat response");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_extract_command() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Give server time to load thesaurus
    thread::sleep(Duration::from_secs(3));

    let test_text = "This is a test paragraph about Rust programming. Rust is a systems programming language that focuses on safety and performance. It has concepts like ownership, borrowing, and lifetimes.";

    // Test extract command with real server
    let (stdout, stderr, code) = run_server_command(&server_url, &["extract", test_text])?;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode extract should succeed, stderr: {}",
        stderr
    );

    println!("Extract results: {}", stdout);

    // Should either find matches or report no matches
    assert!(
        stdout.contains("Found") || stdout.contains("No matches"),
        "Should report extract results: {}",
        stdout
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_mode_config_set() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Test config set with real server
    let (stdout, stderr, code) = run_server_command(
        &server_url,
        &["config", "set", "selected_role", "Terraphim Engineer"],
    )?;

    // Cleanup server first
    let _ = server.kill();
    let _ = server.wait();

    assert_eq!(
        code, 0,
        "Server mode config set should succeed, stderr: {}",
        stderr
    );
    assert!(
        stdout.contains("updated selected_role to Terraphim Engineer"),
        "Should confirm config update: {}",
        stdout
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_vs_offline_mode_comparison() -> Result<()> {
    // Start server for comparison
    let (mut server, server_url) = start_test_server().await?;

    // Test the same command in both modes
    let (server_stdout, _server_stderr, server_code) =
        run_server_command(&server_url, &["config", "show"])?;

    // Cleanup server
    let _ = server.kill();
    let _ = server.wait();

    // Run offline command
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_tui", "--"])
        .args(&["config", "show"]);

    let offline_output = cmd.output()?;
    let offline_stdout = String::from_utf8_lossy(&offline_output.stdout);
    let offline_code = offline_output.status.code().unwrap_or(-1);

    // Both should succeed
    assert_eq!(server_code, 0, "Server mode should succeed");
    assert_eq!(offline_code, 0, "Offline mode should succeed");

    // Parse both configs
    let parse_config = |output: &str| -> serde_json::Value {
        let lines: Vec<&str> = output.lines().collect();
        let json_start = lines.iter().position(|line| line.starts_with('{')).unwrap();
        let json_lines = &lines[json_start..];
        let json_str = json_lines.join("\n");
        serde_json::from_str(&json_str).unwrap()
    };

    let server_config = parse_config(&server_stdout);
    let offline_config = parse_config(&offline_stdout);

    // Compare key differences
    assert_eq!(
        server_config["id"], "Server",
        "Server should use Server config"
    );
    assert_eq!(
        offline_config["id"], "Embedded",
        "Offline should use Embedded config"
    );

    println!("Server config ID: {}", server_config["id"]);
    println!("Offline config ID: {}", offline_config["id"]);
    println!("Server selected_role: {}", server_config["selected_role"]);
    println!("Offline selected_role: {}", offline_config["selected_role"]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_server_startup_and_health() -> Result<()> {
    let (mut server, server_url) = start_test_server().await?;

    // Test that server is actually healthy
    let client = reqwest::Client::new();
    let health_url = format!("{}/health", server_url);

    let response = timeout(Duration::from_secs(5), client.get(&health_url).send()).await??;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    assert!(
        response.status().is_success(),
        "Server health check should pass"
    );

    println!("Server health check passed: {}", response.status());

    Ok(())
}
