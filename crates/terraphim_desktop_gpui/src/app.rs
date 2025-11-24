use gpui::*;

use crate::actions::{NavigateToChat, NavigateToEditor, NavigateToSearch};
use crate::theme::TerraphimTheme;
use crate::views::chat::ChatView;
use crate::views::editor::EditorView;
use crate::views::search::SearchView;

/// Main application state
pub struct TerraphimApp {
    current_view: AppView,
    search_view: View<SearchView>,
    chat_view: View<ChatView>,
    editor_view: View<EditorView>,
    theme: Model<TerraphimTheme>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

impl TerraphimApp {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        // Initialize theme
        let theme = cx.new_model(|cx| TerraphimTheme::new(cx));

        // Initialize views
        let search_view = cx.new_view(|cx| SearchView::new(cx));
        let chat_view = cx.new_view(|cx| ChatView::new(cx));
        let editor_view = cx.new_view(|cx| EditorView::new(cx));

        // Subscribe to navigation actions
        cx.on_action(|this: &mut Self, _: &NavigateToSearch, cx| {
            this.navigate_to(AppView::Search, cx);
        });

        cx.on_action(|this: &mut Self, _: &NavigateToChat, cx| {
            this.navigate_to(AppView::Chat, cx);
        });

        cx.on_action(|this: &mut Self, _: &NavigateToEditor, cx| {
            this.navigate_to(AppView::Editor, cx);
        });

        log::info!("TerraphimApp initialized with view: {:?}", AppView::Search);

        Self {
            current_view: AppView::Search,
            search_view,
            chat_view,
            editor_view,
            theme,
        }
    }

    pub fn navigate_to(&mut self, view: AppView, cx: &mut ViewContext<Self>) {
        if self.current_view != view {
            log::info!("Navigating from {:?} to {:?}", self.current_view, view);
            self.current_view = view;
            cx.notify();
        }
    }

    fn render_navigation(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_4()
            .p_4()
            .bg(rgb(0xf5f5f5))
            .border_b_1()
            .border_color(rgb(0xdddddd))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        // Logo
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(0x333333))
                            .child("Terraphim AI"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_nav_button("Search", AppView::Search, cx))
                    .child(self.render_nav_button("Chat", AppView::Chat, cx))
                    .child(self.render_nav_button("Editor", AppView::Editor, cx)),
            )
    }

    fn render_nav_button(
        &self,
        label: &str,
        view: AppView,
        _cx: &mut ViewContext<Self>,
    ) -> impl IntoElement {
        let is_active = self.current_view == view;

        div()
            .px_4()
            .py_2()
            .rounded_md()
            .when(is_active, |this| {
                this.bg(rgb(0x3273dc)).text_color(rgb(0xffffff))
            })
            .when(!is_active, |this| {
                this.bg(rgb(0xffffff))
                    .text_color(rgb(0x363636))
                    .hover(|style| style.bg(rgb(0xf0f0f0)))
                    .cursor_pointer()
            })
            .child(label)
    }
}

impl Render for TerraphimApp {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .child(self.render_navigation(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(match self.current_view {
                        AppView::Search => self.search_view.clone().into_any_element(),
                        AppView::Chat => self.chat_view.clone().into_any_element(),
                        AppView::Editor => self.editor_view.clone().into_any_element(),
                    }),
            )
    }
}
