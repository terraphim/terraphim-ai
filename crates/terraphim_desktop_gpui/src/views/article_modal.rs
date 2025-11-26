/// Article Modal - Full document viewer matching Tauri ArticleModal.svelte
///
/// Shows full document content with markdown rendering
/// Pattern from desktop/src/lib/Search/ArticleModal.svelte

use gpui::*;
use gpui_component::{button::*, IconName, StyledExt};
use terraphim_types::Document;

/// Article modal for viewing full document content
pub struct ArticleModal {
    document: Option<Document>,
    is_open: bool,
}

impl ArticleModal {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            document: None,
            is_open: false,
        }
    }

    /// Open modal with document
    pub fn open(&mut self, document: Document, cx: &mut Context<Self>) {
        log::info!("Opening modal for: {}", document.title);
        self.document = Some(document);
        self.is_open = true;
        cx.notify();
    }

    /// Close modal
    pub fn close(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!("Closing article modal");
        self.is_open = false;
        self.document = None;
        cx.notify();
    }
}

impl Render for ArticleModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_open || self.document.is_none() {
            return div().into_any_element();
        }

        let doc = self.document.as_ref().unwrap();
        let title = doc.title.clone();
        let body = doc.body.clone();
        let url = doc.url.clone();

        // Modal overlay (pattern from Tauri ArticleModal.svelte)
        div()
            .absolute()
            .inset_0()
            .bg(rgb(0x000000))
            .opacity(0.95)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .relative()
                    .w(px(1000.0))      // Reasonable width for most screens
                    .max_w_full()       // Don't exceed parent width
                    .h(px(600.0))       // Reasonable height for laptop screens
                    .max_h(px(700.0))   // Maximum height cap
                    .bg(rgb(0xffffff))
                    .rounded_lg()
                    .shadow_xl()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        // Header with close button
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .py_4()
                            .border_b_1()
                            .border_color(rgb(0xe0e0e0))
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(rgb(0x333333))
                                    .child(title)
                            )
                            .child(
                                Button::new("close-modal")
                                    .label("Close")
                                    .icon(IconName::Delete)
                                    .ghost()
                                    .on_click(cx.listener(Self::close))
                            )
                    )
                    .child(
                        // Document content area - truncates overflow for now
                        // TODO: Add proper scrolling when GPUI supports it better
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .px_6()
                            .py_4()
                            .child(
                                div()
                                    .text_sm()
                                    .line_height(px(24.0))
                                    .text_color(rgb(0x333333))
                                    .child(body)
                            )
                    )
                    .child(
                        // Footer with URL
                        div()
                            .px_6()
                            .py_3()
                            .border_t_1()
                            .border_color(rgb(0xe0e0e0))
                            .bg(rgb(0xf8f8f8))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .child(format!("Source: {}", url))
                            )
                    )
            )
            .into_any_element()
    }
}
