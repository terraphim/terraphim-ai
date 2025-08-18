use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::env;

#[test]
fn test_tools_list_only() {
    // Set the environment variable for local dev settings
    env::set_var("TERRAPHIM_SETTINGS_PATH", "../terraphim_settings/default/settings_local_dev.toml");
    
    println!("Starting MCP server test for tools list...");
    
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
    reader.read_line(&mut response).expect("Failed to read response");
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
    reader.read_line(&mut response).expect("Failed to read response");
    println!("Tools list response: '{}'", response.trim());
    
    // Check if response is empty
    if response.trim().is_empty() {
        println!("ERROR: Tools list response is empty!");
        // Try to read more lines to see if there's a delayed response
        for i in 0..5 {
            response.clear();
            if let Ok(_) = reader.read_line(&mut response) {
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
