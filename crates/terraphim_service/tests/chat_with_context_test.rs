use ahash::AHashMap;
use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_service::llm::{build_llm_from_role, ChatOptions};
use terraphim_types::{ContextItem, ContextType, ConversationId, RoleName};

/// Test chat completion with context using real Ollama (requires Ollama running)
#[tokio::test]
#[ignore] // Only run when Ollama is available
async fn test_ollama_chat_with_context_real() {
    // Skip if Ollama is not running
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

    // Create Ollama role configuration
    let role = create_test_ollama_role(ollama_url);

    // Create LLM client
    let llm_client = build_llm_from_role(&role).expect("Should build Ollama client");

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

    // Verify we got a response (content may vary with real LLM)
    assert!(
        !response.is_empty(),
        "Should get non-empty response from Ollama"
    );
}

/// Test chat completion with multiple contexts using real Ollama
#[tokio::test]
#[ignore] // Only run when Ollama is available
async fn test_ollama_multi_context_chat() {
    // Skip if Ollama is not running
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

    // Create Ollama role configuration
    let role = create_test_ollama_role(ollama_url);

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

    // Verify we got a response (content may vary with real LLM)
    assert!(
        !response.is_empty(),
        "Should get non-empty response from Ollama"
    );
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
            context_type: ContextType::SearchResult,
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

fn create_test_ollama_role(base_url: &str) -> terraphim_config::Role {
    let mut role = terraphim_config::Role {
        shortname: Some("TestOllama".into()),
        name: "Test Ollama".into(),
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
    };

    role.extra
        .insert("llm_provider".to_string(), serde_json::json!("ollama"));
    role.extra
        .insert("ollama_base_url".to_string(), serde_json::json!(base_url));

    role
}
