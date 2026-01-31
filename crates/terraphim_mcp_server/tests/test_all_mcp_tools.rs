use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn test_all_mcp_tools() {
    // Set the environment variable for local dev settings
    unsafe {
        env::set_var(
            "TERRAPHIM_SETTINGS_PATH",
            "../terraphim_settings/default/settings_local_dev.toml",
        );
    }

    println!("Starting comprehensive MCP server test for all tools...");

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

    // Wait for server to start
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
    println!("Init Response: {}", response.trim());

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

    // Read the tools list response
    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Tools list response: '{}'", response.trim());

    // Check if response is empty
    if response.trim().is_empty() {
        println!("ERROR: Tools list response is empty!");
        // Try to read more lines to see if there's a delayed response
        for i in 0..5 {
            response.clear();
            if reader.read_line(&mut response).is_ok() {
                println!("Additional response line {}: '{}'", i, response.trim());
            }
        }
    } else {
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

                        // If we have tools, test a few of them
                        if !tools_array.is_empty() {
                            test_specific_tools(&mut stdin, &mut reader);
                        }
                    }
                }
            }
        } else {
            println!("Failed to parse tools response as JSON");
        }
    }

    println!("Test completed!");

    // Clean up
    child.kill().expect("Failed to kill child process");
    child.wait().expect("Failed to wait for child");
}

fn test_specific_tools(
    stdin: &mut std::process::ChildStdin,
    reader: &mut BufReader<std::process::ChildStdout>,
) {
    println!("Testing specific tools...");

    // Test 3: Build autocomplete index
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

    println!("3. Testing build_autocomplete_index...");
    writeln!(stdin, "{}", build_index_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Build index response: '{}'", response.trim());

    // Test 4: Autocomplete terms
    let autocomplete_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "autocomplete_terms",
            "arguments": {
                "query": "terraphim",
                "limit": 5
            }
        }
    });

    println!("4. Testing autocomplete_terms...");
    writeln!(stdin, "{}", autocomplete_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response
    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Autocomplete response: '{}'", response.trim());

    // Test 5: Search
    let search_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "search",
            "arguments": {
                "query": "terraphim",
                "limit": 3
            }
        }
    });

    println!("5. Testing search...");
    writeln!(stdin, "{}", search_request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response
    response.clear();
    reader
        .read_line(&mut response)
        .expect("Failed to read response");
    println!("Search response: '{}'", response.trim());
}
