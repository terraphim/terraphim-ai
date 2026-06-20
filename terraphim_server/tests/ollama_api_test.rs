use axum_test::TestServer;
use serde_json::json;

/// Return true if Ollama is running AND the specified model is loaded.
///
/// Contacts /api/tags with a 2-second timeout. Returns false (with a skip
/// message) if Ollama is unreachable or the model is absent from the list.
async fn require_ollama_with_model(ollama_url: &str, model: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping test: failed to build HTTP client: {e}");
            return false;
        }
    };

    let tags_url = format!("{}/api/tags", ollama_url);
    let resp = match client.get(&tags_url).send().await {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Skipping test: Ollama not running on {}", ollama_url);
            return false;
        }
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Skipping test: failed to parse /api/tags response: {e}");
            return false;
        }
    };

    let has_model = body
        .get("models")
        .and_then(|m| m.as_array())
        .map(|models| {
            models.iter().any(|m| {
                m.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.starts_with(model))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !has_model {
        eprintln!(
            "Skipping test: model '{}' not loaded in Ollama ({})",
            model, ollama_url
        );
        return false;
    }

    true
}

/// Test the /chat endpoint with real Ollama backend
#[tokio::test]
#[ignore] // Only run when Ollama is available with llama3.2:3b
async fn test_chat_endpoint_with_ollama() {
    let ollama_url = "http://127.0.0.1:11434";
    if !require_ollama_with_model(ollama_url, "llama3.2:3b").await {
        return;
    }

    // Create test server
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app);

    // Test chat request
    let payload = json!({
        "role": "Terraphim Engineer",
        "messages": [
            {"role": "user", "content": "What is Rust?"}
        ]
    });

    let response = server.post("/chat").json(&payload).await;

    // Verify response
    response.assert_status_ok();

    let json: serde_json::Value = response.json();
    assert_eq!(json["status"], "Success");
    assert!(json["message"].is_string());

    let message = json["message"].as_str().unwrap();
    assert!(!message.is_empty(), "Response should not be empty");
}

/// Test /chat endpoint with invalid role
#[tokio::test]
async fn test_chat_endpoint_invalid_role() {
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app);

    let payload = json!({
        "role": "NonExistentRole",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let response = server.post("/chat").json(&payload).await;

    // Endpoint returns an error payload, but currently responds with 200.
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["status"], "error");
    assert!(json["error"].is_string());
}

/// Test /chat endpoint with empty messages
///
/// Uses a valid role ("Terraphim Engineer") so the test exercises the
/// empty-messages validation path rather than the unknown-role error path.
#[tokio::test]
async fn test_chat_endpoint_empty_messages() {
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app);

    let payload = json!({
        "role": "Terraphim Engineer",
        "messages": []
    });

    let response = server.post("/chat").json(&payload).await;

    // Endpoint returns an error payload, but currently responds with 200.
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["status"], "error");
    assert!(json["error"].is_string());
}
