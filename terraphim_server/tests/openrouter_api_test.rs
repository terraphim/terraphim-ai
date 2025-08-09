#![cfg(feature = "openrouter")]

use std::env;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use axum::http::StatusCode;
use axum::routing::post;
use axum::{Router};

#[tokio::test]
async fn chat_endpoint_returns_reply_from_openrouter() {
    // Mock OpenRouter
    let mock_or = MockServer::start().await;
    env::set_var("OPENROUTER_BASE_URL", mock_or.uri());
    env::set_var("OPENROUTER_KEY", "sk-or-v1-test");

    let or_body = serde_json::json!({
        "choices": [{ "message": {"content": "Hello from mock openrouter"}}]
    });
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(or_body))
        .mount(&mock_or)
        .await;

    // Build server with minimal router including /chat from crate
    let app = terraphim_server::build_router_for_tests().await;

    let payload = serde_json::json!({
        "role": "Default",
        "messages": [ {"role":"user","content":"hi"} ]
    });

    let response = reqwest::Client::new()
        .post(format!("{}/chat", app.base_url))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "Success");
    assert!(json["message"].as_str().unwrap().contains("Hello"));
}


