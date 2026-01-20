#![cfg(feature = "legacy-components")]

use std::sync::Arc;
use terraphim_desktop_gpui::components::{
    ContextComponent, ContextComponentConfig, ContextItemComponent, ContextItemComponentConfig,
    SearchContextBridge, SearchContextBridgeConfig,
};
use terraphim_types::{ContextType, Document};

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
    let context_item = bridge
        .add_document_to_context(document.clone(), &mut cx)
        .await
        .unwrap();

    // Verify context item was created correctly
    assert_eq!(context_item.context_type, ContextType::Document);
    assert_eq!(context_item.title, "Test Document");
    assert!(context_item.content.contains("This is the content"));
}
