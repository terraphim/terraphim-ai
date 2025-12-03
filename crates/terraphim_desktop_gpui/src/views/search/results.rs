use gpui::*;
use gpui_component::{button::*, IconName, StyledExt};
use terraphim_types::Document;

use crate::state::search::SearchState;
use crate::theme::colors::theme;

/// Event emitted when user wants to add document to context
pub struct AddToContextEvent {
    pub document: Document,
    pub navigate_to_chat: bool,  // If true, also navigate to chat after adding
}

/// Event emitted when user wants to view full document
pub struct OpenArticleEvent {
    pub document: Document,
}

impl EventEmitter<AddToContextEvent> for SearchResults {}
impl EventEmitter<OpenArticleEvent> for SearchResults {}

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
            // Determine the appropriate scheme for the URL
            let url = if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://") {
                // URL already has a valid scheme
                url
            } else if url.starts_with('/') || url.starts_with("~/") {
                // This is a local file path - use file:// scheme
                let expanded = if url.starts_with("~/") {
                    // Expand home directory
                    if let Some(home) = std::env::var_os("HOME") {
                        url.replacen("~", &home.to_string_lossy(), 1)
                    } else {
                        url
                    }
                } else {
                    url
                };
                format!("file://{}", expanded)
            } else if url.contains('/') && !url.contains('.') {
                // Likely a relative file path
                let cwd = std::env::current_dir().unwrap_or_default();
                format!("file://{}/{}", cwd.display(), url)
            } else {
                // Assume it's a web URL without scheme
                format!("https://{}", url)
            };

            log::info!("Opening URL/file: {}", url);

            // Open URL or file using the webbrowser crate (handles both file:// and http(s)://)
            match webbrowser::open(&url) {
                Ok(()) => {
                    log::info!("Successfully opened URL/file");
                }
                Err(e) => {
                    log::error!("Failed to open URL/file: {}", e);
                    // TODO: Show error notification to user
                }
            }
        }
    }

    fn handle_add_to_context(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Adding to context: {}", document.title);
        // Directly add to context (no modal, no navigation)
        cx.emit(AddToContextEvent { 
            document,
            navigate_to_chat: false,
        });
    }

    fn handle_chat_with_document(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Chat with document: {}", document.title);
        // Add to context AND navigate to chat
        cx.emit(AddToContextEvent { 
            document,
            navigate_to_chat: true,
        });
    }

    fn handle_open_article(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Opening article modal for: {}", document.title);
        cx.emit(OpenArticleEvent { document });
    }

    fn render_result_item(&self, doc: &Document, idx: usize, cx: &Context<Self>) -> impl IntoElement {
        let doc_url = doc.url.clone();
        let doc_clone_for_context = doc.clone();
        let doc_clone_for_chat = doc.clone();
        let doc_clone_for_modal = doc.clone();

        div()
            .p_4()
            .mb_2()
            .bg(theme::background())
            .border_1()
            .border_color(theme::border())
            .rounded_md()
            .hover(|style| style.bg(theme::surface_hover()))
            .child(
                // Clickable title to open modal
                div()
                    .text_lg()
                    .font_semibold()
                    .text_color(theme::primary())
                    .mb_2()
                    .cursor_pointer()
                    .hover(|style| style.text_color(theme::primary_hover()))
                    .child(
                        Button::new(("open-modal", idx))
                            .label(doc.title.clone())
                            .ghost()
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.handle_open_article(doc_clone_for_modal.clone(), cx);
                            }))
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_secondary())
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
                    .text_color(theme::text_disabled())
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
                    .text_color(theme::text_secondary())
                    .mb_2()
                    .child("Search Terraphim Knowledge Graph"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme::text_disabled())
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
                        .text_color(theme::text_secondary())
                        .child("Searching...")
                )
                .into_any_element()
        } else if has_error {
            div()
                .px_4()
                .py_3()
                .bg(theme::warning())  // Use warning color directly
                .border_1()
                .border_color(theme::warning())
                .rounded_md()
                .text_color(theme::text_primary())
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
                                .text_color(theme::text_secondary())
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
