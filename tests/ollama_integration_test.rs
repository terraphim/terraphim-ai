#[cfg(test)]
mod tests {
    use reqwest;
    use serde_json::Value;
    use tokio;

    #[derive(Clone, Debug)]
    pub struct TestChatMessage {
        pub role: String,
        pub content: String,
    }

    async fn test_ollama_connectivity() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let client = reqwest::Client::new();
        let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

        let resp = client.get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama connection failed: {}", resp.status()).into());
        }

        let json: Value = resp.json().await?;
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

    async fn test_ollama_summarization(model: &str, content: &str) -> Result<String, Box<dyn std::error::Error>> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let client = reqwest::Client::new();
        let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

        let prompt = format!(
            "Please provide a concise and informative summary in EXACTLY 150 characters or less. Be brief and focused.\n\nContent:\n{}",
            content
        );

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "stream": false
        });

        let resp = client.post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama summarization failed: {}", resp.status()).into());
        }

        let json: Value = resp.json().await?;
        let summary = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(summary)
    }

    async fn test_ollama_chat(model: &str, messages: Vec<TestChatMessage>) -> Result<String, Box<dyn std::error::Error>> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

        let client = reqwest::Client::new();
        let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

        let ollama_messages: Vec<Value> = messages.into_iter()
            .map(|msg| serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }))
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "num_predict": 300,
                "temperature": 0.7
            }
        });

        let resp = client.post(&url)
            .timeout(std::time::Duration::from_secs(60))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(format!("Ollama chat failed: {}", resp.status()).into());
        }

        let json: Value = resp.json().await?;
        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        Ok(content)
    }

    #[tokio::test]
    #[ignore] // Only run when explicitly requested with --ignored
    async fn test_ollama_live_integration() {
        println!("🧪 Testing Live Ollama Integration");
        println!("===================================");

        // Test 1: Connectivity and model listing
        println!("📡 Test 1: Checking Ollama connectivity...");
        let models = match test_ollama_connectivity().await {
            Ok(models) => {
                println!("✅ Connected successfully! Found {} models:", models.len());
                for model in &models {
                    println!("   - {}", model);
                }
                models
            }
            Err(e) => {
                println!("❌ Connection failed: {}", e);
                println!("⚠️  Make sure Ollama is running and accessible at the configured URL");
                return;
            }
        };

        if models.is_empty() {
            println!("❌ No models found. Please install a model with: ollama pull llama3.2:3b");
            return;
        }

        // Use the first available model for testing
        let test_model = &models[0];
        println!("🤖 Using model '{}' for tests", test_model);
        println!();

        // Test 2: Summarization
        println!("📝 Test 2: Testing text summarization...");
        let test_content = r#"
        Rust is a systems programming language that runs blazingly fast, prevents segfaults,
        and guarantees thread safety. It offers memory safety without garbage collection,
        concurrency without data races, and abstraction without overhead. Rust is used in
        many domains including web backends, operating systems, blockchain, and embedded systems.
        "#;

        match test_ollama_summarization(test_model, test_content).await {
            Ok(summary) => {
                println!("✅ Summarization successful!");
                println!("📊 Summary length: {} characters", summary.len());
                println!("📄 Summary: '{}'", summary);
                assert!(!summary.is_empty(), "Summary should not be empty");
                assert!(summary.len() <= 300, "Summary should respect length constraints");
            }
            Err(e) => {
                println!("❌ Summarization failed: {}", e);
                panic!("Summarization test failed");
            }
        }
        println!();

        // Test 3: Single-turn chat
        println!("💬 Test 3: Testing single-turn chat...");
        let chat_messages = vec![
            TestChatMessage {
                role: "user".to_string(),
                content: "What is 2 + 2? Please give a brief answer.".to_string(),
            }
        ];

        match test_ollama_chat(test_model, chat_messages).await {
            Ok(response) => {
                println!("✅ Chat successful!");
                println!("💬 AI Response: '{}'", response);
                assert!(!response.is_empty(), "Chat response should not be empty");
            }
            Err(e) => {
                println!("❌ Chat failed: {}", e);
                panic!("Chat test failed");
            }
        }
        println!();

        // Test 4: Multi-turn conversation
        println!("🗣️  Test 4: Testing multi-turn conversation...");
        let multi_turn_messages = vec![
            TestChatMessage {
                role: "user".to_string(),
                content: "What is the capital of Japan?".to_string(),
            },
            TestChatMessage {
                role: "assistant".to_string(),
                content: "The capital of Japan is Tokyo.".to_string(),
            },
            TestChatMessage {
                role: "user".to_string(),
                content: "What is its approximate population?".to_string(),
            },
        ];

        match test_ollama_chat(test_model, multi_turn_messages).await {
            Ok(response) => {
                println!("✅ Multi-turn conversation successful!");
                println!("💬 AI Response: '{}'", response);
                assert!(!response.is_empty(), "Multi-turn response should not be empty");
            }
            Err(e) => {
                println!("❌ Multi-turn conversation failed: {}", e);
                panic!("Multi-turn conversation test failed");
            }
        }
        println!();

        println!("🎉 All Ollama integration tests passed!");
        println!("✨ Both summarization and chat functionality are working correctly!");
    }
}
