use gpui::*;
use std::sync::Arc;
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use terraphim_types::SearchQuery;

use crate::models::{ResultItemViewModel, TermChipSet};
use crate::search_service::SearchService;

/// Autocomplete suggestion from KG search
#[derive(Clone, Debug)]
pub struct AutocompleteSuggestion {
    pub term: String,
    pub normalized_term: String,
    pub url: Option<String>,
    pub score: f64,
}

/// Search state management with real backend integration
pub struct SearchState {
    config_state: Option<ConfigState>,
    query: String,
    parsed_query: String,
    results: Vec<ResultItemViewModel>,
    term_chips: TermChipSet,
    loading: bool,
    error: Option<String>,
    current_role: String,
    // Autocomplete state
    autocomplete_suggestions: Vec<AutocompleteSuggestion>,
    autocomplete_loading: bool,
    show_autocomplete: bool,
    selected_suggestion_index: usize,
}

impl SearchState {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        log::info!("SearchState initialized");

        Self {
            config_state: None,
            query: String::new(),
            parsed_query: String::new(),
            results: vec![],
            term_chips: TermChipSet::new(),
            loading: false,
            error: None,
            current_role: "Terraphim Engineer".to_string(),
            autocomplete_suggestions: vec![],
            autocomplete_loading: false,
            show_autocomplete: false,
            selected_suggestion_index: 0,
        }
    }

    /// Initialize with config state for backend access
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        self.config_state = Some(config_state);
        self
    }

    /// Initialize search service from config
    pub fn initialize_service(&mut self, _config_path: Option<&str>, cx: &mut Context<Self>) {
        // TODO: GPUI 0.2.2 migration - SearchService initialization needs update
        // These methods don't exist in current API:
        // - SearchService::from_config_file()
        // - Config::load()

        log::warn!("Search service initialization temporarily disabled during GPUI migration");
        self.error = Some("Search service initialization not yet implemented".to_string());
        cx.notify();

        // Placeholder for future implementation:
        // match SearchService::from_config(...) {
        //     Ok(service) => {
        //         self.service = Some(Arc::new(service));
        //         cx.notify();
        //     }
        //     Err(e) => {
        //         self.error = Some(format!("Failed to load search service: {}", e));
        //         cx.notify();
        //     }
        // }
    }

    /// Set current role
    pub fn set_role(&mut self, role: String, cx: &mut Context<Self>) {
        self.current_role = role;
        log::info!("Role changed to: {}", self.current_role);
        cx.notify();
    }

    /// Execute search using real TerraphimService
    pub fn search(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() {
            self.clear_results(cx);
            return;
        }

        self.query = query.clone();
        self.loading = true;
        self.error = None;
        cx.notify();

        log::info!("Search initiated for query: '{}'", query);

        // Parse query for term chips
        self.update_term_chips(&query);

        let config_state = match &self.config_state {
            Some(state) => state.clone(),
            None => {
                self.error = Some("Config not initialized".to_string());
                self.loading = false;
                cx.notify();
                return;
            }
        };

        // Create search query from pattern in Tauri cmd.rs
        let search_query = SearchQuery {
            search_term: query.clone().into(),
            search_terms: None,
            operator: None,
            role: Some(terraphim_types::RoleName::from(self.current_role.as_str())),
            limit: Some(20),
            skip: Some(0),
        };

        cx.spawn(async move |this, cx| {
            // Create service instance (pattern from Tauri cmd.rs)
            let mut terraphim_service = TerraphimService::new(config_state);

            match terraphim_service.search(&search_query).await {
                Ok(documents) => {
                    log::info!("Search completed: {} results found", documents.len());

                    this.update(cx, |this, cx| {
                        this.results = documents
                            .into_iter()
                            .map(|doc| ResultItemViewModel::new(doc).with_highlights(&query))
                            .collect();
                        this.loading = false;
                        this.parsed_query = query;
                        cx.notify();
                    })
                    .ok();
                }
                Err(e) => {
                    log::error!("Search failed: {}", e);

                    this.update(cx, |this, cx| {
                        this.error = Some(format!("Search failed: {}", e));
                        this.loading = false;
                        this.results = vec![];
                        cx.notify();
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    /// Get autocomplete suggestions from KG (pattern from Tauri search_kg_terms)
    pub fn get_autocomplete(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() || query.len() < 2 {
            self.autocomplete_suggestions.clear();
            self.show_autocomplete = false;
            cx.notify();
            return;
        }

        let config_state = match &self.config_state {
            Some(state) => state.clone(),
            None => {
                log::warn!("Cannot get autocomplete: config not initialized");
                return;
            }
        };

        self.autocomplete_loading = true;
        cx.notify();

        let role_name = self.current_role.clone();

        cx.spawn(async move |this, cx| {
            // Use terraphim_automata for KG autocomplete (from Tauri cmd.rs pattern)
            use terraphim_automata::{autocomplete_search, build_autocomplete_index, fuzzy_autocomplete_search};
            use terraphim_types::RoleName;

            let role = RoleName::from(role_name.as_str());

            // Get the rolegraph for autocomplete
            let autocomplete_index = if let Some(rolegraph_sync) = config_state.roles.get(&role) {
                let rolegraph = rolegraph_sync.lock().await;

                match build_autocomplete_index(rolegraph.thesaurus.clone(), None) {
                    Ok(index) => Some(index),
                    Err(e) => {
                        log::error!("Failed to build autocomplete index: {}", e);
                        None
                    }
                }
            } else {
                log::warn!("Role '{}' not found for autocomplete", role);
                None
            };

            let suggestions = if let Some(index) = autocomplete_index {
                // Use fuzzy search for queries >= 3 chars, exact for shorter
                let results = if query.len() >= 3 {
                    fuzzy_autocomplete_search(&index, &query, 0.7, Some(8))
                        .unwrap_or_else(|_| autocomplete_search(&index, &query, Some(8)).unwrap_or_default())
                } else {
                    autocomplete_search(&index, &query, Some(8)).unwrap_or_default()
                };

                results.into_iter()
                    .map(|r| AutocompleteSuggestion {
                        term: r.term,
                        normalized_term: r.normalized_term.to_string(),
                        url: r.url,
                        score: r.score,
                    })
                    .collect()
            } else {
                vec![]
            };

            this.update(cx, |this, cx| {
                this.autocomplete_suggestions = suggestions;
                this.autocomplete_loading = false;
                this.show_autocomplete = !this.autocomplete_suggestions.is_empty();
                this.selected_suggestion_index = 0;
                cx.notify();
            }).ok();
        }).detach();
    }

    /// Select next autocomplete suggestion
    pub fn autocomplete_next(&mut self, cx: &mut Context<Self>) {
        if !self.autocomplete_suggestions.is_empty() {
            self.selected_suggestion_index =
                (self.selected_suggestion_index + 1).min(self.autocomplete_suggestions.len() - 1);
            cx.notify();
        }
    }

    /// Select previous autocomplete suggestion
    pub fn autocomplete_previous(&mut self, cx: &mut Context<Self>) {
        self.selected_suggestion_index = self.selected_suggestion_index.saturating_sub(1);
        cx.notify();
    }

    /// Accept selected autocomplete suggestion
    pub fn accept_autocomplete(&mut self, cx: &mut Context<Self>) -> Option<String> {
        if let Some(suggestion) = self.autocomplete_suggestions.get(self.selected_suggestion_index) {
            let term = suggestion.term.clone();
            self.autocomplete_suggestions.clear();
            self.show_autocomplete = false;
            cx.notify();
            Some(term)
        } else {
            None
        }
    }

    /// Get current autocomplete suggestions
    pub fn get_suggestions(&self) -> &[AutocompleteSuggestion] {
        &self.autocomplete_suggestions
    }

    /// Check if autocomplete is showing
    pub fn is_autocomplete_visible(&self) -> bool {
        self.show_autocomplete && !self.autocomplete_suggestions.is_empty()
    }

    fn update_term_chips(&mut self, query: &str) {
        // Parse query to extract term chips
        let parsed = SearchService::parse_query(query);

        // Check if query is complex (multiple terms or has operator)
        let is_complex = parsed.terms.len() > 1 || parsed.operator.is_some();

        if is_complex {
            self.term_chips = TermChipSet::from_query_string(query, |_term| false);
        } else {
            self.term_chips.clear();
        }
    }

    fn clear_results(&mut self, cx: &mut Context<Self>) {
        self.results.clear();
        self.query.clear();
        self.parsed_query.clear();
        self.term_chips.clear();
        self.error = None;
        cx.notify();
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Get search results for display
    pub fn get_results(&self) -> &[ResultItemViewModel] {
        &self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here - require test config
}
