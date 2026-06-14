mod common;

use axum_test::TestServer;
use serde_json::json;

/// Test the /chat endpoint with real Ollama backend.
///
/// Requires Ollama running AND the active server role configured with
/// `"llm_provider": "ollama"`. Run manually with:
///   cargo test -p terraphim_server --test ollama_api_test test_chat_endpoint_with_ollama -- --ignored
#[tokio::test]
#[ignore = "requires Ollama running with a role configured for Ollama (llm_provider=ollama)"]
async fn test_chat_endpoint_with_ollama() {
    if !common::llm_reachability::require_ollama().await {
        return;
    }

    // Create test server
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app);

    // Test chat request
    let payload = json!({
        "role": "Engineer",
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
#[tokio::test]
async fn test_chat_endpoint_empty_messages() {
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app);

    let payload = json!({
        "role": "Engineer",
        "messages": []
    });

    let response = server.post("/chat").json(&payload).await;

    // Endpoint returns an error payload, but currently responds with 200.
    response.assert_status_ok();
    let json: serde_json::Value = response.json();
    assert_eq!(json["status"], "error");
    assert!(json["error"].is_string());
}
