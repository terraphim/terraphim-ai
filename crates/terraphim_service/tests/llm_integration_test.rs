use ahash::AHashMap;
use serial_test::serial;
use terraphim_service::llm::{ChatOptions, SummarizeOptions, build_llm_from_role};

/// Test LLM client integration with real Ollama
#[tokio::test]
#[serial]
#[ignore] // Only run when Ollama is available
async fn test_llm_client_integration() {
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

    // Create test role configuration
    let mut role = terraphim_config::Role {
        shortname: Some("TestLLM".into()),
        name: "Test LLM Integration".into(),
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
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra
        .insert("ollama_base_url".to_string(), serde_json::json!(ollama_url));

    let llm_client = build_llm_from_role(&role).expect("Should build LLM client");

    // Test summarization
    let summary = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        llm_client.summarize(
            "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It achieves these goals without requiring a garbage collector or runtime. Rust's ownership system ensures memory safety without sacrificing performance.",
            SummarizeOptions { max_length: 100 },
        ),
    )
    .await
    .expect("Summarization should not timeout")
    .expect("Summarization should succeed");

    assert!(!summary.is_empty(), "Summary should not be empty");
    println!("Summary: {}", summary);

    // Test chat completion
    let messages = vec![
        serde_json::json!({"role": "system", "content": "You are a helpful assistant."}),
        serde_json::json!({"role": "user", "content": "What is 2+2?"}),
    ];

    let chat_response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        llm_client.chat_completion(
            messages,
            ChatOptions {
                max_tokens: Some(50),
                temperature: Some(0.1),
            },
        ),
    )
    .await
    .expect("Chat should not timeout")
    .expect("Chat should succeed");

    assert!(
        !chat_response.is_empty(),
        "Chat response should not be empty"
    );
    println!("Chat response: {}", chat_response);
}

/// Test LLM client error handling
#[tokio::test]
#[serial]
async fn test_llm_client_error_handling() {
    // Create role with invalid Ollama URL
    let mut role = terraphim_config::Role {
        shortname: Some("TestError".into()),
        name: "Test Error Handling".into(),
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
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };
    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra.insert(
        "ollama_base_url".to_string(),
        serde_json::json!("http://invalid-url:9999"),
    );

    let llm_client = build_llm_from_role(&role).expect("Should build LLM client");

    // Test that network errors are handled gracefully
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        llm_client.summarize("Test content", SummarizeOptions { max_length: 50 }),
    )
    .await;

    // Should either timeout or return an error (not panic)
    match result {
        Ok(Err(_)) => println!("Correctly handled network error"),
        Err(_) => println!("Request timed out as expected"),
        Ok(Ok(_)) => panic!("Should not succeed with invalid URL"),
    }
}

/// Test role validation and LLM client creation
#[tokio::test]
async fn test_role_validation() {
    // Test role without LLM configuration
    let role_without_llm = terraphim_config::Role {
        shortname: Some("NoLLM".into()),
        name: "No LLM Config".into(),
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
        extra: AHashMap::new(),
        llm_router_enabled: false,
        llm_router_config: None,
    };

    // Should return error for disabled LLM
    let result = build_llm_from_role(&role_without_llm);
    assert!(
        result.is_none(),
        "Should fail to build LLM client when disabled"
    );

    // Test role with missing provider
    let mut role_missing_provider = role_without_llm.clone();
    role_missing_provider.llm_enabled = true;

    let result = build_llm_from_role(&role_missing_provider);
    assert!(
        result.is_none(),
        "Should fail without llm_provider in extra"
    );
}
