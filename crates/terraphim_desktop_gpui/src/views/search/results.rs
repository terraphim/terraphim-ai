use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;
use terraphim_types::Document;

/// Search results list component
pub struct SearchResults {
    results: Vec<Document>,
}

impl SearchResults {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            results: vec![],
        }
    }

    fn render_result_item(&self, doc: &Document, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .mb_2()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_md()
            .hover(|style| style.bg(rgb(0xf5f5f5)).cursor_pointer())
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
                    .child(doc.url.clone()),
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
        if self.results.is_empty() {
            self.render_empty_state(cx).into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(self.results.iter().map(|doc| self.render_result_item(doc, cx)))
                .into_any_element()
        }
    }
}
