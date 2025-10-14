use axum_test::TestServer;
use serde_json::json;

/// Test the /chat endpoint with real Ollama backend
#[tokio::test]
#[ignore] // Only run when Ollama is available
async fn test_chat_endpoint_with_ollama() {
    // Check if Ollama is running
    let ollama_url = "http://127.0.0.1:11434";
    let client = reqwest::Client::new();
    if client
        .get(format!("{}/api/tags", ollama_url))
        .send()
        .await
        .is_err()
    {
        eprintln!("Skipping test: Ollama not running on {}", ollama_url);
        return;
    }

    // Create test server
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app).expect("Failed to create test server");

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

    let server = TestServer::new(app).expect("Failed to create test server");

    let payload = json!({
        "role": "NonExistentRole",
        "messages": [
            {"role": "user", "content": "Hello"}
        ]
    });

    let response = server.post("/chat").json(&payload).await;

    // Should return error for invalid role
    response.assert_status_bad_request();
}

/// Test /chat endpoint with empty messages
#[tokio::test]
async fn test_chat_endpoint_empty_messages() {
    let _config_path = "terraphim_server/default/terraphim_engineer_config.json";
    let app = terraphim_server::build_router_for_tests().await;

    let server = TestServer::new(app).expect("Failed to create test server");

    let payload = json!({
        "role": "Engineer",
        "messages": []
    });

    let response = server.post("/chat").json(&payload).await;

    // Should handle empty messages gracefully
    response.assert_status_bad_request();
}
