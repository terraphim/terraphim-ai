use gpui::*;

/// Editor view (placeholder)
pub struct EditorView {}

impl EditorView {
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        log::info!("EditorView initialized");
        Self {}
    }
}

impl Render for EditorView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .p_4()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(rgb(0x363636))
                    .mb_4()
                    .child("Markdown Editor"),
            )
            .child(
                div()
                    .text_color(rgb(0x7a7a7a))
                    .child("Editor interface coming soon..."),
            )
    }
}
