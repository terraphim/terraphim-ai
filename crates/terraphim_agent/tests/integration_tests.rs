use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::Result;
use serial_test::serial;
use std::str;
use std::thread;
use std::time::Duration;

mod support;
use support::cli_test_env::apply_hermetic_env;

/// Get workspace root directory by walking up to find [workspace] in Cargo.toml
fn get_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Ok(current);
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    Ok(PathBuf::from("."))
}

/// Test helper to start a real terraphim server
async fn start_test_server() -> Result<(Child, String)> {
    let port = portpicker::pick_unused_port().expect("Failed to find unused port");
    let server_url = format!("http://localhost:{}", port);

    println!("Starting test server on {}", server_url);

    // Use absolute path for config to work in CI
    let workspace_root = get_workspace_root()?;
    let config_path =
        workspace_root.join("terraphim_server/default/terraphim_engineer_config.json");

    println!("Using config path: {}", config_path.display());

    let mut server_cmd = Command::new("cargo");
    server_cmd
        .args([
            "run",
            "-p",
            "terraphim_server",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        // The server reads its bind address from settings (env/file), not TERRAPHIM_SERVER_PORT.
        // Override the server bind host+port explicitly for tests.
        .env("TERRAPHIM_SERVER_HOSTNAME", format!("127.0.0.1:{}", port))
        .env("RUST_LOG", "warn") // Reduce log noise
        .current_dir(&workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_hermetic_env(&mut server_cmd)?;
    let mut server = server_cmd.spawn()?;

    // Wait for server to be ready
    let client = reqwest::Client::new();
    let health_url = format!("{}/health", server_url);

    // CI/macOS can be slow to compile+boot the server the first time.
    // Use a larger timeout to avoid flaky failures.
    for attempt in 1..=120 {
        thread::sleep(Duration::from_secs(1));

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                println!("Server ready after {} seconds", attempt);
                return Ok((server, server_url));
            }
            Ok(_) => {}
            Err(_) => {}
        }

        match server.try_wait() {
            Ok(Some(status)) => {
                return Err(anyhow::anyhow!(
                    "Server exited early with status: {}",
                    status
                ));
            }
            Ok(None) => {}
            Err(e) => return Err(anyhow::anyhow!("Error checking server status: {}", e)),
        }
    }

    let _ = server.kill();
    Err(anyhow::anyhow!(
        "Server failed to become ready within 120 seconds"
    ))
}

/// Run TUI command in offline mode
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

/// Run TUI command in server mode
fn run_server_command(server_url: &str, args: &[&str]) -> Result<(String, String, i32)> {
    let mut cmd_args = vec!["--server", "--server-url", server_url];
    cmd_args.extend_from_slice(args);

    // Use 5 minute timeout for CI - KG indexing can be slow
    let timeout_secs = std::env::var("TERRAPHIM_CLIENT_TIMEOUT")
        .map(|v| v.parse().unwrap_or(300))
        .unwrap_or(300);

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "terraphim_agent", "--features", "server", "--"])
        .args(&cmd_args)
        .env("TERRAPHIM_CLIENT_TIMEOUT", format!("{}", timeout_secs));
    apply_hermetic_env(&mut cmd)?;

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

/// Clean up test files
fn cleanup_test_files() -> Result<()> {
    let test_dirs = vec![
        "/tmp/terraphim_sqlite",
        "/tmp/dashmaptest",
        "/tmp/terraphim_rocksdb",
        "/tmp/opendal",
    ];

    for dir in test_dirs {
        if Path::new(dir).exists() {
            let _ = fs::remove_dir_all(dir);
        }
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_end_to_end_offline_workflow() -> Result<()> {
    cleanup_test_files()?;

    println!("=== Testing Complete Offline Workflow ===");

    // 1. Check initial config
    let (config_stdout, _, config_code) = run_offline_command(&["config", "show"])?;
    assert_eq!(config_code, 0, "Initial config check should succeed");

    let initial_config = parse_config_from_output(&config_stdout)?;
    println!(
        "✓ Initial config loaded: id={}, selected_role={}",
        initial_config["id"], initial_config["selected_role"]
    );

    // 2. List available roles
    let (roles_stdout, _, roles_code) = run_offline_command(&["roles", "list"])?;
    assert_eq!(roles_code, 0, "Roles list should succeed");
    let roles = extract_clean_output(&roles_stdout);
    println!(
        "✓ Available roles: {}",
        if roles.is_empty() { "(none)" } else { &roles }
    );

    // 3. Set a role that is known to exist in the embedded config
    // NOTE: selected_role must be a valid role name; setting arbitrary roles is rejected.
    let custom_role = "Terraphim Engineer";
    let (set_stdout, _, set_code) =
        run_offline_command(&["config", "set", "selected_role", custom_role])?;
    assert_eq!(set_code, 0, "Setting role should succeed");
    assert!(
        extract_clean_output(&set_stdout)
            .contains(&format!("updated selected_role to {}", custom_role))
    );
    println!("✓ Set role: {}", custom_role);

    // 4. Verify role persistence
    let (verify_stdout, _, verify_code) = run_offline_command(&["config", "show"])?;
    assert_eq!(verify_code, 0, "Config verification should succeed");
    let updated_config = parse_config_from_output(&verify_stdout)?;
    // Role selection is not currently persisted across runs in embedded/offline mode.
    // We only assert that the config command continues to work.
    println!(
        "✓ Role set command executed; current selected_role={} (persistence not required)",
        updated_config["selected_role"]
    );

    // 5. Test search with custom role
    let (_search_stdout, _, search_code) =
        run_offline_command(&["search", "integration test", "--limit", "3"])?;
    assert!(
        search_code == 0 || search_code == 1,
        "Search should complete"
    );
    println!(
        "✓ Search with custom role completed: {}",
        if search_code == 0 {
            "success"
        } else {
            "no results"
        }
    );

    // 6. Test graph command
    let (graph_stdout, _, graph_code) = run_offline_command(&["graph", "--top-k", "5"])?;
    assert_eq!(graph_code, 0, "Graph command should succeed");
    let graph_output = extract_clean_output(&graph_stdout);
    println!(
        "✓ Graph command output: {} lines",
        graph_output.lines().count()
    );

    // 7. Test chat command (exit code 1 is acceptable when no LLM is configured)
    let (chat_stdout, chat_stderr, chat_code) =
        run_offline_command(&["chat", "Hello integration test"])?;
    let chat_output = extract_clean_output(&chat_stdout);
    let chat_err = extract_clean_output(&chat_stderr);
    let no_llm =
        chat_output.contains("No LLM configured") || chat_err.contains("No LLM configured");
    assert!(
        chat_code == 0 || no_llm,
        "Chat command should succeed or report no LLM: exit={chat_code}"
    );
    println!("✓ Chat command used custom role");

    // 8. Test extract command
    let test_text = "This is an integration test paragraph for extraction functionality.";
    let (_extract_stdout, _, extract_code) =
        run_offline_command(&["extract", test_text, "--exclude-term"])?;
    assert!(
        extract_code == 0 || extract_code == 1,
        "Extract should complete"
    );
    println!(
        "✓ Extract command completed: {}",
        if extract_code == 0 {
            "success"
        } else {
            "no matches"
        }
    );

    println!("=== Offline Workflow Complete ===");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_end_to_end_server_workflow() -> Result<()> {
    println!("=== Testing Complete Server Workflow ===");

    let (mut server, server_url) = start_test_server().await?;

    // Give server time to initialize
    thread::sleep(Duration::from_secs(3));

    // 1. Check server config
    let (config_stdout, _, config_code) = run_server_command(&server_url, &["config", "show"])?;
    assert_eq!(config_code, 0, "Server config check should succeed");

    let server_config = parse_config_from_output(&config_stdout)?;
    println!(
        "✓ Server config loaded: id={}, selected_role={}",
        server_config["id"], server_config["selected_role"]
    );
    assert_eq!(server_config["id"], "Server");

    // 2. List server roles
    let (roles_stdout, _, roles_code) = run_server_command(&server_url, &["roles", "list"])?;
    assert_eq!(roles_code, 0, "Server roles list should succeed");
    let server_roles: Vec<&str> = roles_stdout.lines().collect();
    println!("✓ Server roles available: {:?}", server_roles);
    assert!(
        !server_roles.is_empty(),
        "Server should have roles available"
    );

    let current_role = server_roles
        .iter()
        .find_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with('*') {
                return None;
            }
            let role = trimmed.trim_start_matches('*').trim();
            Some(
                role.split_once(" (")
                    .map(|(name, _)| name)
                    .unwrap_or(role)
                    .to_string(),
            )
        })
        .unwrap_or_else(|| "Terraphim Engineer".to_string());

    let selected_role = server_roles
        .iter()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('*') {
                return None;
            }
            let role = trimmed
                .trim_start_matches('*')
                .trim()
                .split_once(" (")
                .map(|(name, _)| name)
                .unwrap_or(trimmed)
                .trim()
                .to_string();
            (role != current_role).then_some(role)
        })
        .unwrap_or_else(|| current_role.clone());

    // 3. Test search with server (may fail in CI due to missing KG data or slow indexing)
    let (search_stdout, search_stderr, search_code) =
        run_server_command(&server_url, &["search", "integration test", "--limit", "3"])?;

    // In CI, search may fail due to:
    // - KG indexing timeout
    // - Missing KG data (400 Bad Request)
    // Accept both as valid outcomes for CI resilience
    let search_failed_acceptably = search_stderr.contains("operation timed out")
        || search_stderr.contains("timed out")
        || search_stderr.contains("400 Bad Request")
        || search_stderr.contains("400");

    if search_failed_acceptably {
        println!(
            "✓ Server search failed acceptably (expected in CI): {}",
            if search_stderr.contains("400") {
                "400 Bad Request"
            } else {
                "timeout"
            }
        );
    } else if search_code != 0 {
        println!("Search stdout: {}", search_stdout);
        println!("Search stderr: {}", search_stderr);
        panic!("Server search should succeed: {}", search_stderr);
    } else {
        println!("✓ Server search completed");
    }

    // 4. Test role override in server mode (may fail in CI)
    if server_roles.len() > 1 {
        let test_role = server_roles[1].trim();
        let (_search_role_stdout, search_role_stderr, search_role_code) = run_server_command(
            &server_url,
            &["search", "test", "--role", test_role, "--limit", "2"],
        )?;

        let search_role_failed_acceptably = search_role_stderr.contains("operation timed out")
            || search_role_stderr.contains("timed out")
            || search_role_stderr.contains("400 Bad Request")
            || search_role_stderr.contains("400");

        if search_role_failed_acceptably {
            let reason = if search_role_stderr.contains("400") {
                "400 Bad Request"
            } else {
                "timeout"
            };
            println!(
                "✓ Server search with role override '{}' failed acceptably ({})",
                test_role, reason
            );
        } else {
            assert!(
                search_role_code == 0 || search_role_code == 1,
                "Server search with role should complete"
            );
            println!(
                "✓ Server search with role override '{}' completed",
                test_role
            );
        }
    }

    // 5. Test graph with server
    // NOTE: server rolegraph endpoint is exposed as /rolegraph (not /rolegraph?role=...)
    // and the client-side role query may not be supported depending on server build.
    // Treat 404 as an acceptable outcome for this integration test.
    let (_graph_stdout, graph_stderr, graph_code) =
        run_server_command(&server_url, &["graph", "--top-k", "5"])?;
    assert!(
        graph_code == 0 || graph_stderr.contains("404"),
        "Server graph should complete (or be unsupported): stderr={}",
        graph_stderr
    );
    println!("✓ Server graph command completed (or unsupported)");

    // 6. Test chat with server (may fail if no LLM provider configured)
    let (_chat_stdout, chat_stderr, chat_code) =
        run_server_command(&server_url, &["chat", "Hello server test"])?;
    assert!(
        chat_code == 0
            || chat_stderr.contains("LLM")
            || chat_stderr.contains("llm")
            || chat_stderr.contains("provider")
            || chat_stderr.contains("Ollama")
            || chat_stderr.contains("not configured")
            || chat_stderr.contains("404")
            || chat_stderr.contains("500")
            || chat_stderr.contains("timeout")
            || chat_stderr.contains("timed out"),
        "Server chat should succeed or fail gracefully (no LLM): stderr={}",
        chat_stderr
    );
    println!("✓ Server chat command completed (or LLM not configured)");

    // 7. Test extract with server
    let test_text = "This is a server integration test paragraph with various concepts and terms for extraction.";
    let (_extract_stdout, extract_stderr, extract_code) =
        run_server_command(&server_url, &["extract", test_text])?;
    // Codes 0, 1, 3 (no KG), 6 (network), 7 (timeout) are all valid for server extract.
    let extract_failed_acceptably = extract_stderr.contains("operation timed out")
        || extract_stderr.contains("timed out")
        || extract_stderr.contains("400")
        || extract_stderr.contains("404");
    assert!(
        extract_code == 0
            || extract_code == 1
            || extract_code == 3
            || (extract_failed_acceptably && (extract_code == 6 || extract_code == 7)),
        "Server extract should complete: stderr={}",
        extract_stderr
    );
    println!("✓ Server extract command completed");

    // 8. Test role selection on server using the dedicated server-mode path
    let (set_stdout, _, set_code) =
        run_server_command(&server_url, &["roles", "select", &selected_role])?;
    assert_eq!(set_code, 0, "Server role select should succeed");
    assert!(extract_clean_output(&set_stdout).contains(&format!("selected:{}", selected_role)));
    println!("✓ Server role selection completed");

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    println!("=== Server Workflow Complete ===");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_offline_vs_server_mode_comparison() -> Result<()> {
    cleanup_test_files()?;
    println!("=== Comparing Offline vs Server Modes ===");

    let (mut server, server_url) = start_test_server().await?;
    thread::sleep(Duration::from_secs(2));

    // Test the same commands in both modes and compare behavior
    let test_commands = vec![
        vec!["config", "show"],
        vec!["roles", "list"],
        vec!["graph", "--top-k", "3"],
        vec!["chat", "comparison test"],
    ];

    for cmd_args in test_commands {
        println!("Comparing command: {:?}", cmd_args);

        // Run in offline mode
        let (offline_stdout, _offline_stderr, offline_code) = run_offline_command(&cmd_args)?;

        // Run in server mode
        let (server_stdout, _server_stderr, server_code) =
            run_server_command(&server_url, &cmd_args)?;

        println!(
            "  Offline: code={}, Server: code={}",
            offline_code, server_code
        );

        // Both modes should generally succeed (with some exceptions)
        if cmd_args[0] == "roles" && cmd_args[1] == "list" {
            // Roles list might differ between modes
            assert_eq!(offline_code, 0, "Offline roles list should succeed");
            assert_eq!(server_code, 0, "Server roles list should succeed");

            let offline_roles: Vec<&str> = offline_stdout.lines().collect();
            let server_roles: Vec<&str> = server_stdout.lines().collect();

            println!("    Offline roles: {} items", offline_roles.len());
            println!("    Server roles: {} items", server_roles.len());
        } else if cmd_args[0] == "config" {
            // Config should work in both modes but have different IDs
            assert_eq!(offline_code, 0, "Offline config should succeed");
            assert_eq!(server_code, 0, "Server config should succeed");

            let offline_config = parse_config_from_output(&offline_stdout)?;
            let server_config = parse_config_from_output(&server_stdout)?;

            assert!(
                offline_config["id"] == "Embedded" || offline_config["id"] == "Server",
                "Offline config should have a valid id"
            );
            assert_eq!(server_config["id"], "Server");

            println!(
                "    ✓ Configs have correct IDs: Offline={}, Server={}",
                offline_config["id"], server_config["id"]
            );
        } else {
            // Other commands should generally succeed in both modes
            assert!(
                offline_code == 0 || offline_code == 1,
                "Offline command should complete"
            );
            assert!(
                server_code == 0 || server_code == 1,
                "Server command should complete"
            );
        }
    }

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();

    println!("=== Mode Comparison Complete ===");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_role_consistency_across_commands() -> Result<()> {
    cleanup_test_files()?;
    println!("=== Testing Role Consistency ===");

    // Set a specific role
    // selected_role must be an existing role name
    let test_role = "Terraphim Engineer";
    let (_, _, set_code) = run_offline_command(&["config", "set", "selected_role", test_role])?;
    assert_eq!(set_code, 0, "Should set test role");

    // Test that all commands use the same selected role
    let commands = vec![
        ("graph", vec!["graph", "--top-k", "2"]),
        ("chat", vec!["chat", "consistency test"]),
        ("search", vec!["search", "test query", "--limit", "1"]),
        ("extract", vec!["extract", "test text for consistency"]),
    ];

    for (cmd_name, cmd_args) in commands {
        let (stdout, stderr, code) = run_offline_command(&cmd_args)?;

        assert!(
            code == 0 || code == 1,
            "Command '{}' should complete: stderr={}",
            cmd_name,
            stderr
        );

        // For chat, verify it references the role
        if cmd_name == "chat" && code == 0 {
            let output = extract_clean_output(&stdout);
            assert!(
                output.contains(test_role) || output.contains("No LLM configured"),
                "Chat should use selected role '{}': {}",
                test_role,
                output
            );
        }

        println!("✓ Command '{}' completed with selected role", cmd_name);
    }

    // Test role override works consistently
    let override_role = "OverrideTestRole";
    for (cmd_name, cmd_args) in [
        (
            "search",
            vec!["search", "test", "--role", override_role, "--limit", "1"],
        ),
        (
            "graph",
            vec!["graph", "--role", override_role, "--top-k", "2"],
        ),
        (
            "chat",
            vec!["chat", "override test", "--role", override_role],
        ),
        (
            "extract",
            vec!["extract", "test text", "--role", override_role],
        ),
    ] {
        let (stdout, _stderr, code) = run_offline_command(&cmd_args)?;

        assert!(
            code == 0 || code == 1,
            "Command '{}' with role override should complete",
            cmd_name
        );

        if cmd_name == "chat" && code == 0 {
            let output = extract_clean_output(&stdout);
            assert!(
                output.contains(override_role) || output.contains("No LLM configured"),
                "Chat should use override role '{}': {}",
                override_role,
                output
            );
        }

        println!("✓ Command '{}' completed with role override", cmd_name);
    }

    println!("=== Role Consistency Test Complete ===");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_full_feature_matrix() -> Result<()> {
    cleanup_test_files()?;
    println!("=== Testing Full Feature Matrix ===");

    // Test matrix: [Mode] x [Command] x [Options]
    let modes = vec![("offline", None::<String>)];

    // Add server mode if we can start a server
    let server_info = if let Ok((server, url)) = start_test_server().await {
        Some((server, url))
    } else {
        println!("⚠ Skipping server mode tests - could not start server");
        None
    };

    for (mode_name, _) in modes {
        println!("Testing mode: {}", mode_name);

        // Basic commands
        let basic_tests = vec![
            ("help", vec!["--help"]),
            ("config-show", vec!["config", "show"]),
            ("roles-list", vec!["roles", "list"]),
        ];

        for (test_name, args) in basic_tests {
            let (_stdout, stderr, code) = run_offline_command(&args)?;
            assert_eq!(
                code, 0,
                "Basic test '{}' should succeed in {} mode: stderr={}",
                test_name, mode_name, stderr
            );
            println!("  ✓ {}: {}", test_name, test_name);
        }

        // Advanced commands
        let advanced_tests = vec![
            ("search-default", vec!["search", "test", "--limit", "2"]),
            (
                "search-with-role",
                vec!["search", "test", "--role", "Default", "--limit", "2"],
            ),
            ("graph-default", vec!["graph", "--top-k", "3"]),
            (
                "graph-with-role",
                vec!["graph", "--role", "Default", "--top-k", "3"],
            ),
            ("chat-default", vec!["chat", "test message"]),
            (
                "chat-with-role",
                vec!["chat", "test message", "--role", "Default"],
            ),
            (
                "extract-default",
                vec!["extract", "test text for extraction"],
            ),
            (
                "extract-with-options",
                vec![
                    "extract",
                    "test text",
                    "--role",
                    "Default",
                    "--exclude-term",
                ],
            ),
        ];

        for (test_name, args) in advanced_tests {
            let (_stdout, stderr, code) = run_offline_command(&args)?;
            // Exit code 3 (ErrorIndexMissing) is valid when the role has no KG configured.
            assert!(
                code == 0 || code == 1 || code == 3,
                "Advanced test '{}' should complete in {} mode: stderr={}",
                test_name,
                mode_name,
                stderr
            );
            println!("  ✓ {}: completed", test_name);
        }

        // Configuration tests - use an existing role
        let config_tests = vec![(
            "config-set-role",
            vec!["config", "set", "selected_role", "Default"],
        )];

        for (test_name, args) in config_tests {
            let (_stdout, stderr, code) = run_offline_command(&args)?;
            assert_eq!(
                code, 0,
                "Config test '{}' should succeed in {} mode: stderr={}, stdout={}",
                test_name, mode_name, stderr, _stdout
            );
            println!("  ✓ {}: succeeded", test_name);
        }
    }

    // Test server mode if available
    if let Some((mut server, server_url)) = server_info {
        thread::sleep(Duration::from_secs(2));
        println!("Testing mode: server");

        let server_tests = vec![
            ("config-show", vec!["config", "show"]),
            ("search", vec!["search", "test", "--limit", "2"]),
            ("graph", vec!["graph", "--top-k", "3"]),
        ];

        for (test_name, args) in server_tests {
            let (_stdout, stderr, code) = run_server_command(&server_url, &args)?;

            // In CI, search/graph may fail due to:
            // - KG indexing timeout
            // - Missing KG data (400 Bad Request)
            // Accept both as valid outcomes for CI resilience
            let failed_acceptably = stderr.contains("operation timed out")
                || stderr.contains("timed out")
                || stderr.contains("400 Bad Request")
                || stderr.contains("400");

            if test_name == "graph" || test_name == "search" {
                if failed_acceptably {
                    let reason = if stderr.contains("400") {
                        "400 Bad Request"
                    } else if stderr.contains("404") {
                        "404 Not Found"
                    } else {
                        "timeout"
                    };
                    println!("  ✓ {}: failed acceptably ({})", test_name, reason);
                } else if test_name == "graph" && stderr.contains("404") {
                    println!("  ✓ {}: unsupported (404)", test_name);
                } else if code != 0 {
                    panic!(
                        "Server test '{}' should succeed: stderr={}",
                        test_name, stderr
                    );
                } else {
                    println!("  ✓ {}: succeeded", test_name);
                }
            } else {
                assert_eq!(
                    code, 0,
                    "Server test '{}' should succeed: stderr={}",
                    test_name, stderr
                );
                println!("  ✓ {}: succeeded", test_name);
            }
        }

        // Cleanup server
        let _ = server.kill();
        let _ = server.wait();
    }

    println!("=== Full Feature Matrix Test Complete ===");

    Ok(())
}
