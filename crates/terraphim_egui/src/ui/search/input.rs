//! Search input component with autocomplete support
//!
//! This module provides the search input widget with real-time
//! autocomplete functionality using WASM-based terraphim_automata.

use eframe::egui;
use tracing::{debug, info};

use super::{AutocompleteWidget, SearchResults};
use crate::state::AppState;

/// Search input widget with integrated autocomplete
pub struct SearchInput {
    /// Current search query
    query: String,

    /// Placeholder text
    placeholder: String,

    /// Whether autocomplete is enabled
    autocomplete_enabled: bool,

    /// Autocomplete widget
    autocomplete: AutocompleteWidget,

    /// Search results (for displaying)
    results: SearchResults,
}

impl SearchInput {
    /// Create a new SearchInput widget
    pub fn new(state: &AppState) -> Self {
        let settings = state.get_ui_state().settings.clone();
        Self {
            query: String::new(),
            placeholder: "Search articles, concepts, and knowledge...".to_string(),
            autocomplete_enabled: settings.show_autocomplete,
            autocomplete: AutocompleteWidget::new(),
            results: SearchResults::new(state),
        }
    }

    /// Render the search input with autocomplete
    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Search").heading().strong());

            ui.add_space(4.0);

            // Search input field
            let text_edit = egui::TextEdit::singleline(&mut self.query)
                .hint_text(&self.placeholder)
                .desired_width(f32::INFINITY);

            let response = ui.add(text_edit);

            // Handle user input
            if response.changed() {
                debug!("Search query changed: {}", self.query);
                // Update autocomplete
                if self.autocomplete_enabled {
                    self.autocomplete.set_query(self.query.clone());
                }
            }

            // Handle keyboard events for autocomplete
            if self.autocomplete_enabled {
                self.autocomplete.handle_keyboard(ui.ctx());
            }

            // Render autocomplete dropdown
            if self.autocomplete_enabled {
                ui.add_space(2.0);
                self.autocomplete.render(ui, state);
            }

            ui.add_space(8.0);

            // Search button and options
            ui.horizontal(|ui| {
                if ui
                    .button("🔍 Search")
                    .on_hover_text("Execute search")
                    .clicked()
                {
                    self.execute_search(state);
                }

                if ui
                    .button("🎯 Add to Context")
                    .on_hover_text("Add all results to context")
                    .clicked()
                {
                    self.add_to_context(state);
                }

                // Toggle autocomplete
                if ui
                    .checkbox(&mut self.autocomplete_enabled, "Autocomplete")
                    .on_hover_text("Enable/disable autocomplete")
                    .changed()
                {
                    debug!("Autocomplete toggled: {}", self.autocomplete_enabled);
                }
            });
        });
    }

    /// Execute search operation
    pub fn execute_search(&mut self, state: &AppState) {
        if self.query.trim().is_empty() {
            info!("Empty search query, skipping");
            return;
        }

        info!("Executing search for query: {}", self.query);

        // Set loading state on results
        self.results.set_loading(true);
        self.results.set_search_query(self.query.clone());

        // TODO: Implement actual search with terraphim_service
        // For now, create mock results for demonstration
        let mock_results = self.create_mock_results(&self.query);
        state.set_search_results(mock_results);
        self.results.set_loading(false);

        debug!("Search feature implementation in progress");
    }

    /// Create mock search results for demonstration
    fn create_mock_results(&self, query: &str) -> Vec<terraphim_types::Document> {
        vec![
            terraphim_types::Document {
                id: "1".to_string(),
                url: format!("https://example.com/{}", query),
                title: format!("{} - Official Documentation", query),
                body: format!("This is a comprehensive guide about {}. It covers all the essential concepts and best practices.", query),
                description: Some(format!("Complete guide to {} with examples and tutorials", query)),
                summarization: None,
                stub: Some(format!("Learn {} fundamentals", query)),
                tags: Some(vec!["documentation".to_string(), "tutorial".to_string()]),
                rank: Some(100),
                source_haystack: Some("Local Files".to_string()),
            },
            terraphim_types::Document {
                id: "2".to_string(),
                url: format!("https://github.com/search?q={}", query),
                title: format!("{} - GitHub Repository", query),
                body: format!("Repository containing examples and projects related to {}.", query),
                description: Some(format!("Open source {} implementation", query)),
                summarization: None,
                stub: Some(format!("{} codebase", query)),
                tags: Some(vec!["github".to_string(), "code".to_string()]),
                rank: Some(85),
                source_haystack: Some("GitHub".to_string()),
            },
            terraphim_types::Document {
                id: "3".to_string(),
                url: format!("https://docs.rs/{}", query),
                title: format!("{} - Rust Crate Documentation", query),
                body: format!("API documentation and usage examples for the {} crate.", query),
                description: Some(format!("Rust crate documentation for {}", query)),
                summarization: None,
                stub: Some(format!("{} crate docs", query)),
                tags: Some(vec!["rust".to_string(), "crate".to_string()]),
                rank: Some(70),
                source_haystack: Some("docs.rs".to_string()),
            },
        ]
    }

    /// Add search results to context
    pub fn add_to_context(&self, state: &AppState) {
        if self.query.trim().is_empty() {
            return;
        }

        info!("Adding search results to context for query: {}", self.query);

        // Add current search results to context
        let results = self.results.get_filtered_results();
        for result in results {
            state.add_document_to_context(result.clone());
        }

        debug!("Added {} results to context", results.len());
    }

    /// Get current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
    }

    /// Check if autocomplete is ready
    pub fn is_autocomplete_ready(&self) -> bool {
        self.autocomplete.is_ready()
    }

    /// Get autocomplete suggestions count
    pub fn autocomplete_suggestions_count(&self) -> usize {
        self.autocomplete.suggestions().len()
    }

    /// Get autocomplete search service
    pub fn search_service(&self) -> &terraphim_automata::AutocompleteIndex {
        // Simplified for now
        panic!("search_service not implemented yet");
    }
}
