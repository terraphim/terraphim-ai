use terraphim_config::{Config, RoleName};
use terraphim_service::llm;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🔍 Testing LLM provider detection...");

    // Load the config
    // Run from the repository root.
    let config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;

    let role_name = RoleName::new("Terraphim Engineer");
    let role = config.roles.get(&role_name).expect("Role should exist");

    println!("📋 Role extra fields: {:?}", role.extra);
    println!("📋 Role extra keys: {:?}", role.extra.keys().collect::<Vec<_>>());

    // Test LLM provider detection
    let llm_provider = role.extra.get("llm_provider").and_then(|v| v.as_str()).unwrap_or("");
    println!("🤖 LLM provider from extra: '{}'", llm_provider);

    // Test LLM client building
    let llm_client = llm::build_llm_from_role(role);
    match llm_client {
        Some(_) => println!("✅ LLM client built successfully!"),
        None => println!("❌ Failed to build LLM client"),
    }

    Ok(())
}
