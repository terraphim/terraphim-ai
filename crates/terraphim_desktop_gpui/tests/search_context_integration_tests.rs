use gpui::*;
use std::sync::Arc;
use terraphim_desktop_gpui::components::{
    SearchContextBridge, ContextComponent, ContextItemComponent, SearchContextBridgeConfig,
    ContextComponentConfig, ContextItemComponentConfig
};
use terraphim_types::{Document, ContextType};

mod app;

/// Comprehensive tests for search results to context integration
///
/// This test suite validates the complete workflow from search results
/// to context management, ensuring that users can seamlessly add documents,
/// search results, and knowledge graph items to their conversations.
#[tokio::test]
async fn test_search_to_context_basic_workflow() {
    // Create bridge component
    let config = SearchContextBridgeConfig::default();
    let mut bridge = SearchContextBridge::new(config);

    // Initialize
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Create test document
    let document = Arc::new(Document {
        id: Some("test-doc-1".to_string()),
        title: "Test Document".to_string(),
        description: Some("A test document for context integration".to_string()),
        body: "This is the content of the test document. It contains useful information that should be added to context.".to_string(),
        url: "https://example.com/test-doc".to_string(),
        tags: Some(vec!["test".to_string(), "document".to_string()]),
        rank: Some(0.9),
        metadata: ahash::AHashMap::new(),
    });

    // Add document to context
    let context_item = bridge.add_document_to_context(document.clone(), &mut cx).await.unwrap();

    // Verify context item was created correctly
    assert_eq!(context_item.title, "Test Document");
    assert_eq!(context_item.context_type, ContextType::SearchResult);
    assert_eq!(context_item.metadata.get("source").unwrap(), "search");
    assert_eq!(context_item.metadata.get("url").unwrap(), "https://example.com/test-doc");
    assert_eq!(context_item.relevance_score, Some(0.9));

    // Verify bridge state updated
    let stats = bridge.get_stats();
    assert_eq!(stats.items_added, 1);
    assert_eq!(stats.total_contexts, 1);

    // Verify last add result
    let last_result = bridge.get_last_add_result();
    assert!(last_result.is_some());
    assert!(last_result.unwrap().contains("Added 'Test Document' to context"));
}

#[tokio::test]
async fn test_batch_add_documents_to_context() {
    let config = SearchContextBridgeConfig {
        max_batch_size: 5,
        ..Default::default()
    };
    let mut bridge = SearchContextBridge::new(config);
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Create multiple test documents
    let documents: Vec<Arc<Document>> = (1..=3).map(|i| {
        Arc::new(Document {
            id: Some(format!("test-doc-{}", i)),
            title: format!("Test Document {}", i),
            description: Some(format!("Description for document {}", i)),
            body: format!("Content for test document {}", i),
            url: format!("https://example.com/test-doc-{}", i),
            tags: Some(vec![format!("tag-{}", i)]),
            rank: Some(0.8 + (i as f32 * 0.05)),
            metadata: ahash::AHashMap::new(),
        })
    }).collect();

    // Add all documents to context
    let context_items = bridge.add_documents_to_context(documents.clone(), &mut cx).await.unwrap();

    // Verify all documents were added
    assert_eq!(context_items.len(), 3);
    for (i, context_item) in context_items.iter().enumerate() {
        assert_eq!(context_item.title, format!("Test Document {}", i + 1));
        assert_eq!(context_item.context_type, ContextType::SearchResult);
        assert!(context_item.metadata.contains_key("url"));
    }

    // Verify bridge state
    let stats = bridge.get_stats();
    assert_eq!(stats.items_added, 3);
    assert_eq!(stats.total_contexts, 3);
}

#[tokio::test]
async fn test_batch_add_exceeds_limit() {
    let config = SearchContextBridgeConfig {
        max_batch_size: 2,
        ..Default::default()
    };
    let mut bridge = SearchContextBridge::new(config);
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Create more documents than batch limit
    let documents: Vec<Arc<Document>> = (1..=4).map(|i| {
        Arc::new(Document {
            id: Some(format!("test-doc-{}", i)),
            title: format!("Test Document {}", i),
            description: None,
            body: format!("Content {}", i),
            url: format!("https://example.com/test-doc-{}", i),
            tags: None,
            rank: Some(0.8),
            metadata: ahash::AHashMap::new(),
        })
    }).collect();

    // Should fail due to batch size limit
    let result = bridge.add_documents_to_context(documents, &mut cx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot add more than 2 items"));
}

#[tokio::test]
async fn test_chat_with_document_workflow() {
    let config = SearchContextBridgeConfig::default();
    let mut bridge = SearchContextBridge::new(config);
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Test document for chat
    let document = Arc::new(Document {
        id: Some("chat-doc-1".to_string()),
        title: "Chat Document".to_string(),
        description: Some("Document for chat testing".to_string()),
        body: "This document contains information that will be used for chat context.".to_string(),
        url: "https://example.com/chat-doc".to_string(),
        tags: Some(vec!["chat".to_string()]),
        rank: Some(0.95),
        metadata: ahash::AHashMap::new(),
    });

    // Subscribe to chat events
    let mut chat_events = Vec::new();
    let _subscription = bridge.subscribe(&mut cx, |event, _cx| {
        chat_events.push(event.clone());
    });

    // Chat with document
    let context_item = bridge.chat_with_document(document.clone(), &mut cx).await.unwrap();

    // Verify context item created
    assert_eq!(context_item.title, "Chat Document");
    assert_eq!(context_item.context_type, ContextType::SearchResult);

    // Verify chat events were emitted
    assert_eq!(chat_events.len(), 2); // DocumentAdded + ChatWithDocument

    match &chat_events[0] {
        terraphim_desktop_gpui::components::SearchContextBridgeEvent::DocumentAdded { document, context_item: _ } => {
            assert_eq!(document.title, "Chat Document");
        }
        _ => panic!("Expected DocumentAdded event"),
    }

    match &chat_events[1] {
        terraphim_desktop_gpui::components::SearchContextBridgeEvent::ChatWithDocument { document, context_item: _ } => {
            assert_eq!(document.title, "Chat Document");
        }
        _ => panic!("Expected ChatWithDocument event"),
    }
}

#[tokio::test]
async fn test_document_validation() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Test empty title
    let mut document = Arc::new(Document {
        id: Some("empty-title".to_string()),
        title: String::new(),
        description: Some("Document with empty title".to_string()),
        body: "Some content".to_string(),
        url: "https://example.com/empty-title".to_string(),
        tags: None,
        rank: None,
        metadata: ahash::AHashMap::new(),
    });

    let result = bridge.document_to_context_item(document.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("title cannot be empty"));

    // Test empty content
    Arc::get_mut(&mut document).unwrap().title = "Valid Title".to_string();
    Arc::get_mut(&mut document).unwrap().body = String::new();

    let result = bridge.document_to_context_item(document.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("content cannot be empty"));
}

#[test]
fn test_batch_selection_workflow() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Create test documents
    let documents: Vec<Arc<Document>> = (1..=3).map(|i| {
        Arc::new(Document {
            id: Some(format!("batch-doc-{}", i)),
            title: format!("Batch Document {}", i),
            description: None,
            body: format!("Content {}", i),
            url: format!("https://example.com/batch-doc-{}", i),
            tags: None,
            rank: Some(0.8),
            metadata: ahash::AHashMap::new(),
        })
    }).collect();

    // Initially no selection
    assert!(!bridge.state().show_batch_mode);
    assert_eq!(bridge.get_selected_documents().len(), 0);

    // Enable batch mode
    bridge.toggle_batch_mode();
    assert!(bridge.state().show_batch_mode);

    // Select documents
    bridge.toggle_document_selection(documents[0].clone());
    assert_eq!(bridge.get_selected_documents().len(), 1);
    assert!(bridge.is_document_selected(&documents[0]));

    bridge.toggle_document_selection(documents[1].clone());
    assert_eq!(bridge.get_selected_documents().len(), 2);
    assert!(bridge.is_document_selected(&documents[1]));

    // Deselect a document
    bridge.toggle_document_selection(documents[0].clone());
    assert_eq!(bridge.get_selected_documents().len(), 1);
    assert!(!bridge.is_document_selected(&documents[0]));

    // Clear selection
    bridge.clear_selection();
    assert_eq!(bridge.get_selected_documents().len(), 0);

    // Select all documents
    bridge.select_all_documents(&documents);
    assert_eq!(bridge.get_selected_documents().len(), documents.len());

    // Exit batch mode (should clear selection)
    bridge.toggle_batch_mode();
    assert!(!bridge.state().show_batch_mode);
    assert_eq!(bridge.get_selected_documents().len(), 0);
}

#[test]
fn test_context_preview_functionality() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    let document = Arc::new(Document {
        id: Some("preview-doc".to_string()),
        title: "Preview Document".to_string(),
        description: Some("Document for preview testing".to_string()),
        body: "This content will be shown in the preview.".to_string(),
        url: "https://example.com/preview".to_string(),
        tags: None,
        rank: Some(0.75),
        metadata: ahash::AHashMap::new(),
    });

    // Show preview
    let result = bridge.show_context_preview(document.clone());
    assert!(result.is_ok());
    assert!(bridge.state().preview_item.is_some());

    let preview_item = bridge.state().preview_item.as_ref().unwrap();
    assert_eq!(preview_item.title, "Preview Document");
    assert_eq!(preview_item.content, "This content will be shown in the preview.");

    // Hide preview
    bridge.hide_context_preview();
    assert!(bridge.state().preview_item.is_none());
}

#[test]
fn test_suggestions_toggle() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Initially suggestions are disabled
    assert!(!bridge.state().show_suggestions);

    // Enable suggestions
    bridge.toggle_suggestions();
    assert!(bridge.state().show_suggestions);

    // Disable suggestions
    bridge.toggle_suggestions();
    assert!(!bridge.state().show_suggestions);
}

#[test]
fn test_bridge_statistics() {
    let config = SearchContextBridgeConfig {
        max_batch_size: 3,
        ..Default::default()
    };
    let mut bridge = SearchContextBridge::new(config);
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Initial stats
    let stats = bridge.get_stats();
    assert_eq!(stats.items_added, 0);
    assert_eq!(stats.selected_for_batch, 0);
    assert_eq!(stats.max_batch_size, 3);
    assert_eq!(stats.total_contexts, 0);

    // Enable batch mode and select items
    bridge.toggle_batch_mode();

    let documents: Vec<Arc<Document>> = (1..=2).map(|i| {
        Arc::new(Document {
            id: Some(format!("stats-doc-{}", i)),
            title: format!("Stats Document {}", i),
            description: None,
            body: format!("Content {}", i),
            url: format!("https://example.com/stats-doc-{}", i),
            tags: None,
            rank: Some(0.8),
            metadata: ahash::AHashMap::new(),
        })
    }).collect();

    bridge.toggle_document_selection(documents[0].clone());
    bridge.toggle_document_selection(documents[1].clone());

    // Updated stats
    let stats = bridge.get_stats();
    assert_eq!(stats.selected_for_batch, 2);
    assert_eq!(stats.max_batch_size, 3);
}

#[tokio::test]
async fn test_error_handling_workflow() {
    let config = SearchContextBridgeConfig::default();
    let mut bridge = SearchContextBridge::new(config);
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Subscribe to error events
    let mut error_events = Vec::new();
    let _subscription = bridge.subscribe(&mut cx, |event, _cx| {
        if let terraphim_desktop_gpui::components::SearchContextBridgeEvent::OperationFailed { error, .. } = event {
            error_events.push(error.clone());
        }
    });

    // Try to add invalid document (empty title)
    let invalid_document = Arc::new(Document {
        id: Some("invalid-doc".to_string()),
        title: String::new(),
        description: Some("Invalid document".to_string()),
        body: "Content".to_string(),
        url: "https://example.com/invalid".to_string(),
        tags: None,
        rank: None,
        metadata: ahash::AHashMap::new(),
    });

    let result = bridge.add_document_to_context(invalid_document, &mut cx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("title cannot be empty"));
}

#[test]
fn test_context_component_integration() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();
    bridge.initialize(&mut cx).unwrap();

    // Get access to the context component
    let context_component = bridge.get_context_component();
    assert_eq!(context_component.get_items().len(), 0);

    // Get mutable access to context component for direct testing
    let context_component_mut = bridge.get_context_component_mut();

    // Add a context item directly to the component
    let test_item = terraphim_types::ContextItem {
        id: "direct-item-1".to_string(),
        title: "Direct Context Item".to_string(),
        summary: Some("Added directly to context component".to_string()),
        content: "This content was added directly to the context component for testing integration.".to_string(),
        context_type: ContextType::Document,
        created_at: chrono::Utc::now(),
        relevance_score: Some(0.9),
        metadata: ahash::AHashMap::new(),
    };

    context_component_mut.add_item(test_item).unwrap();

    // Verify the item was added
    assert_eq!(context_component_mut.get_items().len(), 1);
    assert_eq!(context_component_mut.get_items()[0].title, "Direct Context Item");
}

#[test]
fn test_performance_tracking() {
    let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
    let mut cx = gpui::TestAppContext::new();

    // Initialize should track performance
    bridge.initialize(&mut cx).unwrap();

    // Get performance stats
    let stats = bridge.get_stats();
    assert!(stats.performance_stats.operation_count > 0);
    assert!(stats.performance_stats.total_operations_time.as_millis() >= 0);
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integration test for the complete search-to-context workflow
    /// This test simulates a real user workflow:
    /// 1. User searches for documents
    /// 2. User selects multiple documents for batch operations
    /// 3. User adds selected documents to context
    /// 4. User chats with one of the documents
    #[tokio::test]
    async fn test_complete_user_workflow() {
        let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
        let mut cx = gpui::TestAppContext::new();
        bridge.initialize(&mut cx).unwrap();

        // Simulate search results
        let search_results: Vec<Arc<Document>> = vec![
            Arc::new(Document {
                id: Some("search-1".to_string()),
                title: "Rust Programming Guide".to_string(),
                description: Some("Comprehensive guide to Rust programming".to_string()),
                body: "Rust is a systems programming language that runs blazingly fast...".to_string(),
                url: "https://example.com/rust-guide".to_string(),
                tags: Some(vec!["rust".to_string(), "programming".to_string()]),
                rank: Some(0.95),
                metadata: ahash::AHashMap::new(),
            }),
            Arc::new(Document {
                id: Some("search-2".to_string()),
                title: "GPUI Framework Documentation".to_string(),
                description: Some("Documentation for the GPUI framework".to_string()),
                body: "GPUI is a cross-platform UI framework for Rust...".to_string(),
                url: "https://example.com/gpui-docs".to_string(),
                tags: Some(vec!["gpui".to_string(), "ui".to_string()]),
                rank: Some(0.92),
                metadata: ahash::AHashMap::new(),
            }),
            Arc::new(Document {
                id: Some("search-3".to_string()),
                title: "Context Management Best Practices".to_string(),
                description: Some("Best practices for managing conversation context".to_string()),
                body: "Effective context management is crucial for AI conversations...".to_string(),
                url: "https://example.com/context-best-practices".to_string(),
                tags: Some(vec!["context".to_string(), "ai".to_string()]),
                rank: Some(0.88),
                metadata: ahash::AHashMap::new(),
            }),
        ];

        // User enables batch mode
        bridge.toggle_batch_mode();
        assert!(bridge.state().show_batch_mode);

        // User selects first two documents for batch operations
        bridge.toggle_document_selection(search_results[0].clone());
        bridge.toggle_document_selection(search_results[1].clone());
        assert_eq!(bridge.get_selected_documents().len(), 2);

        // User adds selected documents to context
        let context_items = bridge.add_documents_to_context(
            bridge.get_selected_documents().to_vec(),
            &mut cx
        ).await.unwrap();
        assert_eq!(context_items.len(), 2);

        // Verify context items were created with proper metadata
        for (i, context_item) in context_items.iter().enumerate() {
            assert_eq!(context_item.context_type, ContextType::SearchResult);
            assert_eq!(context_item.metadata.get("source").unwrap(), "search");
            assert!(context_item.metadata.contains_key("tags"));
        }

        // User now wants to chat with the third document
        let chat_context_item = bridge.chat_with_document(search_results[2].clone(), &mut cx).await.unwrap();
        assert_eq!(chat_context_item.title, "Context Management Best Practices");

        // Verify final state
        let stats = bridge.get_stats();
        assert_eq!(stats.items_added, 3); // 2 from batch + 1 from chat
        assert_eq!(stats.total_contexts, 3);

        // User clears batch mode
        bridge.toggle_batch_mode();
        assert!(!bridge.state().show_batch_mode);
        assert_eq!(bridge.get_selected_documents().len(), 0);
    }
}