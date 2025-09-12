use ahash::AHashMap;
use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_service::llm::{build_llm_from_role, ChatOptions};
use terraphim_types::{ContextItem, ContextType, ConversationId, RoleName};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test chat completion with context using mocked OpenRouter
#[tokio::test]
#[cfg(feature = "openrouter")]
async fn test_openrouter_chat_with_context() {
    // Setup mock OpenRouter server
    let server = MockServer::start().await;
    std::env::set_var("OPENROUTER_BASE_URL", server.uri());

    // Mock response with context-aware answer
    let body = serde_json::json!({
        "choices": [{
            "message": {"content": "Based on the context about Rust async programming, I can help you implement tokio tasks efficiently."}
        }]
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    // Create OpenRouter role configuration
    let mut role = create_test_openrouter_role();
    role.extra.insert(
        "openrouter_base_url".to_string(),
        serde_json::json!(server.uri()),
    );

    // Create LLM client
    let llm_client = build_llm_from_role(&role).expect("Should build OpenRouter client");

    // Create context manager and conversation
    let mut context_manager = ContextManager::new(ContextConfig::default());
    let conversation_id = context_manager
        .create_conversation("Test Chat".to_string(), RoleName::new("Test"))
        .await
        .expect("Should create conversation");

    // Add context about Rust async programming
    let context_item = ContextItem {
        id: "rust-async-1".to_string(),
        context_type: ContextType::Document,
        title: "Rust Async Programming Guide".to_string(),
        summary: Some("Guide to async programming in Rust with tokio".to_string()),
        content: "Rust async programming uses the tokio runtime for concurrent execution. Key concepts include async/await syntax, futures, and tasks.".to_string(),
        metadata: {
            let mut map = AHashMap::new();
            map.insert("source".to_string(), "rust-doc".to_string());
            map
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(95.0),
    };

    context_manager
        .add_context(&conversation_id, context_item)
        .expect("Should add context");

    // Get conversation and build messages with context
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Should get conversation");

    let messages_with_context =
        terraphim_service::context::build_llm_messages_with_context(&conversation, true);

    // Test that context is properly injected
    assert!(!messages_with_context.is_empty());
    let context_message = &messages_with_context[0];
    assert_eq!(context_message["role"], "system");
    let content = context_message["content"].as_str().unwrap();
    assert!(content.contains("Context Information:"));
    assert!(content.contains("### Rust Async Programming Guide"));
    assert!(content.contains("tokio runtime"));

    // Add user message
    let mut messages = messages_with_context;
    messages.push(serde_json::json!({
        "role": "user",
        "content": "How do I create async tasks in Rust?"
    }));

    // Test chat completion with context
    let chat_opts = ChatOptions {
        max_tokens: Some(512),
        temperature: Some(0.7),
    };

    let response = llm_client
        .chat_completion(messages, chat_opts)
        .await
        .expect("Chat completion should succeed");

    // Verify the response acknowledges the context
    assert!(response.contains("context"));
    assert!(response.contains("tokio"));
    assert!(response.contains("implement"));
}

/// Test chat completion with context using mocked Ollama
#[tokio::test]
#[cfg(feature = "ollama")]
async fn test_ollama_chat_with_context() {
    // Setup mock Ollama server
    let server = MockServer::start().await;

    // Mock response with context-aware answer
    let body = serde_json::json!({
        "message": {
            "role": "assistant",
            "content": "Based on the provided context about Docker containers, I can help you optimize your containerization strategy."
        }
    });

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    // Create Ollama role configuration
    let role = create_test_ollama_role(&server.uri());

    // Create LLM client
    let llm_client = build_llm_from_role(&role).expect("Should build Ollama client");

    // Create context manager and conversation
    let mut context_manager = ContextManager::new(ContextConfig::default());
    let conversation_id = context_manager
        .create_conversation("Docker Chat".to_string(), RoleName::new("DevOps"))
        .await
        .expect("Should create conversation");

    // Add context about Docker
    let context_item = ContextItem {
        id: "docker-1".to_string(),
        context_type: ContextType::Document,
        title: "Docker Best Practices".to_string(),
        summary: Some("Best practices for Docker containerization".to_string()),
        content: "Docker containers provide lightweight virtualization. Best practices include multi-stage builds, minimal base images, and proper layer caching.".to_string(),
        metadata: {
            let mut map = AHashMap::new();
            map.insert("category".to_string(), "devops".to_string());
            map
        },
        created_at: chrono::Utc::now(),
        relevance_score: Some(88.0),
    };

    context_manager
        .add_context(&conversation_id, context_item)
        .expect("Should add context");

    // Get conversation and build messages with context
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Should get conversation");

    let messages_with_context =
        terraphim_service::context::build_llm_messages_with_context(&conversation, true);

    // Add user message
    let mut messages = messages_with_context;
    messages.push(serde_json::json!({
        "role": "user",
        "content": "What are the best practices for Docker containers?"
    }));

    // Test chat completion with context
    let chat_opts = ChatOptions {
        max_tokens: Some(1024),
        temperature: Some(0.8),
    };

    let response = llm_client
        .chat_completion(messages, chat_opts)
        .await
        .expect("Chat completion should succeed");

    // Verify the response acknowledges the context
    assert!(response.contains("context"));
    assert!(response.contains("Docker"));
    assert!(response.contains("containerization"));
}

/// Test that context is properly formatted for LLM consumption
#[tokio::test]
async fn test_context_formatting() {
    let mut context_manager = ContextManager::new(ContextConfig::default());
    let conversation_id = context_manager
        .create_conversation("Format Test".to_string(), RoleName::new("Test"))
        .await
        .expect("Should create conversation");

    // Add multiple context items with different types
    let contexts = vec![
        ContextItem {
            id: "doc-1".to_string(),
            context_type: ContextType::Document,
            title: "API Documentation".to_string(),
            summary: Some("REST API documentation".to_string()),
            content: "The API supports GET, POST, PUT, and DELETE operations.".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: Some(92.0),
        },
        ContextItem {
            id: "search-1".to_string(),
            context_type: ContextType::Document,
            title: "Search Result: API Examples".to_string(),
            summary: Some("Code examples for API usage".to_string()),
            content: "Example: curl -X GET https://api.example.com/users".to_string(),
            metadata: {
                let mut map = AHashMap::new();
                map.insert("query".to_string(), "API examples".to_string());
                map
            },
            created_at: chrono::Utc::now(),
            relevance_score: Some(85.0),
        },
    ];

    for context in contexts {
        context_manager
            .add_context(&conversation_id, context)
            .expect("Should add context");
    }

    // Get conversation and build messages
    let conversation = context_manager
        .get_conversation(&conversation_id)
        .expect("Should get conversation");

    let messages = terraphim_service::context::build_llm_messages_with_context(&conversation, true);

    // Verify context formatting
    assert_eq!(messages.len(), 1); // Only system message with context
    let context_message = &messages[0];
    assert_eq!(context_message["role"], "system");

    let content = context_message["content"].as_str().unwrap();

    // Verify proper structure (using the actual format from context.rs)
    assert!(content.contains("Context Information:"));
    assert!(content.contains("### API Documentation"));
    assert!(content.contains("### Search Result: API Examples"));
    assert!(content.contains("GET, POST, PUT"));
    assert!(content.contains("curl -X GET"));

    // The context.rs format doesn't include relevance scores or metadata
    // Those are only in the server's api.rs format
}

/// Test empty context handling
#[tokio::test]
async fn test_empty_context_handling() {
    let _context_manager = ContextManager::new(ContextConfig::default());
    let _conversation_id = ConversationId::from_string("empty-test".to_string());

    // Create empty conversation (not added to manager)
    let conversation =
        terraphim_types::Conversation::new("Empty Test".to_string(), RoleName::new("Test"));

    // Build messages with no context
    let messages = terraphim_service::context::build_llm_messages_with_context(&conversation, true);

    // Should return empty array when no context is present
    assert!(messages.is_empty());

    // Test with include_global_context = false
    let messages =
        terraphim_service::context::build_llm_messages_with_context(&conversation, false);
    assert!(messages.is_empty());
}

// Helper functions

#[cfg(feature = "openrouter")]
fn create_test_openrouter_role() -> terraphim_config::Role {
    let mut role = terraphim_config::Role {
        shortname: Some("TestOpenRouter".into()),
        name: "Test OpenRouter".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        openrouter_enabled: true,
        openrouter_api_key: Some("sk-test-key".to_string()),
        openrouter_model: Some("openai/gpt-3.5-turbo".to_string()),
        openrouter_auto_summarize: false,
        openrouter_chat_enabled: true,
        openrouter_chat_system_prompt: Some("You are a helpful assistant.".to_string()),
        openrouter_chat_model: Some("openai/gpt-3.5-turbo".to_string()),
        extra: AHashMap::new(),
    };

    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("openrouter"));

    role
}

#[cfg(feature = "ollama")]
fn create_test_ollama_role(base_url: &str) -> terraphim_config::Role {
    let mut role = terraphim_config::Role {
        shortname: Some("TestOllama".into()),
        name: "Test Ollama".into(),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        terraphim_it: false,
        theme: "default".into(),
        kg: None,
        haystacks: vec![],
        #[cfg(feature = "openrouter")]
        openrouter_enabled: false,
        #[cfg(feature = "openrouter")]
        openrouter_api_key: None,
        #[cfg(feature = "openrouter")]
        openrouter_model: None,
        #[cfg(feature = "openrouter")]
        openrouter_auto_summarize: false,
        #[cfg(feature = "openrouter")]
        openrouter_chat_enabled: false,
        #[cfg(feature = "openrouter")]
        openrouter_chat_system_prompt: None,
        #[cfg(feature = "openrouter")]
        openrouter_chat_model: None,
        extra: AHashMap::new(),
    };

    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra
        .insert("llm_model".to_string(), serde_json::json!("llama3.2:3b"));
    role.extra
        .insert("llm_base_url".to_string(), serde_json::json!(base_url));
    role.extra.insert(
        "system_prompt".to_string(),
        serde_json::json!("You are a helpful DevOps assistant."),
    );

    role
}
