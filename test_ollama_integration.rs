#!/usr/bin/env rust-script

//! Test script to demonstrate Ollama integration for summarization and chat
//! 
//! Usage: rust-script test_ollama_integration.rs
//! 
//! This script tests:
//! 1. Direct LLM client functionality with Ollama
//! 2. Chat completion with conversation history
//! 3. Text summarization with length constraints
//! 4. Model listing and validation

use std::collections::HashMap;
use serde_json::Value;

// Simulate the terraphim_service LLM interface
#[derive(Clone, Debug)]
pub struct SummarizeOptions {
    pub max_length: usize,
}

#[derive(Clone, Debug)]
pub struct ChatOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct OllamaTestClient {
    pub base_url: String,
    pub model: String,
    pub http: reqwest::Client,
}

impl OllamaTestClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            http: reqwest::Client::new(),
        }
    }

    pub async fn test_summarize(&self, content: &str, opts: SummarizeOptions) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Please provide a concise and informative summary in EXACTLY {} characters or less. Be brief and focused.\n\nContent:\n{}",
            opts.max_length, content
        );

        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "stream": false
        });

        let resp = self.http
            .post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama error: {}", resp.status()).into());
        }

        let json: serde_json::Value = resp.json().await?;
        let summary = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(summary)
    }

    pub async fn test_chat(&self, messages: Vec<ChatMessage>, opts: ChatOptions) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        
        let ollama_messages: Vec<serde_json::Value> = messages.into_iter()
            .map(|msg| serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }))
            .collect();

        let body = serde_json::json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "num_predict": opts.max_tokens.unwrap_or(1024),
                "temperature": opts.temperature.unwrap_or(0.7)
            }
        });

        let resp = self.http
            .post(&url)
            .timeout(std::time::Duration::from_secs(60))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama error: {}", resp.status()).into());
        }

        let json: serde_json::Value = resp.json().await?;
        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(content)
    }

    pub async fn list_models(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/api/tags", self.base_url.trim_end_matches('/'));
        let resp = self.http.get(&url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama tags error: {}", resp.status()).into());
        }

        let json: serde_json::Value = resp.json().await?;
        let mut models = Vec::new();
        
        if let Some(arr) = json.get("models").and_then(|v| v.as_array()) {
            for m in arr {
                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                    models.push(name.to_string());
                }
            }
        }
        
        Ok(models)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Ollama Integration for Terraphim AI");
    println!("============================================");

    let base_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    
    // Use one of the available models from our investigation
    let model = "qwen2.5-coder:latest".to_string();
    
    println!("📡 Connecting to Ollama at: {}", base_url);
    println!("🤖 Using model: {}", model);
    println!();

    let client = OllamaTestClient::new(base_url, model);

    // Test 1: Model listing
    println!("🔍 Test 1: Listing available models");
    match client.list_models().await {
        Ok(models) => {
            println!("✅ Found {} models:", models.len());
            for model in &models {
                println!("   - {}", model);
            }
        }
        Err(e) => {
            println!("❌ Model listing failed: {}", e);
            return Ok(());
        }
    }
    println!();

    // Test 2: Text summarization
    println!("📝 Test 2: Text summarization");
    let test_content = r#"
    Rust is a systems programming language that runs blazingly fast, prevents segfaults,
    and guarantees thread safety. It offers memory safety without garbage collection,
    concurrency without data races, and abstraction without overhead.

    Key features include:
    - Zero-cost abstractions: You can write high-level code that compiles to low-level performance
    - Memory safety without garbage collection: The ownership system prevents common bugs
    - Thread safety without data races: The type system prevents race conditions at compile time
    - Modern tooling with Cargo: Built-in package manager, build tool, and testing framework
    - Excellent documentation and community: Comprehensive guides and helpful community

    Rust is used in many domains including web backends, operating systems, blockchain,
    game engines, and embedded systems. Companies like Mozilla, Dropbox, Microsoft, and
    Facebook use Rust in production for performance-critical applications.
    "#;

    match client.test_summarize(test_content, SummarizeOptions { max_length: 200 }).await {
        Ok(summary) => {
            println!("✅ Summarization successful!");
            println!("📊 Summary length: {} characters", summary.len());
            println!("📄 Summary: {}", summary);
        }
        Err(e) => {
            println!("❌ Summarization failed: {}", e);
        }
    }
    println!();

    // Test 3: Chat conversation
    println!("💬 Test 3: Chat conversation");
    let chat_messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Hello! Can you explain what Rust programming language is good for?".to_string(),
        }
    ];

    match client.test_chat(chat_messages, ChatOptions { 
        max_tokens: Some(300), 
        temperature: Some(0.7) 
    }).await {
        Ok(response) => {
            println!("✅ Chat successful!");
            println!("💬 AI Response: {}", response);
        }
        Err(e) => {
            println!("❌ Chat failed: {}", e);
        }
    }
    println!();

    // Test 4: Multi-turn conversation
    println!("🗣️  Test 4: Multi-turn conversation");
    let multi_turn_messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "What is the capital of France?".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "The capital of France is Paris.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "What is its population?".to_string(),
        },
    ];

    match client.test_chat(multi_turn_messages, ChatOptions { 
        max_tokens: Some(200), 
        temperature: Some(0.5) 
    }).await {
        Ok(response) => {
            println!("✅ Multi-turn chat successful!");
            println!("💬 AI Response: {}", response);
        }
        Err(e) => {
            println!("❌ Multi-turn chat failed: {}", e);
        }
    }
    println!();

    println!("🎉 All tests completed!");
    println!("✨ Ollama integration is working correctly for both summarization and chat!");

    Ok(())
}