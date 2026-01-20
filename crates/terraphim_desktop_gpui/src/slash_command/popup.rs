//! GPUI Popup Component for Slash Commands
//!
//! This module provides the visual popup component that displays
//! slash command and KG autocomplete suggestions.

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::sync::Arc;

use super::providers::CompositeProvider;
use super::registry::CommandRegistry;
use super::trigger::{TriggerDetectionResult, TriggerEngine};
use super::types::*;
use crate::autocomplete::AutocompleteEngine;
use crate::theme::colors::theme;

/// Events emitted by the SlashCommandPopup
#[derive(Clone, Debug)]
pub enum SlashCommandPopupEvent {
    /// A suggestion was selected
    SuggestionSelected {
        suggestion: UniversalSuggestion,
        trigger: Option<TriggerInfo>,
    },
    /// The popup was closed
    Closed,
}

impl EventEmitter<SlashCommandPopupEvent> for SlashCommandPopup {}

/// Slash Command Popup State
pub struct SlashCommandPopup {
    /// Whether popup is visible
    is_open: bool,
    /// Current suggestions
    suggestions: Vec<UniversalSuggestion>,
    /// Selected index
    selected_index: usize,
    /// Current trigger info
    trigger_info: Option<TriggerInfo>,
    /// Trigger engine
    trigger_engine: TriggerEngine,
    /// Suggestion providers
    provider: Arc<CompositeProvider>,
    /// Command registry for execution
    registry: Arc<CommandRegistry>,
    /// Loading state
    is_loading: bool,
    /// Current view scope
    view_scope: ViewScope,
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
}

impl SlashCommandPopup {
    /// Create a new popup with default providers
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, view_scope: ViewScope) -> Self {
        let registry = Arc::new(CommandRegistry::with_builtin_commands());
        let provider = Arc::new(CompositeProvider::with_defaults(registry.clone(), None));
        let mut trigger_engine = TriggerEngine::new();
        trigger_engine.set_view(view_scope);

        Self {
            is_open: false,
            suggestions: Vec::new(),
            selected_index: 0,
            trigger_info: None,
            trigger_engine,
            provider,
            registry,
            is_loading: false,
            view_scope,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Create popup with custom providers
    pub fn with_providers(
        _window: &mut Window,
        cx: &mut Context<Self>,
        registry: Arc<CommandRegistry>,
        engine: Option<Arc<AutocompleteEngine>>,
        view_scope: ViewScope,
    ) -> Self {
        let provider = Arc::new(CompositeProvider::with_defaults(registry.clone(), engine));
        let mut trigger_engine = TriggerEngine::new();
        trigger_engine.set_view(view_scope);

        Self {
            is_open: false,
            suggestions: Vec::new(),
            selected_index: 0,
            trigger_info: None,
            trigger_engine,
            provider,
            registry,
            is_loading: false,
            view_scope,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Set the autocomplete engine
    pub fn set_autocomplete_engine(&mut self, engine: Arc<AutocompleteEngine>) {
        self.provider = Arc::new(CompositeProvider::with_defaults(
            self.registry.clone(),
            Some(engine),
        ));
    }

    /// Process input text and detect triggers
    pub fn process_input(&mut self, text: &str, cursor: usize, cx: &mut Context<Self>) {
        let result = self.trigger_engine.process_input(text, cursor);

        match result {
            TriggerDetectionResult::Triggered(trigger_info) => {
                log::debug!(
                    "Trigger detected: {:?} at {} query='{}'",
                    trigger_info.trigger_type,
                    trigger_info.start_position,
                    trigger_info.query
                );

                self.trigger_info = Some(trigger_info.clone());
                self.fetch_suggestions(trigger_info, cx);
            }
            TriggerDetectionResult::Cancelled => {
                log::debug!("Trigger cancelled");
                self.close(cx);
            }
            TriggerDetectionResult::InputChanged { .. } => {
                // No trigger, close if open
                if self.is_open {
                    self.close(cx);
                }
            }
            TriggerDetectionResult::None => {
                if self.is_open {
                    self.close(cx);
                }
            }
        }
    }

    /// Fetch suggestions for the current trigger
    fn fetch_suggestions(&mut self, trigger_info: TriggerInfo, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.is_open = true;
        cx.notify();

        let query = trigger_info.query.clone();
        let trigger = trigger_info.clone();
        let provider = self.provider.clone();

        cx.spawn(async move |this, cx| {
            // Get suggestions from provider
            let suggestions = provider.suggest(&query, &trigger, 10).await;

            this.update(cx, |popup, cx| {
                popup.suggestions = suggestions;
                popup.selected_index = 0;
                popup.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Select next suggestion
    pub fn select_next(&mut self, cx: &mut Context<Self>) {
        if !self.suggestions.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.suggestions.len();
            log::debug!("Select next: index={}", self.selected_index);
            cx.notify();
        }
    }

    /// Select previous suggestion
    pub fn select_previous(&mut self, cx: &mut Context<Self>) {
        if !self.suggestions.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.suggestions.len() - 1
            } else {
                self.selected_index - 1
            };
            log::debug!("Select previous: index={}", self.selected_index);
            cx.notify();
        }
    }

    /// Accept the currently selected suggestion
    pub fn accept_selected(&mut self, cx: &mut Context<Self>) -> Option<UniversalSuggestion> {
        if self.suggestions.is_empty() {
            return None;
        }

        let suggestion = self.suggestions.get(self.selected_index)?.clone();
        log::info!("Accepting suggestion: {}", suggestion.text);

        // Emit event
        cx.emit(SlashCommandPopupEvent::SuggestionSelected {
            suggestion: suggestion.clone(),
            trigger: self.trigger_info.clone(),
        });

        self.close(cx);
        Some(suggestion)
    }

    /// Accept suggestion at specific index
    pub fn accept_at_index(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<UniversalSuggestion> {
        if index < self.suggestions.len() {
            self.selected_index = index;
            self.accept_selected(cx)
        } else {
            None
        }
    }

    /// Close the popup
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.is_open {
            log::debug!("Closing slash command popup");
            self.is_open = false;
            self.suggestions.clear();
            self.selected_index = 0;
            self.trigger_info = None;
            self.trigger_engine.cancel_trigger();
            cx.emit(SlashCommandPopupEvent::Closed);
            cx.notify();
        }
    }

    /// Check if popup is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Get focus handle for external focus management
    pub fn focus_handle(&self, _cx: &Context<Self>) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Get current suggestions
    pub fn suggestions(&self) -> &[UniversalSuggestion] {
        &self.suggestions
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the replacement range for the current trigger
    pub fn get_replacement_range(&self, input_text: &str) -> Option<(usize, usize)> {
        self.trigger_engine.get_replacement_range(input_text)
    }

    /// Render a single suggestion item
    fn render_suggestion_item(
        &self,
        index: usize,
        suggestion: &UniversalSuggestion,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let is_selected = index == self.selected_index;
        let text = suggestion.text.clone();
        let description = suggestion.description.clone();
        let icon = suggestion.icon.clone();
        let from_kg = suggestion.from_kg;
        let category = suggestion.category;

        div()
            .id(("suggestion", index))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .w_full()
            .cursor_pointer()
            .when(is_selected, |d| {
                d.bg(theme::autocomplete_selected()).rounded_md()
            })
            .hover(|style| style.bg(theme::surface_hover()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _ev, _window, cx| {
                    log::info!("Suggestion clicked: index={}", index);
                    this.accept_at_index(index, cx);
                }),
            )
            .child(
                // Icon
                div()
                    .w_6()
                    .h_6()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(match icon {
                        CommandIcon::Emoji(emoji) => emoji,
                        CommandIcon::Named(_name) => "*".to_string(), // Fallback
                        CommandIcon::None => {
                            if from_kg {
                                "KG".to_string()
                            } else {
                                ">".to_string()
                            }
                        }
                    }),
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme::text_primary())
                                    .truncate()
                                    .child(text),
                            )
                            .when(from_kg, |d| {
                                d.child(
                                    div()
                                        .text_xs()
                                        .text_color(theme::text_secondary())
                                        .bg(theme::surface())
                                        .px_1()
                                        .rounded_sm()
                                        .child("KG"),
                                )
                            }),
                    )
                    .when_some(description, |d, desc| {
                        d.child(
                            div()
                                .text_sm()
                                .text_color(theme::text_secondary())
                                .truncate()
                                .child(desc),
                        )
                    }),
            )
            .when_some(category, |d, cat| {
                d.child(
                    div()
                        .text_xs()
                        .text_color(theme::text_secondary())
                        .child(format!("{}", cat)),
                )
            })
    }

    /// Render the popup content
    fn render_popup(&self, cx: &Context<Self>) -> impl IntoElement {
        let suggestions = &self.suggestions;
        let is_loading = self.is_loading;
        let suggestion_count = suggestions.len();

        div()
            .absolute()
            .left_0()
            .w_full()
            .max_w(px(400.0))
            .max_h(px(300.0))
            .mt_1()
            .bg(theme::background())
            .border_1()
            .border_color(theme::border())
            .rounded_lg()
            .shadow_lg()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .max_h(px(250.0))
                    .overflow_hidden()
                    .when(is_loading, |d| {
                        d.child(
                            div()
                                .p_4()
                                .text_center()
                                .text_color(theme::text_secondary())
                                .child("Loading..."),
                        )
                    })
                    .when(!is_loading && suggestions.is_empty(), |d| {
                        d.child(
                            div()
                                .p_4()
                                .text_center()
                                .text_color(theme::text_secondary())
                                .child("No suggestions"),
                        )
                    })
                    .when(!is_loading && !suggestions.is_empty(), |d| {
                        d.children(suggestions.iter().enumerate().map(|(idx, suggestion)| {
                            self.render_suggestion_item(idx, suggestion, cx)
                        }))
                    }),
            )
            .child(
                // Footer with keyboard hints
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_1()
                    .border_t_1()
                    .border_color(theme::border())
                    .bg(theme::surface())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_xs()
                            .text_color(theme::text_secondary())
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child("↑↓")
                                    .child("navigate"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child("⏎")
                                    .child("select"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child("esc")
                                    .child("close"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::text_secondary())
                            .child(format!("{} results", suggestion_count)),
                    ),
            )
    }
}

impl Render for SlashCommandPopup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open {
            return div().into_any_element();
        }

        div()
            .relative()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                log::debug!("SlashCommandPopup key_down: {:?}", ev.keystroke.key);
                match ev.keystroke.key.as_str() {
                    "down" => {
                        this.select_next(cx);
                    }
                    "up" => {
                        this.select_previous(cx);
                    }
                    "enter" | "tab" => {
                        this.accept_selected(cx);
                    }
                    "escape" => {
                        this.close(cx);
                    }
                    _ => {}
                }
            }))
            .child(self.render_popup(cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GPUI tests require the gpui test harness which may have issues
    // These tests verify the basic logic without GPUI context

    #[test]
    fn test_suggestion_action_types() {
        let insert_action = SuggestionAction::Insert {
            text: "hello".to_string(),
            replace_trigger: true,
        };

        let exec_action = SuggestionAction::ExecuteCommand {
            command_id: "search".to_string(),
            args: Some("rust".to_string()),
        };

        // Verify action types exist and can be constructed
        match insert_action {
            SuggestionAction::Insert {
                text,
                replace_trigger,
            } => {
                assert_eq!(text, "hello");
                assert!(replace_trigger);
            }
            _ => panic!("Expected Insert action"),
        }

        match exec_action {
            SuggestionAction::ExecuteCommand { command_id, args } => {
                assert_eq!(command_id, "search");
                assert_eq!(args, Some("rust".to_string()));
            }
            _ => panic!("Expected ExecuteCommand action"),
        }
    }

    #[test]
    fn test_universal_suggestion_creation() {
        let suggestion = UniversalSuggestion::from_kg_term(
            "rust".to_string(),
            0.95,
            Some("https://rust-lang.org".to_string()),
        );

        assert_eq!(suggestion.text, "rust");
        assert_eq!(suggestion.score, 0.95);
        assert!(suggestion.from_kg);
    }
}
