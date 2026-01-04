use serial_test::serial;
use terraphim_service::llm;

#[tokio::test]
#[serial]
#[ignore] // Only run when Ollama is available
async fn test_ollama_summarize_real() {
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

    // Build a Role with llm configuration for real Ollama
    let mut role = terraphim_config::Role {
        shortname: Some("test".into()),
        name: "test".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        llm_enabled: true,
        llm_api_key: None,
        llm_model: Some("gemma3:270m".to_string()),
        llm_auto_summarize: false,
        llm_chat_enabled: true,
        llm_chat_system_prompt: Some("You are a helpful assistant.".to_string()),
        llm_chat_model: Some("gemma3:270m".to_string()),
        llm_context_window: None,
        extra: ahash::AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
        llm_router_enabled: false,
        llm_router_config: None,
    };
    role.extra
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    role.extra
        .insert("ollama_base_url".into(), serde_json::json!(ollama_url));

    let llm_client = llm::build_llm_from_role(&role).expect("Should build Ollama client");

    // Test summarization with real Ollama
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30), // Give more time for real LLM
        llm_client.summarize(
            "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It achieves these goals without requiring a garbage collector or runtime.",
            llm::SummarizeOptions { max_length: 120 },
        ),
    )
    .await
    .expect("Request should not timeout")
    .expect("Summarization should succeed");

    // Verify we got a non-empty result (content may vary with real LLM)
    assert!(!result.is_empty(), "Summary should not be empty");
    assert!(result.len() <= 200, "Summary should be reasonably short"); // Allow some flexibility
}
