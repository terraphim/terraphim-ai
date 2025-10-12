use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_mcp_autocomplete_via_stdio() {
    // Set the environment variable for local dev settings
    env::set_var(
        "TERRAPHIM_SETTINGS_PATH",
        "../terraphim_settings/default/settings_local_dev.toml",
    );

    // Start the MCP server
    let mut child = Command::new("cargo")
        .args(["run", "--", "--verbose"])
        .current_dir(".")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Wait a bit for server to start
    std::thread::sleep(std::time::Duration::from_secs(3));

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

    // Read the complete tools list response
    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Tools list response: {}", response.trim());

    // Parse the response to see what tools are available
    if let Ok(tools_response) = serde_json::from_str::<serde_json::Value>(&response) {
        println!("Parsed tools response: {:#?}", tools_response);

        // Check if tools are present
        if let Some(result) = tools_response.get("result") {
            if let Some(tools) = result.get("tools") {
                if let Some(tools_array) = tools.as_array() {
                    println!("Number of tools available: {}", tools_array.len());
                    for (i, tool) in tools_array.iter().enumerate() {
                        println!("Tool {}: {:?}", i, tool.get("name"));
                    }
                }
            }
        }
    }

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
