/// Context Management Service for LLM Conversations
///
/// This module provides functionality to manage conversation contexts, including:
/// - Context item creation and management
/// - Conversation persistence and retrieval
/// - Context history tracking
/// - Integration with search results and documents
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_types::{
    ChatMessage, ContextItem, Conversation, ConversationId, ConversationSummary, Document,
    MessageId, RoleName,
};

use crate::{Result as ServiceResult, ServiceError};

/// Configuration for the context management service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum number of context items per conversation
    pub max_context_items: usize,
    /// Maximum context length in characters (approximation)
    pub max_context_length: usize,
    /// Maximum number of conversations to keep in memory
    pub max_conversations_cache: usize,
    /// Default number of search results to include as context
    pub default_search_results_limit: usize,
    /// Enable automatic context suggestions based on conversation content
    pub enable_auto_suggestions: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_items: 50,
            max_context_length: 100_000, // ~100K characters
            max_conversations_cache: 100,
            default_search_results_limit: 5,
            enable_auto_suggestions: true,
        }
    }
}

/// Service for managing LLM conversation contexts
pub struct ContextManager {
    config: ContextConfig,
    /// In-memory cache of recent conversations
    conversations_cache: AHashMap<ConversationId, Arc<Conversation>>,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            conversations_cache: AHashMap::new(),
        }
    }

    /// Create a new conversation
    pub async fn create_conversation(
        &mut self,
        title: String,
        role: RoleName,
    ) -> ServiceResult<ConversationId> {
        let conversation = Conversation::new(title, role);
        let id = conversation.id.clone();

        // Add to cache (for now we'll only use in-memory storage)
        self.conversations_cache
            .insert(id.clone(), Arc::new(conversation));

        // Clean cache if needed
        self.clean_cache();

        Ok(id)
    }

    /// Get a conversation by ID
    pub fn get_conversation(&self, id: &ConversationId) -> Option<Arc<Conversation>> {
        // For now, only check cache (in-memory storage)
        self.conversations_cache.get(id).cloned()
    }

    /// List conversation summaries
    pub fn list_conversations(&self, limit: Option<usize>) -> Vec<ConversationSummary> {
        let mut summaries: Vec<ConversationSummary> = self
            .conversations_cache
            .values()
            .map(|c| ConversationSummary::from(&**c))
            .collect();

        // Sort by updated_at descending
        summaries.sort_by_key(|s| std::cmp::Reverse(s.updated_at));

        if let Some(limit) = limit {
            summaries.truncate(limit);
        }

        summaries
    }

    /// Add a message to a conversation
    pub fn add_message(
        &mut self,
        conversation_id: &ConversationId,
        message: ChatMessage,
    ) -> ServiceResult<MessageId> {
        let message_id = message.id.clone();

        // Get conversation from cache
        let conversation = self.get_conversation(conversation_id).ok_or_else(|| {
            ServiceError::Config(format!("Conversation {} not found", conversation_id))
        })?;

        // Create a mutable copy and add message
        let mut updated_conversation = (*conversation).clone();
        updated_conversation.add_message(message);

        // Update cache
        self.conversations_cache
            .insert(conversation_id.clone(), Arc::new(updated_conversation));

        Ok(message_id)
    }

    /// Add context to a conversation
    pub fn add_context(
        &mut self,
        conversation_id: &ConversationId,
        context: ContextItem,
    ) -> ServiceResult<()> {
        let conversation = self.get_conversation(conversation_id).ok_or_else(|| {
            ServiceError::Config(format!("Conversation {} not found", conversation_id))
        })?;

        // Check context limits
        let total_context_count = conversation.global_context.len()
            + conversation
                .messages
                .iter()
                .map(|m| m.context_items.len())
                .sum::<usize>();

        if total_context_count >= self.config.max_context_items {
            return Err(ServiceError::Config(
                "Maximum context items reached for this conversation".to_string(),
            ));
        }

        // Check context length
        let estimated_length = conversation.estimated_context_length() + context.content.len();
        if estimated_length > self.config.max_context_length {
            return Err(ServiceError::Config(
                "Adding this context would exceed maximum context length".to_string(),
            ));
        }

        // Create a mutable copy and add context
        let mut updated_conversation = (*conversation).clone();
        updated_conversation.add_global_context(context);

        // Update cache
        self.conversations_cache
            .insert(conversation_id.clone(), Arc::new(updated_conversation));

        Ok(())
    }

    /// Delete a context item from a conversation
    pub fn delete_context(
        &mut self,
        conversation_id: &ConversationId,
        context_id: &str,
    ) -> ServiceResult<()> {
        let conversation = self.get_conversation(conversation_id).ok_or_else(|| {
            ServiceError::Config(format!("Conversation {} not found", conversation_id))
        })?;

        // Create a mutable copy and remove the context item
        let mut updated_conversation = (*conversation).clone();

        // Find and remove the context item from global_context
        let initial_len = updated_conversation.global_context.len();
        updated_conversation
            .global_context
            .retain(|item| item.id != context_id);

        // Check if item was found and removed
        if updated_conversation.global_context.len() == initial_len {
            return Err(ServiceError::Config(format!(
                "Context item {} not found in conversation {}",
                context_id, conversation_id
            )));
        }

        // Update timestamp
        updated_conversation.updated_at = chrono::Utc::now();

        // Update cache
        self.conversations_cache
            .insert(conversation_id.clone(), Arc::new(updated_conversation));

        Ok(())
    }

    /// Update a context item in a conversation
    pub fn update_context(
        &mut self,
        conversation_id: &ConversationId,
        context_id: &str,
        updated_context: ContextItem,
    ) -> ServiceResult<ContextItem> {
        let conversation = self.get_conversation(conversation_id).ok_or_else(|| {
            ServiceError::Config(format!("Conversation {} not found", conversation_id))
        })?;

        // Create a mutable copy and update the context item
        let mut updated_conversation = (*conversation).clone();

        // Find and update the context item
        let mut found = false;
        for context_item in &mut updated_conversation.global_context {
            if context_item.id == context_id {
                // Preserve the original ID and created_at timestamp
                let original_created_at = context_item.created_at;
                *context_item = updated_context.clone();
                context_item.id = context_id.to_string();
                context_item.created_at = original_created_at;
                found = true;
                break;
            }
        }

        if !found {
            return Err(ServiceError::Config(format!(
                "Context item {} not found in conversation {}",
                context_id, conversation_id
            )));
        }

        // Update conversation timestamp
        updated_conversation.updated_at = chrono::Utc::now();

        // Update cache
        self.conversations_cache
            .insert(conversation_id.clone(), Arc::new(updated_conversation));

        Ok(updated_context)
    }

    /// Create context item from search results
    pub fn create_search_context(
        &self,
        query: &str,
        documents: &[Document],
        limit: Option<usize>,
    ) -> ContextItem {
        let limit_count = limit.unwrap_or(self.config.default_search_results_limit);
        let limited_docs = if documents.len() > limit_count {
            &documents[..limit_count]
        } else {
            documents
        };

        ContextItem::from_search_result(query, limited_docs)
    }

    /// Create context item from a single document
    pub fn create_document_context(&self, document: &Document) -> ContextItem {
        ContextItem::from_document(document)
    }

    /// Create context item directly from context item data (for frontend use)
    pub fn create_context_from_item(&self, mut context_item: ContextItem) -> ContextItem {
        // Generate new ID if empty
        if context_item.id.is_empty() {
            context_item.id = uuid::Uuid::new_v4().to_string();
        }
        // Update timestamp
        context_item.created_at = chrono::Utc::now();
        context_item
    }

    /// Get context suggestions based on conversation content
    pub fn get_context_suggestions(
        &self,
        conversation_id: &ConversationId,
        _limit: usize,
    ) -> Vec<String> {
        if !self.config.enable_auto_suggestions {
            return Vec::new();
        }

        let _conversation = match self.get_conversation(conversation_id) {
            Some(conv) => conv,
            None => return Vec::new(),
        };

        // TODO: Implement intelligent context suggestions based on:
        // - Recent messages in the conversation
        // - Similar conversations
        // - Frequently used context items
        // - Knowledge graph relationships

        Vec::new()
    }

    /// Clean the conversation cache if it exceeds limits
    fn clean_cache(&mut self) {
        if self.conversations_cache.len() > self.config.max_conversations_cache {
            // Remove oldest conversations (this is a simple implementation)
            // In production, you might want to use LRU cache
            let excess = self.conversations_cache.len() - self.config.max_conversations_cache;
            let mut to_remove = Vec::new();

            for (id, conversation) in &self.conversations_cache {
                to_remove.push((id.clone(), conversation.updated_at));
            }

            to_remove.sort_by_key(|(_, updated_at)| *updated_at);

            for (id, _) in to_remove.iter().take(excess) {
                self.conversations_cache.remove(id);
            }
        }
    }
}

/// Build LLM messages with context injection
pub fn build_llm_messages_with_context(
    conversation: &Conversation,
    include_global_context: bool,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();

    // Add global context as system message if requested
    if include_global_context && !conversation.global_context.is_empty() {
        let context_content = conversation
            .global_context
            .iter()
            .map(|ctx| format!("### {}\n{}\n", ctx.title, ctx.content))
            .collect::<Vec<_>>()
            .join("\n");

        let system_message = serde_json::json!({
            "role": "system",
            "content": format!("Context Information:\n{}", context_content)
        });
        messages.push(system_message);
    }

    // Add conversation messages with their context
    for message in &conversation.messages {
        let mut content = message.content.clone();

        // Append message-specific context
        if !message.context_items.is_empty() {
            let message_context = message
                .context_items
                .iter()
                .map(|ctx| format!("\n--- {} ---\n{}", ctx.title, ctx.content))
                .collect::<Vec<_>>()
                .join("\n");
            content.push_str(&message_context);
        }

        let llm_message = serde_json::json!({
            "role": message.role,
            "content": content
        });
        messages.push(llm_message);
    }

    messages
}

// Removed unsafe const_cast_mut_ref function as it's no longer needed

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use terraphim_types::{ContextType, Document};

    #[tokio::test]
    async fn test_conversation_creation() {
        let mut context_manager = ContextManager::new(ContextConfig::default());

        let conversation_id = context_manager
            .create_conversation("Test Conversation".to_string(), RoleName::new("Test"))
            .await
            .unwrap();

        let conversation = context_manager.get_conversation(&conversation_id).unwrap();

        assert_eq!(conversation.title, "Test Conversation");
        assert_eq!(conversation.role.as_str(), "Test");
        assert_eq!(conversation.messages.len(), 0);
    }

    #[tokio::test]
    async fn test_add_message_to_conversation() {
        let mut context_manager = ContextManager::new(ContextConfig::default());

        let conversation_id = context_manager
            .create_conversation("Test Conversation".to_string(), RoleName::new("Test"))
            .await
            .unwrap();

        let message = ChatMessage::user("Hello, world!".to_string());
        let message_id = context_manager
            .add_message(&conversation_id, message)
            .unwrap();

        let conversation = context_manager.get_conversation(&conversation_id).unwrap();

        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].id, message_id);
        assert_eq!(conversation.messages[0].content, "Hello, world!");
        assert_eq!(conversation.messages[0].role, "user");
    }

    #[test]
    fn test_create_document_context() {
        let context_manager = ContextManager::new(ContextConfig::default());

        let document = Document {
            id: "test-doc".to_string(),
            url: "https://example.com".to_string(),
            title: "Test Document".to_string(),
            body: "This is a test document body.".to_string(),
            description: Some("Test description".to_string()),
            summarization: None,
            stub: None,
            tags: Some(vec!["test".to_string(), "document".to_string()]),
            rank: Some(10),
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        let context = context_manager.create_document_context(&document);

        assert_eq!(context.context_type, ContextType::Document);
        assert_eq!(context.title, "Test Document");
        assert!(context.content.contains("Test Document"));
        assert!(context.content.contains("This is a test document body."));
        assert_eq!(context.relevance_score, Some(10.0));
    }

    #[test]
    fn test_create_search_context() {
        let context_manager = ContextManager::new(ContextConfig::default());

        let documents = vec![
            Document {
                id: "doc1".to_string(),
                url: "https://example.com/1".to_string(),
                title: "Document 1".to_string(),
                body: "Content 1".to_string(),
                description: Some("Description 1".to_string()),
                summarization: None,
                stub: None,
                tags: None,
                rank: Some(5),
                source_haystack: None,
                doc_type: terraphim_types::DocumentType::KgEntry,
                synonyms: None,
                route: None,
                priority: None,
            },
            Document {
                id: "doc2".to_string(),
                url: "https://example.com/2".to_string(),
                title: "Document 2".to_string(),
                body: "Content 2".to_string(),
                description: Some("Description 2".to_string()),
                summarization: None,
                stub: None,
                tags: None,
                rank: Some(3),
                source_haystack: None,
                doc_type: terraphim_types::DocumentType::KgEntry,
                synonyms: None,
                route: None,
                priority: None,
            },
        ];

        let context = context_manager.create_search_context("test query", &documents, Some(2));

        assert_eq!(context.context_type, ContextType::Document);
        assert_eq!(context.title, "Search: test query");
        assert!(context.content.contains("test query"));
        assert!(context.content.contains("Document 1"));
        assert!(context.content.contains("Document 2"));
        assert_eq!(context.relevance_score, Some(5.0));
    }

    #[test]
    fn test_build_llm_messages_with_context() {
        let mut conversation = Conversation::new("Test".to_string(), RoleName::new("Test"));

        // Add global context
        let global_context = ContextItem {
            id: "global-1".to_string(),
            context_type: ContextType::System,
            title: "System Info".to_string(),
            summary: Some("System information summary".to_string()),
            content: "This is system information".to_string(),
            metadata: AHashMap::new(),
            created_at: Utc::now(),
            relevance_score: None,
        };
        conversation.add_global_context(global_context);

        // Add a user message with context
        let mut user_message = ChatMessage::user("Hello".to_string());
        let message_context = ContextItem {
            id: "msg-ctx-1".to_string(),
            context_type: ContextType::Document,
            title: "Relevant Doc".to_string(),
            summary: Some("Document summary".to_string()),
            content: "Document content".to_string(),
            metadata: AHashMap::new(),
            created_at: Utc::now(),
            relevance_score: None,
        };
        user_message.add_context(message_context);
        conversation.add_message(user_message);

        let messages = build_llm_messages_with_context(&conversation, true);

        assert_eq!(messages.len(), 2); // System message + user message

        // Check system message with global context
        assert_eq!(messages[0]["role"], "system");
        assert!(messages[0]["content"]
            .as_str()
            .unwrap()
            .contains("This is system information"));

        // Check user message with message context
        assert_eq!(messages[1]["role"], "user");
        let user_content = messages[1]["content"].as_str().unwrap();
        assert!(user_content.contains("Hello"));
        assert!(user_content.contains("Document content"));
    }

    #[tokio::test]
    async fn test_delete_context_real_manager() {
        let mut context_manager = ContextManager::new(ContextConfig::default());

        // Create a real conversation
        let conversation_id = context_manager
            .create_conversation("Test Delete".to_string(), RoleName::new("test"))
            .await
            .unwrap();

        // Create and add a real context item
        let context_item = ContextItem {
            id: "test-context-1".to_string(),
            context_type: ContextType::UserInput,
            title: "Test Context for Deletion".to_string(),
            summary: Some("Test summary".to_string()),
            content: "This is test content for deletion testing.".to_string(),
            metadata: AHashMap::new(),
            created_at: Utc::now(),
            relevance_score: None,
        };

        let context_id = context_item.id.clone();
        context_manager
            .add_context(&conversation_id, context_item)
            .unwrap();

        // Verify context was added
        let conversation = context_manager.get_conversation(&conversation_id).unwrap();
        assert_eq!(conversation.global_context.len(), 1);
        assert_eq!(conversation.global_context[0].id, context_id);

        // Test successful deletion
        let result = context_manager.delete_context(&conversation_id, &context_id);
        assert!(result.is_ok());

        // Verify context was removed
        let updated_conversation = context_manager.get_conversation(&conversation_id).unwrap();
        assert_eq!(updated_conversation.global_context.len(), 0);

        // Test deletion of non-existent context
        let result = context_manager.delete_context(&conversation_id, "non-existent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        // Test deletion from non-existent conversation
        let fake_conv_id = ConversationId::from_string("fake-conversation".to_string());
        let result = context_manager.delete_context(&fake_conv_id, &context_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_update_context_real_manager() {
        let mut context_manager = ContextManager::new(ContextConfig::default());

        // Create a real conversation
        let conversation_id = context_manager
            .create_conversation("Test Update".to_string(), RoleName::new("test"))
            .await
            .unwrap();

        // Create and add a real context item
        let original_context = ContextItem {
            id: "test-context-2".to_string(),
            context_type: ContextType::Document,
            title: "Original Title".to_string(),
            summary: Some("Original summary".to_string()),
            content: "Original content".to_string(),
            metadata: {
                let mut map = AHashMap::new();
                map.insert("original_key".to_string(), "original_value".to_string());
                map
            },
            created_at: Utc::now(),
            relevance_score: Some(85.5),
        };

        let context_id = original_context.id.clone();
        let original_created_at = original_context.created_at;
        context_manager
            .add_context(&conversation_id, original_context)
            .unwrap();

        // Create updated context
        let updated_context = ContextItem {
            id: context_id.clone(),
            context_type: ContextType::UserInput,
            title: "Updated Title".to_string(),
            summary: Some("Updated summary with more details".to_string()),
            content: "Updated content with additional information".to_string(),
            metadata: {
                let mut map = AHashMap::new();
                map.insert("updated_key".to_string(), "updated_value".to_string());
                map.insert("new_key".to_string(), "new_value".to_string());
                map
            },
            created_at: Utc::now(), // This should be preserved
            relevance_score: Some(92.3),
        };

        // Test successful update
        let result =
            context_manager.update_context(&conversation_id, &context_id, updated_context.clone());
        assert!(result.is_ok());

        // Verify context was updated correctly
        let conversation = context_manager.get_conversation(&conversation_id).unwrap();
        assert_eq!(conversation.global_context.len(), 1);

        let updated_item = &conversation.global_context[0];
        assert_eq!(updated_item.id, context_id);
        assert_eq!(updated_item.title, "Updated Title");
        assert_eq!(
            updated_item.summary,
            Some("Updated summary with more details".to_string())
        );
        assert_eq!(
            updated_item.content,
            "Updated content with additional information"
        );
        assert_eq!(updated_item.context_type, ContextType::UserInput);
        assert_eq!(updated_item.relevance_score, Some(92.3));

        // Verify original created_at is preserved
        assert_eq!(updated_item.created_at, original_created_at);

        // Verify metadata was updated
        assert_eq!(
            updated_item.metadata.get("updated_key"),
            Some(&"updated_value".to_string())
        );
        assert_eq!(
            updated_item.metadata.get("new_key"),
            Some(&"new_value".to_string())
        );
        assert!(!updated_item.metadata.contains_key("original_key"));

        // Verify conversation updated_at was updated
        assert!(conversation.updated_at > conversation.created_at);

        // Test update of non-existent context
        let result = context_manager.update_context(
            &conversation_id,
            "non-existent",
            updated_context.clone(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        // Test update in non-existent conversation
        let fake_conv_id = ConversationId::from_string("fake-conversation".to_string());
        let result = context_manager.update_context(&fake_conv_id, &context_id, updated_context);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_context_with_summary_field() {
        let context_manager = ContextManager::new(ContextConfig::default());

        // Test document context with summary
        let document = Document {
            id: "doc-with-summary".to_string(),
            url: "https://example.com/summary".to_string(),
            title: "Document with Summary".to_string(),
            body: "This is a document that should have a summary from its description.".to_string(),
            description: Some(
                "This is the document description that becomes the summary".to_string(),
            ),
            summarization: Some("AI-generated summary of the document".to_string()),
            stub: None,
            tags: Some(vec!["test".to_string(), "summary".to_string()]),
            rank: Some(88),
            source_haystack: None,
            doc_type: terraphim_types::DocumentType::KgEntry,
            synonyms: None,
            route: None,
            priority: None,
        };

        let context_item = context_manager.create_document_context(&document);

        // Verify summary field is populated from description
        assert_eq!(
            context_item.summary,
            Some("This is the document description that becomes the summary".to_string())
        );
        assert_eq!(context_item.title, "Document with Summary");
        assert!(context_item
            .content
            .contains("This is the document description that becomes the summary"));

        // Test search result context with summary
        let documents = vec![document];
        let search_context = context_manager.create_search_context("test query", &documents, None);

        // Verify search context has a summary
        assert!(search_context.summary.is_some());
        assert!(search_context
            .summary
            .as_ref()
            .unwrap()
            .contains("test query"));
        assert!(search_context
            .summary
            .as_ref()
            .unwrap()
            .contains("1 documents found"));
    }

    #[tokio::test]
    async fn test_partial_context_update() {
        let mut context_manager = ContextManager::new(ContextConfig::default());

        let conversation_id = context_manager
            .create_conversation("Test Partial Update".to_string(), RoleName::new("test"))
            .await
            .unwrap();

        // Create original context
        let original_context = ContextItem {
            id: "partial-update-test".to_string(),
            context_type: ContextType::Document,
            title: "Original Title".to_string(),
            summary: Some("Original summary".to_string()),
            content: "Original content".to_string(),
            metadata: {
                let mut map = AHashMap::new();
                map.insert("preserve".to_string(), "this_value".to_string());
                map
            },
            created_at: Utc::now(),
            relevance_score: Some(75.0),
        };

        let context_id = original_context.id.clone();
        let original_created_at = original_context.created_at;
        context_manager
            .add_context(&conversation_id, original_context)
            .unwrap();

        // Update only summary and title, keeping other fields
        let mut partial_update = context_manager
            .get_conversation(&conversation_id)
            .unwrap()
            .global_context[0]
            .clone();

        partial_update.title = "Updated Title Only".to_string();
        partial_update.summary = Some("Updated summary only".to_string());

        let result = context_manager.update_context(&conversation_id, &context_id, partial_update);
        assert!(result.is_ok());

        // Verify only specified fields were updated
        let conversation = context_manager.get_conversation(&conversation_id).unwrap();
        let updated_item = &conversation.global_context[0];

        assert_eq!(updated_item.title, "Updated Title Only");
        assert_eq!(
            updated_item.summary,
            Some("Updated summary only".to_string())
        );
        assert_eq!(updated_item.content, "Original content"); // Should remain unchanged
        assert_eq!(updated_item.context_type, ContextType::Document); // Should remain unchanged
        assert_eq!(updated_item.relevance_score, Some(75.0)); // Should remain unchanged
        assert_eq!(updated_item.created_at, original_created_at); // Should be preserved
        assert_eq!(
            updated_item.metadata.get("preserve"),
            Some(&"this_value".to_string())
        ); // Should be preserved
    }
}
