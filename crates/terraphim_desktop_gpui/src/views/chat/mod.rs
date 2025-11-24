use gpui::*;

/// Chat view (placeholder)
pub struct ChatView {}

impl ChatView {
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        log::info!("ChatView initialized");
        Self {}
    }
}

impl Render for ChatView {
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
                    .child("Chat"),
            )
            .child(
                div()
                    .text_color(rgb(0x7a7a7a))
                    .child("Chat interface coming soon..."),
            )
    }
}
