use gpui::*;
use std::sync::Arc;
use terraphim_types::{ContextItem, ContextType};

/// Context management state
/// Handles CRUD operations for context items in conversations
pub struct ContextManager {
    items: Vec<Arc<ContextItem>>,
    selected_items: Vec<String>, // IDs of selected items
    max_items: usize,
}

impl ContextManager {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        log::info!("ContextManager initialized");

        Self {
            items: Vec::new(),
            selected_items: Vec::new(),
            max_items: 50, // Reasonable limit for context items
        }
    }

    /// Add a context item
    pub fn add_item(&mut self, item: ContextItem, cx: &mut Context<Self>) -> Result<(), String> {
        // Check if we've reached the limit
        if self.items.len() >= self.max_items {
            return Err(format!("Maximum context items ({}) reached", self.max_items));
        }

        // Check for duplicate IDs
        if self.items.iter().any(|existing| existing.id == item.id) {
            return Err(format!("Context item with ID '{}' already exists", item.id));
        }

        log::info!("Adding context item: {} ({})", item.title, item.id);
        self.items.push(Arc::new(item));
        cx.notify();
        Ok(())
    }

    /// Update an existing context item
    pub fn update_item(&mut self, id: &str, item: ContextItem, cx: &mut Context<Self>) -> Result<(), String> {
        let index = self
            .items
            .iter()
            .position(|existing| existing.id.as_str() == id)
            .ok_or_else(|| format!("Context item with ID '{}' not found", id))?;

        log::info!("Updating context item: {}", id);
        self.items[index] = Arc::new(item);
        cx.notify();
        Ok(())
    }

    /// Remove a context item
    pub fn remove_item(&mut self, id: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let initial_len = self.items.len();
        self.items.retain(|item| item.id.as_str() != id);

        if self.items.len() == initial_len {
            return Err(format!("Context item with ID '{}' not found", id));
        }

        // Also remove from selected items
        self.selected_items.retain(|selected_id| selected_id != id);

        log::info!("Removed context item: {}", id);
        cx.notify();
        Ok(())
    }

    /// Get a context item by ID
    pub fn get_item(&self, id: &str) -> Option<Arc<ContextItem>> {
        self.items
            .iter()
            .find(|item| item.id.as_str() == id)
            .cloned()
    }

    /// Get all context items
    pub fn get_all_items(&self) -> Vec<Arc<ContextItem>> {
        self.items.clone()
    }

    /// Get selected context items
    pub fn get_selected_items(&self) -> Vec<Arc<ContextItem>> {
        self.items
            .iter()
            .filter(|item| self.selected_items.contains(&item.id.to_string()))
            .cloned()
            .collect()
    }

    /// Select a context item
    pub fn select_item(&mut self, id: &str, cx: &mut Context<Self>) -> Result<(), String> {
        if !self.items.iter().any(|item| item.id.as_str() == id) {
            return Err(format!("Context item with ID '{}' not found", id));
        }

        if !self.selected_items.contains(&id.to_string()) {
            self.selected_items.push(id.to_string());
            log::info!("Selected context item: {}", id);
            cx.notify();
        }
        Ok(())
    }

    /// Deselect a context item
    pub fn deselect_item(&mut self, id: &str, cx: &mut Context<Self>) {
        let initial_len = self.selected_items.len();
        self.selected_items.retain(|selected_id| selected_id != id);

        if self.selected_items.len() < initial_len {
            log::info!("Deselected context item: {}", id);
            cx.notify();
        }
    }

    /// Toggle selection of a context item
    pub fn toggle_selection(&mut self, id: &str, cx: &mut Context<Self>) -> Result<(), String> {
        if self.selected_items.contains(&id.to_string()) {
            self.deselect_item(id, cx);
        } else {
            self.select_item(id, cx)?;
        }
        Ok(())
    }

    /// Select all context items
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selected_items = self.items.iter().map(|item| item.id.to_string()).collect();
        log::info!("Selected all {} context items", self.selected_items.len());
        cx.notify();
    }

    /// Deselect all context items
    pub fn deselect_all(&mut self, cx: &mut Context<Self>) {
        self.selected_items.clear();
        log::info!("Deselected all context items");
        cx.notify();
    }

    /// Clear all context items
    pub fn clear_all(&mut self, cx: &mut Context<Self>) {
        let count = self.items.len();
        self.items.clear();
        self.selected_items.clear();
        log::info!("Cleared all {} context items", count);
        cx.notify();
    }

    /// Filter items by type
    pub fn filter_by_type(&self, context_type: ContextType) -> Vec<Arc<ContextItem>> {
        self.items
            .iter()
            .filter(|item| item.context_type == context_type)
            .cloned()
            .collect()
    }

    /// Search items by title or content
    pub fn search(&self, query: &str) -> Vec<Arc<ContextItem>> {
        let query_lower = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&query_lower)
                    || item.content.to_lowercase().contains(&query_lower)
                    || item
                        .summary
                        .as_ref()
                        .map_or(false, |s| s.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Sort items by relevance score (descending)
    pub fn sort_by_relevance(&mut self, cx: &mut Context<Self>) {
        self.items.sort_by(|a, b| {
            let score_a = a.relevance_score.unwrap_or(0.0);
            let score_b = b.relevance_score.unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        cx.notify();
    }

    /// Sort items by creation date (newest first)
    pub fn sort_by_date(&mut self, cx: &mut Context<Self>) {
        self.items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        cx.notify();
    }

    /// Get item count
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Get selected count
    pub fn selected_count(&self) -> usize {
        self.selected_items.len()
    }

    /// Check if item is selected
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected_items.contains(&id.to_string())
    }

    /// Get statistics
    pub fn get_stats(&self) -> ContextStats {
        let mut stats = ContextStats {
            total: self.items.len(),
            selected: self.selected_items.len(),
            by_type: std::collections::HashMap::new(),
            total_relevance: 0.0,
            avg_relevance: 0.0,
        };

        for item in &self.items {
            // Convert ContextType to string for HashMap key
            let type_str = format!("{:?}", item.context_type);
            *stats.by_type.entry(type_str).or_insert(0) += 1;
            if let Some(score) = item.relevance_score {
                stats.total_relevance += score;
            }
        }

        if !self.items.is_empty() {
            stats.avg_relevance = stats.total_relevance / self.items.len() as f64;
        }

        stats
    }
}

/// Context statistics
#[derive(Debug, Clone)]
pub struct ContextStats {
    pub total: usize,
    pub selected: usize,
    pub by_type: std::collections::HashMap<String, usize>, // Changed from ContextType to String
    pub total_relevance: f64,
    pub avg_relevance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_item(id: &str, title: &str) -> ContextItem {
        ContextItem {
            id: id.into(),
            title: title.to_string(),
            summary: None,
            content: format!("Content for {}", title),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: Some(0.8),
            metadata: ahash::AHashMap::new(),
        }
    }

    #[test]
    fn test_context_stats() {
        let stats = ContextStats {
            total: 10,
            selected: 3,
            by_type: std::collections::HashMap::new(),
            total_relevance: 8.5,
            avg_relevance: 0.85,
        };

        assert_eq!(stats.total, 10);
        assert_eq!(stats.selected, 3);
        assert_eq!(stats.avg_relevance, 0.85);
    }

    #[test]
    fn test_context_item_creation() {
        let item = create_test_item("test_1", "Test Item");

        assert_eq!(item.id.as_str(), "test_1");
        assert_eq!(item.title, "Test Item");
        assert_eq!(item.context_type, ContextType::Document);
        assert_eq!(item.relevance_score, Some(0.8));
    }
}
