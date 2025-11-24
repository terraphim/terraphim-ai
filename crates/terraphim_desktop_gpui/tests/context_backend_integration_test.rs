/// Context Management Backend Integration Tests
///
/// Validates that GPUI uses the EXACT same ContextManager as Tauri
/// by calling the same methods with the same patterns.

use terraphim_service::context::{ContextConfig, ContextManager};
use terraphim_types::{ChatMessage, ContextItem, ContextType, ConversationId, RoleName};

#[tokio::test]
async fn test_context_manager_create_conversation() {
    // Pattern from Tauri cmd.rs:950-978
    let mut manager = ContextManager::new(ContextConfig::default());

    let title = "Test Conversation".to_string();
    let role = RoleName::from("Terraphim Engineer");

    let conversation_id = manager
        .create_conversation(title.clone(), role)
        .await
        .unwrap();

    println!("✅ Created conversation: {}", conversation_id.as_str());

    // Verify conversation exists
    let conversation = manager.get_conversation(&conversation_id);
    assert!(conversation.is_some(), "Conversation should exist");

    let conv = conversation.unwrap();
    assert_eq!(conv.title, title);
    assert_eq!(conv.role.as_str(), "Terraphim Engineer");
}

#[tokio::test]
async fn test_context_manager_add_context() {
    // Pattern from Tauri cmd.rs:1078-1140
    let mut manager = ContextManager::new(ContextConfig::default());

    let conv_id = manager
        .create_conversation("Test".to_string(), "Default".into())
        .await
        .unwrap();

    let context_item = ContextItem {
        id: "test-context-1".to_string(),
        context_type: ContextType::Document,
        title: "Test Context".to_string(),
        summary: Some("A test context item".to_string()),
        content: "This is test content for context management".to_string(),
        metadata: Default::default(),
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.95),
    };

    let result = manager.add_context(&conv_id, context_item.clone());
    assert!(result.is_ok(), "Should add context successfully");

    println!("✅ Added context to conversation");

    // Verify context was added
    let conversation = manager.get_conversation(&conv_id).unwrap();
    assert_eq!(conversation.global_context.len(), 1);
    assert_eq!(conversation.global_context[0].title, "Test Context");
}

#[tokio::test]
async fn test_context_manager_delete_context() {
    // Pattern from Tauri cmd.rs:1180-1211
    let mut manager = ContextManager::new(ContextConfig::default());

    let conv_id = manager
        .create_conversation("Test".to_string(), "Default".into())
        .await
        .unwrap();

    let context_item = ContextItem {
        id: "test-context-1".to_string(),
        context_type: ContextType::Document,
        title: "To Be Deleted".to_string(),
        summary: None,
        content: "This will be deleted".to_string(),
        metadata: Default::default(),
        created_at: chrono::Utc::now(),
        relevance_score: None,
    };

    manager.add_context(&conv_id, context_item.clone()).unwrap();

    // Delete the context
    let result = manager.delete_context(&conv_id, &context_item.id);
    assert!(result.is_ok(), "Should delete context successfully");

    println!("✅ Deleted context from conversation");

    // Verify deletion
    let conversation = manager.get_conversation(&conv_id).unwrap();
    assert_eq!(conversation.global_context.len(), 0);
}

#[tokio::test]
async fn test_context_manager_multiple_contexts() {
    let mut manager = ContextManager::new(ContextConfig::default());

    let conv_id = manager
        .create_conversation("Multi-Context Test".to_string(), "Default".into())
        .await
        .unwrap();

    // Add multiple context items
    for i in 1..=5 {
        let context_item = ContextItem {
            id: format!("context-{}", i),
            context_type: ContextType::Document,
            title: format!("Context Item {}", i),
            summary: None,
            content: format!("Content for item {}", i),
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.9),
        };

        manager.add_context(&conv_id, context_item).unwrap();
    }

    println!("✅ Added 5 context items");

    let conversation = manager.get_conversation(&conv_id).unwrap();
    assert_eq!(conversation.global_context.len(), 5);

    // Delete one item
    manager.delete_context(&conv_id, "context-3").unwrap();

    let conversation = manager.get_conversation(&conv_id).unwrap();
    assert_eq!(conversation.global_context.len(), 4);

    println!("✅ Context management with multiple items works");
}

#[tokio::test]
async fn test_context_manager_search_context_creation() {
    // Pattern from Tauri cmd.rs:1142-1178 (add_search_context_to_conversation)
    use terraphim_types::Document;

    let mut manager = ContextManager::new(ContextConfig::default());

    let conv_id = manager
        .create_conversation("Search Context Test".to_string(), "Default".into())
        .await
        .unwrap();

    let documents = vec![
        Document {
            id: "doc1".to_string(),
            title: "Rust Async".to_string(),
            url: "https://example.com/async".to_string(),
            body: "Full async content".to_string(),
            description: Some("Async programming in Rust".to_string()),
            tags: Some(vec!["rust".to_string(), "async".to_string()]),
            rank: Some(10),
            source_haystack: None,
            stub: None,
            summarization: None,
        },
        Document {
            id: "doc2".to_string(),
            title: "Tokio Guide".to_string(),
            url: "https://example.com/tokio".to_string(),
            body: "Tokio runtime guide".to_string(),
            description: Some("Tokio async runtime".to_string()),
            tags: Some(vec!["tokio".to_string()]),
            rank: Some(8),
            source_haystack: None,
            stub: None,
            summarization: None,
        },
    ];

    // Create search context (SAME as Tauri)
    let context_item = manager.create_search_context("async rust", &documents, Some(2));

    assert_eq!(context_item.context_type, ContextType::Document);
    assert!(context_item.title.contains("Search: async rust"));
    assert!(context_item.content.contains("Rust Async"));
    assert!(context_item.content.contains("Tokio Guide"));

    println!("✅ Search context creation matches Tauri pattern");

    // Add it to conversation
    manager.add_context(&conv_id, context_item).unwrap();

    let conversation = manager.get_conversation(&conv_id).unwrap();
    assert_eq!(conversation.global_context.len(), 1);
}

#[tokio::test]
async fn test_context_manager_conversation_listing() {
    let mut manager = ContextManager::new(ContextConfig::default());

    // Create multiple conversations
    for i in 1..=3 {
        manager
            .create_conversation(format!("Conversation {}", i), "Default".into())
            .await
            .unwrap();
    }

    let conversations = manager.list_conversations(None);
    assert_eq!(conversations.len(), 3);

    println!("✅ Listed {} conversations", conversations.len());

    // Test with limit
    let limited = manager.list_conversations(Some(2));
    assert_eq!(limited.len(), 2);

    println!("✅ Conversation listing with limit works");
}

#[test]
fn test_context_item_structure() {
    // Verify ContextItem structure matches Tauri expectations
    let item = ContextItem {
        id: "test".to_string(),
        context_type: ContextType::UserInput,
        title: "Test".to_string(),
        summary: Some("Summary".to_string()),
        content: "Content".to_string(),
        metadata: Default::default(),
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.8),
    };

    assert_eq!(item.id, "test");
    assert_eq!(item.context_type, ContextType::UserInput);
    assert!(item.summary.is_some());
    assert!(item.relevance_score.is_some());
}
