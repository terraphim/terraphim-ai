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
            return Err(format!(
                "Maximum context items ({}) reached",
                self.max_items
            ));
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
    pub fn update_item(
        &mut self,
        id: &str,
        item: ContextItem,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
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
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
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

    fn create_test_item_with_summary(id: &str, title: &str, summary: &str) -> ContextItem {
        ContextItem {
            id: id.into(),
            title: title.to_string(),
            summary: Some(summary.to_string()),
            content: format!("Content for {}", title),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: Some(0.9),
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

    #[test]
    fn test_add_item_success() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");

        let result = manager.add_item(item, &mut gpui::test::Context::default());

        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_add_item_duplicate_id() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item1 = create_test_item("test_1", "Test Item 1");
        let item2 = create_test_item("test_1", "Test Item 2");

        let result1 = manager.add_item(item1, &mut gpui::test::Context::default());
        let result2 = manager.add_item(item2, &mut gpui::test::Context::default());

        assert!(result1.is_ok());
        assert!(result2.is_err());
        assert_eq!(manager.count(), 1);
        assert_eq!(
            result2.unwrap_err(),
            "Context item with ID 'test_1' already exists"
        );
    }

    #[test]
    fn test_add_item_max_limit() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        // Add maximum allowed items
        for i in 0..50 {
            let item = create_test_item(&format!("test_{}", i), &format!("Test Item {}", i));
            let result = manager.add_item(item, &mut gpui::test::Context::default());
            assert!(result.is_ok(), "Failed to add item {}", i);
        }

        assert_eq!(manager.count(), 50);

        // Try to add one more - should fail
        let extra_item = create_test_item("extra", "Extra Item");
        let result = manager.add_item(extra_item, &mut gpui::test::Context::default());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Maximum context items (50) reached");
        assert_eq!(manager.count(), 50);
    }

    #[test]
    fn test_get_item() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item.clone(), &mut gpui::test::Context::default())
            .unwrap();

        let retrieved = manager.get_item("test_1");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test_1");
        assert_eq!(retrieved.unwrap().title, "Test Item");
    }

    #[test]
    fn test_get_item_not_found() {
        let manager = ContextManager::new(&mut gpui::test::Context::default());

        let retrieved = manager.get_item("nonexistent");

        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_all_items() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Test Item 1");
        let item2 = create_test_item("test_2", "Test Item 2");

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        let all_items = manager.get_all_items();

        assert_eq!(all_items.len(), 2);
    }

    #[test]
    fn test_update_item_success() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let original = create_test_item("test_1", "Original Title");
        manager
            .add_item(original, &mut gpui::test::Context::default())
            .unwrap();

        let updated = create_test_item("test_1", "Updated Title");
        let result = manager.update_item("test_1", updated, &mut gpui::test::Context::default());

        assert!(result.is_ok());
        let retrieved = manager.get_item("test_1").unwrap();
        assert_eq!(retrieved.title, "Updated Title");
    }

    #[test]
    fn test_update_item_not_found() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");

        let result = manager.update_item("nonexistent", item, &mut gpui::test::Context::default());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Context item with ID 'nonexistent' not found"
        );
    }

    #[test]
    fn test_remove_item_success() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.count(), 1);

        let result = manager.remove_item("test_1", &mut gpui::test::Context::default());

        assert!(result.is_ok());
        assert_eq!(manager.count(), 0);
        assert!(manager.get_item("test_1").is_none());
    }

    #[test]
    fn test_remove_item_not_found() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let result = manager.remove_item("nonexistent", &mut gpui::test::Context::default());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Context item with ID 'nonexistent' not found"
        );
    }

    #[test]
    fn test_remove_item_removes_from_selected() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.selected_count(), 1);

        manager
            .remove_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_select_item() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        let result = manager.select_item("test_1", &mut gpui::test::Context::default());

        assert!(result.is_ok());
        assert!(manager.is_selected("test_1"));
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn test_select_item_not_found() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let result = manager.select_item("nonexistent", &mut gpui::test::Context::default());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Context item with ID 'nonexistent' not found"
        );
        assert!(!manager.is_selected("nonexistent"));
    }

    #[test]
    fn test_select_duplicate_item() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        // Select twice
        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();
        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        // Should only be counted once
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn test_deselect_item() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.selected_count(), 1);

        manager.deselect_item("test_1", &mut gpui::test::Context::default());

        assert!(!manager.is_selected("test_1"));
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_deselect_item_not_selected() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        // Deselect without selecting first
        manager.deselect_item("test_1", &mut gpui::test::Context::default());

        assert!(!manager.is_selected("test_1"));
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_toggle_selection() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        // Toggle on
        manager
            .toggle_selection("test_1", &mut gpui::test::Context::default())
            .unwrap();
        assert!(manager.is_selected("test_1"));
        assert_eq!(manager.selected_count(), 1);

        // Toggle off
        manager
            .toggle_selection("test_1", &mut gpui::test::Context::default())
            .unwrap();
        assert!(!manager.is_selected("test_1"));
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_select_all() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        for i in 0..5 {
            let item = create_test_item(&format!("test_{}", i), &format!("Test Item {}", i));
            manager
                .add_item(item, &mut gpui::test::Context::default())
                .unwrap();
        }

        manager.select_all(&mut gpui::test::Context::default());

        assert_eq!(manager.selected_count(), 5);
        for i in 0..5 {
            assert!(manager.is_selected(&format!("test_{}", i)));
        }
    }

    #[test]
    fn test_deselect_all() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        for i in 0..5 {
            let item = create_test_item(&format!("test_{}", i), &format!("Test Item {}", i));
            manager
                .add_item(item, &mut gpui::test::Context::default())
                .unwrap();
        }

        manager.select_all(&mut gpui::test::Context::default());
        assert_eq!(manager.selected_count(), 5);

        manager.deselect_all(&mut gpui::test::Context::default());
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        for i in 0..5 {
            let item = create_test_item(&format!("test_{}", i), &format!("Test Item {}", i));
            manager
                .add_item(item, &mut gpui::test::Context::default())
                .unwrap();
        }

        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        manager.clear_all(&mut gpui::test::Context::default());

        assert_eq!(manager.count(), 0);
        assert_eq!(manager.selected_count(), 0);
    }

    #[test]
    fn test_get_selected_items() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Test Item 1");
        let item2 = create_test_item("test_2", "Test Item 2");
        let item3 = create_test_item("test_3", "Test Item 3");

        manager
            .add_item(item1.clone(), &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2.clone(), &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item3.clone(), &mut gpui::test::Context::default())
            .unwrap();

        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();
        manager
            .select_item("test_3", &mut gpui::test::Context::default())
            .unwrap();

        let selected = manager.get_selected_items();

        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|item| item.id == "test_1"));
        assert!(selected.iter().any(|item| item.id == "test_3"));
    }

    #[test]
    fn test_filter_by_type() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let mut item1 = create_test_item("test_1", "Test Item 1");
        item1.context_type = ContextType::Document;

        let mut item2 = create_test_item("test_2", "Test Item 2");
        item2.context_type = ContextType::Note;

        let mut item3 = create_test_item("test_3", "Test Item 3");
        item3.context_type = ContextType::Document;

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item3, &mut gpui::test::Context::default())
            .unwrap();

        let docs = manager.filter_by_type(ContextType::Document);
        assert_eq!(docs.len(), 2);

        let notes = manager.filter_by_type(ContextType::Note);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_search_by_title() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Rust Programming");
        let item2 = create_test_item("test_2", "JavaScript Guide");

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("Rust");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming");
    }

    #[test]
    fn test_search_by_content() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Item 1");
        let mut item2 = create_test_item("test_2", "Item 2");
        item2.content = "This content contains async programming".to_string();

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("async");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test_2");
    }

    #[test]
    fn test_search_by_summary() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 =
            create_test_item_with_summary("test_1", "Item 1", "Summary about web development");
        let item2 = create_test_item("test_2", "Item 2");

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("web");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test_1");
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item = create_test_item("test_1", "Rust Programming");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("rust");
        assert_eq!(results.len(), 1);

        let results = manager.search("RUST");
        assert_eq!(results.len(), 1);

        let results = manager.search("RuSt");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty_query() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_matches() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Rust Programming");
        let item2 = create_test_item("test_2", "JavaScript Guide");

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        let results = manager.search("Python");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_sort_by_relevance() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let mut item1 = create_test_item("test_1", "Item 1");
        item1.relevance_score = Some(0.5);

        let mut item2 = create_test_item("test_2", "Item 2");
        item2.relevance_score = Some(0.9);

        let mut item3 = create_test_item("test_3", "Item 3");
        item3.relevance_score = Some(0.2);

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item3, &mut gpui::test::Context::default())
            .unwrap();

        manager.sort_by_relevance(&mut gpui::test::Context::default());

        let items = manager.get_all_items();
        assert_eq!(items[0].relevance_score, Some(0.9)); // Highest first
        assert_eq!(items[1].relevance_score, Some(0.5));
        assert_eq!(items[2].relevance_score, Some(0.2)); // Lowest last
    }

    #[test]
    fn test_sort_by_relevance_none_scores() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let mut item1 = create_test_item("test_1", "Item 1");
        item1.relevance_score = None;

        let mut item2 = create_test_item("test_2", "Item 2");
        item2.relevance_score = Some(0.5);

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        // Should not panic - None scores treated as 0.0
        manager.sort_by_relevance(&mut gpui::test::Context::default());

        let items = manager.get_all_items();
        // Should complete without panic
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_sort_by_date() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let mut item1 = create_test_item("test_1", "Item 1");
        item1.created_at = Utc::now() - chrono::Duration::hours(2);

        let mut item2 = create_test_item("test_2", "Item 2");
        item2.created_at = Utc::now() - chrono::Duration::hours(1);

        let mut item3 = create_test_item("test_3", "Item 3");
        item3.created_at = Utc::now();

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item3, &mut gpui::test::Context::default())
            .unwrap();

        manager.sort_by_date(&mut gpui::test::Context::default());

        let items = manager.get_all_items();
        assert_eq!(items[0].id, "test_3"); // Newest first
        assert_eq!(items[1].id, "test_2");
        assert_eq!(items[2].id, "test_1"); // Oldest last
    }

    #[test]
    fn test_get_stats() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let mut item1 = create_test_item("test_1", "Item 1");
        item1.context_type = ContextType::Document;
        item1.relevance_score = Some(0.5);

        let mut item2 = create_test_item("test_2", "Item 2");
        item2.context_type = ContextType::Note;
        item2.relevance_score = Some(0.7);

        let mut item3 = create_test_item("test_3", "Item 3");
        item3.context_type = ContextType::Document;
        item3.relevance_score = Some(0.3);

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item3, &mut gpui::test::Context::default())
            .unwrap();

        let stats = manager.get_stats();

        assert_eq!(stats.total, 3);
        assert_eq!(stats.selected, 0);
        assert_eq!(stats.total_relevance, 1.5);
        assert_eq!(stats.avg_relevance, 0.5);
        assert_eq!(stats.by_type.get("Document"), Some(&2));
        assert_eq!(stats.by_type.get("Note"), Some(&1));
    }

    #[test]
    fn test_get_stats_with_selected_items() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        let item1 = create_test_item("test_1", "Item 1");
        let item2 = create_test_item("test_2", "Item 2");

        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();
        manager
            .add_item(item2, &mut gpui::test::Context::default())
            .unwrap();

        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        let stats = manager.get_stats();

        assert_eq!(stats.total, 2);
        assert_eq!(stats.selected, 1);
    }

    #[test]
    fn test_get_stats_empty_manager() {
        let manager = ContextManager::new(&mut gpui::test::Context::default());

        let stats = manager.get_stats();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.selected, 0);
        assert_eq!(stats.total_relevance, 0.0);
        assert_eq!(stats.avg_relevance, 0.0);
        assert!(stats.by_type.is_empty());
    }

    #[test]
    fn test_count_and_selected_count() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());

        assert_eq!(manager.count(), 0);
        assert_eq!(manager.selected_count(), 0);

        let item1 = create_test_item("test_1", "Item 1");
        manager
            .add_item(item1, &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.count(), 1);
        assert_eq!(manager.selected_count(), 0);

        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();

        assert_eq!(manager.count(), 1);
        assert_eq!(manager.selected_count(), 1);
    }

    #[test]
    fn test_is_selected() {
        let mut manager = ContextManager::new(&mut gpui::test::Context::default());
        let item = create_test_item("test_1", "Test Item");
        manager
            .add_item(item, &mut gpui::test::Context::default())
            .unwrap();

        assert!(!manager.is_selected("test_1"));

        manager
            .select_item("test_1", &mut gpui::test::Context::default())
            .unwrap();
        assert!(manager.is_selected("test_1"));

        manager.deselect_item("test_1", &mut gpui::test::Context::default());
        assert!(!manager.is_selected("test_1"));
    }

    #[test]
    fn test_is_selected_nonexistent() {
        let manager = ContextManager::new(&mut gpui::test::Context::default());

        assert!(!manager.is_selected("nonexistent"));
    }
}
