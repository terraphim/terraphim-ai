use gpui::*;

use crate::state::search::SearchState;

/// Search input component with autocomplete
pub struct SearchInput {
    query: SharedString,
    search_state: Entity<SearchState>,
    is_focused: bool,
}

impl SearchInput {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_state = cx.new(|cx| SearchState::new(cx));

        Self {
            query: "".into(),
            search_state,
            is_focused: false,
        }
    }

    fn handle_input(&mut self, text: String, cx: &mut Context<Self>) {
        self.query = text.clone().into();

        // Trigger search
        self.search_state.update(cx, |state, cx| {
            state.search(text, cx);
        });
    }
}

impl Render for SearchInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.query.clone();
        let is_empty = query.is_empty();

        div()
            .flex()
            .items_center()
            .gap_2()
            .p_3()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(if self.is_focused {
                rgb(0x3273dc)
            } else {
                rgb(0xdbdbdb)
            })
            .rounded_md()
            .child(
                // Search icon
                div()
                    .text_color(rgb(0x7a7a7a))
                    .child("🔍"),
            )
            .child(
                // Input field (simplified for now)
                div()
                    .flex_1()
                    .child(if is_empty {
                        "Search knowledge graph...".into()
                    } else {
                        query
                    })
                    .text_color(if is_empty {
                        rgb(0xb5b5b5)
                    } else {
                        rgb(0x363636)
                    }),
            )
    }
}
