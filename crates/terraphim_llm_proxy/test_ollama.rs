use terraphim_llm_proxy::{
    client::LlmClient,
    config::Provider,
    token_counter::{ChatRequest, Message, MessageContent},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Ollama Streaming");

    let provider = Provider {
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
    };

    let llm_client = LlmClient::new(None)?;
    let model = &request.model;

    println!("📡 Testing Ollama with model: {}", model);

    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        llm_client.send_streaming_request(&provider, model, &request),
    )
    .await
    {
        Ok(Ok(mut stream)) => {
            println!("✅ Ollama streaming request initiated");

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
                                println!("\n✅ Ollama stream ended normally");
                                break;
                            }
                            _ => {}
                        }

                        if event_count >= 20 {
                            println!("\n⏰ Stopping after 20 events");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("❌ Ollama stream error: {}", e);
                        return Err(e.into());
                    }
                }
            }

            println!("📊 Ollama Results:");
            println!("   Events received: {}", event_count);
            println!("   Total content length: {}", total_content.len());
            println!("   Has content: {}", has_content);

            if !has_content {
                println!("⚠️  WARNING: Ollama streaming returned no content!");
                return Err("Ollama streaming returned empty content".into());
            }
        }
        Ok(Err(e)) => {
            println!("❌ Ollama client error: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            println!("⏰ Ollama streaming timeout");
            return Err("Ollama streaming timeout".into());
        }
    }

    Ok(())
}