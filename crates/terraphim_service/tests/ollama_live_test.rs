#![cfg(feature = "ollama")]

use serial_test::serial;
use terraphim_service::llm;

/// End-to-end live test against a local Ollama instance.
/// Defaults to http://127.0.0.1:11434, can be overridden with OLLAMA_BASE_URL.
#[tokio::test]
#[serial]
async fn live_ollama_summarize_deepseek_coder() {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    // Quick connectivity check; if not running, exit early (treat as skipped)
    let http = terraphim_service::http_client::create_default_client()
        .unwrap_or_else(|_| reqwest::Client::new());
    let health = http
        .get(format!("{}/api/tags", base_url.trim_end_matches('/')))
        .send()
        .await;
    if health.is_err() {
        eprintln!("Ollama not reachable at {} — skipping live test", base_url);
        return;
    }

    // Build a Role configured for Ollama deepseek-coder:latest
    let mut role = terraphim_config::Role {
        shortname: Some("OllamaCoder".into()),
        name: "Ollama Coder".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        extra: ahash::AHashMap::new(),
    };
    role.extra.insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra
        .insert("llm_model".into(), serde_json::json!("deepseek-coder:latest"));
    role.extra
        .insert("llm_base_url".into(), serde_json::json!(base_url.clone()));
    role.extra
        .insert("llm_auto_summarize".into(), serde_json::json!(true));

    let client = match llm::build_llm_from_role(&role) {
        Some(c) => c,
        None => {
            panic!("Failed to initialize Ollama LLM client from role config");
        }
    };

    let content = r#"
    This repository contains a Rust service with several crates. Please summarize
    the purpose of a provider-agnostic LLM layer that abstracts over OpenRouter and
    Ollama to provide summarization. Keep it short and focused.
    "#;

    let summary = client
        .summarize(
            content,
            llm::SummarizeOptions {
                max_length: 160,
            },
        )
        .await
        .expect("summarize call should succeed");

    assert!(summary.len() > 0, "summary should be non-empty");
}
