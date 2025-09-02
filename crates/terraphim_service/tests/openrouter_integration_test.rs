#![cfg(feature = "openrouter")]

use serial_test::serial;
use std::env;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use terraphim_service::openrouter::OpenRouterService;

#[tokio::test]
#[serial]
async fn test_generate_summary_success() {
    // Arrange: mock OpenRouter API
    let server = MockServer::start().await;
    env::set_var("OPENROUTER_BASE_URL", server.uri());

    // Response payload mimicking OpenRouter chat/completions
    let body = serde_json::json!({
        "choices": [{
            "message": {"content": "This is a concise summary."}
        }]
    });

    Mock::given(method("POST"))
        .and(path_regex(r"/chat/completions$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = OpenRouterService::new("sk-or-v1-test", "openai/gpt-3.5-turbo").unwrap();

    // Act
    let summary = client
        .generate_summary("This is a long article content.", 120)
        .await
        .expect("summary should succeed");

    // Assert
    assert!(summary.contains("concise summary"));
}

#[tokio::test]
#[serial]
async fn test_chat_completion_success() {
    let server = MockServer::start().await;
    env::set_var("OPENROUTER_BASE_URL", server.uri());

    let body = serde_json::json!({
        "choices": [{
            "message": {"content": "Hello! How can I assist you today?"}
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = OpenRouterService::new("sk-or-v1-test", "openai/gpt-3.5-turbo").unwrap();

    let reply = client
        .chat_completion(
            vec![serde_json::json!({"role":"user","content":"hi"})],
            Some(128),
            Some(0.2),
        )
        .await
        .expect("chat should succeed");

    assert!(!reply.trim().is_empty(), "chat reply should be non-empty");
}

#[tokio::test]
#[serial]
async fn test_list_models_handles_both_formats() {
    let server = MockServer::start().await;
    env::set_var("OPENROUTER_BASE_URL", server.uri());

    // First response style: { data: [{id: "model-a"},{id:"model-b"}] }
    let body1 = serde_json::json!({
        "data": [{"id":"openai/gpt-3.5-turbo"},{"id":"anthropic/claude-3-haiku"}]
    });
    Mock::given(method("GET"))
        .and(path_regex(r"/models$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body1))
        .mount(&server)
        .await;

    let client = OpenRouterService::new("sk-or-v1-test", "openai/gpt-3.5-turbo").unwrap();
    let models = client
        .list_models()
        .await
        .expect("list models should succeed");
    assert!(!models.is_empty(), "models list should not be empty");

    // Second response style: { models: ["model-a","model-b"] }
    server.reset().await;
    let body2 = serde_json::json!({
        "models": ["openai/gpt-4","mistralai/mixtral-8x7b-instruct"]
    });
    Mock::given(method("GET"))
        .and(path_regex(r"/models$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body2))
        .mount(&server)
        .await;
    let models2 = client
        .list_models()
        .await
        .expect("list models format 2 should succeed");
    assert!(models2.iter().any(|m| m.contains("gpt-4")));
}

#[tokio::test]
#[serial]
async fn test_rate_limit_and_error_paths() {
    let server = MockServer::start().await;
    env::set_var("OPENROUTER_BASE_URL", server.uri());

    let client = OpenRouterService::new("sk-or-v1-test", "openai/gpt-3.5-turbo").unwrap();

    // 500 error triggers ApiError path
    server.reset().await;
    Mock::given(method("POST"))
        .and(path_regex(r"/chat/completions$"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    let res2 = client.chat_completion(vec![], None, None).await;
    assert!(res2.is_err(), "server error must error");
}
