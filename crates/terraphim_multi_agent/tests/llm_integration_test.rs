//! Integration tests for LLM functionality with both llama3.2:3b and gemma3:270m models

use std::sync::Arc;
use terraphim_config::Role;
use terraphim_multi_agent::{
    CommandInput, CommandType, TerraphimAgent, GenAiLlmClient, AgentConfig
};
use terraphim_persistence::DeviceStorage;
use ahash::AHashMap;

/// Test agent creation and response with llama3.2:3b model
#[tokio::test]
async fn test_llama_model_response() {
    // Skip if Ollama is not running
    if !check_ollama_available().await {
        println!("⏭️ Skipping test - Ollama not available");
        return;
    }

    println!("🧪 Testing llama3.2:3b model");
    
    // Create role with llama3.2:3b configuration
    let mut extra = AHashMap::new();
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("llm_model".to_string(), serde_json::json!("llama3.2:3b"));
    extra.insert("llm_base_url".to_string(), serde_json::json!("http://127.0.0.1:11434"));

    let role = Role {
        name: "Test Llama Engineer".to_string(),
        shortname: Some("TestLlama".to_string()),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: None,
        haystacks: Vec::new(),
        extra,
    };

    // Create storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .expect("Failed to create test storage");
    
    let mut agent = TerraphimAgent::new(role, persistence, None)
        .await
        .expect("Failed to create agent");
    
    agent.initialize().await.expect("Failed to initialize agent");

    // Test with a specific programming task
    let input = CommandInput::new(
        "Write a simple 'hello world' function in Rust".to_string(),
        CommandType::Generate
    );

    let output = agent.process_command(input).await.expect("Command processing failed");
    
    println!("🦙 Llama response: {}", output.text);
    
    // Verify response contains actual content, not just acknowledgment
    assert!(output.text.len() > 50, "Response too short: {}", output.text);
    assert!(!output.text.contains("I'm ready to be your"), "Got generic acknowledgment instead of content");
    assert!(output.text.to_lowercase().contains("rust") || 
           output.text.contains("fn") || 
           output.text.contains("println!"), "Response doesn't contain Rust content");
}

/// Test agent creation and response with gemma3:270m model
#[tokio::test]
async fn test_gemma_model_response() {
    // Skip if Ollama is not running
    if !check_ollama_available().await {
        println!("⏭️ Skipping test - Ollama not available");
        return;
    }

    println!("🧪 Testing gemma3:270m model");
    
    // Create role with gemma3:270m configuration
    let mut extra = AHashMap::new();
    extra.insert("llm_provider".to_string(), serde_json::json!("ollama"));
    extra.insert("llm_model".to_string(), serde_json::json!("gemma3:270m"));
    extra.insert("llm_base_url".to_string(), serde_json::json!("http://127.0.0.1:11434"));

    let role = Role {
        name: "Test Gemma Engineer".to_string(),
        shortname: Some("TestGemma".to_string()),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".to_string(),
        kg: None,
        haystacks: Vec::new(),
        extra,
    };

    // Create storage and agent
    let persistence = DeviceStorage::arc_memory_only()
        .await
        .expect("Failed to create test storage");
    
    let mut agent = TerraphimAgent::new(role, persistence, None)
        .await
        .expect("Failed to create agent");
    
    agent.initialize().await.expect("Failed to initialize agent");

    // Test with a specific programming task
    let input = CommandInput::new(
        "Write a simple 'hello world' function in Rust".to_string(),
        CommandType::Generate
    );

    let output = agent.process_command(input).await.expect("Command processing failed");
    
    println!("💎 Gemma response: {}", output.text);
    
    // Verify response contains actual content, not just acknowledgment
    assert!(output.text.len() > 50, "Response too short: {}", output.text);
    assert!(!output.text.contains("I'm ready to be your"), "Got generic acknowledgment instead of content");
    assert!(output.text.to_lowercase().contains("rust") || 
           output.text.contains("fn") || 
           output.text.contains("println!"), "Response doesn't contain Rust content");
}

/// Test direct LLM client functionality with both models
#[tokio::test]
async fn test_direct_llm_clients() {
    // Skip if Ollama is not running
    if !check_ollama_available().await {
        println!("⏭️ Skipping test - Ollama not available");
        return;
    }

    // Test llama3.2:3b client
    println!("🧪 Testing direct llama3.2:3b client");
    let llama_client = GenAiLlmClient::new_ollama(Some("llama3.2:3b".to_string()))
        .expect("Failed to create llama client");
    
    let request = terraphim_multi_agent::LlmRequest::new(vec![
        terraphim_multi_agent::LlmMessage::system("You are a helpful programming assistant.".to_string()),
        terraphim_multi_agent::LlmMessage::user("Write a hello world function in Rust".to_string()),
    ]);
    
    let llama_response = llama_client.generate(request).await.expect("Llama generation failed");
    println!("🦙 Direct llama response: {}", llama_response.content);
    
    assert!(llama_response.content.len() > 10, "Llama response too short");
    
    // Test gemma3:270m client
    println!("🧪 Testing direct gemma3:270m client");
    let gemma_client = GenAiLlmClient::new_ollama(Some("gemma3:270m".to_string()))
        .expect("Failed to create gemma client");
    
    let request = terraphim_multi_agent::LlmRequest::new(vec![
        terraphim_multi_agent::LlmMessage::system("You are a helpful programming assistant.".to_string()),
        terraphim_multi_agent::LlmMessage::user("Write a hello world function in Rust".to_string()),
    ]);
    
    let gemma_response = gemma_client.generate(request).await.expect("Gemma generation failed");
    println!("💎 Direct gemma response: {}", gemma_response.content);
    
    assert!(gemma_response.content.len() > 10, "Gemma response too short");
}

/// Check if Ollama is available by making a simple request
async fn check_ollama_available() -> bool {
    let client = reqwest::Client::new();
    match client.get("http://127.0.0.1:11434/api/tags").send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Test comparison between models to ensure different responses
#[tokio::test]
async fn test_model_comparison() {
    // Skip if Ollama is not running
    if !check_ollama_available().await {
        println!("⏭️ Skipping test - Ollama not available");
        return;
    }

    println!("🧪 Comparing model responses");
    
    let prompt = "Explain async programming in Rust in one sentence";
    
    // Test with llama3.2:3b
    let llama_client = GenAiLlmClient::new_ollama(Some("llama3.2:3b".to_string()))
        .expect("Failed to create llama client");
    
    let request = terraphim_multi_agent::LlmRequest::new(vec![
        terraphim_multi_agent::LlmMessage::user(prompt.to_string()),
    ]);
    
    let llama_response = llama_client.generate(request).await.expect("Llama generation failed");
    
    // Test with gemma3:270m
    let gemma_client = GenAiLlmClient::new_ollama(Some("gemma3:270m".to_string()))
        .expect("Failed to create gemma client");
    
    let request = terraphim_multi_agent::LlmRequest::new(vec![
        terraphim_multi_agent::LlmMessage::user(prompt.to_string()),
    ]);
    
    let gemma_response = gemma_client.generate(request).await.expect("Gemma generation failed");
    
    println!("🦙 Llama: {}", llama_response.content);
    println!("💎 Gemma: {}", gemma_response.content);
    
    // Both should produce responses
    assert!(llama_response.content.len() > 10, "Llama response too short");
    assert!(gemma_response.content.len() > 10, "Gemma response too short");
    
    // Responses should contain relevant content
    assert!(llama_response.content.to_lowercase().contains("async") || 
           llama_response.content.to_lowercase().contains("await") ||
           llama_response.content.to_lowercase().contains("concurrent"),
           "Llama response doesn't contain async concepts");
    
    assert!(gemma_response.content.to_lowercase().contains("async") || 
           gemma_response.content.to_lowercase().contains("await") ||
           gemma_response.content.to_lowercase().contains("concurrent"),
           "Gemma response doesn't contain async concepts");
}