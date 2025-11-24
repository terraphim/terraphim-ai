use gpui::*;
use std::sync::Arc;
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use terraphim_types::SearchQuery;

use crate::models::{ResultItemViewModel, TermChipSet};
use crate::search_service::SearchService;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here - require test config
}
