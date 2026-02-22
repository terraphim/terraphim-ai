//! Comprehensive streaming test for both OpenRouter and Ollama

use terraphim_llm_proxy::{
    client::LlmClient,
    config::Provider,
    token_counter::{ChatRequest, Message, MessageContent},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Comprehensive Streaming Test");
    println!("================================\n");

    // Test 1: OpenRouter Streaming
    println!("1️⃣ Testing OpenRouter Streaming");
    println!("-------------------------------");

    let openrouter_provider = Provider {
        name: "openrouter".to_string(),
        api_base_url: "https://openrouter.ai/api/v1".to_string(),
        api_key: std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY not set"),
        models: vec!["anthropic/claude-3.5-sonnet".to_string()],
        transformers: vec![],
    };

    let request = ChatRequest {
        model: "anthropic/claude-3.5-sonnet".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Say exactly: 'OpenRouter streaming works!'".to_string()),
        }],
        system: None,
        max_tokens: Some(50),
        temperature: Some(0.1),
        stream: Some(true),
        tools: None,
        thinking: None,
            ..Default::default()
    };

    test_provider_streaming(&openrouter_provider, &request, "OpenRouter").await?;

    // Test 2: Ollama Streaming
    println!("\n2️⃣ Testing Ollama Streaming");
    println!("---------------------------");

    let ollama_provider = Provider {
        name: "ollama".to_string(),
        api_base_url: "http://127.0.0.1:11434".to_string(),
        api_key: "ollama".to_string(),
        models: vec!["qwen2.5-coder:latest".to_string()],
        transformers: vec!["ollama".to_string()],
    };

    let request = ChatRequest {
        model: "qwen2.5-coder:latest".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Say exactly: 'Ollama streaming works!'".to_string()),
        }],
        system: None,
        max_tokens: Some(50),
        temperature: Some(0.1),
        stream: Some(true),
        tools: None,
        thinking: None,
            ..Default::default()
    };

    test_provider_streaming(&ollama_provider, &request, "Ollama").await?;

    println!("\n✅ All streaming tests completed!");
    Ok(())
}

async fn test_provider_streaming(
    provider: &Provider,
    request: &ChatRequest,
    provider_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client = LlmClient::new(None)?;
    let model = &request.model;

    println!("📡 Testing {} with model: {}", provider_name, model);

    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        llm_client.send_streaming_request(provider, model, request),
    )
    .await
    {
        Ok(Ok(mut stream)) => {
            println!("✅ {} streaming request initiated", provider_name);

            use futures::StreamExt;
            let mut event_count = 0;
            let mut total_content = String::new();
            let mut has_content = false;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => {
                        event_count += 1;
                        match event {
                            genai::chat::ChatStreamEvent::Chunk(chunk) => {
                                if !chunk.content.is_empty() {
                                    print!("{}", chunk.content);
                                    total_content.push_str(&chunk.content);
                                    has_content = true;
                                }
                            }
                            genai::chat::ChatStreamEvent::End(_) => {
                                println!("\n✅ {} stream ended normally", provider_name);
                                break;
                            }
                            _ => {
                                println!("\nℹ️ {} other event: {:?}", provider_name, event);
                            }
                        }

                        if event_count >= 20 {
                            println!("\n⏰ Stopping {} after 20 events", provider_name);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("❌ {} stream error: {}", provider_name, e);
                        return Err(e.into());
                    }
                }
            }

            println!("📊 {} Results:", provider_name);
            println!("   Events received: {}", event_count);
            println!("   Total content length: {}", total_content.len());
            println!("   Has content: {}", has_content);
            println!("   Content preview: '{}'",
                if total_content.len() > 100 {
                    format!("{}...", &total_content[..100])
                } else {
                    total_content
                }
            );

            if !has_content {
                println!("⚠️  WARNING: {} streaming returned no content!", provider_name);
                return Err(format!("{} streaming returned empty content", provider_name).into());
            }
        }
        Ok(Err(e)) => {
            println!("❌ {} client error: {}", provider_name, e);
            return Err(e.into());
        }
        Err(_) => {
            println!("⏰ {} streaming timeout", provider_name);
            return Err(format!("{} streaming timeout", provider_name).into());
        }
    }

    Ok(())
}