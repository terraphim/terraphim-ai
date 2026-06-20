use axum_test::TestServer;
use serde_json::json;

/// Test the /chat endpoint with real Ollama backend
#[tokio::test]
#[ignore] // Only run when Ollama is available with llama3.2:3b loaded
async fn test_chat_endpoint_with_ollama() {
    let ollama_url = "http://127.0.0.1:11434";
    let required_model = "llama3.2:3b";
    let client = reqwest::Client::new();

    let tags_resp = match client.get(format!("{}/api/tags", ollama_url)).send().await {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Skipping test: Ollama not running on {}", ollama_url);
            return;
        }
    };

    let tags: serde_json::Value = match tags_resp.json().await {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Skipping test: failed to parse Ollama /api/tags response");
            return;
        }
    };

    let model_loaded = tags["models"]
        .as_array()
        .map(|models| {
            models.iter().any(|m| {
                m["name"]
                    .as_str()
                    .map(|n| n.starts_with(required_model))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !model_loaded {
        eprintln!(
            "Skipping test: model '{}' not loaded in Ollama (run: ollama pull {})",
            required_model, required_model
        );
        return;
    }

    let app = terraphim_server::build_router_for_tests().await;
    let server = TestServer::new(app);

    let payload = json!({
        "role": "Terraphim Engineer",
        "messages": [
            {"role": "user", "content": "What is Rust?"}
        ]
    });

    let response = server.post("/chat").json(&payload).await;

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
