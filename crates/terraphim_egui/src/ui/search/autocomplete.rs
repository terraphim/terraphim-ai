//! Autocomplete widget using WASM-based search
//!
//! This module provides the autocomplete functionality leveraging
//! terraphim_automata's WASM-based search capabilities.

use eframe::egui;
use tracing::{debug, info, warn};

use crate::state::AppState;

/// Simple SearchService placeholder for compilation
struct SearchService {
    // Simplified for now
    index: Option<()>,
}
impl SearchService {
    fn new() -> Self {
        Self { index: None }
    }
    fn load_thesaurus_from_data(&self, _data: &str) -> Result<(), ()> {
        Ok(())
    }
    fn autocomplete_search(
        &self,
        _query: &str,
        _limit: Option<usize>,
    ) -> Vec<terraphim_automata::AutocompleteResult> {
        Vec::new()
    }
    fn is_ready(&self) -> bool {
        true
    }
    fn autocomplete_size(&self) -> usize {
        0
    }
}

/// Autocomplete dropdown widget
pub struct AutocompleteWidget {
    /// Search service
    search_service: SearchService,

    /// Current query
    query: String,

    /// Suggestions list
    suggestions: Vec<terraphim_automata::AutocompleteResult>,

    /// Currently selected suggestion index
    selected_index: Option<usize>,

    /// Whether to show suggestions
    show_suggestions: bool,

    /// Debounce timer for triggering searches
    debounce_timer: std::time::Instant,
}

impl AutocompleteWidget {
    /// Create a new AutocompleteWidget
    pub fn new() -> Self {
        let mut widget = Self {
            search_service: SearchService::new(),
            query: String::new(),
            suggestions: Vec::new(),
            selected_index: None,
            show_suggestions: false,
            debounce_timer: std::time::Instant::now(),
        };

        // Load default thesaurus (simplified for compilation)
        // if let Err(e) = widget.search_service.load_thesaurus_from_data(DEFAULT_TERMS) {
        //     warn!("Failed to load default thesaurus: {}", e);
        // }

        widget
    }

    /// Render the autocomplete widget
    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        // Update autocomplete when query changes
        self.update_autocomplete();

        // Show suggestions if available
        if self.show_suggestions && !self.suggestions.is_empty() {
            self.render_suggestions_dropdown(ui);
        }
    }

    /// Render the suggestions dropdown
    fn render_suggestions_dropdown(&mut self, ui: &mut egui::Ui) {
        // Get suggestions count first to avoid borrow issues
        let suggestions_count = self.suggestions.len();

        // Create a frame for the dropdown
        egui::Frame::default()
            .fill(ui.visuals().panel_fill)
            .stroke(ui.visuals().window_stroke)
            .show(ui, |ui| {
                // Scroll area for suggestions
                egui::ScrollArea::vertical()
                    .id_source("autocomplete_dropdown")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for idx in 0..suggestions_count {
                            // Get suggestion without borrowing self
                            if let Some(suggestion) = self.suggestions.get(idx) {
                                let is_selected = self.selected_index == Some(idx);
                                let term = suggestion.term.clone();
                                let id = suggestion.id;
                                let url = suggestion.url.clone();

                                // Create a selectable label for each suggestion
                                let response =
                                    ui.selectable_label(is_selected, format!("• {}", term));

                                // Handle selection
                                if response.clicked() {
                                    self.select_suggestion(idx);
                                }

                                // Show metadata on hover
                                if response.hovered() {
                                    let mut hover_text = format!("ID: {}", id);
                                    if let Some(ref url) = url {
                                        hover_text.push_str(&format!("\nURL: {}", url));
                                    }
                                    response.on_hover_text(hover_text);
                                }
                            }
                        }
                    });
            });
    }

    /// Update autocomplete suggestions based on current query
    fn update_autocomplete(&mut self) {
        // Debounce: Only search if enough time has passed or query is empty
        let now = std::time::Instant::now();
        let should_update =
            self.query.is_empty() || now.duration_since(self.debounce_timer).as_millis() >= 50;

        if should_update {
            if self.query.is_empty() {
                // Clear suggestions when query is empty
                self.suggestions.clear();
                self.show_suggestions = false;
                self.selected_index = None;
            } else {
                // Perform autocomplete search
                self.suggestions = self
                    .search_service
                    .autocomplete_search(&self.query, Some(8));
                self.show_suggestions = !self.suggestions.is_empty();
                self.selected_index = None;
            }
            self.debounce_timer = now;
        }
    }

    /// Select a suggestion
    fn select_suggestion(&mut self, index: usize) {
        if let Some(suggestion) = self.suggestions.get(index) {
            self.query = suggestion.term.clone();
            self.selected_index = Some(index);
            self.show_suggestions = false;
            info!("Selected autocomplete suggestion: {}", suggestion.term);
        }
    }

    /// Handle keyboard input
    pub fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if !self.show_suggestions || self.suggestions.is_empty() {
            return;
        }

        // Navigate with arrow keys
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            self.selected_index = Some(
                self.selected_index
                    .map(|i| (i + 1).min(self.suggestions.len() - 1))
                    .unwrap_or(0),
            );
        } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            self.selected_index = Some(
                self.selected_index
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(self.suggestions.len() - 1),
            );
        } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(index) = self.selected_index {
                self.select_suggestion(index);
            }
        } else if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_suggestions = false;
            self.selected_index = None;
        }
    }

    /// Set the query and trigger autocomplete
    pub fn set_query(&mut self, query: String) {
        let query_len = query.len();
        self.query = query;
        self.show_suggestions = query_len > 0;
        debug!("Set query for autocomplete: {}", self.query);
    }

    /// Get current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Get current suggestions
    pub fn suggestions(&self) -> &[terraphim_automata::AutocompleteResult] {
        &self.suggestions
    }

    /// Check if autocomplete is ready
    pub fn is_ready(&self) -> bool {
        self.search_service.is_ready()
    }

    /// Get search service for external access
    pub fn search_service(&self) -> &SearchService {
        &self.search_service
    }
}
