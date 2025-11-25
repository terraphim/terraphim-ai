use gpui::*;
use gpui_component::{button::*, IconName, StyledExt};
use terraphim_types::Document;

use crate::state::search::SearchState;

/// Event emitted when user wants to add document to context
pub struct AddToContextEvent {
    pub document: Document,
}

impl EventEmitter<AddToContextEvent> for SearchResults {}

/// Search results with action buttons matching Tauri desktop
pub struct SearchResults {
    search_state: Entity<SearchState>,
}

impl SearchResults {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>, search_state: Entity<SearchState>) -> Self {
        Self { search_state }
    }

    fn handle_open_url(&self, url: String) {
        if !url.is_empty() {
            log::info!("Opening URL: {}", url);
            // TODO: Open in browser (use webbrowser crate)
        }
    }

    fn handle_add_to_context(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Adding to context: {}", document.title);
        cx.emit(AddToContextEvent { document });
    }

    fn handle_chat_with_document(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Chat with document: {}", document.title);
        // Add to context and navigate to chat
        cx.emit(AddToContextEvent { document });
    }

    fn render_result_item(&self, doc: &Document, idx: usize, cx: &Context<Self>) -> impl IntoElement {
        let doc_url = doc.url.clone();
        let doc_clone_for_context = doc.clone();
        let doc_clone_for_chat = doc.clone();

        div()
            .p_4()
            .mb_2()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_md()
            .hover(|style| style.bg(rgb(0xf5f5f5)))
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(rgb(0x3273dc))
                    .mb_2()
                    .child(doc.title.clone()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x7a7a7a))
                    .mb_2()
                    .child(
                        doc.description
                            .as_ref()
                            .map(|d| d.clone())
                            .unwrap_or_else(|| "No description".to_string()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xb5b5b5))
                    .mb_2()
                    .child(doc.url.clone()),
            )
            .child(
                // Action buttons (Tauri ResultItem.svelte pattern)
                div()
                    .flex()
                    .gap_2()
                    .mt_2()
                    .child(
                        Button::new(("open-url", idx))
                            .label("Open")
                            .icon(IconName::ExternalLink)
                            .ghost()
                            .on_click(cx.listener(move |this, _ev, _window, _cx| {
                                this.handle_open_url(doc_url.clone());
                            }))
                    )
                    .child(
                        Button::new(("add-ctx", idx))
                            .label("Add to Context")
                            .icon(IconName::Plus)
                            .outline()
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.handle_add_to_context(doc_clone_for_context.clone(), cx);
                            }))
                    )
                    .child(
                        Button::new(("chat-doc", idx))
                            .label("Chat")
                            .icon(IconName::Bot)
                            .primary()
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.handle_chat_with_document(doc_clone_for_chat.clone(), cx);
                            }))
                    )
            )
    }

    fn render_empty_state(&self, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .py_12()
            .child(
                div()
                    .text_2xl()
                    .mb_4()
                    .child("🔍"),
            )
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0x7a7a7a))
                    .mb_2()
                    .child("Search Terraphim Knowledge Graph"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xb5b5b5))
                    .child("Enter a query to search across your knowledge sources"),
            )
    }
}

impl Render for SearchResults {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.search_state.read(cx);
        let is_loading = state.is_loading();
        let has_error = state.has_error();
        let results = state.get_results();

        if is_loading {
            div()
                .flex()
                .items_center()
                .justify_center()
                .py_12()
                .child(
                    div()
                        .text_lg()
                        .text_color(rgb(0x7a7a7a))
                        .child("Searching...")
                )
                .into_any_element()
        } else if has_error {
            div()
                .px_4()
                .py_3()
                .bg(rgb(0xfef5e7))
                .border_1()
                .border_color(rgb(0xf1c40f))
                .rounded_md()
                .child("Search error - please try again")
                .into_any_element()
        } else if results.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .pb_2()
                        .border_b_1()
                        .border_color(rgb(0xf0f0f0))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x7a7a7a))
                                .child(format!("Found {} results", results.len()))
                        )
                )
                .children(results.iter().enumerate().map(|(idx, result)| {
                    // Render using the document from ResultItemViewModel
                    self.render_result_item(&result.document, idx, cx)
                }))
                .into_any_element()
        }
    }
}
