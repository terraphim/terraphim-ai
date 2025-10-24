use std::io::{self, Write};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    role: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    status: String,
    response: Option<String>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Terraphim TUI Demo - Simple Client");
    println!("====================================");

    let client = Client::new();
    let server_url = "http://localhost:8000";

    // Test server connection
    println!("🔍 Testing server connection...");
    let health_response = client.get(&format!("{}/health", server_url)).send().await?;

    if health_response.status().is_success() {
        println!("✅ Server is running and accessible");
    } else {
        println!("❌ Server not accessible");
        return Ok(());
    }

    // Get server configuration
    println!("\n📋 Getting server configuration...");
    let config_response = client.get(&format!("{}/config", server_url)).send().await?;

    if config_response.status().is_success() {
        let config_text = config_response.text().await?;
        println!("✅ Server configuration loaded successfully");

        // Parse JSON to show available roles
        if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_text) {
            if let Some(roles) = config_json.pointer("/config/roles") {
                if let Some(roles_obj) = roles.as_object() {
                    println!("📝 Available roles:");
                    for (role_name, role_config) in roles_obj {
                        println!("  - {} (relevance: {:?})", role_name,
                               role_config.get("relevance_function").and_then(|v| v.as_str()).unwrap_or("unknown"));

                        // Check for LLM configuration
                        if let Some(extra) = role_config.get("extra") {
                            if let Some(extra_obj) = extra.as_object() {
                                if extra_obj.contains_key("llm_provider") {
                                    println!("    🤖 LLM Provider: {:?}", extra_obj.get("llm_provider").and_then(|v| v.as_str()).unwrap_or("unknown"));
                                    println!("    🔑 API Key: {:?}", extra_obj.get("openrouter_api_key").map(|v| if v.as_str().unwrap_or("").len() > 8 { "configured" } else { "missing" }).unwrap_or("missing"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Interactive TUI loop
    println!("\n💬 Terraphim TUI Chat Interface");
    println!("Type 'quit' to exit, 'help' for commands");
    println!("=====================================");

    let mut current_role = "Terraphim Engineer".to_string();

    loop {
        print!("\n[{}] > ", current_role);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "quit" | "exit" => {
                println!("👋 Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  help     - Show this help");
                println!("  quit     - Exit the TUI");
                println!("  roles    - List available roles");
                println!("  health   - Check server health");
                println!("  Any other text will be sent as a chat message");
            }
            "roles" => {
                println!("Fetching available roles...");
                let roles_response = client.get(&format!("{}/config", server_url)).send().await?;
                if roles_response.status().is_success() {
                    let roles_text = roles_response.text().await?;
                    if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&roles_text) {
                        if let Some(roles) = config_json.pointer("/config/roles") {
                            if let Some(roles_obj) = roles.as_object() {
                                println!("Available roles:");
                                for (role_name, _) in roles_obj {
                                    println!("  - {}", role_name);
                                }
                            }
                        }
                    }
                }
            }
            "health" => {
                println!("Checking server health...");
                let health_response = client.get(&format!("{}/health", server_url)).send().await?;
                if health_response.status().is_success() {
                    println!("✅ Server is healthy");
                } else {
                    println!("❌ Server health check failed");
                }
            }
            "" => continue, // Skip empty input
            _ => {
                // Send as chat message
                println!("📤 Sending chat message...");

                let chat_request = ChatRequest {
                    messages: vec![ChatMessage {
                        role: "user".to_string(),
                        content: input.to_string(),
                    }],
                    role: current_role.clone(),
                };

                let chat_response = client
                    .post(&format!("{}/chat", server_url))
                    .header("Content-Type", "application/json")
                    .json(&chat_request)
                    .send()
                    .await;

                match chat_response {
                    Ok(response) => {
                        let response_text = response.text().await?;
                        println!("📥 Response Status: {}", response.status());

                        if response.status().is_success() {
                            println!("✅ Chat Response:");
                            println!("{}", response_text);
                        } else {
                            println!("⚠️ Chat returned error:");
                            println!("{}", response_text);
                        }
                    }
                    Err(e) => {
                        println!("❌ Request failed: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}