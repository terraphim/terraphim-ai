//! Unit tests for the LLM Context Management service
//!
//! This module provides comprehensive testing coverage for the ContextManager
//! and related functionality including conversation management, context items,
//! and all the factory methods.

#[cfg(test)]
mod tests {
    use super::super::context::{ContextConfig, ContextManager};
    use ahash::AHashMap;
    use terraphim_types::{
        ChatMessage, ContextItem, ContextType, ConversationId, Document, RoleName,
    };
    use tokio::test;

    // Test fixtures
    fn create_test_config() -> ContextConfig {
        ContextConfig {
            max_context_items: 20,
            max_context_length: 50000,
            max_conversations_cache: 10,
            default_search_results_limit: 5,
            enable_auto_suggestions: false,
        }
    }

    fn create_test_document() -> Document {
        Document {
            id: "test-doc-1".to_string(),
            url: "https://example.com/test".to_string(),
            title: "Test Document".to_string(),
            body: "This is a test document with some content for testing context management."
                .to_string(),
            description: Some("A test document".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "document".to_string()]),
            rank: Some(1),
        }
    }

    fn create_test_documents(count: usize) -> Vec<Document> {
        (0..count)
            .map(|i| Document {
                id: format!("doc-{}", i),
                url: format!("https://example.com/doc-{}", i),
                title: format!("Test Document {}", i),
                body: format!("Content of document {} for testing.", i),
                description: Some(format!("Description for document {}", i)),
                summarization: None,
                stub: None,
                tags: Some(vec![format!("tag-{}", i), "test".to_string()]),
                rank: Some(i as u64),
            })
            .collect()
    }

    // Core functionality tests

    #[test]
    async fn test_create_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let title = "Test Conversation".to_string();
        let role = RoleName::new("engineer");

        let result = manager.create_conversation(title.clone(), role).await;

        assert!(result.is_ok());
        let conversation_id = result.unwrap();
        assert!(!conversation_id.as_str().is_empty());

        // Verify conversation exists
        let conversation = manager.get_conversation(&conversation_id);
        assert!(conversation.is_some());

        let conv = conversation.unwrap();
        assert_eq!(conv.title, title);
        assert_eq!(conv.role, RoleName::new("engineer"));
        assert!(conv.messages.is_empty());
        assert!(conv.global_context.is_empty());
    }

    #[test]
    async fn test_create_conversation_with_empty_title() {
        let mut manager = ContextManager::new(create_test_config());
        let title = "".to_string();
        let role = RoleName::new("engineer");

        let result = manager.create_conversation(title.clone(), role).await;

        assert!(result.is_ok());
        let conversation_id = result.unwrap();

        let conversation = manager.get_conversation(&conversation_id);
        assert!(conversation.is_some());
        // Should accept empty title
        assert_eq!(conversation.unwrap().title, "");
    }

    #[test]
    async fn test_list_conversations() {
        let mut manager = ContextManager::new(create_test_config());

        // Create multiple conversations
        let conv1 = manager
            .create_conversation("Conv 1".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        let conv2 = manager
            .create_conversation("Conv 2".to_string(), RoleName::new("researcher"))
            .await
            .unwrap();
        let conv3 = manager
            .create_conversation("Conv 3".to_string(), RoleName::new("writer"))
            .await
            .unwrap();

        // Test listing without limit
        let all_conversations = manager.list_conversations(None);
        assert_eq!(all_conversations.len(), 3);

        // Test listing with limit
        let limited_conversations = manager.list_conversations(Some(2));
        assert_eq!(limited_conversations.len(), 2);

        // Verify conversations are returned in order (most recent first)
        let conversations = manager.list_conversations(None);
        assert_eq!(conversations[0].id, conv3);
        assert_eq!(conversations[1].id, conv2);
        assert_eq!(conversations[2].id, conv1);
    }

    #[test]
    async fn test_get_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // Test getting existing conversation
        let result = manager.get_conversation(&conversation_id);
        assert!(result.is_some());

        // Test getting non-existent conversation
        let fake_id = ConversationId::from_string("non-existent".to_string());
        let result = manager.get_conversation(&fake_id);
        assert!(result.is_none());
    }

    #[test]
    async fn test_add_message_to_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        let message = ChatMessage::user("Hello, test message!".to_string());

        let result = manager.add_message(&conversation_id, message.clone());
        assert!(result.is_ok());

        // Verify message was added
        let conversation = manager.get_conversation(&conversation_id).unwrap();
        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].content, message.content);
        assert_eq!(conversation.messages[0].role, "user");
    }

    #[test]
    async fn test_add_context_to_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        let context_item = ContextItem {
            id: "test-context".to_string(),
            context_type: ContextType::Document,
            title: "Test Context".to_string(),
            summary: Some("Test context summary".to_string()),
            content: "This is test context content".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.8),
        };

        let result = manager.add_context(&conversation_id, context_item.clone());
        assert!(result.is_ok());

        // Verify context was added
        let conversation = manager.get_conversation(&conversation_id).unwrap();
        assert_eq!(conversation.global_context.len(), 1);
        assert_eq!(conversation.global_context[0].title, context_item.title);
        assert_eq!(conversation.global_context[0].content, context_item.content);
    }

    #[test]
    async fn test_create_search_context() {
        let manager = ContextManager::new(create_test_config());
        let documents = create_test_documents(3);
        let query = "test query";

        let context_item = manager.create_search_context(query, &documents, Some(2));

        assert_eq!(context_item.context_type, ContextType::Document);
        assert_eq!(context_item.title, "Search: test query");
        assert!(context_item.content.contains(query));
        assert!(context_item.content.contains("Test Document 0"));
        assert!(context_item.content.contains("Test Document 1"));
        // Should only include 2 documents due to limit
        assert!(!context_item.content.contains("Test Document 2"));
        assert_eq!(
            context_item.metadata.get("source_type").unwrap(),
            "search_result"
        );
        assert_eq!(context_item.metadata.get("query").unwrap(), query);
        assert_eq!(context_item.metadata.get("result_count").unwrap(), "2");
    }

    #[test]
    async fn test_create_search_context_with_empty_documents() {
        let manager = ContextManager::new(create_test_config());
        let documents: Vec<Document> = vec![];
        let query = "empty query";

        let context_item = manager.create_search_context(query, &documents, None);

        assert_eq!(context_item.context_type, ContextType::Document);
        assert!(context_item.content.contains("No results found"));
        assert_eq!(context_item.metadata.get("result_count").unwrap(), "0");
    }

    #[test]
    async fn test_create_document_context() {
        let manager = ContextManager::new(create_test_config());
        let document = create_test_document();

        let context_item = manager.create_document_context(&document);

        assert_eq!(context_item.context_type, ContextType::Document);
        assert_eq!(context_item.title, document.title);
        assert!(context_item.content.contains(&document.body));
        assert_eq!(
            context_item.metadata.get("source_type").unwrap(),
            "document"
        );
        assert_eq!(
            context_item.metadata.get("document_id").unwrap(),
            &document.id
        );
        assert_eq!(context_item.metadata.get("url").unwrap(), &document.url);
    }

    #[test]
    async fn test_context_item_from_document() {
        let document = create_test_document();
        let context_item = ContextItem::from_document(&document);

        assert_eq!(context_item.context_type, ContextType::Document);
        assert_eq!(context_item.title, document.title);
        assert!(context_item.content.contains(&document.title));
        assert!(context_item.content.contains(&document.body));
        assert_eq!(
            context_item.metadata.get("document_id").unwrap(),
            &document.id
        );
        assert!(context_item.relevance_score.is_some());
    }

    #[test]
    async fn test_context_item_from_search_result() {
        let documents = create_test_documents(3);
        let query = "test search";

        let context_item = ContextItem::from_search_result(query, &documents);

        assert_eq!(context_item.context_type, ContextType::Document);
        assert_eq!(context_item.title, "Search: test search");
        assert!(context_item.content.contains(query));
        assert!(context_item.content.contains("Test Document 0"));
        assert!(context_item.content.contains("Test Document 1"));
        assert!(context_item.content.contains("Test Document 2"));
        assert_eq!(context_item.metadata.get("result_count").unwrap(), "3");
    }

    // Edge case and error handling tests

    #[test]
    async fn test_add_message_to_nonexistent_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let fake_id = ConversationId::from_string("non-existent".to_string());

        let message = ChatMessage::user("Test".to_string());

        let result = manager.add_message(&fake_id, message);
        assert!(result.is_err());
    }

    #[test]
    async fn test_add_context_to_nonexistent_conversation() {
        let mut manager = ContextManager::new(create_test_config());
        let fake_id = ConversationId::from_string("non-existent".to_string());

        let context_item = ContextItem {
            id: "test".to_string(),
            context_type: ContextType::Document,
            title: "Test".to_string(),
            summary: Some("Test summary".to_string()),
            content: "Test content".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };

        let result = manager.add_context(&fake_id, context_item);
        assert!(result.is_err());
    }

    #[test]
    async fn test_conversation_limits() {
        let config = ContextConfig {
            max_conversations_cache: 2,
            max_context_items: 20,
            max_context_length: 50000,
            default_search_results_limit: 5,
            enable_auto_suggestions: false,
        };
        let mut manager = ContextManager::new(config);

        // Create max conversations
        let _conv1 = manager
            .create_conversation("Conv 1".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        let _conv2 = manager
            .create_conversation("Conv 2".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // Creating a third should still work (will remove oldest)
        let conv3 = manager
            .create_conversation("Conv 3".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // Should only have 2 conversations now
        let conversations = manager.list_conversations(None);
        assert_eq!(conversations.len(), 2);

        // Most recent should be available
        let latest = manager.get_conversation(&conv3);
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().title, "Conv 3");
    }

    #[test]
    async fn test_max_context_exceeded() {
        let config = ContextConfig {
            max_conversations_cache: 10,
            max_context_items: 2,
            max_context_length: 50000,
            default_search_results_limit: 5,
            enable_auto_suggestions: false,
        };
        let mut manager = ContextManager::new(config);
        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // Add contexts up to limit
        let context1 = ContextItem {
            id: "ctx1".to_string(),
            context_type: ContextType::Document,
            title: "Context 1".to_string(),
            summary: Some("Summary 1".to_string()),
            content: "Content 1".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };
        let context2 = ContextItem {
            id: "ctx2".to_string(),
            context_type: ContextType::Document,
            title: "Context 2".to_string(),
            summary: Some("Summary 2".to_string()),
            content: "Content 2".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };
        let context3 = ContextItem {
            id: "ctx3".to_string(),
            context_type: ContextType::Document,
            title: "Context 3".to_string(),
            summary: Some("Summary 3".to_string()),
            content: "Content 3".to_string(),
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };

        assert!(manager.add_context(&conversation_id, context1).is_ok());
        assert!(manager.add_context(&conversation_id, context2).is_ok());

        // Third should exceed limit
        let result = manager.add_context(&conversation_id, context3);
        assert!(result.is_err());
    }

    #[test]
    async fn test_max_context_length_exceeded() {
        let config = ContextConfig {
            max_conversations_cache: 10,
            max_context_items: 20,
            max_context_length: 100, // Very small limit
            default_search_results_limit: 5,
            enable_auto_suggestions: false,
        };
        let mut manager = ContextManager::new(config);
        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        let large_context = ContextItem {
            id: "large".to_string(),
            context_type: ContextType::Document,
            title: "Large Context".to_string(),
            summary: Some("Large content summary".to_string()),
            content: "A".repeat(200), // Exceeds the 100 char limit
            metadata: AHashMap::new(),
            created_at: chrono::Utc::now(),
            relevance_score: None,
        };

        let result = manager.add_context(&conversation_id, large_context);
        assert!(result.is_err());
    }

    #[test]
    async fn test_conversation_cache_limits() {
        let config = ContextConfig {
            max_conversations_cache: 3,
            max_context_items: 20,
            max_context_length: 50000,
            default_search_results_limit: 5,
            enable_auto_suggestions: false,
        };
        let mut manager = ContextManager::new(config);

        // Create conversations up to limit
        let conv1 = manager
            .create_conversation("Conv 1".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        let conv2 = manager
            .create_conversation("Conv 2".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        let conv3 = manager
            .create_conversation("Conv 3".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // All conversations should exist
        assert!(manager.get_conversation(&conv1).is_some());
        assert!(manager.get_conversation(&conv2).is_some());
        assert!(manager.get_conversation(&conv3).is_some());

        // Create a 4th conversation (should either work or remove oldest)
        let conv4 = manager
            .create_conversation("Conv 4".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        // The new conversation should exist
        assert!(manager.get_conversation(&conv4).is_some());

        // At most 3 should exist (cache limit), but exact behavior depends on implementation
        let all_conversations = manager.list_conversations(None);
        assert!(all_conversations.len() <= 3);
    }

    #[test]
    async fn test_concurrent_access() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let manager = Arc::new(Mutex::new(ContextManager::new(create_test_config())));
        let mut handles = vec![];

        // Spawn multiple tasks to create conversations concurrently
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let mut mgr = manager_clone.lock().await;
                mgr.create_conversation(format!("Concurrent Conv {}", i), RoleName::new("engineer"))
                    .await
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // Verify all succeeded
        for result in &results {
            assert!(result.is_ok());
        }

        // Verify we have the expected number of conversations
        let mgr = manager.lock().await;
        let conversations = mgr.list_conversations(None);
        assert_eq!(conversations.len(), 10);
    }

    #[test]
    async fn test_default_search_results_limit() {
        let config = ContextConfig {
            max_conversations_cache: 10,
            max_context_items: 20,
            max_context_length: 50000,
            default_search_results_limit: 2, // Limit to 2
            enable_auto_suggestions: false,
        };
        let manager = ContextManager::new(config);
        let documents = create_test_documents(5);

        // Test with no explicit limit (should use default)
        let context_item = manager.create_search_context("test", &documents, None);

        // Should only include 2 documents (the default limit)
        assert!(context_item.content.contains("Test Document 0"));
        assert!(context_item.content.contains("Test Document 1"));
        assert!(!context_item.content.contains("Test Document 2"));
    }

    #[test]
    async fn test_context_metadata_preservation() {
        let mut document = create_test_document();
        document.tags = Some(vec!["rust".to_string(), "test".to_string()]);
        document.rank = Some(42);

        let context_item = ContextItem::from_document(&document);

        assert_eq!(
            context_item.metadata.get("document_id").unwrap(),
            &document.id
        );
        assert_eq!(context_item.metadata.get("url").unwrap(), &document.url);
        assert_eq!(context_item.metadata.get("tags").unwrap(), "rust, test");
        assert_eq!(context_item.metadata.get("rank").unwrap(), "42");
    }

    #[test]
    async fn test_conversation_role_assignment() {
        let mut manager = ContextManager::new(create_test_config());

        // Test different role assignments
        let engineer_conv = manager
            .create_conversation("Engineer".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        let researcher_conv = manager
            .create_conversation("Researcher".to_string(), RoleName::new("researcher"))
            .await
            .unwrap();

        let eng_conversation = manager.get_conversation(&engineer_conv).unwrap();
        let res_conversation = manager.get_conversation(&researcher_conv).unwrap();

        assert_eq!(eng_conversation.role, RoleName::new("engineer"));
        assert_eq!(res_conversation.role, RoleName::new("researcher"));
    }

    #[test]
    async fn test_timestamp_ordering() {
        let mut manager = ContextManager::new(create_test_config());

        // Create conversations with small delays to ensure different timestamps
        let conv1 = manager
            .create_conversation("First".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let conv2 = manager
            .create_conversation("Second".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let conv3 = manager
            .create_conversation("Third".to_string(), RoleName::new("engineer"))
            .await
            .unwrap();

        let conversations = manager.list_conversations(None);

        // Should be ordered by creation time, most recent first
        assert_eq!(conversations[0].id, conv3);
        assert_eq!(conversations[0].title, "Third");
        assert_eq!(conversations[1].id, conv2);
        assert_eq!(conversations[1].title, "Second");
        assert_eq!(conversations[2].id, conv1);
        assert_eq!(conversations[2].title, "First");
    }
}
