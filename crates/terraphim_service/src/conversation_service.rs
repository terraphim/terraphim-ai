use std::sync::Arc;
use terraphim_persistence::conversation::{
    ConversationPersistence, OpenDALConversationPersistence,
};
use terraphim_types::{Conversation, ConversationId, ConversationSummary, RoleName};
use tokio::sync::Mutex;

use crate::ServiceError as Error;

type Result<T> = std::result::Result<T, Error>;

/// Filter criteria for listing conversations
#[derive(Debug, Clone, Default)]
pub struct ConversationFilter {
    /// Filter by role name
    pub role: Option<RoleName>,
    /// Filter by date range (start)
    pub date_start: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by date range (end)
    pub date_end: Option<chrono::DateTime<chrono::Utc>>,
    /// Search query for title/content
    pub search_query: Option<String>,
    /// Show archived conversations
    pub show_archived: bool,
    /// Limit number of results
    pub limit: Option<usize>,
}

/// Statistics about conversations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationStatistics {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_context_items: usize,
    pub conversations_by_role: std::collections::HashMap<String, usize>,
    pub average_messages_per_conversation: f64,
}

/// Service for managing conversations
pub struct ConversationService {
    persistence: Arc<Mutex<dyn ConversationPersistence>>,
}

impl ConversationService {
    /// Create a new conversation service with default OpenDAL persistence
    pub fn new() -> Self {
        let persistence = OpenDALConversationPersistence::new();
        Self {
            persistence: Arc::new(Mutex::new(persistence)),
        }
    }

    /// Create a new conversation service with custom persistence
    pub fn with_persistence(persistence: Arc<Mutex<dyn ConversationPersistence>>) -> Self {
        Self { persistence }
    }

    /// Create a new conversation
    pub async fn create_conversation(&self, title: String, role: RoleName) -> Result<Conversation> {
        log::info!("Creating new conversation: '{}' for role '{}'", title, role);

        let conversation = Conversation::new(title, role);
        let persistence = self.persistence.lock().await;
        persistence
            .save(&conversation)
            .await
            .map_err(Error::Persistence)?;

        log::info!("Created conversation: {}", conversation.id.as_str());
        Ok(conversation)
    }

    /// Get a conversation by ID
    pub async fn get_conversation(&self, id: &ConversationId) -> Result<Conversation> {
        log::debug!("Getting conversation: {}", id.as_str());

        let persistence = self.persistence.lock().await;
        persistence.load(id).await.map_err(Error::Persistence)
    }

    /// Update an existing conversation
    pub async fn update_conversation(&self, conversation: Conversation) -> Result<Conversation> {
        log::debug!("Updating conversation: {}", conversation.id.as_str());

        let persistence = self.persistence.lock().await;
        persistence
            .save(&conversation)
            .await
            .map_err(Error::Persistence)?;

        Ok(conversation)
    }

    /// Delete a conversation
    pub async fn delete_conversation(&self, id: &ConversationId) -> Result<()> {
        log::info!("Deleting conversation: {}", id.as_str());

        let persistence = self.persistence.lock().await;
        persistence.delete(id).await.map_err(Error::Persistence)
    }

    /// List conversations with optional filtering
    pub async fn list_conversations(
        &self,
        filter: ConversationFilter,
    ) -> Result<Vec<ConversationSummary>> {
        log::debug!("Listing conversations with filter: {:?}", filter);

        let persistence = self.persistence.lock().await;
        let mut summaries = persistence
            .list_summaries()
            .await
            .map_err(Error::Persistence)?;

        // Apply filters
        if let Some(ref role) = filter.role {
            summaries.retain(|s| s.role == *role);
        }

        if let Some(start) = filter.date_start {
            summaries.retain(|s| s.updated_at >= start);
        }

        if let Some(end) = filter.date_end {
            summaries.retain(|s| s.updated_at <= end);
        }

        if let Some(ref query) = filter.search_query {
            let query_lower = query.to_lowercase();
            summaries.retain(|s| {
                s.title.to_lowercase().contains(&query_lower)
                    || s.preview
                        .as_ref()
                        .is_some_and(|p| p.to_lowercase().contains(&query_lower))
            });
        }

        // Apply limit
        if let Some(limit) = filter.limit {
            summaries.truncate(limit);
        }

        log::debug!("Found {} conversations after filtering", summaries.len());
        Ok(summaries)
    }

    /// Search conversations by content
    pub async fn search_conversations(&self, query: &str) -> Result<Vec<ConversationSummary>> {
        log::debug!("Searching conversations for: '{}'", query);

        let filter = ConversationFilter {
            search_query: Some(query.to_string()),
            ..Default::default()
        };

        let results = self.list_conversations(filter).await?;

        log::debug!("Found {} search results", results.len());
        Ok(results)
    }

    /// Export a conversation to JSON
    pub async fn export_conversation(&self, id: &ConversationId) -> Result<String> {
        log::info!("Exporting conversation: {}", id.as_str());

        let conversation = self.get_conversation(id).await?;
        serde_json::to_string_pretty(&conversation)
            .map_err(|e| Error::Config(format!("Failed to serialize conversation: {}", e)))
    }

    /// Import a conversation from JSON
    pub async fn import_conversation(&self, json_data: &str) -> Result<Conversation> {
        log::info!("Importing conversation from JSON");

        let conversation: Conversation = serde_json::from_str(json_data)
            .map_err(|e| Error::Config(format!("Failed to deserialize conversation: {}", e)))?;

        let persistence = self.persistence.lock().await;
        persistence
            .save(&conversation)
            .await
            .map_err(Error::Persistence)?;

        log::info!("Imported conversation: {}", conversation.id.as_str());
        Ok(conversation)
    }

    /// Get conversation statistics
    pub async fn get_statistics(&self) -> Result<ConversationStatistics> {
        log::debug!("Calculating conversation statistics");

        let persistence = self.persistence.lock().await;
        let summaries = persistence
            .list_summaries()
            .await
            .map_err(Error::Persistence)?;

        let total_conversations = summaries.len();
        let total_messages: usize = summaries.iter().map(|s| s.message_count).sum();
        let total_context_items: usize = summaries.iter().map(|s| s.context_count).sum();

        let mut conversations_by_role = std::collections::HashMap::new();
        for summary in &summaries {
            *conversations_by_role
                .entry(summary.role.as_str().to_string())
                .or_insert(0) += 1;
        }

        let average_messages_per_conversation = if total_conversations > 0 {
            total_messages as f64 / total_conversations as f64
        } else {
            0.0
        };

        let stats = ConversationStatistics {
            total_conversations,
            total_messages,
            total_context_items,
            conversations_by_role,
            average_messages_per_conversation,
        };

        log::debug!(
            "Statistics: {} conversations, {} messages",
            stats.total_conversations,
            stats.total_messages
        );
        Ok(stats)
    }
}

impl Default for ConversationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_persistence::DeviceStorage;
    use terraphim_types::ChatMessage;

    #[tokio::test]
    async fn test_create_and_get_conversation() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let service = ConversationService::new();
        let conversation = service
            .create_conversation("Test".to_string(), RoleName::new("Test Role"))
            .await
            .unwrap();

        let loaded = service.get_conversation(&conversation.id).await.unwrap();
        assert_eq!(loaded.id, conversation.id);
        assert_eq!(loaded.title, "Test");
    }

    #[tokio::test]
    #[ignore = "Flaky due to shared state pollution between tests - needs test isolation fix"]
    async fn test_list_and_filter_conversations() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let service = ConversationService::new();

        // Create conversations with different roles
        service
            .create_conversation("Test 1".to_string(), RoleName::new("Role A"))
            .await
            .unwrap();
        service
            .create_conversation("Test 2".to_string(), RoleName::new("Role B"))
            .await
            .unwrap();
        service
            .create_conversation("Test 3".to_string(), RoleName::new("Role A"))
            .await
            .unwrap();

        // List all
        let all = service
            .list_conversations(ConversationFilter::default())
            .await
            .unwrap();
        assert_eq!(all.len(), 3);

        // Filter by role
        let filtered = service
            .list_conversations(ConversationFilter {
                role: Some(RoleName::new("Role A")),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[tokio::test]
    async fn test_search_conversations() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let service = ConversationService::new();

        service
            .create_conversation("Machine Learning".to_string(), RoleName::new("Test"))
            .await
            .unwrap();
        service
            .create_conversation("Rust Programming".to_string(), RoleName::new("Test"))
            .await
            .unwrap();

        let results = service.search_conversations("machine").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Machine Learning");
    }

    #[tokio::test]
    async fn test_export_import_conversation() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let service = ConversationService::new();
        let mut conversation = service
            .create_conversation("Export Test".to_string(), RoleName::new("Test"))
            .await
            .unwrap();

        conversation.add_message(ChatMessage::user("Test message".to_string()));
        let conversation = service.update_conversation(conversation).await.unwrap();

        // Export
        let json = service.export_conversation(&conversation.id).await.unwrap();
        assert!(json.contains("Export Test"));
        assert!(json.contains("Test message"));

        // Delete original
        service.delete_conversation(&conversation.id).await.unwrap();

        // Import
        let imported = service.import_conversation(&json).await.unwrap();
        assert_eq!(imported.title, "Export Test");
        assert_eq!(imported.messages.len(), 1);
    }

    #[tokio::test]
    #[ignore = "Flaky due to shared state pollution between tests - needs test isolation fix"]
    async fn test_get_statistics() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let service = ConversationService::new();

        let mut conv1 = service
            .create_conversation("Test 1".to_string(), RoleName::new("Role A"))
            .await
            .unwrap();
        conv1.add_message(ChatMessage::user("Message 1".to_string()));
        conv1.add_message(ChatMessage::assistant("Response 1".to_string(), None));
        service.update_conversation(conv1).await.unwrap();

        let mut conv2 = service
            .create_conversation("Test 2".to_string(), RoleName::new("Role B"))
            .await
            .unwrap();
        conv2.add_message(ChatMessage::user("Message 2".to_string()));
        service.update_conversation(conv2).await.unwrap();

        let stats = service.get_statistics().await.unwrap();
        assert_eq!(stats.total_conversations, 2);
        assert_eq!(stats.total_messages, 3);
        assert_eq!(stats.conversations_by_role.len(), 2);
    }
}
