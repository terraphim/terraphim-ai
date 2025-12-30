use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use terraphim_types::{Conversation, ConversationId, ConversationSummary};

use crate::{DeviceStorage, Error, Result};

/// Trait for conversation persistence operations
#[async_trait]
pub trait ConversationPersistence: Send + Sync {
    /// Save a conversation to storage
    async fn save(&self, conversation: &Conversation) -> Result<()>;

    /// Load a conversation by ID
    async fn load(&self, id: &ConversationId) -> Result<Conversation>;

    /// Delete a conversation by ID
    async fn delete(&self, id: &ConversationId) -> Result<()>;

    /// List all conversation IDs
    async fn list_ids(&self) -> Result<Vec<ConversationId>>;

    /// Check if a conversation exists
    async fn exists(&self, id: &ConversationId) -> Result<bool>;

    /// List conversations with summaries
    async fn list_summaries(&self) -> Result<Vec<ConversationSummary>>;
}

/// Index structure for fast conversation lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationIndex {
    /// Version of the index format
    pub version: String,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Map of conversation ID to summary
    pub conversations: HashMap<String, ConversationSummary>,
}

impl ConversationIndex {
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            updated_at: chrono::Utc::now(),
            conversations: HashMap::new(),
        }
    }

    pub fn add(&mut self, summary: ConversationSummary) {
        self.conversations
            .insert(summary.id.as_str().to_string(), summary);
        self.updated_at = chrono::Utc::now();
    }

    pub fn remove(&mut self, id: &ConversationId) {
        self.conversations.remove(id.as_str());
        self.updated_at = chrono::Utc::now();
    }

    pub fn get(&self, id: &ConversationId) -> Option<&ConversationSummary> {
        self.conversations.get(id.as_str())
    }

    pub fn list(&self) -> Vec<ConversationSummary> {
        self.conversations.values().cloned().collect()
    }
}

impl Default for ConversationIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenDAL-based conversation persistence implementation
pub struct OpenDALConversationPersistence {
    /// Cached index for fast lookups
    index_cache: tokio::sync::RwLock<Option<ConversationIndex>>,
}

impl OpenDALConversationPersistence {
    pub fn new() -> Self {
        Self {
            index_cache: tokio::sync::RwLock::new(None),
        }
    }

    /// Get the storage key for a conversation
    fn conversation_key(id: &ConversationId) -> String {
        format!("conversations/{}.json", id.as_str())
    }

    /// Get the storage key for the index
    fn index_key() -> String {
        "conversations/index.json".to_string()
    }

    /// Load the index from storage
    async fn load_index(&self) -> Result<ConversationIndex> {
        // Check cache first
        {
            let cache = self.index_cache.read().await;
            if let Some(ref index) = *cache {
                return Ok(index.clone());
            }
        }

        // Load from storage
        let storage = DeviceStorage::instance().await?;
        let key = Self::index_key();

        match storage.fastest_op.read(&key).await {
            Ok(data) => {
                let index: ConversationIndex = serde_json::from_slice(&data.to_vec())
                    .map_err(|e| Error::Serde(e.to_string()))?;

                // Update cache
                {
                    let mut cache = self.index_cache.write().await;
                    *cache = Some(index.clone());
                }

                Ok(index)
            }
            Err(_) => {
                // Index doesn't exist yet, create new one
                let index = ConversationIndex::new();
                self.save_index(&index).await?;
                Ok(index)
            }
        }
    }

    /// Save the index to storage
    async fn save_index(&self, index: &ConversationIndex) -> Result<()> {
        let storage = DeviceStorage::instance().await?;
        let key = Self::index_key();
        let json = serde_json::to_string(index).map_err(|e| Error::Serde(e.to_string()))?;

        // Save to all operators
        for (op, _time) in storage.ops.values() {
            op.write(&key, json.clone())
                .await
                .map_err(|e| Error::OpenDal(Box::new(e)))?;
        }

        // Update cache
        {
            let mut cache = self.index_cache.write().await;
            *cache = Some(index.clone());
        }

        Ok(())
    }

    /// Update the index with a conversation summary
    async fn update_index(&self, conversation: &Conversation) -> Result<()> {
        let mut index = self.load_index().await?;
        let summary = ConversationSummary::from(conversation);
        index.add(summary);
        self.save_index(&index).await
    }

    /// Remove a conversation from the index
    async fn remove_from_index(&self, id: &ConversationId) -> Result<()> {
        let mut index = self.load_index().await?;
        index.remove(id);
        self.save_index(&index).await
    }
}

impl Default for OpenDALConversationPersistence {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConversationPersistence for OpenDALConversationPersistence {
    async fn save(&self, conversation: &Conversation) -> Result<()> {
        log::debug!("Saving conversation: {}", conversation.id.as_str());

        let storage = DeviceStorage::instance().await?;
        let key = Self::conversation_key(&conversation.id);
        let json = serde_json::to_string(conversation).map_err(|e| Error::Serde(e.to_string()))?;

        // Save to all operators
        for (op, _time) in storage.ops.values() {
            op.write(&key, json.clone())
                .await
                .map_err(|e| Error::OpenDal(Box::new(e)))?;
        }

        // Update index
        self.update_index(conversation).await?;

        log::debug!(
            "Successfully saved conversation: {}",
            conversation.id.as_str()
        );
        Ok(())
    }

    async fn load(&self, id: &ConversationId) -> Result<Conversation> {
        log::debug!("Loading conversation: {}", id.as_str());

        let storage = DeviceStorage::instance().await?;
        let key = Self::conversation_key(id);

        // Load from fastest operator
        let data = storage
            .fastest_op
            .read(&key)
            .await
            .map_err(|e| Error::OpenDal(Box::new(e)))?;

        let conversation: Conversation =
            serde_json::from_slice(&data.to_vec()).map_err(|e| Error::Serde(e.to_string()))?;

        log::debug!("Successfully loaded conversation: {}", id.as_str());
        Ok(conversation)
    }

    async fn delete(&self, id: &ConversationId) -> Result<()> {
        log::debug!("Deleting conversation: {}", id.as_str());

        let storage = DeviceStorage::instance().await?;
        let key = Self::conversation_key(id);

        // Delete from all operators
        for (op, _time) in storage.ops.values() {
            // Ignore errors if file doesn't exist
            let _ = op.delete(&key).await;
        }

        // Remove from index
        self.remove_from_index(id).await?;

        log::debug!("Successfully deleted conversation: {}", id.as_str());
        Ok(())
    }

    async fn list_ids(&self) -> Result<Vec<ConversationId>> {
        log::debug!("Listing conversation IDs");

        let index = self.load_index().await?;
        let ids = index
            .conversations
            .keys()
            .map(|k| ConversationId::from_string(k.clone()))
            .collect();

        log::debug!("Found {} conversations", index.conversations.len());
        Ok(ids)
    }

    async fn exists(&self, id: &ConversationId) -> Result<bool> {
        let index = self.load_index().await?;
        Ok(index.get(id).is_some())
    }

    async fn list_summaries(&self) -> Result<Vec<ConversationSummary>> {
        log::debug!("Listing conversation summaries");

        let index = self.load_index().await?;
        let mut summaries = index.list();

        // Sort by updated_at descending (most recent first)
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        log::debug!("Found {} conversation summaries", summaries.len());
        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use terraphim_types::{ChatMessage, RoleName};

    #[tokio::test]
    async fn test_conversation_index() {
        let mut index = ConversationIndex::new();
        assert_eq!(index.conversations.len(), 0);

        let conversation =
            Conversation::new("Test Conversation".to_string(), RoleName::new("Test Role"));
        let summary = ConversationSummary::from(&conversation);

        index.add(summary.clone());
        assert_eq!(index.conversations.len(), 1);
        assert!(index.get(&conversation.id).is_some());

        index.remove(&conversation.id);
        assert_eq!(index.conversations.len(), 0);
        assert!(index.get(&conversation.id).is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_conversation_persistence_save_and_load() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let persistence = OpenDALConversationPersistence::new();
        let mut conversation =
            Conversation::new("Test Conversation".to_string(), RoleName::new("Test Role"));

        // Add a test message
        conversation.add_message(ChatMessage::user("Hello, world!".to_string()));

        // Save
        persistence.save(&conversation).await.unwrap();

        // Load
        let loaded = persistence.load(&conversation.id).await.unwrap();
        assert_eq!(loaded.id, conversation.id);
        assert_eq!(loaded.title, conversation.title);
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Hello, world!");
    }

    #[tokio::test]
    #[serial]
    async fn test_conversation_persistence_list() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let persistence = OpenDALConversationPersistence::new();

        // Clean up any existing conversations first
        let existing = persistence.list_ids().await.unwrap();
        for id in existing {
            let _ = persistence.delete(&id).await;
        }

        // Create multiple conversations
        for i in 0..3 {
            let conversation = Conversation::new(
                format!("Test Conversation {}", i),
                RoleName::new("Test Role"),
            );
            persistence.save(&conversation).await.unwrap();
        }

        // List
        let summaries = persistence.list_summaries().await.unwrap();
        assert_eq!(summaries.len(), 3);
    }

    #[tokio::test]
    #[serial]
    async fn test_conversation_persistence_delete() {
        // Initialize memory-only storage for testing
        let _ = DeviceStorage::init_memory_only().await.unwrap();

        let persistence = OpenDALConversationPersistence::new();
        let conversation =
            Conversation::new("Test Conversation".to_string(), RoleName::new("Test Role"));

        // Save
        persistence.save(&conversation).await.unwrap();
        assert!(persistence.exists(&conversation.id).await.unwrap());

        // Delete
        persistence.delete(&conversation.id).await.unwrap();
        assert!(!persistence.exists(&conversation.id).await.unwrap());
    }
}
