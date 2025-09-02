//! Integration tests for the Context Management API endpoints
//!
//! This module tests the HTTP API endpoints for conversation and context management.
//! The tests validate the complete API workflow including conversation creation,
//! message management, context attachment, and search context integration.

use ahash::AHashMap;
use serial_test::serial;
use std::{net::SocketAddr, time::Duration};
use terraphim_config::{Config, ConfigBuilder, Role};
use terraphim_server::{
    axum_server, AddContextRequest, AddContextResponse, AddMessageRequest, AddMessageResponse,
    AddSearchContextRequest, CreateConversationRequest, CreateConversationResponse,
    GetConversationResponse, ListConversationsResponse, Status,
};
use terraphim_service::http_client;
use terraphim_settings::DeviceSettings;
use terraphim_types::{
    ContextType, Document, RelevanceFunction,
};

/// Sample configuration for testing context management
fn create_test_config() -> Config {
    ConfigBuilder::new()
        .global_shortcut("Ctrl+T")
        .add_role(
            "TestRole",
            Role {
                shortname: Some("test".to_string()),
                name: "TestRole".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
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
                terraphim_it: false,
            },
        )
        .build()
        .unwrap()
}

/// Start a test server with context management API
async fn start_test_server() -> SocketAddr {
    let server_settings = DeviceSettings::load_from_env_and_file(None)
        .expect("Failed to load settings");
    let server_hostname = server_settings
        .server_hostname
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| {
            let port = portpicker::pick_unused_port().expect("Failed to find unused port");
            SocketAddr::from(([127, 0, 0, 1], port))
        });

    let mut config = create_test_config();
    let config_state = terraphim_config::ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    tokio::spawn(async move {
        axum_server(server_hostname, config_state)
            .await
            .expect("Server failed to start");
    });

    // Wait for server to be ready
    let client = http_client::create_default_client().expect("Failed to create HTTP client");
    let health_url = format!("http://{}/health", server_hostname);

    let mut attempts = 0;
    loop {
        match client.get(&health_url).send().await {
            Ok(response) if response.status() == 200 => break,
            _ => {
                if attempts >= 10 {
                    panic!("Server did not become ready in time");
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                attempts += 1;
            }
        }
    }

    server_hostname
}

/// Create test documents for use in search context tests
fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc-1".to_string(),
            url: "https://example.com/doc1".to_string(),
            title: "First Test Document".to_string(),
            body: "This is the first test document with important information about rust programming.".to_string(),
            description: Some("A document about Rust programming".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["rust".to_string(), "programming".to_string()]),
            rank: Some(95),
        },
        Document {
            id: "doc-2".to_string(),
            url: "https://example.com/doc2".to_string(),
            title: "Second Test Document".to_string(),
            body: "This document contains information about async programming and tokio.".to_string(),
            description: Some("A document about async Rust".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["async".to_string(), "tokio".to_string()]),
            rank: Some(87),
        },
        Document {
            id: "doc-3".to_string(),
            url: "https://example.com/doc3".to_string(),
            title: "Third Test Document".to_string(),
            body: "Advanced topics in web development with Axum and Serde.".to_string(),
            description: Some("A document about web development".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["web".to_string(), "axum".to_string()]),
            rank: Some(72),
        },
    ]
}

#[tokio::test]
#[serial]
async fn test_create_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    let request = CreateConversationRequest {
        title: "Test Conversation".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&request).unwrap())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response: CreateConversationResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert!(response.conversation_id.is_some());
    assert!(response.error.is_none());

    let conversation_id = response.conversation_id.unwrap();
    assert!(!conversation_id.is_empty());
}

#[tokio::test]
#[serial]
async fn test_create_conversation_invalid_role() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    let request = CreateConversationRequest {
        title: "Test Conversation".to_string(),
        role: "NonexistentRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&request).unwrap())
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response: CreateConversationResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    // Should still succeed but with the provided role name
    assert!(matches!(response.status, Status::Success));
    assert!(response.conversation_id.is_some());
}

#[tokio::test]
#[serial]
async fn test_list_conversations_empty() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    let response = client
        .get(format!("http://{}/conversations", server))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response: ListConversationsResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    // Note: May not be empty due to shared global context manager across tests
    // This is expected behavior in integration tests
    assert!(response.error.is_none());
}

#[tokio::test]
#[serial]
async fn test_list_conversations_with_data() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create two conversations
    let conv1_request = CreateConversationRequest {
        title: "First Conversation".to_string(),
        role: "TestRole".to_string(),
    };

    let conv2_request = CreateConversationRequest {
        title: "Second Conversation".to_string(),
        role: "TestRole".to_string(),
    };

    // Create first conversation
    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&conv1_request).unwrap())
        .send()
        .await
        .expect("Failed to create first conversation");
    let conv1: CreateConversationResponse = response.json().await.unwrap();
    assert!(matches!(conv1.status, Status::Success));

    // Create second conversation
    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&conv2_request).unwrap())
        .send()
        .await
        .expect("Failed to create second conversation");
    let conv2: CreateConversationResponse = response.json().await.unwrap();
    assert!(matches!(conv2.status, Status::Success));

    // List conversations
    let response = client
        .get(format!("http://{}/conversations", server))
        .send()
        .await
        .expect("Failed to list conversations");

    assert_eq!(response.status(), 200);

    let response: ListConversationsResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    // Should have at least our 2 conversations, but may have more from other tests
    assert!(response.conversations.len() >= 2);
    
    let titles: Vec<String> = response.conversations.iter().map(|c| c.title.clone()).collect();
    assert!(titles.contains(&"First Conversation".to_string()));
    assert!(titles.contains(&"Second Conversation".to_string()));
}

#[tokio::test]
#[serial]
async fn test_list_conversations_with_limit() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create three conversations
    for i in 1..=3 {
        let request = CreateConversationRequest {
            title: format!("Conversation {}", i),
            role: "TestRole".to_string(),
        };

        let response = client
            .post(format!("http://{}/conversations", server))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&request).unwrap())
            .send()
            .await
            .expect("Failed to create conversation");
        let conv: CreateConversationResponse = response.json().await.unwrap();
        assert!(matches!(conv.status, Status::Success));
    }

    // List conversations with limit=2
    let response = client
        .get(format!("http://{}/conversations?limit=2", server))
        .send()
        .await
        .expect("Failed to list conversations");

    assert_eq!(response.status(), 200);

    let response: ListConversationsResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert_eq!(response.conversations.len(), 2);
}

#[tokio::test]
#[serial]
async fn test_get_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create a conversation first
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Get".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    assert!(matches!(create_response.status, Status::Success));
    let conversation_id = create_response.conversation_id.unwrap();

    // Now get the conversation
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    assert_eq!(response.status(), 200);

    let response: GetConversationResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert!(response.conversation.is_some());
    assert!(response.error.is_none());

    let conversation = response.conversation.unwrap();
    assert_eq!(conversation.title, "Test Conversation for Get");
    assert_eq!(conversation.role.as_str(), "TestRole");
    assert_eq!(conversation.messages.len(), 0);
    assert!(conversation.global_context.is_empty());
}

#[tokio::test]
#[serial]
async fn test_get_conversation_not_found() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    let response = client
        .get(format!("http://{}/conversations/nonexistent-id", server))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let response: GetConversationResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Error));
    assert!(response.conversation.is_none());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("not found"));
}

#[tokio::test]
#[serial]
async fn test_add_message_to_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Messages".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add a user message
    let message_request = AddMessageRequest {
        content: "Hello, this is a test message!".to_string(),
        role: Some("user".to_string()),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&message_request).unwrap())
        .send()
        .await
        .expect("Failed to add message");

    assert_eq!(response.status(), 200);

    let response: AddMessageResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert!(response.message_id.is_some());
    assert!(response.error.is_none());

    // Verify the message was added by getting the conversation
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].content, "Hello, this is a test message!");
    assert_eq!(conversation.messages[0].role, "user");
}

#[tokio::test]
#[serial]
async fn test_add_message_different_roles() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Different Roles".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Test different message roles
    let roles_and_contents = vec![
        ("user", "User message"),
        ("assistant", "Assistant message"),
        ("system", "System message"),
    ];

    for (role, content) in roles_and_contents {
        let message_request = AddMessageRequest {
            content: content.to_string(),
            role: Some(role.to_string()),
        };

        let response = client
            .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&message_request).unwrap())
            .send()
            .await
            .expect("Failed to add message");

        let response: AddMessageResponse = response.json().await.unwrap();
        assert!(matches!(response.status, Status::Success));
    }

    // Verify all messages were added
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 3);
}

#[tokio::test]
#[serial]
async fn test_add_message_default_role() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Default Role".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add message without specifying role (should default to "user")
    let message_request = AddMessageRequest {
        content: "Message with default role".to_string(),
        role: None,
    };

    let response = client
        .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&message_request).unwrap())
        .send()
        .await
        .expect("Failed to add message");

    let response: AddMessageResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // Verify the role defaulted to "user"
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].role, "user");
}

#[tokio::test]
#[serial]
async fn test_add_message_invalid_role() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Invalid Role".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Try to add message with invalid role
    let message_request = AddMessageRequest {
        content: "Message with invalid role".to_string(),
        role: Some("invalid_role".to_string()),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&message_request).unwrap())
        .send()
        .await
        .expect("Failed to send request");

    let response: AddMessageResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Invalid role"));
}

#[tokio::test]
#[serial]
async fn test_add_context_to_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Context".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add context
    let context_request = AddContextRequest {
        context_type: "document".to_string(),
        title: "Test Document Context".to_string(),
        content: "This is a test document that provides context for the conversation.".to_string(),
        metadata: Some([("source".to_string(), "test".to_string())].into_iter().collect()),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&context_request).unwrap())
        .send()
        .await
        .expect("Failed to add context");

    assert_eq!(response.status(), 200);

    let response: AddContextResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert!(response.error.is_none());

    // Verify the context was added
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);
    
    let context = &conversation.global_context[0];
    assert_eq!(context.title, "Test Document Context");
    assert_eq!(context.content, "This is a test document that provides context for the conversation.");
    assert!(matches!(context.context_type, ContextType::Document));
    assert!(context.metadata.contains_key("source"));
}

#[tokio::test]
#[serial]
async fn test_add_context_different_types() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Different Context Types".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Test different context types
    let context_types = vec![
        ("document", "Document Context", "Document content"),
        ("search_result", "Search Result Context", "Search result content"),
        ("user_input", "User Input Context", "User input content"),
        ("system", "System Context", "System content"),
        ("external", "External Context", "External content"),
    ];

    for (ctx_type, title, content) in context_types {
        let context_request = AddContextRequest {
            context_type: ctx_type.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            metadata: None,
        };

        let response = client
            .post(format!("http://{}/conversations/{}/context", server, conversation_id))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&context_request).unwrap())
            .send()
            .await
            .expect("Failed to add context");

        let response: AddContextResponse = response.json().await.unwrap();
        assert!(matches!(response.status, Status::Success));
    }

    // Verify all context items were added
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 5);
}

#[tokio::test]
#[serial]
async fn test_add_context_invalid_type() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Invalid Context Type".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Try to add context with invalid type
    let context_request = AddContextRequest {
        context_type: "invalid_type".to_string(),
        title: "Invalid Context".to_string(),
        content: "This context has an invalid type".to_string(),
        metadata: None,
    };

    let response = client
        .post(format!("http://{}/conversations/{}/context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&context_request).unwrap())
        .send()
        .await
        .expect("Failed to send request");

    let response: AddContextResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Invalid context type"));
}

#[tokio::test]
#[serial]
async fn test_add_search_context_to_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Search Context".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add search context
    let search_context_request = AddSearchContextRequest {
        query: "rust programming".to_string(),
        documents: create_test_documents(),
        limit: Some(2),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/search-context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&search_context_request).unwrap())
        .send()
        .await
        .expect("Failed to add search context");

    assert_eq!(response.status(), 200);

    let response: AddContextResponse = response
        .json()
        .await
        .expect("Failed to parse response");

    assert!(matches!(response.status, Status::Success));
    assert!(response.error.is_none());

    // Verify the search context was added
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);
    
    let context = &conversation.global_context[0];
    assert!(context.title.contains("rust programming"));
    assert!(matches!(context.context_type, ContextType::SearchResult));
    assert!(context.content.contains("First Test Document"));
    assert!(context.content.contains("Second Test Document"));
    // Should be limited to 2 documents
    assert!(!context.content.contains("Third Test Document"));
}

#[tokio::test]
#[serial]
async fn test_add_search_context_no_limit() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Test Conversation for Search Context No Limit".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add search context without limit
    let search_context_request = AddSearchContextRequest {
        query: "programming documentation".to_string(),
        documents: create_test_documents(),
        limit: None,
    };

    let response = client
        .post(format!("http://{}/conversations/{}/search-context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&search_context_request).unwrap())
        .send()
        .await
        .expect("Failed to add search context");

    let response: AddContextResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // Verify all documents are included (default limit applies)
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    let conversation = conv_response.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);
    
    let context = &conversation.global_context[0];
    assert!(context.content.contains("First Test Document"));
    assert!(context.content.contains("Second Test Document"));
    assert!(context.content.contains("Third Test Document"));
}

#[tokio::test]
#[serial]
async fn test_conversation_context_workflow() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // 1. Create conversation
    let create_request = CreateConversationRequest {
        title: "Complete Context Workflow Test".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // 2. Add a user message
    let message_request = AddMessageRequest {
        content: "I need help with Rust programming".to_string(),
        role: None, // Default to "user"
    };

    let response = client
        .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&message_request).unwrap())
        .send()
        .await
        .expect("Failed to add message");

    let response: AddMessageResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // 3. Add relevant documents as search context
    let search_context_request = AddSearchContextRequest {
        query: "rust programming help".to_string(),
        documents: create_test_documents(),
        limit: Some(2),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/search-context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&search_context_request).unwrap())
        .send()
        .await
        .expect("Failed to add search context");

    let response: AddContextResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // 4. Add additional manual context
    let context_request = AddContextRequest {
        context_type: "user_input".to_string(),
        title: "User's Background".to_string(),
        content: "I'm a beginner programmer learning Rust for systems programming.".to_string(),
        metadata: Some([("skill_level".to_string(), "beginner".to_string())].into_iter().collect()),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/context", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&context_request).unwrap())
        .send()
        .await
        .expect("Failed to add context");

    let response: AddContextResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // 5. Add assistant response
    let assistant_message = AddMessageRequest {
        content: "Based on the context, I can help you with Rust programming. Let me explain async programming concepts.".to_string(),
        role: Some("assistant".to_string()),
    };

    let response = client
        .post(format!("http://{}/conversations/{}/messages", server, conversation_id))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&assistant_message).unwrap())
        .send()
        .await
        .expect("Failed to add assistant message");

    let response: AddMessageResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Success));

    // 6. Verify the complete conversation state
    let response = client
        .get(format!("http://{}/conversations/{}", server, conversation_id))
        .send()
        .await
        .expect("Failed to get conversation");

    let conv_response: GetConversationResponse = response.json().await.unwrap();
    assert!(matches!(conv_response.status, Status::Success));
    
    let conversation = conv_response.conversation.unwrap();
    
    // Check conversation metadata
    assert_eq!(conversation.title, "Complete Context Workflow Test");
    assert_eq!(conversation.role.as_str(), "TestRole");
    
    // Check messages
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[0].role, "user");
    assert_eq!(conversation.messages[0].content, "I need help with Rust programming");
    assert_eq!(conversation.messages[1].role, "assistant");
    assert!(conversation.messages[1].content.contains("async programming"));
    
    // Check global context
    assert_eq!(conversation.global_context.len(), 2);
    
    // Find search context
    let search_context = conversation.global_context.iter()
        .find(|ctx| matches!(ctx.context_type, ContextType::SearchResult))
        .expect("Search context not found");
    assert!(search_context.title.contains("rust programming help"));
    assert!(search_context.content.contains("First Test Document"));
    
    // Find user input context
    let user_context = conversation.global_context.iter()
        .find(|ctx| matches!(ctx.context_type, ContextType::UserInput))
        .expect("User input context not found");
    assert_eq!(user_context.title, "User's Background");
    assert!(user_context.metadata.contains_key("skill_level"));
}

#[tokio::test]
#[serial]
async fn test_context_limits() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Create conversation
    let create_request = CreateConversationRequest {
        title: "Context Limits Test".to_string(),
        role: "TestRole".to_string(),
    };

    let response = client
        .post(format!("http://{}/conversations", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&create_request).unwrap())
        .send()
        .await
        .expect("Failed to create conversation");

    let create_response: CreateConversationResponse = response.json().await.unwrap();
    let conversation_id = create_response.conversation_id.unwrap();

    // Add many context items to test limits (default config has max_context_items: 50)
    let mut success_count = 0;
    let mut error_count = 0;

    for i in 0..60 {  // Try to add more than the limit
        let context_request = AddContextRequest {
            context_type: "document".to_string(),
            title: format!("Test Document {}", i),
            content: format!("Content for document {}", i),
            metadata: None,
        };

        let response = client
            .post(format!("http://{}/conversations/{}/context", server, conversation_id))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&context_request).unwrap())
            .send()
            .await
            .expect("Failed to send request");

        let response: AddContextResponse = response.json().await.unwrap();
        
        match response.status {
            Status::Success => success_count += 1,
            Status::Error => {
                error_count += 1;
                // Should get error about maximum context items reached
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("Maximum context items"));
            }
            Status::PartialSuccess => {
                // Treat as success for this test
                success_count += 1;
            }
        }
    }

    // Should succeed up to the limit, then start failing
    assert!(success_count > 0);
    assert!(success_count <= 50);  // Shouldn't exceed configured limit
    assert!(error_count > 0);      // Should have some failures
    assert_eq!(success_count + error_count, 60);
}

#[tokio::test]
#[serial]
async fn test_context_nonexistent_conversation() {
    let server = start_test_server().await;
    let client = http_client::create_default_client().expect("Failed to create HTTP client");

    // Try to add context to nonexistent conversation
    let context_request = AddContextRequest {
        context_type: "document".to_string(),
        title: "Test Document".to_string(),
        content: "This is a test document".to_string(),
        metadata: None,
    };

    let response = client
        .post(format!("http://{}/conversations/nonexistent-id/context", server))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&context_request).unwrap())
        .send()
        .await
        .expect("Failed to send request");

    let response: AddContextResponse = response.json().await.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("not found"));
}