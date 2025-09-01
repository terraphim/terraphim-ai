#![cfg(feature = "ollama")]

use serial_test::serial;
use terraphim_service::llm;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
#[serial]
async fn test_ollama_summarize_minimal() {
    let server = MockServer::start().await;

    // Emulate Ollama /api/chat response shape
    let body = serde_json::json!({
        "message": {"role": "assistant", "content": "Short summary."}
    });

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    // Build a Role with llm extra hints
    let mut role = terraphim_config::Role {
        shortname: Some("test".into()),
        name: "test".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        ..Default::default()
    };
    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra
        .insert("llm_model".into(), serde_json::json!("llama3.1"));
    role.extra
        .insert("llm_base_url".into(), serde_json::json!(server.uri()));

    let client = llm::build_llm_from_role(&role).expect("client");
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        client.summarize(
            "This is a long article content.",
            llm::SummarizeOptions { max_length: 120 },
        ),
    )
    .await
    .expect("no timeout")
    .expect("ok");

    assert!(out.contains("Short summary"));
}
