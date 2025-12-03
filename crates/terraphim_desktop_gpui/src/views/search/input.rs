use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::state::search::{AutocompleteSuggestion, SearchState};

/// Search input component with real-time autocomplete
pub struct SearchInput {
    input_state: Entity<InputState>,
    search_state: Entity<SearchState>,
    show_autocomplete_dropdown: bool,
    suppress_autocomplete: bool,
    _subscriptions: Vec<Subscription>,
}

impl SearchInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, search_state: Entity<SearchState>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search knowledge graph...")
        });

        // Subscribe to input changes for autocomplete
        let search_state_clone = search_state.clone();
        let input_state_clone = input_state.clone();
        let autocomplete_sub = cx.subscribe_in(&input_state, window, move |this, _, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => {
                    // Skip autocomplete if we just programmatically set the value
                    if this.suppress_autocomplete {
                        log::debug!("Suppressing autocomplete (programmatic value update)");
                        this.suppress_autocomplete = false;
                        return;
                    }

                    let value = input_state_clone.read(cx).value();

                    // Trigger autocomplete for user keystrokes
                    search_state_clone.update(cx, |state, cx| {
                        state.get_autocomplete(value.to_string(), cx);
                    });

                    // Update dropdown visibility
                    this.show_autocomplete_dropdown = value.len() >= 2;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    // Trigger search on Enter
                    let value = input_state_clone.read(cx).value();
                    search_state_clone.update(cx, |state, cx| {
                        state.search(value.to_string(), cx);
                        state.clear_autocomplete(cx);  // Clear autocomplete after search
                    });
                    this.show_autocomplete_dropdown = false;
                    cx.notify();
                }
                _ => {}
            }
        });

        Self {
            input_state,
            search_state,
            show_autocomplete_dropdown: false,
            suppress_autocomplete: false,
            _subscriptions: vec![autocomplete_sub],
        }
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
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(0xdbdbdb))
                .rounded_md()
                .shadow_lg()
                .overflow_hidden()
                .children(
                    suggestions.iter().enumerate().map(|(idx, suggestion)| {
                        self.render_suggestion_item(idx, suggestion, cx)
                    })
                )
        )
    }

    fn render_suggestion_item(&self, index: usize, suggestion: &AutocompleteSuggestion, cx: &Context<Self>) -> impl IntoElement {
        // Use actual selected index from SearchState
        let selected_idx = self.search_state.read(cx).get_selected_index();
        let is_selected = index == selected_idx;
        let term = suggestion.term.clone();
        let url = suggestion.url.clone();
        let score = suggestion.score;

        log::debug!("Rendering suggestion item {}: '{}' (selected: {})", index, term, is_selected);

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
            .when(is_selected, |div| {
                div.bg(rgb(0xe3f2fd))
            })
            .child(
                // Use Button for clickable suggestions as well (backup click handler)
                Button::new(("autocomplete-item", index))
                    .label(term.clone())
                    .when(is_selected, |btn| btn.primary())
                    .ghost()
                    .on_click(cx.listener(move |this, ev, window, cx| {
                        log::info!("Button clicked: suggestion '{}' at index {}", term, index);

                        // Accept the suggestion at the clicked index (not the currently selected one)
                        let accepted_term = search_state.update(cx, |state, cx| {
                            log::debug!("Button: Calling accept_autocomplete_at_index for index {}", index);
                            state.accept_autocomplete_at_index(index, cx)
                        });

                        if let Some(selected_term) = accepted_term {
                            log::info!("Button autocomplete accepted: '{}' - updating input field", selected_term);

                            // Suppress autocomplete temporarily to prevent race condition
                            this.suppress_autocomplete = true;

                            // Update input field with selected term
                            input_state.update(cx, |input, input_cx| {
                                log::debug!("Button: Updating input value to: '{}'", selected_term);
                                input.set_value(gpui::SharedString::from(selected_term.clone()), window, input_cx);
                            });

                            // Verify the value was updated
                            let updated_value = input_state.read(cx).value();
                            log::debug!("Button: Input value after update: '{}' (expected: '{}')", updated_value, selected_term);

                            // Trigger search immediately and clear autocomplete (matching Tauri pattern)
                            search_state.update(cx, |state, cx| {
                                log::debug!("Button: Triggering search for: '{}'", selected_term);
                                state.search(selected_term.clone(), cx);
                                state.clear_autocomplete(cx);  // Ensure dropdown stays hidden
                            });

                            this.show_autocomplete_dropdown = false;
                            cx.notify();
                        } else {
                            log::warn!("Button: No autocomplete suggestion accepted for index {}", index);
                        }
                    }))
            )
            .children(url.map(|u| {
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
        div()
            .relative()
            .w_full()
            .track_focus(&self.input_state.read(cx).focus_handle(cx))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
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
                        let accepted_term = this.search_state.update(cx, |state, cx| {
                            state.accept_autocomplete(cx)
                        });

                        if let Some(selected_term) = accepted_term {
                            log::info!("Tab autocomplete accepted: '{}' - updating input field", selected_term);

                            // Suppress autocomplete temporarily to prevent race condition
                            this.suppress_autocomplete = true;

                            // Update input field with selected term
                            this.input_state.update(cx, |input, input_cx| {
                                log::debug!("Tab: updating input value to: '{}'", selected_term);
                                input.set_value(gpui::SharedString::from(selected_term.clone()), window, input_cx);
                            });

                            // Verify the value was updated
                            let updated_value = this.input_state.read(cx).value();
                            log::debug!("Tab: Input value after update: '{}' (expected: '{}')", updated_value, selected_term);

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
                    .child(
                        div()
                            .text_color(rgb(0x7a7a7a))
                            .child("🔍")
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.input_state))
                    )
            )
            .children(self.render_autocomplete_dropdown(cx))
    }
}
