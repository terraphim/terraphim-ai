use gpui::*;

mod autocomplete;
mod input;
mod results;

pub use input::SearchInput;
pub use results::SearchResults;

/// Main search view
pub struct SearchView {
    search_input: View<SearchInput>,
    search_results: View<SearchResults>,
}

impl SearchView {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let search_input = cx.new_view(|cx| SearchInput::new(cx));
        let search_results = cx.new_view(|cx| SearchResults::new(cx));

        log::info!("SearchView initialized");

        Self {
            search_input,
            search_results,
        }
    }
}

impl Render for SearchView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
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
                    .overflow_y_scroll()
                    .child(self.search_results.clone()),
            )
    }
}
