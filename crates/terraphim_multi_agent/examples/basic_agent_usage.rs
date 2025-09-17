//! Basic Agent Usage Example
//!
//! This is the simplest possible test to verify GenAI HTTP client-Ollama integration works

use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Basic Agent Usage Example - GenAI HTTP Client");
    println!("=================================================");

    // Create GenAI LLM client for Ollama
    let client = GenAiLlmClient::new_ollama(Some("gemma3:270m".to_string()))?;
    
    println!("✅ Created GenAI Ollama client");
    println!("   Model: {}", client.model());
    println!("   Provider: {}", client.provider());

    // Simple test prompt
    let prompt = "What is 2+2?";
    println!("🤖 Asking: '{}'", prompt);
    
    // Create request
    let messages = vec![
        LlmMessage::system("You are a helpful assistant that gives concise, accurate answers.".to_string()),
        LlmMessage::user(prompt.to_string()),
    ];
    let request = LlmRequest::new(messages);
    
    // Make LLM call
    match client.generate(request).await {
        Ok(response) => {
            println!("✅ Response: {}", response.content);
            println!("   Model: {}", response.model);
            println!("   Duration: {}ms", response.duration_ms);
            println!("   Tokens: {} in / {} out", response.usage.input_tokens, response.usage.output_tokens);
            println!("🎉 Basic agent usage successful!");
        }
        Err(e) => {
            println!("❌ Error: {:?}", e);
            println!("💡 Make sure Ollama is running: ollama serve");
            println!("💡 Make sure gemma3:270m model is installed: ollama pull gemma3:270m");
            return Err(e.into());
        }
    }

    Ok(())
}