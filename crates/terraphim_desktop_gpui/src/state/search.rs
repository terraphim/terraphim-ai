use gpui::*;
use std::sync::Arc;

use crate::models::{ResultItemViewModel, TermChipSet};
use crate::search_service::{SearchOptions, SearchService};

/// Search state management with real integration
pub struct SearchState {
    service: Option<Arc<SearchService>>,
    query: String,
    parsed_query: String,
    results: Vec<ResultItemViewModel>,
    term_chips: TermChipSet,
    loading: bool,
    error: Option<String>,
    current_role: String,
}

impl SearchState {
    pub fn new(_cx: &mut ModelContext<Self>) -> Self {
        log::info!("SearchState initialized");

        Self {
            service: None,
            query: String::new(),
            parsed_query: String::new(),
            results: vec![],
            term_chips: TermChipSet::new(),
            loading: false,
            error: None,
            current_role: "default".to_string(),
        }
    }

    /// Initialize search service from config
    pub fn initialize_service(&mut self, config_path: Option<&str>, cx: &mut ModelContext<Self>) {
        match if let Some(path) = config_path {
            SearchService::from_config_file(path)
        } else {
            SearchService::new(terraphim_config::Config::load().unwrap())
        } {
            Ok(service) => {
                log::info!("Search service initialized successfully");
                self.service = Some(Arc::new(service));
                cx.notify();
            }
            Err(e) => {
                log::error!("Failed to initialize search service: {}", e);
                self.error = Some(format!("Failed to load search service: {}", e));
                cx.notify();
            }
        }
    }

    /// Set current role
    pub fn set_role(&mut self, role: String, cx: &mut ModelContext<Self>) {
        self.current_role = role;
        log::info!("Role changed to: {}", self.current_role);
        cx.notify();
    }

    /// Execute search
    pub fn search(&mut self, query: String, cx: &mut ModelContext<Self>) {
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

        let service = match &self.service {
            Some(svc) => Arc::clone(svc),
            None => {
                self.error = Some("Search service not initialized".to_string());
                self.loading = false;
                cx.notify();
                return;
            }
        };

        let options = SearchOptions {
            role: self.current_role.clone(),
            limit: 20,
            ..Default::default()
        };

        cx.spawn(|this, mut cx| async move {
            match service.search(&query, options).await {
                Ok(results) => {
                    log::info!(
                        "Search completed: {} results (total: {})",
                        results.documents.len(),
                        results.total
                    );

                    this.update(&mut cx, |this, cx| {
                        this.results = results
                            .documents
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

                    this.update(&mut cx, |this, cx| {
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

        if parsed.is_complex() {
            // TODO: Check which terms are from KG using autocomplete engine
            self.term_chips = TermChipSet::from_query_string(query, |_term| false);
        } else {
            self.term_chips.clear();
        }
    }

    fn clear_results(&mut self, cx: &mut ModelContext<Self>) {
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
