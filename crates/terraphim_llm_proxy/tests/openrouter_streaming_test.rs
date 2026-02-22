//! Integration test for OpenRouter streaming compatibility
//!
//! This test reproduces the Issue #1: OpenRouter SSE streaming format compatibility

use genai::chat::ChatStreamEvent;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use terraphim_llm_proxy::{
    client::LlmClient,
    config::Provider,
    token_counter::{ChatRequest, Message, MessageContent, SystemPrompt},
};
use tokio::time::timeout;

#[tokio::test]
#[ignore] // Requires real API key - run with cargo test -- --ignored
async fn test_openrouter_client_streaming() {
    // Test the LlmClient streaming directly - this reproduces Issue #1
    let provider = Provider {
        name: "openrouter".to_string(),
        api_base_url: "https://openrouter.ai/api/v1".to_string(),
        api_key: std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "test-key".to_string()),
        models: vec!["anthropic/claude-3.5-sonnet".to_string()],
        transformers: vec![],
    };

    let request = ChatRequest {
        model: "anthropic/claude-3.5-sonnet".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Say 'Hello client!'".to_string()),
            ..Default::default()
        }],
        system: Some(SystemPrompt::Text(
            "You are a helpful assistant.".to_string(),
        )),
        max_tokens: Some(50),
        temperature: Some(0.7),
        stream: Some(true),
        tools: None,
        thinking: None,
        ..Default::default()
    };

    let client = LlmClient::new(None).unwrap();

    match timeout(
        Duration::from_secs(30),
        client.send_streaming_request(&provider, "anthropic/claude-3.5-sonnet", &request),
    )
    .await
    {
        Ok(Ok(stream)) => {
            println!("✅ Client streaming request initiated");

            let mut event_count = 0;
            use futures::StreamExt;

            tokio::pin!(stream);
            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => {
                        event_count += 1;
                        println!("Stream event {}: {:?}", event_count, event);

                        // Check if we got actual content
                        match &event {
                            ChatStreamEvent::Chunk(chunk) => {
                                if !chunk.content.is_empty() {
                                    println!("✅ Got content: '{}'", chunk.content);
                                } else {
                                    println!("⚠️ Empty content chunk");
                                }
                            }
                            ChatStreamEvent::End(_) => {
                                println!("✅ Stream ended normally");
                            }
                            _ => {
                                println!("ℹ️ Other event type: {:?}", event);
                            }
                        }

                        if event_count >= 10 {
                            // Limit for test
                            break;
                        }
                    }
                    Err(e) => {
                        println!("❌ Stream error: {}", e);
                        if e.to_string().contains("text/html") {
                            println!("❌ Confirmed: HTML error in client streaming (Issue #1 reproduced)");
                        }
                        break;
                    }
                }
            }

            if event_count > 0 {
                println!("✅ Received {} streaming events", event_count);
            } else {
                println!("⚠️ No streaming events received");
            }
        }
        Ok(Err(e)) => {
            println!("❌ Client streaming failed: {}", e);
            if e.to_string().contains("text/html") {
                println!("❌ Confirmed: HTML error in client streaming (Issue #1 reproduced)");
            }
        }
        Err(_) => {
            println!("⏰ Client streaming timed out");
        }
    }
}

#[tokio::test]
#[ignore] // Requires real API key - run with cargo test -- --ignored
async fn test_openrouter_direct_api() {
    // Test direct OpenRouter API call to verify it works outside our proxy
    let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| "test-key".to_string());

    let client = Client::new();
    let request = json!({
        "model": "anthropic/claude-3.5-sonnet",
        "messages": [
            {
                "role": "user",
                "content": "Say 'Hello OpenRouter!'"
            }
        ],
        "stream": true
    });

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://terraphim.ai")
        .header("X-Title", "Terraphim LLM Proxy Test")
        .json(&request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Direct OpenRouter response status: {}", resp.status());
            println!("Direct OpenRouter response headers: {:?}", resp.headers());

            if resp.status().is_success() {
                println!("✅ Direct OpenRouter API call successful");

                // Try to read some streaming data
                let bytes = resp.bytes().await;
                match bytes {
                    Ok(data) => {
                        let text = String::from_utf8_lossy(&data);
                        println!("Response preview: {}", &text[..text.len().min(500)]);

                        // Print first few lines to understand the format
                        for (i, line) in text.lines().take(10).enumerate() {
                            println!("Line {}: {:?}", i + 1, line);
                        }

                        if text.starts_with("data: ") {
                            println!("✅ Valid SSE format detected in direct API");
                        } else {
                            println!("⚠️ Unexpected format from direct API");
                        }
                    }
                    Err(e) => println!("Error reading response: {}", e),
                }
            } else {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                println!("❌ Direct OpenRouter API failed: {} - {}", status, text);
            }
        }
        Err(e) => println!("❌ Direct OpenRouter request failed: {}", e),
    }
}
