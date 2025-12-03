//! Comprehensive UI Integration Tests
//!
//! End-to-end tests that validate the complete UI integration and workflows
//! across all standardized ReusableComponent implementations.

use std::sync::Arc;
use std::time::Duration;

use gpui::*;
use terraphim_types::{ContextItem, ContextType, Document, RoleName};
use ahash::AHashMap;

use terraphim_desktop_gpui::{
    components::{
        ContextComponent, ContextItemComponent, SearchContextBridge, EnhancedChatComponent,
        AddDocumentModal, ComponentConfig, PerformanceTracker, ServiceRegistry,
    },
    views::search::{SearchComponent, SearchComponentConfig},
};

/// Mock service registry for testing
#[derive(Debug, Default)]
struct MockServiceRegistry {
    services: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl MockServiceRegistry {
    fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }

    fn register_service<T: std::any::Any + Send + Sync + 'static>(&mut self, name: &str, service: T) {
        self.services.insert(name.to_string(), Box::new(service));
    }

    fn get_service<T: std::any::Any + Send + Sync + 'static>(&self, name: &str) -> Option<&T> {
        self.services.get(name)?.downcast_ref()
    }
}

/// Test utilities for creating test data
mod test_utils {
    use super::*;

    pub fn create_test_context_item(id: &str, title: &str, content: &str) -> Arc<ContextItem> {
        Arc::new(ContextItem {
            id: id.to_string(),
            title: title.to_string(),
            summary: Some(format!("Summary for {}", title)),
            content: content.to_string(),
            context_type: ContextType::Document,
            created_at: chrono::Utc::now(),
            relevance_score: Some(0.8),
            metadata: AHashMap::new(),
        })
    }

    pub fn create_test_document(id: &str, title: &str, content: &str) -> Document {
        Document {
            id: id.to_string(),
            url: format!("https://example.com/{}", id),
            body: content.to_string(),
            description: Some(format!("Document: {}", title)),
            tags: vec!["test".to_string()],
            rank: Some(1.0),
        }
    }

    pub fn create_mock_registry() -> MockServiceRegistry {
        MockServiceRegistry::new()
    }
}

#[cfg(test)]
mod ui_integration_tests {
    use super::*;

    /// Test complete context management workflow
    #[tokio::test]
    async fn test_complete_context_management_workflow() {
        // Initialize context component
        let config = terraphim_desktop_gpui::components::ContextComponentConfig::default();
        let mut context_component = ContextComponent::new(config);

        // Initialize service registry
        let mut registry = MockServiceRegistry::new();

        // Test context creation
        let test_item = test_utils::create_test_context_item("1", "Test Document", "Test content");
        context_component.add_item(Arc::try_unwrap(test_item.clone())).unwrap();

        // Validate state
        assert_eq!(context_component.get_items().len(), 1);
        assert_eq!(context_component.get_items()[0].title, "Test Document");

        // Test context filtering
        context_component.set_search_query("Test".to_string());
        let filtered_items = context_component.get_filtered_items();
        assert_eq!(filtered_items.len(), 1);

        // Test selection workflow
        context_component.toggle_selection("1");
        assert_eq!(context_component.get_selected_items().len(), 1);

        // Test batch operations
        context_component.select_all();
        assert_eq!(context_component.get_selected_items().len(), 1);

        // Test deletion
        context_component.remove_item("1").unwrap();
        assert_eq!(context_component.get_items().len(), 0);
        assert_eq!(context_component.get_selected_items().len(), 0);

        // Validate performance metrics
        let metrics = context_component.performance_metrics();
        assert!(metrics.get_operation_count("toggle_selection") > 0);
        assert!(metrics.get_operation_count("remove_item") > 0);
    }

    /// Test context item editing workflow
    #[tokio::test]
    async fn test_context_item_editing_workflow() {
        let config = terraphim_desktop_gpui::components::ContextItemComponentConfig::default();
        let mut item_component = ContextItemComponent::new(config);

        // Set up test item
        let test_item = test_utils::create_test_context_item("1", "Original Title", "Original content");
        item_component.set_item(Arc::new(test_item));

        // Test editing initiation
        item_component.start_editing().unwrap();
        assert_eq!(item_component.state().editing_mode,
                  terraphim_desktop_gpui::components::EditingMode::Edit);

        // Test field updates
        item_component.set_edit_title("Updated Title".to_string());
        item_component.set_edit_content("Updated content".to_string());
        item_component.set_edit_relevance(Some(0.9));

        // Test save
        item_component.save_edits(&mut gpui::Context::new(|_| {})).unwrap();
        assert_eq!(item_component.state().editing_mode,
                  terraphim_desktop_gpui::components::EditingMode::View);

        // Validate updated item
        let updated_item = item_component.get_item().unwrap();
        assert_eq!(updated_item.title, "Updated Title");
        assert_eq!(updated_item.content, "Updated content");
        assert_eq!(updated_item.relevance_score, Some(0.9));
    }

    /// Test search-to-context bridge workflow
    #[tokio::test]
    async fn test_search_context_bridge_workflow() {
        let config = terraphim_desktop_gpui::components::SearchContextBridgeConfig::default();
        let mut bridge = SearchContextBridge::new(config);

        // Create test documents
        let doc1 = test_utils::create_test_document("1", "Research Paper", "AI research content");
        let doc2 = test_utils::create_test_document("2", "Tutorial", "Learning material");

        // Test single document addition
        bridge.add_document_to_context(Arc::new(doc1.clone())).await.unwrap();
        assert_eq!(bridge.state().added_contexts.len(), 1);

        // Test batch mode
        bridge.toggle_batch_mode();
        bridge.add_document_to_batch(Arc::new(doc2.clone()));
        assert_eq!(bridge.get_batch_count(), 1);

        // Test batch processing
        bridge.process_batch().await.unwrap();
        assert_eq!(bridge.state().added_contexts.len(), 2);
        assert_eq!(bridge.get_batch_count(), 0);

        // Test statistics
        let stats = bridge.get_stats();
        assert_eq!(stats.items_added, 2);
        assert_eq!(stats.selected_for_batch, 0);
    }

    /// Test enhanced chat component workflow
    #[tokio::test]
    async fn test_enhanced_chat_workflow() {
        let config = EnhancedChatComponent::default();
        let mut chat_component = EnhancedChatComponent::new(config);

        // Test conversation setup
        let conversation_id = terraphim_types::ConversationId::new();
        chat_component.set_conversation(conversation_id);

        // Test message addition
        let message = terraphim_types::ChatMessage {
            id: "msg1".to_string(),
            conversation_id: conversation_id,
            role: terraphim_types::MessageRole::User,
            content: "Hello, AI assistant".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: AHashMap::new(),
        };

        chat_component.add_message(message);
        assert_eq!(chat_component.state().messages.len(), 1);

        // Test context integration
        let context_item = test_utils::create_test_context_item("1", "Context Doc", "Context content");
        chat_component.add_context_item(Arc::new(context_item));
        assert_eq!(chat_component.state().context_items.len(), 1);

        // Test context search
        chat_component.search_context("Context".to_string());
        assert!(chat_component.state().context_search_query.contains("Context"));

        // Test typing indicators
        chat_component.start_typing("user1".to_string());
        assert!(chat_component.state().is_typing);
        assert!(chat_component.state().typing_users.contains(&"user1".to_string()));

        chat_component.stop_typing("user1".to_string());
        assert!(!chat_component.state().is_typing);
    }

    /// Test add document modal workflow
    #[tokio::test]
    async fn test_add_document_modal_workflow() {
        let config = AddDocumentModal::default();
        let mut modal = AddDocumentModal::new(config);

        // Test modal open/close
        modal.open(&mut gpui::Context::new(|_| {}));
        assert!(modal.state().is_open);

        modal.close(&mut gpui::Context::new(|_| {}));
        assert!(!modal.state().is_open);

        // Test text input method
        modal.set_input_method(terraphim_desktop_gpui::components::DocumentInputMethod::TextInput);
        modal.set_title("Test Document".to_string());
        modal.set_content("Test content".to_string());

        // Test validation
        assert_eq!(modal.state().title, "Test Document");
        assert_eq!(modal.state().content, "Test content");

        // Test URL input method
        modal.set_input_method(terraphim_desktop_gpui::components::DocumentInputMethod::UrlImport);
        modal.set_url("https://example.com/document.pdf".to_string());

        // Test file upload simulation
        modal.set_input_method(terraphim_desktop_gpui::components::DocumentInputMethod::FileUpload);
        modal.set_selected_file("test.pdf".to_string());
        modal.set_file_content(Some("File content".to_string()));

        // Test metadata
        modal.add_tag("test".to_string());
        modal.set_custom_metadata("key".to_string(), "value".to_string());

        assert!(modal.state().tags.contains(&"test".to_string()));
        assert_eq!(modal.state().custom_metadata.get("key"), Some(&"value".to_string()));
    }

    /// Test component lifecycle management
    #[tokio::test]
    async fn test_component_lifecycle_management() {
        // Test context component lifecycle
        let config = terraphim_desktop_gpui::components::ContextComponentConfig::default();
        let mut context_component = ContextComponent::new(config);

        assert!(!context_component.is_mounted());
        context_component.mount(&mut gpui::Context::new(|_| {})).unwrap();
        assert!(context_component.is_mounted());
        context_component.unmount(&mut gpui::Context::new(|_| {})).unwrap();
        assert!(!context_component.is_mounted());

        // Test context item component lifecycle
        let config = terraphim_desktop_gpui::components::ContextItemComponentConfig::default();
        let mut item_component = ContextItemComponent::new(config);

        assert!(!item_component.is_mounted());
        item_component.mount(&mut gpui::Context::new(|_| {})).unwrap();
        assert!(item_component.is_mounted());

        // Test performance tracking across lifecycle
        let metrics_before = item_component.performance_metrics().get_total_operations();
        item_component.mount(&mut gpui::Context::new(|_| {})).unwrap();
        let metrics_after = item_component.performance_metrics().get_total_operations();
        assert!(metrics_after > metrics_before);
    }

    /// Test inter-component communication
    #[tokio::test]
    async fn test_inter_component_communication() {
        let config = SearchContextBridgeConfig::default();
        let mut bridge = SearchContextBridge::new(config);

        // Test event subscription
        let mut event_received = false;
        bridge.on_event(|event, _| {
            match event {
                terraphim_desktop_gpui::components::SearchContextBridgeEvent::DocumentAdded { .. } => {
                    event_received = true;
                }
                _ => {}
            }
        });

        // Add document to trigger event
        let doc = test_utils::create_test_document("1", "Test", "Content");
        bridge.add_document_to_context(Arc::new(doc)).await.unwrap();

        assert!(event_received);
    }

    /// Test error handling and recovery
    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        let config = ContextItemComponentConfig::default();
        let mut item_component = ContextItemComponent::new(config);

        // Test empty content validation
        let empty_item = ContextItem {
            id: "1".to_string(),
            title: "".to_string(),
            content: "".to_string(),
            context_type: ContextType::Document,
            created_at: chrono::Utc::now(),
            relevance_score: None,
            metadata: AHashMap::new(),
        };

        item_component.set_item(Arc::new(empty_item));

        // Try to save empty content - should fail validation
        let result = item_component.save_edits(&mut gpui::Context::new(|_| {}));
        assert!(result.is_err());
        assert!(item_component.state().content_error.is_some());

        // Fix content and try again
        item_component.set_edit_content("Valid content".to_string());
        let result = item_component.save_edits(&mut gpui::Context::new(|_| {}));
        assert!(result.is_ok());
        assert!(item_component.state().content_error.is_none());
    }

    /// Test performance characteristics
    #[tokio::test]
    async fn test_performance_characteristics() {
        let config = terraphim_desktop_gpui::components::ContextComponentConfig::default();
        let mut context_component = ContextComponent::new(config);

        // Add many items to test performance
        let start_time = std::time::Instant::now();

        for i in 0..100 {
            let item = test_utils::create_test_context_item(
                &i.to_string(),
                &format!("Document {}", i),
                &format!("Content for document {}", i)
            );
            context_component.add_item(Arc::try_unwrap(item)).unwrap();
        }

        let add_time = start_time.elapsed();
        assert!(add_time < Duration::from_millis(1000)); // Should add 100 items in <1s

        // Test search performance
        let start_time = std::time::Instant::now();
        context_component.set_search_query("Document".to_string());
        let _results = context_component.get_filtered_items();
        let search_time = start_time.elapsed();
        assert!(search_time < Duration::from_millis(100)); // Search should be fast

        // Validate performance metrics
        let metrics = context_component.performance_metrics();
        assert!(metrics.get_operation_count("add_item") == 100);
        assert!(metrics.get_operation_count("set_search_query") > 0);
    }
}

/// Integration test scenarios
#[cfg(test)]
mod integration_scenarios {
    use super::*;

    /// Test complete user workflow from search to chat
    #[tokio::test]
    async fn test_complete_user_workflow() {
        // 1. Initialize all components
        let mut search_component = SearchComponent::new(SearchComponentConfig::default());
        let mut bridge = SearchContextBridge::new(SearchContextBridgeConfig::default());
        let mut chat_component = EnhancedChatComponent::new(EnhancedChatComponent::default());

        // 2. Simulate user searching for documents
        let search_results = vec![
            test_utils::create_test_document("1", "AI Research", "AI content"),
            test_utils::create_test_document("2", "Machine Learning", "ML content"),
        ];

        // 3. User adds documents to context
        for doc in search_results {
            bridge.add_document_to_context(Arc::new(doc)).await.unwrap();
        }

        assert_eq!(bridge.state().added_contexts.len(), 2);

        // 4. User starts chat with enhanced context
        let conversation_id = terraphim_types::ConversationId::new();
        chat_component.set_conversation(conversation_id);

        // 5. Add context items to chat
        for context_item in bridge.state().added_contexts.clone() {
            chat_component.add_context_item(context_item);
        }

        assert_eq!(chat_component.state().context_items.len(), 2);

        // 6. Send message and verify context integration
        let message = terraphim_types::ChatMessage {
            id: "msg1".to_string(),
            conversation_id,
            role: terraphim_types::MessageRole::User,
            content: "Summarize the research documents".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: AHashMap::new(),
        };

        chat_component.add_message(message);
        assert_eq!(chat_component.state().messages.len(), 1);
        assert_eq!(chat_component.state().context_items.len(), 2);
    }

    /// Test memory management and cleanup
    #[tokio::test]
    async fn test_memory_management_and_cleanup() {
        let config = terraphim_desktop_gpui::components::ContextComponentConfig::default();
        let mut context_component = ContextComponent::new(config);

        // Add many items
        for i in 0..1000 {
            let item = test_utils::create_test_context_item(
                &i.to_string(),
                &format!("Large Document {}", i),
                &"x".repeat(1000) // 1KB content each
            );
            context_component.add_item(Arc::try_unwrap(item)).unwrap();
        }

        // Verify items added
        assert_eq!(context_component.get_items().len(), 1000);

        // Test cleanup
        context_component.cleanup().unwrap();
        assert_eq!(context_component.get_items().len(), 0);

        // Verify performance tracker cleanup
        context_component.reset_performance_metrics();
        let metrics = context_component.performance_metrics();
        assert_eq!(metrics.get_total_operations(), 0);
    }
}