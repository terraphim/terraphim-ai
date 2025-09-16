//! Context management for agents

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use crate::{AgentId, MultiAgentError, MultiAgentResult};

/// A single item in the agent's context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique identifier for this context item
    pub id: Uuid,
    /// Type of context item
    pub item_type: ContextItemType,
    /// Content of the item
    pub content: String,
    /// Metadata about the item
    pub metadata: ContextMetadata,
    /// Token count for this item
    pub token_count: u64,
    /// Relevance score (0.0 - 1.0)
    pub relevance_score: f64,
    /// When this item was added to context
    pub added_at: DateTime<Utc>,
}

impl ContextItem {
    pub fn new(
        item_type: ContextItemType,
        content: String,
        token_count: u64,
        relevance_score: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            item_type,
            content,
            metadata: ContextMetadata::default(),
            token_count,
            relevance_score,
            added_at: Utc::now(),
        }
    }

    pub fn with_metadata(mut self, metadata: ContextMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Types of context items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextItemType {
    /// System message or prompt
    System,
    /// User message
    User,
    /// Agent response
    Assistant,
    /// Tool/function call result
    Tool,
    /// Document content
    Document,
    /// Knowledge graph concept
    Concept,
    /// Memory item
    Memory,
    /// Task description
    Task,
    /// Lesson learned
    Lesson,
}

/// Metadata for context items
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMetadata {
    /// Source of the context item
    pub source: Option<String>,
    /// Document ID if applicable
    pub document_id: Option<String>,
    /// Concept IDs from knowledge graph
    pub concept_ids: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Quality score
    pub quality_score: Option<f64>,
    /// Whether this item is pinned (won't be removed)
    pub pinned: bool,
}

/// Agent context manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Agent this context belongs to
    pub agent_id: AgentId,
    /// Current context window
    pub items: VecDeque<ContextItem>,
    /// Maximum tokens allowed in context
    pub max_tokens: u64,
    /// Current token count
    pub current_tokens: u64,
    /// Maximum items to keep
    pub max_items: usize,
    /// Context window strategy
    pub strategy: ContextStrategy,
    /// When context was last updated
    pub last_updated: DateTime<Utc>,
}

impl AgentContext {
    pub fn new(agent_id: AgentId, max_tokens: u64, max_items: usize) -> Self {
        Self {
            agent_id,
            items: VecDeque::new(),
            max_tokens,
            current_tokens: 0,
            max_items,
            strategy: ContextStrategy::RelevanceFirst,
            last_updated: Utc::now(),
        }
    }

    /// Add an item to the context
    pub fn add_item(&mut self, item: ContextItem) -> MultiAgentResult<()> {
        // Check if adding this item would exceed token limit
        if self.current_tokens + item.token_count > self.max_tokens {
            self.make_space(item.token_count)?;
        }

        self.current_tokens += item.token_count;
        self.items.push_back(item);
        self.last_updated = Utc::now();

        // Enforce max items limit
        if self.items.len() > self.max_items {
            self.apply_strategy()?;
        }

        Ok(())
    }

    /// Add multiple items at once
    pub fn add_items(&mut self, items: Vec<ContextItem>) -> MultiAgentResult<()> {
        for item in items {
            self.add_item(item)?;
        }
        Ok(())
    }

    /// Remove an item by ID
    pub fn remove_item(&mut self, item_id: Uuid) -> MultiAgentResult<ContextItem> {
        let position = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .ok_or_else(|| MultiAgentError::ContextError(format!("Item {} not found", item_id)))?;

        let item = self.items.remove(position).unwrap();
        self.current_tokens -= item.token_count;
        self.last_updated = Utc::now();

        Ok(item)
    }

    /// Clear all non-pinned items
    pub fn clear(&mut self) {
        let pinned_items: VecDeque<ContextItem> = self
            .items
            .drain(..)
            .filter(|item| item.metadata.pinned)
            .collect();

        self.current_tokens = pinned_items.iter().map(|item| item.token_count).sum();
        self.items = pinned_items;
        self.last_updated = Utc::now();
    }

    /// Get items by type
    pub fn get_items_by_type(&self, item_type: ContextItemType) -> Vec<&ContextItem> {
        self.items
            .iter()
            .filter(|item| item.item_type == item_type)
            .collect()
    }

    /// Get most relevant items up to token limit
    pub fn get_relevant_items(&self, max_tokens: u64) -> Vec<&ContextItem> {
        let mut items: Vec<&ContextItem> = self.items.iter().collect();
        items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        let mut selected_items = Vec::new();
        let mut token_count = 0;

        for item in items {
            if token_count + item.token_count <= max_tokens {
                selected_items.push(item);
                token_count += item.token_count;
            }
        }

        selected_items
    }

    /// Get items by relevance threshold with optional limit
    pub fn get_items_by_relevance(
        &self,
        threshold: f64,
        limit: Option<usize>,
    ) -> Vec<&ContextItem> {
        let mut items: Vec<&ContextItem> = self
            .items
            .iter()
            .filter(|item| item.relevance_score >= threshold)
            .collect();

        items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

        if let Some(limit) = limit {
            items.truncate(limit);
        }

        items
    }

    /// Format context for LLM consumption
    pub fn format_for_llm(&self) -> String {
        let mut formatted = String::new();

        for item in &self.items {
            match item.item_type {
                ContextItemType::System => {
                    formatted.push_str(&format!("System: {}\n\n", item.content));
                }
                ContextItemType::User => {
                    formatted.push_str(&format!("User: {}\n\n", item.content));
                }
                ContextItemType::Assistant => {
                    formatted.push_str(&format!("Assistant: {}\n\n", item.content));
                }
                ContextItemType::Document => {
                    formatted.push_str(&format!("Document: {}\n\n", item.content));
                }
                ContextItemType::Concept => {
                    formatted.push_str(&format!("Concept: {}\n\n", item.content));
                }
                ContextItemType::Memory => {
                    formatted.push_str(&format!("Memory: {}\n\n", item.content));
                }
                ContextItemType::Task => {
                    formatted.push_str(&format!("Task: {}\n\n", item.content));
                }
                ContextItemType::Lesson => {
                    formatted.push_str(&format!("Lesson: {}\n\n", item.content));
                }
                ContextItemType::Tool => {
                    formatted.push_str(&format!("Tool Result: {}\n\n", item.content));
                }
            }
        }

        formatted
    }

    /// Make space for new content by removing items
    fn make_space(&mut self, needed_tokens: u64) -> MultiAgentResult<()> {
        let mut tokens_to_free =
            needed_tokens.saturating_sub(self.max_tokens - self.current_tokens);

        while tokens_to_free > 0 && !self.items.is_empty() {
            // Find the least relevant, non-pinned item
            let (index, _) = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| !item.metadata.pinned)
                .min_by(|(_, a), (_, b)| a.relevance_score.partial_cmp(&b.relevance_score).unwrap())
                .ok_or_else(|| {
                    MultiAgentError::ContextError("No removable items found".to_string())
                })?;

            let removed_item = self.items.remove(index).unwrap();
            self.current_tokens -= removed_item.token_count;
            tokens_to_free = tokens_to_free.saturating_sub(removed_item.token_count);
        }

        Ok(())
    }

    /// Apply context management strategy
    fn apply_strategy(&mut self) -> MultiAgentResult<()> {
        match self.strategy {
            ContextStrategy::RelevanceFirst => {
                // Keep highest relevance items
                let mut items: Vec<ContextItem> = self.items.drain(..).collect();
                items.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

                self.items = items.into_iter().take(self.max_items).collect();
            }
            ContextStrategy::ChronologicalRecent => {
                // Keep most recent items
                while self.items.len() > self.max_items {
                    if let Some(item) = self.items.pop_front() {
                        if !item.metadata.pinned {
                            self.current_tokens -= item.token_count;
                        } else {
                            // If it's pinned, put it back and remove something else
                            self.items.push_front(item);
                            // Find first non-pinned item to remove
                            let pos = self.items.iter().position(|item| !item.metadata.pinned);
                            if let Some(pos) = pos {
                                let removed = self.items.remove(pos).unwrap();
                                self.current_tokens -= removed.token_count;
                            } else {
                                break; // All items are pinned
                            }
                        }
                    }
                }
            }
            ContextStrategy::Balanced => {
                // Keep a mix of relevant and recent items
                self.apply_balanced_strategy()?;
            }
        }

        // Recalculate token count
        self.current_tokens = self.items.iter().map(|item| item.token_count).sum();
        self.last_updated = Utc::now();

        Ok(())
    }

    fn apply_balanced_strategy(&mut self) -> MultiAgentResult<()> {
        if self.items.len() <= self.max_items {
            return Ok(());
        }

        let items: Vec<ContextItem> = self.items.drain(..).collect();

        // Separate pinned and non-pinned items
        let pinned: Vec<ContextItem> = items
            .iter()
            .filter(|item| item.metadata.pinned)
            .cloned()
            .collect();
        let mut non_pinned: Vec<ContextItem> = items
            .into_iter()
            .filter(|item| !item.metadata.pinned)
            .collect();

        // Calculate how many non-pinned items we can keep
        let available_slots = self.max_items.saturating_sub(pinned.len());

        if non_pinned.len() <= available_slots {
            // All items fit
            self.items = pinned.into_iter().chain(non_pinned.into_iter()).collect();
        } else {
            // Need to select items - 70% by relevance, 30% by recency
            non_pinned.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
            let relevance_count = (available_slots as f64 * 0.7) as usize;
            let recency_count = available_slots - relevance_count;

            let mut selected = Vec::new();

            // Take top relevance items
            selected.extend(non_pinned.iter().take(relevance_count).cloned());

            // Take most recent of remaining items
            let remaining: Vec<ContextItem> =
                non_pinned.into_iter().skip(relevance_count).collect();
            let mut recent = remaining;
            recent.sort_by(|a, b| b.added_at.cmp(&a.added_at));
            selected.extend(recent.into_iter().take(recency_count));

            self.items = pinned.into_iter().chain(selected.into_iter()).collect();
        }

        Ok(())
    }
}

/// Strategy for managing context when limits are reached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextStrategy {
    /// Keep highest relevance items
    RelevanceFirst,
    /// Keep most recent items
    ChronologicalRecent,
    /// Balanced approach - mix of relevant and recent
    Balanced,
}

/// Context snapshot for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Snapshot ID
    pub id: Uuid,
    /// Agent ID
    pub agent_id: AgentId,
    /// Timestamp of snapshot
    pub timestamp: DateTime<Utc>,
    /// Context items at this point
    pub items: Vec<ContextItem>,
    /// Total token count
    pub token_count: u64,
    /// Trigger for this snapshot
    pub trigger: SnapshotTrigger,
}

impl ContextSnapshot {
    pub fn from_context(context: &AgentContext, trigger: SnapshotTrigger) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id: context.agent_id,
            timestamp: Utc::now(),
            items: context.items.iter().cloned().collect(),
            token_count: context.current_tokens,
            trigger,
        }
    }
}

/// Triggers for context snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotTrigger {
    /// Manual snapshot
    Manual,
    /// Before major context change
    PreChange,
    /// After task completion
    TaskComplete,
    /// Periodic backup
    Periodic,
    /// Before context cleanup
    PreCleanup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_item_creation() {
        let item = ContextItem::new(ContextItemType::User, "Hello world".to_string(), 10, 0.8);

        assert_eq!(item.item_type, ContextItemType::User);
        assert_eq!(item.content, "Hello world");
        assert_eq!(item.token_count, 10);
        assert_eq!(item.relevance_score, 0.8);
    }

    #[test]
    fn test_agent_context() {
        let agent_id = AgentId::new_v4();
        let mut context = AgentContext::new(agent_id, 100, 10);

        let item = ContextItem::new(ContextItemType::User, "Test message".to_string(), 20, 0.9);

        context.add_item(item).unwrap();

        assert_eq!(context.items.len(), 1);
        assert_eq!(context.current_tokens, 20);
    }

    #[test]
    fn test_context_token_limit() {
        let agent_id = AgentId::new_v4();
        let mut context = AgentContext::new(agent_id, 50, 10);

        // Add item that uses most of the context
        let item1 = ContextItem::new(ContextItemType::User, "Large message".to_string(), 40, 0.9);
        context.add_item(item1).unwrap();

        // Add item that would exceed limit - should remove previous item
        let item2 = ContextItem::new(
            ContextItemType::User,
            "Another message".to_string(),
            30,
            0.8,
        );
        context.add_item(item2).unwrap();

        assert!(context.current_tokens <= context.max_tokens);
    }

    #[test]
    fn test_pinned_items() {
        let agent_id = AgentId::new_v4();
        let mut context = AgentContext::new(agent_id, 100, 2);

        // Add pinned item
        let mut pinned_item = ContextItem::new(
            ContextItemType::System,
            "System prompt".to_string(),
            30,
            1.0,
        );
        pinned_item.metadata.pinned = true;
        context.add_item(pinned_item).unwrap();

        // Add regular items
        let item1 = ContextItem::new(ContextItemType::User, "Message 1".to_string(), 20, 0.5);
        context.add_item(item1).unwrap();

        let item2 = ContextItem::new(ContextItemType::User, "Message 2".to_string(), 20, 0.6);
        context.add_item(item2).unwrap();

        // Should keep pinned item and highest relevance regular item
        assert_eq!(context.items.len(), 2);
        assert!(context.items.iter().any(|item| item.metadata.pinned));
    }

    #[test]
    fn test_context_formatting() {
        let agent_id = AgentId::new_v4();
        let mut context = AgentContext::new(agent_id, 100, 10);

        let user_item = ContextItem::new(ContextItemType::User, "Hello".to_string(), 5, 0.9);
        context.add_item(user_item).unwrap();

        let assistant_item =
            ContextItem::new(ContextItemType::Assistant, "Hi there!".to_string(), 8, 0.9);
        context.add_item(assistant_item).unwrap();

        let formatted = context.format_for_llm();
        assert!(formatted.contains("User: Hello"));
        assert!(formatted.contains("Assistant: Hi there!"));
    }
}
