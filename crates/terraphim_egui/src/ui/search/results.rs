//! Search results display component with virtual scrolling
//!
//! This module displays search results with document previews,
//! metadata, action buttons, sorting, filtering, and selection.

use eframe::egui;
use terraphim_types::Document;
use tracing::{debug, info, warn};

use crate::state::AppState;

/// Sort options for search results
#[derive(Debug, Clone, PartialEq)]
pub enum SortOption {
    Relevance,
    Title,
    Date,
    Source,
}

/// Filter options for search results
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    pub source_filter: Option<String>,
    pub min_rank: Option<u64>,
    pub tag_filter: Option<String>,
}

/// Virtual scrolling state
#[derive(Debug, Default)]
struct VirtualScrollState {
    /// Current scroll position
    scroll_offset: f32,
    /// Item height in pixels
    item_height: f32,
    /// Visible range of items
    visible_range: (usize, usize),
}

/// Search results widget with enhanced features
pub struct SearchResults {
    /// Search results
    pub results: Vec<Document>,

    /// Filtered results (after applying filters/sorting)
    pub filtered_results: Vec<Document>,

    /// Selected result indices
    pub selected_indices: Vec<usize>,

    /// Current sort option
    pub sort_option: SortOption,

    /// Filter options
    pub filter_options: FilterOptions,

    /// Loading state
    pub is_loading: bool,

    /// Virtual scroll state
    pub virtual_scroll: VirtualScrollState,

    /// Search query for display
    pub search_query: String,
}

impl SearchResults {
    /// Create a new SearchResults widget
    pub fn new(state: &AppState) -> Self {
        let results = state.get_search_results().clone();
        let filtered_results = results.clone();
        Self {
            results,
            filtered_results,
            selected_indices: Vec::new(),
            sort_option: SortOption::Relevance,
            filter_options: FilterOptions::default(),
            is_loading: false,
            virtual_scroll: VirtualScrollState::default(),
            search_query: String::new(),
        }
    }

    /// Update results from state
    pub fn update_results(&mut self, state: &AppState) {
        self.results = state.get_search_results().clone();
        self.apply_filters_and_sort();
    }

    /// Set search query
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
    }

    /// Apply filters and sorting to results
    pub fn apply_filters_and_sort(&mut self) {
        // Start with all results
        let mut filtered = self.results.clone();

        // Apply source filter
        if let Some(ref source) = self.filter_options.source_filter {
            filtered.retain(|doc| {
                doc.source_haystack
                    .as_ref()
                    .map(|s| s.contains(source))
                    .unwrap_or(false)
            });
        }

        // Apply rank filter
        if let Some(min_rank) = self.filter_options.min_rank {
            filtered.retain(|doc| doc.rank.map_or(false, |r| r >= min_rank));
        }

        // Apply tag filter
        if let Some(ref tag) = self.filter_options.tag_filter {
            filtered.retain(|doc| {
                doc.tags
                    .as_ref()
                    .map(|tags| tags.iter().any(|t| t.contains(tag)))
                    .unwrap_or(false)
            });
        }

        // Apply sorting
        match self.sort_option {
            SortOption::Relevance => {
                filtered.sort_by(|a, b| b.rank.cmp(&a.rank).then_with(|| a.title.cmp(&b.title)));
            }
            SortOption::Title => {
                filtered.sort_by(|a, b| a.title.cmp(&b.title));
            }
            SortOption::Date => {
                // Note: Document doesn't have a date field, so we use rank as proxy
                filtered.sort_by(|a, b| b.rank.cmp(&a.rank));
            }
            SortOption::Source => {
                filtered.sort_by(|a, b| {
                    a.source_haystack
                        .cmp(&b.source_haystack)
                        .then_with(|| a.title.cmp(&b.title))
                });
            }
        }

        self.filtered_results = filtered;
    }

    /// Render search results with all features
    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        // Header
        ui.label(egui::RichText::new("Search Results").heading().strong());

        ui.add_space(8.0);

        // Controls section
        self.render_controls(ui, state);

        ui.add_space(8.0);

        // Loading state
        if self.is_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Searching...");
            });
            return;
        }

        // Results count and selection info
        self.render_results_info(ui);

        ui.add_space(8.0);

        // Display message if no results
        if self.filtered_results.is_empty() {
            self.render_empty_state(ui);
            return;
        }

        // Virtual scroll area for results
        self.render_results_list(ui, state);
    }

    /// Render search controls (filters, sorting, actions)
    fn render_controls(&mut self, ui: &mut egui::Ui, state: &AppState) {
        let mut sort_changed = false;
        let mut source_filter_changed = false;
        let mut tag_filter_changed = false;

        // Prepare filter values
        let source_filter = self
            .filter_options
            .source_filter
            .clone()
            .unwrap_or_default();
        let tag_filter = self.filter_options.tag_filter.clone().unwrap_or_default();

        egui::Frame::default()
            .fill(ui.visuals().panel_fill)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Sort and filter options
                    ui.horizontal(|ui| {
                        // Sort dropdown
                        ui.label("Sort by:");
                        egui::ComboBox::from_id_source("sort_options")
                            .selected_text(format!("{:?}", self.sort_option))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.sort_option,
                                    SortOption::Relevance,
                                    "Relevance",
                                );
                                ui.selectable_value(
                                    &mut self.sort_option,
                                    SortOption::Title,
                                    "Title",
                                );
                                ui.selectable_value(
                                    &mut self.sort_option,
                                    SortOption::Date,
                                    "Date",
                                );
                                ui.selectable_value(
                                    &mut self.sort_option,
                                    SortOption::Source,
                                    "Source",
                                );
                            });

                        ui.add_space(16.0);

                        // Filter by source
                        ui.label("Source:");
                        let mut source_text = source_filter.clone();
                        let source_text_edit = egui::TextEdit::singleline(&mut source_text)
                            .hint_text("Filter by source...")
                            .desired_width(150.0);
                        let source_response = ui.add(source_text_edit);
                        if source_response.changed() {
                            self.filter_options.source_filter = if source_text.is_empty() {
                                None
                            } else {
                                Some(source_text)
                            };
                            source_filter_changed = true;
                        }

                        ui.add_space(16.0);

                        // Filter by tag
                        ui.label("Tag:");
                        let mut tag_text = tag_filter.clone();
                        let tag_text_edit = egui::TextEdit::singleline(&mut tag_text)
                            .hint_text("Filter by tag...")
                            .desired_width(150.0);
                        let tag_response = ui.add(tag_text_edit);
                        if tag_response.changed() {
                            self.filter_options.tag_filter = if tag_text.is_empty() {
                                None
                            } else {
                                Some(tag_text)
                            };
                            tag_filter_changed = true;
                        }
                    });

                    ui.add_space(8.0);

                    // Selection and action buttons
                    ui.horizontal(|ui| {
                        // Selection controls
                        if ui.button("✓ Select All").clicked() {
                            self.select_all();
                        }

                        if ui.button("⨯ Clear Selection").clicked() {
                            self.clear_selection();
                        }

                        ui.add_space(16.0);

                        // Batch actions
                        if ui
                            .button(format!(
                                "📋 Add Selected to Context ({})",
                                self.selected_indices.len()
                            ))
                            .clicked()
                        {
                            self.add_selected_to_context(state);
                        }

                        if ui.button("📤 Export Selected").clicked() {
                            self.export_selected();
                        }

                        ui.add_space(16.0);

                        // Refresh button
                        if ui.button("🔄 Refresh").clicked() {
                            self.update_results(state);
                        }
                    });
                });
            });

        // Apply filters when changed
        if sort_changed || source_filter_changed || tag_filter_changed {
            self.apply_filters_and_sort();
        }
    }

    /// Render results information
    fn render_results_info(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Showing {} of {} results",
                    self.filtered_results.len(),
                    self.results.len()
                ))
                .small()
                .weak(),
            );

            if !self.selected_indices.is_empty() {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(format!("{} selected", self.selected_indices.len()))
                        .small()
                        .strong(),
                );
            }

            if !self.search_query.is_empty() {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(format!("Query: \"{}\"", self.search_query))
                        .small()
                        .weak(),
                );
            }
        });
    }

    /// Render empty state
    fn render_empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.label(egui::RichText::new("📭").heading().size(48.0));
            ui.add_space(8.0);

            if self.results.is_empty() {
                ui.label(egui::RichText::new("No search results yet").weak());
                ui.label(
                    egui::RichText::new("Search feature will be fully implemented in Phase 3")
                        .small()
                        .weak(),
                );
            } else {
                ui.label(egui::RichText::new("No results match your filters").weak());
                if ui.button("Clear Filters").clicked() {
                    // Filters will be cleared externally
                }
            }
        });
    }

    /// Render results list with virtual scrolling
    fn render_results_list(&mut self, ui: &mut egui::Ui, state: &AppState) {
        let item_height = 100.0;

        // Use simple scroll area instead of virtual scrolling for now
        // Virtual scrolling will be implemented in a future enhancement
        egui::ScrollArea::vertical()
            .id_source("search_results_scroll")
            .max_height(500.0)
            .show(ui, |ui| {
                // Render all items (simple approach for now)
                for (idx, result) in self.filtered_results.iter().enumerate() {
                    let is_selected = self.selected_indices.contains(&idx);
                    self.render_result(ui, result, idx, is_selected, state);

                    // Add spacing between items
                    if idx < self.filtered_results.len() - 1 {
                        ui.add_space(4.0);
                    }
                }
            });
    }

    /// Render a single search result
    fn render_result(
        &self,
        ui: &mut egui::Ui,
        result: &Document,
        index: usize,
        is_selected: bool,
        state: &AppState,
    ) {
        let bg_color = if is_selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().panel_fill
        };

        egui::Frame::default()
            .fill(bg_color)
            .stroke(ui.visuals().window_stroke)
            .inner_margin(8.0)
            .show(ui, |ui| {
                // Header with selection checkbox and title
                ui.horizontal(|ui| {
                    // Selection checkbox
                    // Note: checkbox needs mutable reference, but we can't mutate self here
                    // Selection will be handled through buttons below
                    ui.label("☐");

                    // Title
                    if result.title.is_empty() {
                        ui.label(egui::RichText::new(&result.url).strong().heading());
                    } else {
                        ui.label(egui::RichText::new(&result.title).strong().heading());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Rank score
                        if let Some(rank) = result.rank {
                            ui.label(
                                egui::RichText::new(format!("Score: {}", rank))
                                    .small()
                                    .weak(),
                            );
                        }
                    });
                });

                ui.add_space(4.0);

                // Description or snippet
                if let Some(ref desc) = result.description {
                    ui.label(egui::RichText::new(desc).small());
                } else if !result.body.is_empty() {
                    // Show snippet from body
                    let snippet = if result.body.len() > 200 {
                        format!("{}...", &result.body[..200])
                    } else {
                        result.body.clone()
                    };
                    ui.label(egui::RichText::new(snippet).small().weak());
                }

                ui.add_space(4.0);

                // Metadata
                ui.horizontal(|ui| {
                    // Source
                    if let Some(ref source) = result.source_haystack {
                        ui.label(egui::RichText::new(format!("📁 {}", source)).small().weak());
                    }

                    // Tags
                    if let Some(ref tags) = result.tags {
                        if !tags.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(format!("🏷️ {}", tags.join(", ")))
                                    .small()
                                    .weak(),
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // URL (shown on hover)
                        if !result.url.is_empty() {
                            ui.label(egui::RichText::new("🔗").small().weak());
                        }
                    });
                });

                ui.add_space(4.0);

                // Actions
                ui.horizontal(|ui| {
                    // Add to Context
                    if ui
                        .button("📋 Add to Context")
                        .on_hover_text("Add this result to LLM context")
                        .clicked()
                    {
                        info!("Adding result to context: {}", result.id);
                        state.add_document_to_context(result.clone());
                    }

                    // Open Link
                    if ui
                        .button("🔗 Open Link")
                        .on_hover_text("Open the document URL")
                        .clicked()
                    {
                        info!("Opening result: {}", result.url);
                        // TODO: Implement cross-platform link opening
                    }

                    // Copy Content
                    if ui
                        .button("📄 Copy")
                        .on_hover_text("Copy content to clipboard")
                        .clicked()
                    {
                        // TODO: Implement clipboard copy
                        debug!("Copying result content to clipboard: {}", result.id);
                    }

                    // Copy URL
                    if ui
                        .button("🔗 Copy URL")
                        .on_hover_text("Copy URL to clipboard")
                        .clicked()
                    {
                        // TODO: Implement URL copy to clipboard
                        debug!("Copying result URL to clipboard: {}", result.url);
                    }
                });
            });
    }

    /// Select all filtered results
    pub fn select_all(&mut self) {
        self.selected_indices = (0..self.filtered_results.len()).collect();
        info!("Selected all {} results", self.selected_indices.len());
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_indices.clear();
        info!("Cleared selection");
    }

    /// Add selected results to context
    pub fn add_selected_to_context(&self, state: &AppState) {
        for &idx in &self.selected_indices {
            if let Some(result) = self.filtered_results.get(idx) {
                state.add_document_to_context(result.clone());
            }
        }
        info!("Added {} results to context", self.selected_indices.len());
    }

    /// Export selected results
    fn export_selected(&self) {
        // TODO: Implement export functionality
        warn!("Export functionality not yet implemented");
    }

    /// Get filtered results
    pub fn get_filtered_results(&self) -> &[Document] {
        &self.filtered_results
    }

    /// Get selected results
    pub fn get_selected_results(&self) -> Vec<&Document> {
        self.selected_indices
            .iter()
            .filter_map(|&idx| self.filtered_results.get(idx))
            .collect()
    }

    /// Toggle selection for a specific index
    pub fn toggle_selection(&mut self, index: usize) {
        if let Some(pos) = self.selected_indices.iter().position(|&i| i == index) {
            self.selected_indices.remove(pos);
        } else {
            self.selected_indices.push(index);
        }
    }

    /// Check if result at index is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_indices.contains(&index)
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filter_options = FilterOptions::default();
        self.sort_option = SortOption::Relevance;
        self.apply_filters_and_sort();
    }
}
