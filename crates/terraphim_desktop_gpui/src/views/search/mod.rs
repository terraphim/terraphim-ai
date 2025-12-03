use gpui::*;
use gpui_component::StyledExt;
use terraphim_config::ConfigState;
use crate::state::search::SearchState;
use crate::views::ArticleModal;
use crate::theme::colors::theme;

mod autocomplete;
mod input;
mod results;
mod term_chips;

pub use input::SearchInput;
pub use results::{AddToContextEvent, OpenArticleEvent, SearchResults};
pub use term_chips::TermChips;

impl EventEmitter<AddToContextEvent> for SearchView {}
impl EventEmitter<OpenArticleEvent> for SearchView {}

/// Main search view with article modal
pub struct SearchView {
    search_state: Entity<SearchState>,
    search_input: Entity<SearchInput>,
    search_results: Entity<SearchResults>,
    article_modal: Entity<ArticleModal>,
    _subscriptions: Vec<Subscription>,
}

impl SearchView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, config_state: ConfigState) -> Self {
        log::info!("=== SearchView INITIALIZATION ===");
        log::info!("ConfigState roles count: {}", config_state.roles.len());
        log::info!("ConfigState roles: {:?}", config_state.roles.keys().collect::<Vec<_>>());

        let search_state = cx.new(|cx| {
            let state = SearchState::new(cx).with_config(config_state);
            log::info!("SearchState created - has_config: {}", state.has_config());
            state
        });
        let search_input = cx.new(|cx| SearchInput::new(window, cx, search_state.clone()));
        let search_results = cx.new(|cx| SearchResults::new(window, cx, search_state.clone()));
        let article_modal = cx.new(|cx| ArticleModal::new(window, cx));

        // Forward AddToContextEvent from SearchResults to App
        let results_sub1 = cx.subscribe(&search_results, |_this: &mut SearchView, _results, event: &AddToContextEvent, cx| {
            log::info!("SearchView forwarding AddToContext event");
            cx.emit(AddToContextEvent { 
                document: event.document.clone(),
                navigate_to_chat: event.navigate_to_chat,
            });
        });

        // Handle OpenArticleEvent to show modal
        let modal_clone = article_modal.clone();
        let results_sub2 = cx.subscribe(&search_results, move |_this: &mut SearchView, _results, event: &OpenArticleEvent, cx| {
            log::info!("Opening article modal for: {}", event.document.title);
            modal_clone.update(cx, |modal, modal_cx| {
                modal.open(event.document.clone(), modal_cx);
            });
        });

        log::info!("SearchView initialized with backend services");

        Self {
            search_state,
            search_input,
            search_results,
            article_modal,
            _subscriptions: vec![results_sub1, results_sub2],
        }
    }

    /// Open article modal with document
    pub fn open_article(&self, document: terraphim_types::Document, cx: &mut Context<Self>) {
        self.article_modal.update(cx, |modal, modal_cx| {
            modal.open(document, modal_cx);
        });
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get term chips from search state
        let term_chips = self.search_state.read(cx).get_term_chips();
        
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme::text_primary())
                    .child("Search"),
            )
            .child(self.search_input.clone())
            .children(if !term_chips.chips.is_empty() {
                Some(cx.new(|_cx| TermChips::new(term_chips.clone())))
            } else {
                None
            })
            .child(
                div()
                    .flex_1()
                    .child(self.search_results.clone()),
            )
            .child(self.article_modal.clone())  // Modal renders on top
    }
}
