use gpui::*;
use gpui_component::StyledExt;
use terraphim_config::ConfigState;
use crate::state::search::SearchState;

mod autocomplete;
mod input;
mod results;

pub use input::SearchInput;
pub use results::{AddToContextEvent, SearchResults};

impl EventEmitter<AddToContextEvent> for SearchView {}

/// Main search view
pub struct SearchView {
    search_state: Entity<SearchState>,
    search_input: Entity<SearchInput>,
    search_results: Entity<SearchResults>,
    _subscriptions: Vec<Subscription>,
}

impl SearchView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, config_state: ConfigState) -> Self {
        let search_state = cx.new(|cx| SearchState::new(cx).with_config(config_state));
        let search_input = cx.new(|cx| SearchInput::new(window, cx, search_state.clone()));
        let search_results = cx.new(|cx| SearchResults::new(window, cx, search_state.clone()));

        // Forward AddToContextEvent from SearchResults to App
        let results_sub = cx.subscribe(&search_results, |this: &mut SearchView, _results, event: &AddToContextEvent, cx| {
            log::info!("SearchView forwarding AddToContext event");
            cx.emit(AddToContextEvent { document: event.document.clone() });
        });

        log::info!("SearchView initialized with backend services");

        Self {
            search_state,
            search_input,
            search_results,
            _subscriptions: vec![results_sub],
        }
    }

    /// Get search state for external access
    pub fn search_state(&self) -> &Entity<SearchState> {
        &self.search_state
    }

    /// Update role (called when role changes)
    pub fn update_role(&mut self, new_role: String, cx: &mut Context<Self>) {
        self.search_state.update(cx, |state, cx| {
            state.set_role(new_role, cx);
        });
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
