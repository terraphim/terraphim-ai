//! Integration tests for Tauri context management commands
//!
//! This module tests the Tauri commands for conversation and context management.
//! These tests call the command functions directly rather than going through the
//! full Tauri IPC layer for simpler and more reliable testing.

use serial_test::serial;
use std::collections::HashMap;
use terraphim_types::Document;

// Import the command functions and types directly from the cmd module
use terraphim_ai_desktop::cmd::{
    add_context_to_conversation, add_message_to_conversation, add_search_context_to_conversation,
    create_conversation, get_conversation, list_conversations, Status,
};

/// Create test documents for use in search context tests
fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "doc-1".to_string(),
            url: "https://example.com/rust-guide".to_string(),
            title: "Rust Programming Guide".to_string(),
            body: "This comprehensive guide covers Rust programming fundamentals including ownership, borrowing, and lifetimes.".to_string(),
            description: Some("A comprehensive guide to Rust programming".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["rust".to_string(), "programming".to_string(), "guide".to_string()]),
            rank: Some(95),
        },
        Document {
            id: "doc-2".to_string(),
            url: "https://example.com/async-rust".to_string(),
            title: "Async Programming in Rust".to_string(),
            body: "Learn how to write asynchronous code in Rust using tokio, futures, and async/await syntax.".to_string(),
            description: Some("Guide to async programming in Rust".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["rust".to_string(), "async".to_string(), "tokio".to_string()]),
            rank: Some(88),
        },
        Document {
            id: "doc-3".to_string(),
            url: "https://example.com/web-rust".to_string(),
            title: "Web Development with Rust".to_string(),
            body: "Building web applications and APIs using Rust frameworks like Axum, Actix, and Rocket.".to_string(),
            description: Some("Web development guide for Rust".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["rust".to_string(), "web".to_string(), "axum".to_string()]),
            rank: Some(82),
        },
    ]
}

#[tokio::test]
#[serial]
async fn test_create_conversation_command() {
    let result = create_conversation("Test Conversation".to_string(), "TestRole".to_string()).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.conversation_id.is_some());
    assert!(response.error.is_none());
}

#[tokio::test]
#[serial]
async fn test_create_conversation_with_different_roles() {
    let roles = vec!["TestRole1", "TestRole2", "EngineerRole"];

    for role in roles {
        let result =
            create_conversation(format!("Conversation for {}", role), role.to_string()).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(response.status, Status::Success));
        assert!(response.conversation_id.is_some());
    }
}

#[tokio::test]
#[serial]
async fn test_list_conversations_command() {
    // Create a few conversations first
    let _conv1 = create_conversation("First Conversation".to_string(), "TestRole".to_string())
        .await
        .unwrap();

    let _conv2 = create_conversation("Second Conversation".to_string(), "TestRole".to_string())
        .await
        .unwrap();

    // List conversations
    let result = list_conversations(None).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.conversations.len() >= 2); // May have more from other tests
    assert!(response.error.is_none());

    // Check that our conversations are in the list
    let titles: Vec<String> = response
        .conversations
        .iter()
        .map(|c| c.title.clone())
        .collect();
    assert!(titles.contains(&"First Conversation".to_string()));
    assert!(titles.contains(&"Second Conversation".to_string()));
}

#[tokio::test]
#[serial]
async fn test_list_conversations_with_limit() {
    // Create several conversations
    for i in 1..=5 {
        let _conv = create_conversation(
            format!("Limited Conversation {}", i),
            "TestRole".to_string(),
        )
        .await
        .unwrap();
    }

    // List with limit = 3
    let result = list_conversations(Some(3)).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.conversations.len() <= 3); // Should respect the limit
}

#[tokio::test]
#[serial]
async fn test_get_conversation_command() {
    // Create a conversation first
    let create_result =
        create_conversation("Test Get Conversation".to_string(), "TestRole".to_string())
            .await
            .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Get the conversation
    let result = get_conversation(conversation_id).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.conversation.is_some());
    assert!(response.error.is_none());

    let conversation = response.conversation.unwrap();
    assert_eq!(conversation.title, "Test Get Conversation");
    assert_eq!(conversation.role.as_str(), "TestRole");
    assert_eq!(conversation.messages.len(), 0);
    assert!(conversation.global_context.is_empty());
}

#[tokio::test]
#[serial]
async fn test_get_conversation_not_found() {
    let result = get_conversation("nonexistent-id".to_string()).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.conversation.is_none());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("not found"));
}

#[tokio::test]
#[serial]
async fn test_add_message_to_conversation_command() {
    // Create a conversation first
    let create_result = create_conversation(
        "Test Message Conversation".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Add a user message
    let result = add_message_to_conversation(
        conversation_id.clone(),
        "Hello, this is a test message!".to_string(),
        Some("user".to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.message_id.is_some());
    assert!(response.error.is_none());

    // Verify the message was added by getting the conversation
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(
        conversation.messages[0].content,
        "Hello, this is a test message!"
    );
    assert_eq!(conversation.messages[0].role, "user");
}

#[tokio::test]
#[serial]
async fn test_add_message_different_roles() {
    // Create a conversation
    let create_result =
        create_conversation("Test Different Roles".to_string(), "TestRole".to_string())
            .await
            .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Test different message roles
    let roles_and_contents = vec![
        ("user", "User message content"),
        ("assistant", "Assistant message content"),
        ("system", "System message content"),
    ];

    for (role, content) in &roles_and_contents {
        let result = add_message_to_conversation(
            conversation_id.clone(),
            content.to_string(),
            Some(role.to_string()),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(response.status, Status::Success));
        assert!(response.message_id.is_some());
    }

    // Verify all messages were added
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 3);

    let roles: Vec<String> = conversation
        .messages
        .iter()
        .map(|m| m.role.clone())
        .collect();
    assert!(roles.contains(&"user".to_string()));
    assert!(roles.contains(&"assistant".to_string()));
    assert!(roles.contains(&"system".to_string()));
}

#[tokio::test]
#[serial]
async fn test_add_message_default_role() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Default Role Message".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Add message without specifying role (should default to "user")
    let result = add_message_to_conversation(
        conversation_id.clone(),
        "Message with default role".to_string(),
        None,
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));

    // Verify the role defaulted to "user"
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.messages.len(), 1);
    assert_eq!(conversation.messages[0].role, "user");
}

#[tokio::test]
#[serial]
async fn test_add_message_invalid_role() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Invalid Role Message".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Try to add message with invalid role
    let result = add_message_to_conversation(
        conversation_id,
        "Message with invalid role".to_string(),
        Some("invalid_role".to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.message_id.is_none());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Invalid role"));
}

#[tokio::test]
#[serial]
async fn test_add_context_to_conversation_command() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Context Conversation".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Add context
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "test".to_string());
    metadata.insert("category".to_string(), "documentation".to_string());

    let result = add_context_to_conversation(
        conversation_id.clone(),
        "document".to_string(),
        "Test Document Context".to_string(),
        "This is a test document that provides context for the conversation.".to_string(),
        Some(metadata),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.error.is_none());

    // Verify the context was added
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);

    let context = &conversation.global_context[0];
    assert_eq!(context.title, "Test Document Context");
    assert_eq!(
        context.content,
        "This is a test document that provides context for the conversation."
    );
    assert!(context.metadata.contains_key("source"));
    assert!(context.metadata.contains_key("category"));
}

#[tokio::test]
#[serial]
async fn test_add_context_different_types() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Different Context Types".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Test different context types
    let context_types = vec![
        ("document", "Document Context", "Document content"),
        (
            "search_result",
            "Search Result Context",
            "Search result content",
        ),
        ("user_input", "User Input Context", "User input content"),
        ("system", "System Context", "System content"),
        ("external", "External Context", "External content"),
    ];

    for (ctx_type, title, content) in &context_types {
        let result = add_context_to_conversation(
            conversation_id.clone(),
            ctx_type.to_string(),
            title.to_string(),
            content.to_string(),
            None,
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(matches!(response.status, Status::Success));
        assert!(response.error.is_none());
    }

    // Verify all context items were added
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 5);

    let titles: Vec<String> = conversation
        .global_context
        .iter()
        .map(|c| c.title.clone())
        .collect();

    for (_, expected_title, _) in &context_types {
        assert!(titles.contains(&expected_title.to_string()));
    }
}

#[tokio::test]
#[serial]
async fn test_add_context_invalid_type() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Invalid Context Type".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Try to add context with invalid type
    let result = add_context_to_conversation(
        conversation_id,
        "invalid_type".to_string(),
        "Invalid Context".to_string(),
        "This context has an invalid type".to_string(),
        None,
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Error));
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("Invalid context type"));
}

#[tokio::test]
#[serial]
async fn test_add_search_context_to_conversation_command() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Search Context Conversation".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Add search context
    let result = add_search_context_to_conversation(
        conversation_id.clone(),
        "rust programming guide".to_string(),
        create_test_documents(),
        Some(2),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));
    assert!(response.error.is_none());

    // Verify the search context was added
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);

    let context = &conversation.global_context[0];
    assert!(context.title.contains("rust programming guide"));
    assert!(context.content.contains("Rust Programming Guide"));
    assert!(context.content.contains("Async Programming in Rust"));
    // Should be limited to 2 documents
    assert!(!context.content.contains("Web Development with Rust"));
}

#[tokio::test]
#[serial]
async fn test_add_search_context_no_limit() {
    // Create a conversation
    let create_result = create_conversation(
        "Test Search Context No Limit".to_string(),
        "TestRole".to_string(),
    )
    .await
    .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // Add search context without limit
    let result = add_search_context_to_conversation(
        conversation_id.clone(),
        "complete programming documentation".to_string(),
        create_test_documents(),
        None,
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, Status::Success));

    // Verify all documents are included (default limit applies)
    let get_result = get_conversation(conversation_id).await.unwrap();
    let conversation = get_result.conversation.unwrap();
    assert_eq!(conversation.global_context.len(), 1);

    let context = &conversation.global_context[0];
    assert!(context.content.contains("Rust Programming Guide"));
    assert!(context.content.contains("Async Programming in Rust"));
    assert!(context.content.contains("Web Development with Rust"));
}

#[tokio::test]
#[serial]
async fn test_complete_conversation_workflow() {
    // 1. Create conversation
    let create_result =
        create_conversation("Complete Workflow Test".to_string(), "TestRole".to_string())
            .await
            .unwrap();

    let conversation_id = create_result.conversation_id.unwrap();

    // 2. Add user message
    let _message_result = add_message_to_conversation(
        conversation_id.clone(),
        "I need help with Rust programming".to_string(),
        None, // Default to "user"
    )
    .await
    .unwrap();

    // 3. Add search context
    let _context_result = add_search_context_to_conversation(
        conversation_id.clone(),
        "rust programming help".to_string(),
        create_test_documents(),
        Some(2),
    )
    .await
    .unwrap();

    // 4. Add manual context
    let mut metadata = HashMap::new();
    metadata.insert("skill_level".to_string(), "beginner".to_string());
    metadata.insert("focus".to_string(), "systems".to_string());

    let _manual_context_result = add_context_to_conversation(
        conversation_id.clone(),
        "user_input".to_string(),
        "User Background".to_string(),
        "I'm a beginner programmer learning Rust for systems programming.".to_string(),
        Some(metadata),
    )
    .await
    .unwrap();

    // 5. Add assistant response
    let _assistant_result = add_message_to_conversation(
        conversation_id.clone(),
        "Based on the context, I can help you with Rust programming. Let me explain key concepts."
            .to_string(),
        Some("assistant".to_string()),
    )
    .await
    .unwrap();

    // 6. Verify the complete conversation state
    let final_result = get_conversation(conversation_id).await.unwrap();
    let conversation = final_result.conversation.unwrap();

    // Check conversation metadata
    assert_eq!(conversation.title, "Complete Workflow Test");
    assert_eq!(conversation.role.as_str(), "TestRole");

    // Check messages
    assert_eq!(conversation.messages.len(), 2);
    assert_eq!(conversation.messages[0].role, "user");
    assert_eq!(
        conversation.messages[0].content,
        "I need help with Rust programming"
    );
    assert_eq!(conversation.messages[1].role, "assistant");
    assert!(conversation.messages[1]
        .content
        .contains("help you with Rust programming"));

    // Check global context
    assert_eq!(conversation.global_context.len(), 2);

    // Find search context
    let search_context = conversation
        .global_context
        .iter()
        .find(|ctx| ctx.title.contains("rust programming help"))
        .expect("Search context not found");
    assert!(search_context.content.contains("Rust Programming Guide"));

    // Find user input context
    let user_context = conversation
        .global_context
        .iter()
        .find(|ctx| ctx.title == "User Background")
        .expect("User input context not found");
    assert!(user_context.metadata.contains_key("skill_level"));
    assert_eq!(
        user_context.metadata.get("skill_level"),
        Some(&"beginner".to_string())
    );
}
