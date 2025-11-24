use gpui::*;
use gpui_component::StyledExt;

mod autocomplete;
mod input;
mod results;

pub use input::SearchInput;
pub use results::SearchResults;

/// Main search view
pub struct SearchView {
    search_input: Entity<SearchInput>,
    search_results: Entity<SearchResults>,
}

impl SearchView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| SearchInput::new(window, cx));
        let search_results = cx.new(|cx| SearchResults::new(window, cx));

        log::info!("SearchView initialized");

        Self {
            search_input,
            search_results,
        }
    }
}

impl Render for SearchView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(rgb(0x363636))
                    .child("Search"),
            )
            .child(self.search_input.clone())
            .child(
                div()
                    .flex_1()
                    .child(self.search_results.clone()),
            )
    }
}
