//! Test LLM Client directly to isolate the segfault

use std::sync::Arc;
use terraphim_multi_agent::{
    AgentId, CostTracker, LlmClientConfig, RigLlmClient, TokenUsageTracker,
};
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing LLM Client Integration");
    println!("==================================");

    // Create Ollama configuration
    let config = LlmClientConfig {
        provider: "ollama".to_string(),
        model: "gemma3:270m".to_string(),
        api_key: None,
        base_url: Some("http://127.0.0.1:11434".to_string()),
        temperature: 0.7,
        max_tokens: 1000,
        timeout_seconds: 30,
        track_costs: true,
    };

    println!("✅ Created LLM config: {:?}", config.provider);

    // Create tracking components
    let agent_id = AgentId::new();
    let token_tracker = Arc::new(RwLock::new(TokenUsageTracker::new()));
    let cost_tracker = Arc::new(RwLock::new(CostTracker::new()));

    println!("✅ Created tracking components");

    // Create RigLlmClient
    match RigLlmClient::new(config, agent_id, token_tracker, cost_tracker).await {
        Ok(client) => {
            println!("✅ Created RigLlmClient successfully");

            // Test a simple prompt
            println!("🤖 Testing prompt...");

            let request = terraphim_multi_agent::LlmRequest {
                messages: vec![terraphim_multi_agent::LlmMessage {
                    role: "user".to_string(),
                    content: "What is 2+2?".to_string(),
                }],
                temperature: Some(0.7),
                max_tokens: Some(100),
            };

            match client.generate(request).await {
                Ok(response) => {
                    println!("✅ Response: {}", response.content);
                    println!("🎉 LLM Client test successful!");
                }
                Err(e) => {
                    println!("❌ LLM generate error: {:?}", e);
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create RigLlmClient: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
