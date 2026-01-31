use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_mcp_autocomplete_via_stdio() {
    // This test drives stdio JSON-RPC manually. It is sensitive to timing/framing and
    // can be flaky in CI environments. The rmcp-based integration tests cover the same
    // functionality more reliably.
    if std::env::var("RUN_MCP_STDIO_TEST").ok().as_deref() != Some("1") {
        eprintln!("Skipping: set RUN_MCP_STDIO_TEST=1 to run");
        return;
    }
    // Set the environment variable for local dev settings
    unsafe {
        env::set_var(
            "TERRAPHIM_SETTINGS_PATH",
            "../terraphim_settings/default/settings_local_dev.toml",
        );
    }

    // Start the MCP server
    // NOTE: Don't pass --verbose here. It can enable non-JSON output on stdout,
    // which breaks stdio JSON-RPC framing.
    let mut child = Command::new("cargo")
        .args(["run", "--"])
        .current_dir(".")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Note: don't sleep here; instead we do a request/response handshake below.
    // (Sleeping can be flaky and can race with server startup.)

    // Step 1: Send initialization request
    let init_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "MCP Test Client",
                "version": "1.0.0"
            }
        }
    });

    println!("1. Sending initialization request...");
    writeln!(stdin, "{}", init_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Response: {}", response.trim());

    // Step 2: List available tools
    let tools_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    println!("2. Listing available tools...");
    writeln!(stdin, "{}", tools_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read tools/list response (with retry, because server may still be initializing)
    response.clear();
    let mut got_tools = false;
    for _ in 0..20 {
        response.clear();
        if reader.read_line(&mut response).is_err() {
            break;
        }
        if response.trim().is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }
        println!("Tools list response: {}", response.trim());

        if let Ok(tools_response) = serde_json::from_str::<serde_json::Value>(&response) {
            if tools_response.get("result").is_some() {
                println!("Parsed tools response: {:#?}", tools_response);
                got_tools = true;
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    assert!(
        got_tools,
        "Did not receive tools/list response from MCP server"
    );

    // Step 3: Build autocomplete index
    let build_index_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "build_autocomplete_index",
            "arguments": {
                "role": "Terraphim Engineer"
            }
        }
    });

    println!("3. Building autocomplete index...");
    writeln!(stdin, "{}", build_index_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Build index response: {}", response.trim());

    // If the server exits unexpectedly, avoid BrokenPipe errors later.
    assert!(
        child
            .try_wait()
            .expect("Failed to check child status")
            .is_none(),
        "MCP server exited early"
    );

    // Step 4: Test autocomplete with snippets
    let autocomplete_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "autocomplete_with_snippets",
            "arguments": {
                "query": "terraphim",
                "limit": 5,
                "role": "Terraphim Engineer"
            }
        }
    });

    println!("4. Testing autocomplete with snippets...");
    writeln!(stdin, "{}", autocomplete_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Autocomplete response: {}", response.trim());

    // Step 5: Test autocomplete terms
    let terms_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "autocomplete_terms",
            "arguments": {
                "query": "terraphim",
                "limit": 5,
                "role": "Terraphim Engineer"
            }
        }
    });

    println!("5. Testing autocomplete terms...");
    writeln!(stdin, "{}", terms_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Terms response: {}", response.trim());

    println!("Test completed successfully!");

    // Clean up
    child.kill().expect("Failed to kill child process");
    child.wait().expect("Failed to wait for child");
}
