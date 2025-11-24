use gpui::*;

use crate::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};

/// Autocomplete state management for UI
pub struct AutocompleteState {
    engine: Option<AutocompleteEngine>,
    suggestions: Vec<AutocompleteSuggestion>,
    selected_index: usize,
    last_query: String,
}

impl AutocompleteState {
    pub fn new(cx: &mut ModelContext<Self>) -> Self {
        log::info!("AutocompleteState initialized");

        Self {
            engine: None,
            suggestions: vec![],
            selected_index: 0,
            last_query: String::new(),
        }
    }

    /// Initialize engine from role
    pub fn initialize_engine(&mut self, role: &str, cx: &mut ModelContext<Self>) {
        match AutocompleteEngine::from_role(role, None) {
            Ok(engine) => {
                log::info!(
                    "Autocomplete engine loaded with {} terms for role '{}'",
                    engine.term_count(),
                    role
                );
                self.engine = Some(engine);
                cx.notify();
            }
            Err(e) => {
                log::error!("Failed to load autocomplete engine: {}", e);
            }
        }
    }

    /// Fetch suggestions for query
    pub fn fetch_suggestions(&mut self, query: &str, cx: &mut ModelContext<Self>) {
        if query == self.last_query {
            return; // Don't refetch same query
        }

        self.last_query = query.to_string();

        if let Some(engine) = &self.engine {
            self.suggestions = if query.len() < 3 {
                // Use exact match for short queries
                engine.autocomplete(query, 8)
            } else {
                // Use fuzzy search for longer queries
                engine.fuzzy_search(query, 8)
            };

            self.selected_index = 0;
            log::debug!("Found {} suggestions for '{}'", self.suggestions.len(), query);
        } else {
            log::warn!("Autocomplete engine not initialized");
            self.suggestions = vec![];
        }

        cx.notify();
    }

    pub fn select_next(&mut self, cx: &mut ModelContext<Self>) {
        if !self.suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.suggestions.len() - 1);
            cx.notify();
        }
    }

    pub fn select_previous(&mut self, cx: &mut ModelContext<Self>) {
        self.selected_index = self.selected_index.saturating_sub(1);
        cx.notify();
    }

    pub fn get_selected(&self) -> Option<&AutocompleteSuggestion> {
        self.suggestions.get(self.selected_index)
    }

    pub fn clear(&mut self, cx: &mut ModelContext<Self>) {
        self.suggestions.clear();
        self.selected_index = 0;
        self.last_query.clear();
        cx.notify();
    }

    pub fn is_empty(&self) -> bool {
        self.suggestions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.suggestions.len()
    }
}
