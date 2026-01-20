//! Test utilities for Terraphim Desktop GPUI testing
//!
//! This module provides common utilities, mock services, and test helpers
//! for comprehensive testing of the GPUI desktop application.

use gpui::*;
use terraphim_types::{ContextItem, ContextType, ChatMessage, ConversationId, Document, RoleName};
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use chrono::Utc;

/// Create a test ContextItem with default values
pub fn create_test_context_item(id: &str, title: &str) -> ContextItem {
    ContextItem {
        id: id.to_string(),
        title: title.to_string(),
        summary: Some(format!("Summary for {}", title)),
        content: format!("Content for {}", title),
        context_type: ContextType::Document,
        created_at: Utc::now(),
        relevance_score: Some(0.8),
        metadata: ahash::AHashMap::new(),
    }
}

/// Create a test ContextItem with custom parameters
pub fn create_context_item_with_params(
    id: &str,
    title: &str,
    content: &str,
    context_type: ContextType,
) -> ContextItem {
    ContextItem {
        id: id.to_string(),
        title: title.to_string(),
        summary: Some(format!("Summary for {}", title)),
        content: content.to_string(),
        context_type,
        created_at: Utc::now(),
        relevance_score: Some(0.9),
        metadata: {
            let mut meta = ahash::AHashMap::new();
            meta.insert("test_key".to_string(), "test_value".to_string());
            meta
        },
    }
}

/// Create a test Document with default values
pub fn create_test_document(id: &str, title: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{}", id),
        title: title.to_string(),
        description: Some(format!("Description for {}", title)),
        body: format!("Body content for {}", title),
        tags: Some(vec!["test".to_string(), "document".to_string()]),
        rank: Some(0.85),
    }
}

/// Create a test ChatMessage
pub fn create_test_chat_message(role: &str, content: &str) -> ChatMessage {
    match role {
        "user" => ChatMessage::user(content.to_string()),
        "assistant" => ChatMessage::assistant(content.to_string()),
        "system" => ChatMessage::system(content.to_string()),
        _ => ChatMessage::user(content.to_string()),
    }
}

/// Create a test ConversationId
pub fn create_test_conversation_id() -> ConversationId {
    ConversationId::new()
}

/// Create a test RoleName
pub fn create_test_role_name(name: &str) -> RoleName {
    RoleName::from(name)
}

/// Mock SearchService for testing
pub struct MockSearchService {
    pub results: Vec<Document>,
    pub should_error: bool,
    pub delay_ms: u64,
}

impl MockSearchService {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            should_error: false,
            delay_ms: 0,
        }
    }

    pub fn with_results(mut self, results: Vec<Document>) -> Self {
        self.results = results;
        self
    }

    pub fn with_error(mut self) -> Self {
        self.should_error = true;
        self
    }

    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Document>, String> {
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        if self.should_error {
            return Err("Mock search error".to_string());
        }

        Ok(self.results.clone())
    }
}

/// Test environment setup
pub struct TestEnvironment {
    pub window: gpui::Window,
    pub context: Context<()>,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            window: gpui::test::Window::default(),
            context: gpui::test::Context::default(),
        }
    }
}

/// Async test helper that runs a test with a real async runtime
#[cfg(test)]
pub async fn run_async_test<F, Fut>(test: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    tokio::test::test(test).await;
}

/// Helper to create multiple test documents
pub fn create_multiple_test_documents(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| create_test_document(&format!("doc_{}", i), &format!("Document {}", i)))
        .collect()
}

/// Helper to create multiple context items
pub fn create_multiple_context_items(count: usize) -> Vec<ContextItem> {
    (0..count)
        .map(|i| create_test_context_item(&format!("ctx_{}", i), &format!("Context Item {}", i)))
        .collect()
}

/// Mock ContextManager for testing
pub struct MockContextManager {
    pub items: Vec<ContextItem>,
    pub conversations: ahash::AHashMap<ConversationId, Vec<ContextItem>>,
}

impl MockContextManager {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            conversations: ahash::AHashMap::new(),
        }
    }

    pub fn add_item(&mut self, item: ContextItem) {
        self.items.push(item);
    }

    pub fn remove_item(&mut self, id: &str) {
        self.items.retain(|item| item.id != id);
    }

    pub fn get_item(&self, id: &str) -> Option<&ContextItem> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn add_to_conversation(&mut self, conversation_id: &ConversationId, item: ContextItem) {
        self.conversations
            .entry(conversation_id.clone())
            .or_insert_with(Vec::new)
            .push(item);
    }

    pub fn get_conversation_items(&self, conversation_id: &ConversationId) -> Vec<&ContextItem> {
        self.conversations
            .get(conversation_id)
            .map(|items| items.iter().collect())
            .unwrap_or_default()
    }
}

/// Performance measurement utilities
pub struct PerformanceTimer {
    start: std::time::Instant,
    name: String,
}

impl PerformanceTimer {
    pub fn new(name: &str) -> Self {
        log::info!("Starting performance measurement: {}", name);
        Self {
            start: std::time::Instant::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }
}

impl Drop for PerformanceTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        log::info!(
            "Performance measurement '{}' completed: {:?}",
            self.name,
            elapsed
        );
    }
}

/// Assertion helpers for testing
pub mod assertions {
    use super::*;

    /// Assert that a ContextItem has the expected properties
    pub fn assert_context_item(
        item: &ContextItem,
        expected_id: &str,
        expected_title: &str,
        expected_type: ContextType,
    ) {
        assert_eq!(item.id, expected_id);
        assert_eq!(item.title, expected_title);
        assert_eq!(item.context_type, expected_type);
        assert!(!item.content.is_empty());
    }

    /// Assert that a Document has the expected properties
    pub fn assert_document(doc: &Document, expected_id: &str, expected_title: &str) {
        assert_eq!(doc.id, expected_id);
        assert_eq!(doc.title, expected_title);
        assert!(!doc.body.is_empty());
    }

    /// Assert that two ContextItems are different
    pub fn assert_different_context_items(item1: &ContextItem, item2: &ContextItem) {
        assert_ne!(item1.id, item2.id);
        assert_ne!(item1.title, item2.title);
    }

    /// Assert that a vector contains a specific ContextItem by ID
    pub fn assert_contains_context_item(items: &[ContextItem], id: &str) {
        assert!(
            items.iter().any(|item| item.id == id),
            "Expected to find context item with ID '{}'",
            id
        );
    }

    /// Assert that a vector does not contain a specific ContextItem by ID
    pub fn assert_not_contains_context_item(items: &[ContextItem], id: &str) {
        assert!(
            !items.iter().any(|item| item.id == id),
            "Did not expect to find context item with ID '{}'",
            id
        );
    }
}

/// Test data generators for different scenarios
pub mod generators {
    use super::*;

    /// Generate context items with various types
    pub fn generate_context_items_mixed_types(count: usize) -> Vec<ContextItem> {
        let types = [ContextType::Document, ContextType::Note, ContextType::Code];

        (0..count)
            .map(|i| {
                let context_type = types[i % types.len()];
                create_context_item_with_params(
                    &format!("ctx_{}", i),
                    &format!("Item {} ({:?})", i, context_type),
                    &format!("Content for item {} with type {:?}", i, context_type),
                    context_type,
                )
            })
            .collect()
    }

    /// Generate documents with varying ranks
    pub fn generate_documents_varying_ranks(count: usize) -> Vec<Document> {
        (0..count)
            .map(|i| {
                let mut doc = create_test_document(&format!("doc_{}", i), &format!("Document {}", i));
                doc.rank = Some(1.0 - (i as f64 / count as f64)); // Descending ranks
                doc
            })
            .collect()
    }

    /// Generate chat messages for a conversation
    pub fn generate_chat_conversation(message_count: usize) -> Vec<ChatMessage> {
        let roles = ["user", "assistant", "user", "assistant"];

        (0..message_count)
            .map(|i| {
                let role = roles[i % roles.len()];
                create_test_chat_message(role, &format!("Message {} from {}", i, role))
            })
            .collect()
    }
}

/// Mock service builders
pub mod builders {
    use super::*;

    pub struct ConfigStateBuilder {
        roles: ahash::AHashMap<RoleName, Arc<TokioMutex<terraphim_rolegraph::RoleGraph>>>,
        selected_role: RoleName,
    }

    impl ConfigStateBuilder {
        pub fn new() -> Self {
            Self {
                roles: ahash::AHashMap::new(),
                selected_role: RoleName::from("default"),
            }
        }

        pub fn with_role(mut self, role: RoleName, rolegraph: terraphim_rolegraph::RoleGraph) -> Self {
            self.roles.insert(role, Arc::new(TokioMutex::new(rolegraph)));
            self
        }

        pub fn with_selected_role(mut self, role: RoleName) -> Self {
            self.selected_role = role;
            self
        }

        pub fn build(self) -> ConfigState {
            // Note: This is a simplified builder
            // Actual ConfigState construction is more complex and requires async setup
            // This is used primarily for testing purposes
            unimplemented!("ConfigState construction requires async setup")
        }
    }
}

/// Cleanup utilities
pub mod cleanup {
    use super::*;

    /// Clean up test resources
    pub fn cleanup_test_resources() {
        // Log cleanup for debugging
        log::info!("Cleaning up test resources");

        // In a real test environment, we might:
        // - Clear caches
        // - Reset global state
        // - Close connections
        // - Clean up temporary files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_context_item() {
        let item = create_test_context_item("test_id", "Test Title");

        assertions::assert_context_item(&item, "test_id", "Test Title", ContextType::Document);
        assert!(item.summary.is_some());
        assert_eq!(item.relevance_score, Some(0.8));
    }

    #[test]
    fn test_create_test_document() {
        let doc = create_test_document("doc_id", "Test Document");

        assertions::assert_document(&doc, "doc_id", "Test Document");
        assert_eq!(doc.url, "https://example.com/doc_id");
        assert!(doc.tags.is_some());
        assert_eq!(doc.rank, Some(0.85));
    }

    #[test]
    fn test_create_test_chat_message_user() {
        let msg = create_test_chat_message("user", "Hello");

        assert!(matches!(msg, ChatMessage::User(_)));
    }

    #[test]
    fn test_create_test_chat_message_assistant() {
        let msg = create_test_chat_message("assistant", "Hello");

        assert!(matches!(msg, ChatMessage::Assistant(_)));
    }

    #[test]
    fn test_mock_search_service() {
        let results = vec![create_test_document("1", "Doc 1")];
        let service = MockSearchService::new()
            .with_results(results.clone())
            .with_delay(10);

        assert_eq!(service.results.len(), 1);
        assert_eq!(service.delay_ms, 10);
        assert!(!service.should_error);
    }

    #[test]
    fn test_mock_context_manager() {
        let mut manager = MockContextManager::new();
        let item = create_test_context_item("1", "Test");

        manager.add_item(item.clone());
        assert_eq!(manager.items.len(), 1);

        let retrieved = manager.get_item("1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "1");

        manager.remove_item("1");
        assert_eq!(manager.items.len(), 0);
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_timer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed();

        assert!(elapsed >= std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_create_multiple_documents() {
        let docs = create_multiple_test_documents(5);

        assert_eq!(docs.len(), 5);
        for (i, doc) in docs.iter().enumerate() {
            assertions::assert_document(doc, &format!("doc_{}", i), &format!("Document {}", i));
        }
    }

    #[test]
    fn test_create_multiple_context_items() {
        let items = create_multiple_context_items(3);

        assert_eq!(items.len(), 3);
        assert_different_context_items(&items[0], &items[1]);
        assert_different_context_items(&items[1], &items[2]);
    }

    #[test]
    fn test_generators_mixed_types() {
        let items = generators::generate_context_items_mixed_types(10);

        assert_eq!(items.len(), 10);
        let types: Vec<_> = items.iter().map(|item| item.context_type).collect();
        assert!(types.contains(&ContextType::Document));
        assert!(types.contains(&ContextType::Note));
        assert!(types.contains(&ContextType::Code));
    }

    #[test]
    fn test_generators_varying_ranks() {
        let docs = generators::generate_documents_varying_ranks(5);

        assert_eq!(docs.len(), 5);
        assert!(docs[0].rank.unwrap() > docs[1].rank.unwrap());
        assert!(docs[1].rank.unwrap() > docs[2].rank.unwrap());
    }

    #[test]
    fn test_generators_chat_conversation() {
        let messages = generators::generate_chat_conversation(8);

        assert_eq!(messages.len(), 8);
        // Should alternate between user and assistant
        assert!(matches!(messages[0], ChatMessage::User(_)));
        assert!(matches!(messages[1], ChatMessage::Assistant(_)));
        assert!(matches!(messages[2], ChatMessage::User(_)));
        assert!(matches!(messages[3], ChatMessage::Assistant(_)));
    }

    #[test]
    fn test_assertions_contains() {
        let items = vec![
            create_test_context_item("1", "Item 1"),
            create_test_context_item("2", "Item 2"),
            create_test_context_item("3", "Item 3"),
        ];

        assertions::assert_contains_context_item(&items, "2");
        assertions::assert_not_contains_context_item(&items, "4");
    }

    #[test]
    fn test_conversation_id_uniqueness() {
        let id1 = create_test_conversation_id();
        let id2 = create_test_conversation_id();

        assert_ne!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_role_name_creation() {
        let role = create_test_role_name("test_role");

        assert_eq!(role.to_string(), "test_role");
    }
}
