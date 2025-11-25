use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::StyledExt;

use crate::state::search::{AutocompleteSuggestion, SearchState};

/// Search input component with real-time autocomplete
pub struct SearchInput {
    input_state: Entity<InputState>,
    search_state: Entity<SearchState>,
    show_autocomplete_dropdown: bool,
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
                    let value = input_state_clone.read(cx).value();

                    // Trigger autocomplete for every keystroke
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
        let is_selected = index == 0; // Simplified: first item is always selected
        let term = suggestion.term.clone();
        let url = suggestion.url.clone();
        let score = suggestion.score;

        use gpui_component::button::*;

        // Use Button for clickable suggestions
        let mut button = Button::new(("autocomplete-item", index))
            .label(term.clone())
            .ghost();

        if is_selected {
            button = button.primary();
        }

        button = button.on_click(cx.listener(move |this, _ev, _window, cx| {
            // Accept the selected suggestion
            this.search_state.update(cx, |state, cx| {
                if let Some(selected_term) = state.accept_autocomplete(cx) {
                    // Update input with selected term and trigger search
                    log::info!("Autocomplete accepted: {}", selected_term);
                    // Search will be triggered by Enter or input change
                }
            });
            this.show_autocomplete_dropdown = false;
            cx.notify();
        }));

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .child(button)
            .children(url.map(|u| {
                div()
                    .text_xs()
                    .opacity(0.6)
                    .child(format!("{:.0}%", score * 100.0))
            }))
    }
}

impl Render for SearchInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .w_full()
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
