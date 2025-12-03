use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{IconName, StyledExt};
use std::sync::Arc;
use terraphim_types::{RoleName, Document};
use crate::kg_search::{KGSearchService, KGTerm, KGSearchResult};
use crate::theme::colors::theme;

/// Knowledge Graph search modal
pub struct KGSearchModal {
    input_state: Entity<InputState>,
    kg_search_service: KGSearchService,
    role_name: RoleName,
    conversation_id: Option<String>,

    // Search state
    is_searching: bool,
    search_error: Option<String>,

    // Results state
    suggestions: Vec<KGTerm>,
    selected_suggestion: Option<KGTerm>,

    // Autocomplete state
    autocomplete_suggestions: Vec<String>,
    suggestion_index: isize,

    _subscriptions: Vec<Subscription>,
}

impl KGSearchModal {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, role_name: RoleName, conversation_id: Option<String>, kg_search_service: KGSearchService) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search knowledge graph terms...")
                .with_icon(IconName::Search)
        });

        let kg_search_clone = kg_search_service.clone();
        let role_name_clone = role_name.clone();
        let input_state_clone = input_state.clone();

        // Subscribe to input changes for search
        let search_sub = cx.subscribe_in(&input_state, window, move |this, _, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => {
                    let query = input_state_clone.read(cx).value();

                    // Clear previous results and search if query has 2+ characters
                    this.update(cx, |this, cx| {
                        this.autocomplete_suggestions.clear();
                        this.suggestion_index = -1;
                        if query.trim().len() >= 2 {
                            this.is_searching = true;
                            this.search_error = None;
                            cx.notify();

                            // Perform actual KG search
                            this.perform_kg_search(query.clone(), cx);
                        } else {
                            this.is_searching = false;
                            this.suggestions.clear();
                            this.selected_suggestion = None;
                            cx.notify();
                        }
                    }).ok();
                }
                _ => {}
            }
        });

        Self {
            input_state,
            kg_search_service,
            role_name,
            conversation_id,
            is_searching: false,
            search_error: None,
            suggestions: Vec::new(),
            selected_suggestion: None,
            autocomplete_suggestions: Vec::new(),
            suggestion_index: -1,
            _subscriptions: vec![search_sub],
        }
    }

    /// Perform KG search
    fn perform_kg_search(&mut self, query: String, cx: &mut Context<Self>) {
        let kg_service = self.kg_search_service.clone();
        let role_name = self.role_name.clone();

        cx.spawn(async move |this, cx| {
            log::info!("Performing KG search for query: '{}' in role: {}", query, role_name);

            // First, try to find exact KG term
            let results = match kg_service.get_kg_term_from_thesaurus(&role_name, &query) {
                Ok(Some(kg_term)) => {
                    // Found exact KG term, now get related documents
                    log::info!("Found KG term: {}", kg_term.term);

                    let documents_result = kg_service.search_kg_term_ids(&role_name, &kg_term.term).unwrap_or_default();

                    let documents: Vec<Document> = documents_result
                        .into_iter()
                        .filter_map(|doc_id| {
                            match kg_service.get_document(&role_name, &doc_id) {
                                Ok(Some(indexed_doc)) => Some(Document {
                                    id: indexed_doc.id.clone(),
                                    title: indexed_doc.title.clone(),
                                    body: indexed_doc.body.clone(),
                                    description: indexed_doc.summary.clone(),
                                    url: indexed_doc.url.clone(),
                                    rank: Some(1.0),
                                    tags: indexed_doc.tags.clone(),
                                }),
                                Ok(None) => None,
                                Err(e) => {
                                    log::warn!("Failed to get document {}: {}", doc_id, e);
                                    None
                                }
                            }
                        })
                        .collect();

                    vec![KGSearchResult {
                        term: kg_term,
                        documents,
                        related_terms: vec![],
                    }]
                }
                Ok(None) => {
                    // Search for related terms instead (fuzzy search simulation)
                    log::info!("No exact KG term found for '{}'", query);
                    vec![]
                }
                Err(e) => {
                    log::error!("KG search error: {}", e);
                    vec![]
                }
            };

            // Update UI with results
            this.update(cx, |this, cx| {
                this.is_searching = false;
                this.suggestions = results.into_iter().map(|r| r.term).collect();

                // Auto-select first suggestion if available
                if let Some(first_suggestion) = self.suggestions.first() {
                    this.selected_suggestion = Some(first_suggestion.clone());
                }

                if self.suggestions.is_empty() && query.trim().len() >= 2 {
                    self.search_error = Some(format!("No knowledge graph terms found for '{}'", query));
                } else {
                    self.search_error = None;
                }

                cx.notify();
            }).ok();
        }).detach();
    }

    /// Select a suggestion
    fn select_suggestion(&mut self, index: isize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= 0 && index < self.autocomplete_suggestions.len() as isize {
            let suggestion = self.autocomplete_suggestions[index as usize].clone();

            let input_state = self.input_state.clone();

            // Update input field with selected suggestion
            input_state.update(cx, |input, cx| {
                input.set_value(gpui::SharedString::from(suggestion.clone()), window, cx);
            });

            self.autocomplete_suggestions.clear();
            self.suggestion_index = -1;

            // Trigger search for selected suggestion
            self.perform_kg_search(suggestion, cx);
            cx.notify();
        }
    }

    /// Add selected term to context
    pub fn add_term_to_context(&mut self, cx: &mut Context<Self>) -> Option<KGTerm> {
        let selected_term = self.selected_suggestion.clone();

        if let Some(term) = selected_term {
            log::info!("Adding KG term to context: {} (URL: {})", term.term, term.url);

            // This would emit an event to the ChatView to add the term to context
            // For now, we'll return the term and let the caller handle the actual context addition
            self.selected_suggestion = None;
            cx.notify();
            Some(term)
        } else {
            log::warn!("No KG term selected to add to context");
            cx.notify();
            None
        }
    }

    /// Close the modal
    pub fn close(&mut self, cx: &mut Context<Self>) {
        cx.emit(KGSearchModalEvent::Closed);
    }

    /// Check if autocomplete dropdown should be shown
    pub fn should_show_autocomplete(&self) -> bool {
        !self.autocomplete_suggestions.is_empty() && self.suggestion_index == -1
    }

    /// Get current input value
    pub fn get_query(&self, cx: &Context<Self>) -> String {
        self.input_state.read(cx).value().to_string()
    }

    /// Check if there are any suggestions
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }

    /// Get selected suggestion
    pub fn get_selected_suggestion(&self) -> Option<&KGTerm> {
        self.selected_suggestion.as_ref()
    }

    /// Check if modal is in searching state
    pub fn is_searching(&self) -> bool {
        self.is_searching
    }

    /// Get search error
    pub fn get_search_error(&self) -> Option<&str> {
        self.search_error.as_deref()
    }
}

/// Events emitted by KGSearchModal
#[derive(Clone, Debug)]
pub enum KGSearchModalEvent {
    Closed,
    TermAddedToContext(KGTerm),
}

impl EventEmitter<KGSearchModalEvent> for KGSearchModal {}

impl Render for KGSearchModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .w(px(600.0))
            .max_h(px(80.0 * 16.0)) // 80vh
            .bg(theme::background())
            .border_2()
            .border_color(theme::border())
            .rounded_lg()
            .shadow_xl()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(
                        // Header with close button
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .border_b_1()
                            .border_color(theme::border())
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(theme::text_primary())
                                    .child("Knowledge Graph Search"),
                            )
                            .child(
                                Button::new("close-kg-modal")
                                    .icon(IconName::Delete)
                                    .ghost()
                                    .on_click(cx.listener(|this, _ev, _window, cx| {
                                        this.close(cx);
                                    })),
                            ),
                    )
                    .child(
                        // Search section
                        div()
                            .px_6()
                            .py_4()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme::text_secondary())
                                    .mb_3()
                                    .child("Search and add terms from the knowledge graph to your context"),
                            )
                            .child(
                                // Input field
                                div()
                                    .relative()
                                    .child(
                                        Input::new(&self.input_state)
                                            .when(self.is_searching(), |input| input.disabled(true)),
                                    ),
                            ),
                    )
            )
            .child(
                // Content area
                div()
                    .flex_1()
                    .px_6()
                    .pb_6()
                    .overflow_hidden()  // Use overflow_hidden instead of overflow_y_scroll
                    .child(
                        if self.is_searching {
                            // Loading state
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py_8()
                                .child(
                                    div()
                                        .w_8()
                                        .h_8()
                                        .border_2()
                                        .border_t_4()
                                        .border_color(theme::primary())
                                        .rounded_full(),
                                        // TODO: Add spinner animation when GPUI supports it
                                )
                                .child(
                                    div()
                                        .mt_4()
                                        .text_color(theme::text_secondary())
                                        .child("Searching knowledge graph..."),
                                )
                        } else if self.suggestions.is_empty() && self.get_query(cx).trim().len() >= 2 {
                            // No results state
                            div()
                                .flex()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py_8()
                                .text_color(theme::text_secondary())
                                .child("No knowledge graph terms found")
                        } else if self.get_query(cx).trim().len() >= 2 {
                            // Results list (simplified - add full implementation later)
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                        } else {
                            // Initial state
                            div()
                                .flex()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .py_8()
                                .text_color(theme::text_secondary())
                                .child("Enter at least 2 characters to search the knowledge graph")
                        }
                    ),
            )
            .child(
                // Error message
                if let Some(error) = self.get_search_error(cx) {
                    div()
                        .px_6()
                        .pb_4()
                        .text_color(theme::danger())
                        .child(error)
                }
            )
            .child(
                // Action buttons
                div()
                    .px_6()
                    .pb_6()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("cancel-kg-search")
                            .label("Cancel")
                            .on_click(cx.listener(|this, _ev, _window, cx| {
                                this.close(cx);
                            })),
                    )
                    .when(self.get_selected_suggestion().is_some(), |this| {
                        this.child(
                            Button::new("add-to-context")
                                .label(format!("Add '{}' to Context", self.get_selected_suggestion().unwrap().term))
                                .primary()
                                .on_click(cx.listener(|this, _ev, _window, cx| {
                                    if let Some(term) = this.add_term_to_context(cx) {
                                        cx.emit(KGSearchModalEvent::TermAddedToContext(term));
                                        this.close(cx);
                                    }
                                })),
                        )
                    }),
            )
    }
}
