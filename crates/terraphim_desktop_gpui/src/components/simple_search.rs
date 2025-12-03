/// Simplified Search Component
///
/// GPUI-aligned search component based on gpui-component patterns.
/// Replaces the complex ReusableComponent implementation with a simpler,
/// more maintainable approach that follows GPUI best practices.

use gpui::*;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use crate::components::gpui_aligned::{GpuiComponentConfig, CommonProps, GpuiComponent, StatefulComponent};
use crate::search_service::SearchResults;
use crate::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
use crate::security::validate_search_query;

/// Search component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSearchConfig {
    pub placeholder: String,
    pub max_results: usize,
    pub max_autocomplete_suggestions: usize,
    pub show_suggestions: bool,
    pub auto_search: bool,
    pub autocomplete_debounce_ms: u64,
    pub enable_fuzzy_search: bool,
    pub common_props: CommonProps,
}

impl Default for SimpleSearchConfig {
    fn default() -> Self {
        Self {
            placeholder: "Search documents...".to_string(),
            max_results: 10,
            max_autocomplete_suggestions: 8,
            show_suggestions: true,
            auto_search: false,
            autocomplete_debounce_ms: 200,
            enable_fuzzy_search: true,
            common_props: CommonProps::default(),
        }
    }
}

impl GpuiComponentConfig for SimpleSearchConfig {
    fn validate(&self) -> Result<(), String> {
        if self.placeholder.is_empty() {
            return Err("placeholder cannot be empty".to_string());
        }
        if self.max_results == 0 {
            return Err("max_results must be greater than 0".to_string());
        }
        self.common_props.validate()?;
        Ok(())
    }

    fn default() -> Self {
        Self::default()
    }

    fn merge_with(&self, other: &Self) -> Self {
        Self {
            placeholder: other.placeholder.clone(),
            max_results: other.max_results,
            show_suggestions: other.show_suggestions,
            auto_search: other.auto_search,
            common_props: self.common_props.merge_with(&other.common_props),
        }
    }
}

/// Search component state
#[derive(Debug, Clone)]
pub struct SimpleSearchState {
    pub query: String,
    pub results: Option<SearchResults>,
    pub loading: bool,
    pub autocomplete_loading: bool,
    pub error: Option<String>,
    pub autocomplete_suggestions: Vec<AutocompleteSuggestion>,
    pub selected_suggestion_index: Option<usize>,
    pub last_autocomplete_query: String,
}

impl Default for SimpleSearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: None,
            loading: false,
            autocomplete_loading: false,
            error: None,
            autocomplete_suggestions: Vec::new(),
            selected_suggestion_index: None,
            last_autocomplete_query: String::new(),
        }
    }
}

/// Search component events
#[derive(Debug, Clone)]
pub enum SimpleSearchEvent {
    QueryChanged(String),
    SearchRequested(String),
    AutocompleteRequested(String),
    AutocompleteSuggestionSelected(usize),
    ClearRequested,
    NavigateUp,
    NavigateDown,
    SelectCurrentSuggestion,
}

/// Simple search component implementation
pub struct SimpleSearchComponent {
    config: SimpleSearchConfig,
    state: SimpleSearchState,
    autocomplete_engine: Option<AutocompleteEngine>,
    debounce_timer: Option<Instant>,
}

impl GpuiComponent for SimpleSearchComponent {
    type State = SimpleSearchState;
    type Config = SimpleSearchConfig;
    type Event = SimpleSearchEvent;

    fn component_name() -> &'static str {
        "SimpleSearch"
    }

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            state: SimpleSearchState::default(),
            autocomplete_engine: None, // Will be set when available
            debounce_timer: None,
        }
    }

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {

        let div_element = div().id("simple-search");
        // Note: test_id method may not exist on Div in this GPUI version
        // if let Some(ref test_id) = self.config.common_props.test_id {
        //     div_element = div_element.test_id(test_id);
        // }
        div_element
            .child(
                // Search input field
                div()
                    .id("search-input-container")
                    .child(
                        // Use standard GPUI input for now
                        // TODO: Replace with gpui_component::input when available
                        div()
                            .id("search-input")
                            .child(
                                if !self.state.query.is_empty() {
                                    div().child(format!("Query: {}", self.state.query))
                                } else {
                                    div().child(self.config.placeholder.clone())
                                }
                            )
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                // For now, simulate typing - would need proper input implementation
                                let test_query = "example search";
                                this.handle_event(SimpleSearchEvent::QueryChanged(test_query.to_string()), cx);
                                this.handle_event(SimpleSearchEvent::SearchRequested(test_query.to_string()), cx);
                            }))
                    )
            )
            .children(self.render_autocomplete_suggestions(cx))
            .children(self.render_loading_indicator())
            .children(self.render_error_message())
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn state(&self) -> &Self::State {
        &self.state
    }

    fn handle_event(&mut self, event: Self::Event, cx: &mut Context<Self>) {
        match event {
            SimpleSearchEvent::QueryChanged(query) => {
                self.state.query = query.clone();

                // Clear previous results when query changes
                if query.is_empty() {
                    self.state.results = None;
                    self.state.autocomplete_suggestions.clear();
                    self.state.selected_suggestion_index = None;
                    self.state.last_autocomplete_query.clear();
                    return;
                }

                // Trigger autocomplete with debouncing
                if self.config.show_suggestions {
                    let now = Instant::now();
                    if let Some(timer) = self.debounce_timer {
                        if now.duration_since(timer).as_millis() >= self.config.autocomplete_debounce_ms {
                            self.debounce_timer = Some(now);
                            self.handle_event(SimpleSearchEvent::AutocompleteRequested(query.clone()), cx);
                        }
                    } else {
                        self.debounce_timer = Some(now);
                        self.handle_event(SimpleSearchEvent::AutocompleteRequested(query.clone()), cx);
                    }
                }
            }
            SimpleSearchEvent::AutocompleteRequested(query) => {
                // Avoid duplicate autocomplete requests
                if query == self.state.last_autocomplete_query || query.len() < 2 {
                    return;
                }

                self.state.autocomplete_loading = true;
                self.state.last_autocomplete_query = query.clone();

                // Perform autocomplete search
                self.perform_autocomplete(query, cx);
            }
            SimpleSearchEvent::AutocompleteSuggestionSelected(index) => {
                if let Some(suggestion) = self.state.autocomplete_suggestions.get(index) {
                    self.state.query = suggestion.term.clone();
                    self.state.autocomplete_suggestions.clear();
                    self.state.selected_suggestion_index = None;

                    // Auto-trigger search when suggestion is selected
                    self.handle_event(SimpleSearchEvent::SearchRequested(suggestion.term.clone()), cx);
                }
            }
            SimpleSearchEvent::SearchRequested(query) => {
                if let Err(e) = validate_search_query(&query) {
                    self.state.error = Some(format!("Invalid query: {}", e));
                    return;
                }

                self.state.loading = true;
                self.state.error = None;
                self.state.autocomplete_suggestions.clear(); // Clear autocomplete when searching

                // Trigger async search
                self.perform_search(query, cx);
            }
            SimpleSearchEvent::NavigateUp => {
                if let Some(selected) = self.state.selected_suggestion_index {
                    if selected > 0 {
                        self.state.selected_suggestion_index = Some(selected - 1);
                    }
                } else if !self.state.autocomplete_suggestions.is_empty() {
                    self.state.selected_suggestion_index = Some(self.state.autocomplete_suggestions.len() - 1);
                }
            }
            SimpleSearchEvent::NavigateDown => {
                if let Some(selected) = self.state.selected_suggestion_index {
                    if selected < self.state.autocomplete_suggestions.len() - 1 {
                        self.state.selected_suggestion_index = Some(selected + 1);
                    } else {
                        self.state.selected_suggestion_index = None;
                    }
                } else if !self.state.autocomplete_suggestions.is_empty() {
                    self.state.selected_suggestion_index = Some(0);
                }
            }
            SimpleSearchEvent::SelectCurrentSuggestion => {
                if let Some(selected) = self.state.selected_suggestion_index {
                    self.handle_event(SimpleSearchEvent::AutocompleteSuggestionSelected(selected), cx);
                }
            }
            SimpleSearchEvent::ClearRequested => {
                self.state.query.clear();
                self.state.results = None;
                self.state.autocomplete_suggestions.clear();
                self.state.selected_suggestion_index = None;
                self.state.error = None;
                self.state.last_autocomplete_query.clear();
            }
        }
    }
}

impl SimpleSearchComponent {
    /// Render autocomplete suggestions dropdown
    fn render_autocomplete_suggestions(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        if !self.config.show_suggestions || self.state.autocomplete_suggestions.is_empty() {
            return None;
        }

        Some(
            div()
                .id("autocomplete-suggestions")
                .children(
                    self.state.autocomplete_suggestions.iter().enumerate().map(|(idx, suggestion)| {
                        let is_selected = self.state.selected_suggestion_index == Some(idx);
                        let mut suggestion_div = div().id(("autocomplete-suggestion", idx));
                        if is_selected {
                            suggestion_div = suggestion_div.bg(gpui::blue().opacity(0.1));
                        }
                        suggestion_div
                            .child(
                                div()
                                    .id(("suggestion-content", idx))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        // Show icon for KG terms
                                        if suggestion.from_kg {
                                            div()
                                                .id(("kg-icon", idx))
                                                .text_size(px(12.0))
                                                .text_color(gpui::blue())
                                                .child("📚")
                                        } else {
                                            div()
                                                .id(("search-icon", idx))
                                                .text_size(px(12.0))
                                                .text_color(gpui::rgb(0x6b7280))
                                                .child("🔍")
                                        }
                                    )
                                    .child(
                                        div()
                                            .id(("suggestion-text", idx))
                                            .flex_1()
                                            .child(suggestion.term.clone())
                                    )
                                    .children(
                                        if suggestion.score > 0.5 {
                                            Some(
                                                div()
                                                    .id(("suggestion-score", idx))
                                                    .text_size(px(10.0))
                                                    .text_color(gpui::rgb(0x6b7280))
                                                    .child(format!("{:.2}", suggestion.score))
                                            )
                                        } else {
                                            None
                                        }
                                    )
                            )
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.handle_event(SimpleSearchEvent::AutocompleteSuggestionSelected(idx), cx);
                            }))
                    })
                )
        )
    }

    /// Render loading indicator
    fn render_loading_indicator(&self) -> Option<impl IntoElement> {
        if !self.state.loading {
            return None;
        }

        Some(
            div()
                .id("loading-indicator")
                .child("Loading...")
        )
    }

    /// Render error message
    fn render_error_message(&self) -> Option<impl IntoElement> {
        if let Some(error) = &self.state.error {
            Some(
                div()
                    .id("error-message")
                    .child(error.clone())
            )
        } else {
            None
        }
    }

    /// Set the autocomplete engine
    pub fn with_autocomplete_engine(mut self, engine: AutocompleteEngine) -> Self {
        self.autocomplete_engine = Some(engine);
        self
    }

    /// Perform autocomplete search
    fn perform_autocomplete(&mut self, query: String, _cx: &mut Context<Self>) {
        if let Some(ref engine) = self.autocomplete_engine {
            let suggestions = if self.config.enable_fuzzy_search {
                engine.fuzzy_search(&query, self.config.max_autocomplete_suggestions)
            } else {
                engine.autocomplete(&query, self.config.max_autocomplete_suggestions)
            };

            self.state.autocomplete_loading = false;
            self.state.autocomplete_suggestions = suggestions;
            self.state.selected_suggestion_index = if suggestions.is_empty() { None } else { Some(0) };
        } else {
            // No autocomplete engine available - create mock suggestions
            self.state.autocomplete_loading = false;
            self.state.autocomplete_suggestions = vec![
                AutocompleteSuggestion {
                    term: format!("{} - recent search", query),
                    nterm: query.clone(),
                    score: 0.9,
                    from_kg: false,
                    definition: None,
                    url: None,
                },
                AutocompleteSuggestion {
                    term: format!("{} in documents", query),
                    nterm: format!("{} documents", query),
                    score: 0.7,
                    from_kg: false,
                    definition: None,
                    url: None,
                },
            ];
            self.state.selected_suggestion_index = Some(0);
        }
    }

    /// Perform search (simplified for now)
    fn perform_search(&mut self, query: String, _cx: &mut Context<Self>) {
        // For now, perform synchronous search
        // TODO: Integrate with actual SearchService and make async

        // Simulate search delay
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Mock search results
        let results = SearchResults {
            documents: vec![],
            total_count: 0,
            query: query.clone(),
            duration_ms: 100,
        };

        self.state.loading = false;
        self.state.results = Some(results);

        // TODO: In real implementation, this would be async:
        // let search_service = SearchService::new();
        // let options = SearchOptions::default().with_max_results(self.config.max_results);
        // let future = search_service.search(&query, options);
        // cx.spawn(future).detach(...)
    }

    /// Public method to update autocomplete engine
    pub fn set_autocomplete_engine(&mut self, engine: AutocompleteEngine) {
        self.autocomplete_engine = Some(engine);
    }
}

/// Convenience function to create a simple search component
pub fn simple_search(config: SimpleSearchConfig) -> SimpleSearchComponent {
    SimpleSearchComponent::new(config)
}

/// Convenience function to create a simple search component with autocomplete
pub fn simple_search_with_autocomplete(
    config: SimpleSearchConfig,
    autocomplete_engine: AutocompleteEngine,
) -> SimpleSearchComponent {
    SimpleSearchComponent::new(config).with_autocomplete_engine(autocomplete_engine)
}

/// Hook-like function for using the search component in views
pub fn use_simple_search(config: SimpleSearchConfig) -> StatefulComponent<SimpleSearchComponent> {
    StatefulComponent::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestContext;

    #[test]
    fn test_simple_search_config_validation() {
        let mut config = SimpleSearchConfig::default();
        assert!(config.validate().is_ok());

        config.placeholder = "".to_string();
        assert!(config.validate().is_err());

        config = SimpleSearchConfig::default();
        config.max_results = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_simple_search_component_creation() {
        let config = SimpleSearchConfig::default();
        let component = SimpleSearchComponent::new(config);

        assert_eq!(component.component_name(), "SimpleSearch");
        assert_eq!(component.state().query, "");
        assert!(component.state().results.is_none());
        assert!(!component.state().loading);
    }

    #[test]
    fn test_simple_search_event_handling() {
        let mut cx = TestContext::new();
        let config = SimpleSearchConfig::default();
        let mut component = SimpleSearchComponent::new(config);

        // Test query change event
        component.handle_event(SimpleSearchEvent::QueryChanged("test".to_string()), &mut cx);
        assert_eq!(component.state().query, "test");

        // Test clear event
        component.handle_event(SimpleSearchEvent::ClearRequested, &mut cx);
        assert_eq!(component.state().query, "");
        assert!(component.state().results.is_none());
    }

    #[test]
    fn test_search_query_validation() {
        assert!(validate_search_query("valid search query").is_ok());
        assert!(validate_search_query("").is_err()); // Empty
        assert!(validate_search_query("javascript:alert(1)").is_err()); // Dangerous pattern
    }

    #[test]
    fn test_simple_search_config_merge() {
        let config1 = SimpleSearchConfig {
            placeholder: "Search...".to_string(),
            max_results: 5,
            show_suggestions: true,
            auto_search: false,
            common_props: CommonProps {
                size: ComponentSize::Small,
                variant: ComponentVariant::Primary,
                disabled: false,
                test_id: Some("search1".to_string()),
            },
        };

        let config2 = SimpleSearchConfig {
            placeholder: "New placeholder".to_string(),
            max_results: 10,
            show_suggestions: false,
            auto_search: true,
            common_props: CommonProps {
                size: ComponentSize::Large,
                variant: ComponentVariant::Secondary,
                disabled: true,
                test_id: None,
            },
        };

        let merged = config1.merge_with(&config2);
        assert_eq!(merged.placeholder, "New placeholder");
        assert_eq!(merged.max_results, 10);
        assert_eq!(merged.show_suggestions, false);
        assert_eq!(merged.auto_search, true);
        assert_eq!(merged.common_props.size, ComponentSize::Large);
        assert_eq!(merged.common_props.variant, ComponentVariant::Secondary);
        assert_eq!(merged.common_props.disabled, true);
        assert_eq!(merged.common_props.test_id, Some("search1".to_string()));
    }
}