use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use std::sync::Arc;

use crate::slash_command::{
    CommandContext, CommandRegistry, SlashCommandPopup, SlashCommandPopupEvent, SuggestionAction,
    ViewScope, replace_text_range,
};
use crate::state::search::{AutocompleteSuggestion, SearchState};
use crate::theme::colors::theme;

/// Search input component with real-time autocomplete and slash commands
pub struct SearchInput {
    input_state: Entity<InputState>,
    search_state: Entity<SearchState>,
    show_autocomplete_dropdown: bool,
    suppress_autocomplete: bool,
    /// Slash command popup for Search-scoped commands
    slash_command_popup: Entity<SlashCommandPopup>,
    _subscriptions: Vec<Subscription>,
}

impl SearchInput {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_state: Entity<SearchState>,
        command_registry: Arc<CommandRegistry>,
    ) -> Self {
        let input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search knowledge graph..."));

        // Create slash command popup for Search scope
        let popup_registry = command_registry.clone();
        let slash_command_popup = cx.new(|cx| {
            SlashCommandPopup::with_providers(window, cx, popup_registry, None, ViewScope::Search)
        });

        // Subscribe to slash command popup events
        let search_state_for_slash = search_state.clone();
        let registry_for_slash = command_registry.clone();
        let slash_sub = cx.subscribe_in(
            &slash_command_popup,
            window,
            move |this, _popup, event: &SlashCommandPopupEvent, window, cx| match event {
                SlashCommandPopupEvent::SuggestionSelected {
                    suggestion,
                    trigger,
                } => {
                    log::info!("Search slash command selected: {}", suggestion.text);
                    this.handle_slash_suggestion(
                        suggestion.clone(),
                        trigger.as_ref(),
                        &search_state_for_slash,
                        &registry_for_slash,
                        window,
                        cx,
                    );
                }
                SlashCommandPopupEvent::Closed => {
                    log::debug!("Search slash command popup closed");
                }
            },
        );

        // Subscribe to input changes for autocomplete and slash commands
        let search_state_clone = search_state.clone();
        let input_state_clone = input_state.clone();
        let slash_popup_clone = slash_command_popup.clone();
        let autocomplete_sub = cx.subscribe_in(
            &input_state,
            window,
            move |this, _, ev: &InputEvent, _window, cx| {
                match ev {
                    InputEvent::Change => {
                        // Skip autocomplete if we just programmatically set the value
                        if this.suppress_autocomplete {
                            log::debug!("Suppressing autocomplete (programmatic value update)");
                            this.suppress_autocomplete = false;
                            return;
                        }

                        let value = input_state_clone.read(cx).value();

                        // Check for slash command trigger
                        let is_slash_command = value.starts_with('/');

                        if is_slash_command {
                            // Process slash command
                            slash_popup_clone.update(cx, |popup, cx| {
                                popup.process_input(&value, value.len(), cx);
                            });
                            this.show_autocomplete_dropdown = false;
                        } else {
                            // Close slash popup if not a command
                            slash_popup_clone.update(cx, |popup, cx| {
                                popup.close(cx);
                            });

                            // Trigger KG autocomplete for regular text
                            search_state_clone.update(cx, |state, cx| {
                                state.get_autocomplete(value.to_string(), cx);
                            });

                            // Update dropdown visibility
                            this.show_autocomplete_dropdown = value.len() >= 2;
                        }
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => {
                        // Check if slash popup is open
                        let slash_open = slash_popup_clone.read(cx).is_open();

                        if slash_open {
                            slash_popup_clone.update(cx, |popup, cx| {
                                popup.accept_selected(cx);
                            });
                        } else {
                            // Trigger search on Enter
                            let value = input_state_clone.read(cx).value();
                            search_state_clone.update(cx, |state, cx| {
                                state.search(value.to_string(), cx);
                                state.clear_autocomplete(cx); // Clear autocomplete after search
                            });
                            this.show_autocomplete_dropdown = false;
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            },
        );

        Self {
            input_state,
            search_state,
            show_autocomplete_dropdown: false,
            suppress_autocomplete: false,
            slash_command_popup,
            _subscriptions: vec![autocomplete_sub, slash_sub],
        }
    }

    fn handle_slash_suggestion(
        &mut self,
        suggestion: crate::slash_command::UniversalSuggestion,
        trigger: Option<&crate::slash_command::TriggerInfo>,
        search_state: &Entity<SearchState>,
        registry: &Arc<CommandRegistry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (input_text, cursor) = {
            let input = self.input_state.read(cx);
            (input.value().to_string(), input.cursor())
        };
        let input_len = input_text.len();

        match &suggestion.action {
            SuggestionAction::Insert {
                text,
                replace_trigger,
            } => {
                let range = if *replace_trigger {
                    trigger
                        .map(|info| info.replacement_range(input_len))
                        .unwrap_or(cursor.min(input_len)..cursor.min(input_len))
                } else {
                    cursor.min(input_len)..cursor.min(input_len)
                };
                let new_value = replace_text_range(&input_text, range, text);
                self.set_input_value(new_value, window, cx);
            }
            SuggestionAction::ExecuteCommand { command_id, args } => {
                let args = args
                    .clone()
                    .or_else(|| trigger.and_then(|info| info.command_args(command_id)))
                    .unwrap_or_default();
                let context = CommandContext::new(args, ViewScope::Search)
                    .with_input(input_text.clone(), cursor);
                let result = registry.execute(command_id, context);
                self.apply_command_result(result, trigger, search_state, window, cx);
            }
            SuggestionAction::Search { query, use_kg } => {
                self.apply_search_action(query, *use_kg, search_state, window, cx);
            }
            SuggestionAction::Navigate { .. } => {}
            SuggestionAction::Custom { .. } => {}
        }
    }

    fn apply_command_result(
        &mut self,
        result: crate::slash_command::CommandResult,
        trigger: Option<&crate::slash_command::TriggerInfo>,
        search_state: &Entity<SearchState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(action) = result.follow_up {
            if let SuggestionAction::Search { query, use_kg } = *action {
                self.apply_search_action(&query, use_kg, search_state, window, cx);
            }
        }

        if let Some(content) = result.content {
            let (input_text, cursor) = {
                let input = self.input_state.read(cx);
                (input.value().to_string(), input.cursor())
            };
            let input_len = input_text.len();
            let range = trigger
                .map(|info| info.replacement_range(input_len))
                .unwrap_or(cursor.min(input_len)..cursor.min(input_len));
            let new_value = replace_text_range(&input_text, range, &content);
            self.set_input_value(new_value, window, cx);
        }

        if result.clear_input {
            self.set_input_value(String::new(), window, cx);
        }
    }

    fn apply_search_action(
        &mut self,
        query: &str,
        _use_kg: bool,
        search_state: &Entity<SearchState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        search_state.update(cx, |state, cx| {
            state.search(query.to_string(), cx);
            state.clear_autocomplete(cx);
        });
        self.show_autocomplete_dropdown = false;
        self.set_input_value(query.to_string(), window, cx);
        cx.notify();
    }

    fn set_input_value(&mut self, value: String, window: &mut Window, cx: &mut Context<Self>) {
        self.suppress_autocomplete = true;
        self.input_state.update(cx, |input, cx| {
            input.set_value(gpui::SharedString::from(value), window, cx);
        });
    }

    fn render_autocomplete_dropdown(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let state = self.search_state.read(cx);

        if !state.is_autocomplete_visible() || !self.show_autocomplete_dropdown {
            return None;
        }

        let suggestions = state.get_suggestions();

        Some(
            div()
                .absolute()
                .top(px(50.0))
                .left(px(0.0))
                .w_full()
                .max_h(px(300.0))
                .bg(theme::background())
                .border_1()
                .border_color(theme::border())
                .rounded_md()
                .shadow_lg()
                .overflow_hidden()
                .children(
                    suggestions
                        .iter()
                        .enumerate()
                        .map(|(idx, suggestion)| self.render_suggestion_item(idx, suggestion, cx)),
                ),
        )
    }

    fn render_suggestion_item(
        &self,
        index: usize,
        suggestion: &AutocompleteSuggestion,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        // Use actual selected index from SearchState
        let selected_idx = self.search_state.read(cx).get_selected_index();
        let is_selected = index == selected_idx;
        let term = suggestion.term.clone();
        let url = suggestion.url.clone();
        let score = suggestion.score;

        log::debug!(
            "Rendering suggestion item {}: '{}' (selected: {})",
            index,
            term,
            is_selected
        );

        use gpui_component::button::*;

        // Simplify closure captures and add debug logging
        let search_state = self.search_state.clone();
        let input_state = self.input_state.clone();

        // Try both button click and div click for better compatibility
        let container = div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .w_full()
            .cursor_pointer()
            .when(is_selected, |div| div.bg(theme::autocomplete_selected()))
            .child(
                // Use Button for clickable suggestions as well (backup click handler)
                Button::new(("autocomplete-item", index))
                    .label(term.clone())
                    .when(is_selected, |btn| btn.primary())
                    .ghost()
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        log::info!("Button clicked: suggestion '{}' at index {}", term, index);

                        // Accept the suggestion at the clicked index (not the currently selected one)
                        let accepted_term = search_state.update(cx, |state, cx| {
                            log::debug!(
                                "Button: Calling accept_autocomplete_at_index for index {}",
                                index
                            );
                            state.accept_autocomplete_at_index(index, cx)
                        });

                        if let Some(selected_term) = accepted_term {
                            log::info!(
                                "Button autocomplete accepted: '{}' - updating input field",
                                selected_term
                            );

                            // Suppress autocomplete temporarily to prevent race condition
                            this.suppress_autocomplete = true;

                            // Update input field with selected term
                            input_state.update(cx, |input, input_cx| {
                                log::debug!("Button: Updating input value to: '{}'", selected_term);
                                input.set_value(
                                    gpui::SharedString::from(selected_term.clone()),
                                    _window,
                                    input_cx,
                                );
                            });

                            // Verify the value was updated
                            let updated_value = input_state.read(cx).value();
                            log::debug!(
                                "Button: Input value after update: '{}' (expected: '{}')",
                                updated_value,
                                selected_term
                            );

                            // Trigger search immediately and clear autocomplete (matching Tauri pattern)
                            search_state.update(cx, |state, cx| {
                                log::debug!("Button: Triggering search for: '{}'", selected_term);
                                state.search(selected_term.clone(), cx);
                                state.clear_autocomplete(cx); // Ensure dropdown stays hidden
                            });

                            this.show_autocomplete_dropdown = false;
                            cx.notify();
                        } else {
                            log::warn!(
                                "Button: No autocomplete suggestion accepted for index {}",
                                index
                            );
                        }
                    })),
            )
            .children(url.map(|_u| {
                div()
                    .text_xs()
                    .opacity(0.6)
                    .child(format!("{:.0}%", score * 100.0))
            }));

        container
    }
}

impl Render for SearchInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slash_popup = &self.slash_command_popup;
        let slash_popup_open = slash_popup.read(cx).is_open();

        div()
            .relative()
            .w_full()
            .track_focus(&self.input_state.read(cx).focus_handle(cx))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                // Check if slash popup is open
                let slash_open = this.slash_command_popup.read(cx).is_open();

                if slash_open {
                    // Handle slash popup navigation
                    match &ev.keystroke.key {
                        key if key == "down" => {
                            this.slash_command_popup.update(cx, |popup, cx| {
                                popup.select_next(cx);
                            });
                        }
                        key if key == "up" => {
                            this.slash_command_popup.update(cx, |popup, cx| {
                                popup.select_previous(cx);
                            });
                        }
                        key if key == "enter" || key == "tab" => {
                            // Accept selected slash command suggestion
                            log::info!("Accepting slash command suggestion");
                            this.slash_command_popup.update(cx, |popup, cx| {
                                popup.accept_selected(cx);
                            });
                        }
                        key if key == "escape" => {
                            this.slash_command_popup.update(cx, |popup, cx| {
                                popup.close(cx);
                            });
                        }
                        _ => {}
                    }
                    return;
                }

                // Only handle keys when autocomplete is visible
                if !this.show_autocomplete_dropdown {
                    return;
                }

                match &ev.keystroke.key {
                    key if key == "down" => {
                        log::info!("Arrow Down pressed - selecting next suggestion");
                        this.search_state.update(cx, |state, cx| {
                            state.autocomplete_next(cx);
                        });
                        cx.notify();
                    }
                    key if key == "up" => {
                        log::info!("Arrow Up pressed - selecting previous suggestion");
                        this.search_state.update(cx, |state, cx| {
                            state.autocomplete_previous(cx);
                        });
                        cx.notify();
                    }
                    key if key == "tab" => {
                        log::info!("Tab pressed - accepting selected suggestion");

                        // Accept selected suggestion
                        let accepted_term = this
                            .search_state
                            .update(cx, |state, cx| state.accept_autocomplete(cx));

                        if let Some(selected_term) = accepted_term {
                            log::info!(
                                "Tab autocomplete accepted: '{}' - updating input field",
                                selected_term
                            );

                            // Suppress autocomplete temporarily to prevent race condition
                            this.suppress_autocomplete = true;

                            // Update input field with selected term
                            this.input_state.update(cx, |input, input_cx| {
                                log::debug!("Tab: updating input value to: '{}'", selected_term);
                                input.set_value(
                                    gpui::SharedString::from(selected_term.clone()),
                                    window,
                                    input_cx,
                                );
                            });

                            // Verify the value was updated
                            let updated_value = this.input_state.read(cx).value();
                            log::debug!(
                                "Tab: Input value after update: '{}' (expected: '{}')",
                                updated_value,
                                selected_term
                            );

                            // Trigger search
                            this.search_state.update(cx, |state, cx| {
                                log::debug!("Tab: triggering search for: '{}'", selected_term);
                                state.search(selected_term.clone(), cx);
                                state.clear_autocomplete(cx);
                            });

                            this.show_autocomplete_dropdown = false;
                            cx.notify();
                        } else {
                            log::warn!("Tab: No autocomplete suggestion accepted");
                        }
                    }
                    key if key == "escape" => {
                        log::info!("Escape pressed - closing autocomplete");
                        this.search_state.update(cx, |state, cx| {
                            state.clear_autocomplete(cx);
                        });
                        this.show_autocomplete_dropdown = false;
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_color(theme::text_secondary()).child("Search"))
                    .child(div().flex_1().child(Input::new(&self.input_state))),
            )
            // Show slash command popup OR autocomplete dropdown (not both)
            .when(slash_popup_open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(50.0))
                        .left(px(0.0))
                        .w_full()
                        .child(slash_popup.clone()),
                )
            })
            .when(!slash_popup_open, |d| {
                d.children(self.render_autocomplete_dropdown(cx))
            })
    }
}
