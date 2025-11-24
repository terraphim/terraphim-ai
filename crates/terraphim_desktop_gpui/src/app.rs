use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;

use crate::actions::{NavigateToChat, NavigateToEditor, NavigateToSearch};
use crate::theme::TerraphimTheme;
use crate::views::chat::ChatView;
use crate::views::editor::EditorView;
use crate::views::search::SearchView;
use crate::views::{RoleSelector, TrayMenu, TrayMenuAction};

/// Main application state
pub struct TerraphimApp {
    current_view: AppView,
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
    editor_view: Entity<EditorView>,
    role_selector: Entity<RoleSelector>,
    tray_menu: Entity<TrayMenu>,
    theme: Entity<TerraphimTheme>,
    show_tray_menu: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

impl TerraphimApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Initialize theme
        let theme = cx.new(|cx| TerraphimTheme::new(cx));

        // Initialize views
        let search_view = cx.new(|cx| SearchView::new(window, cx));
        let chat_view = cx.new(|cx| ChatView::new(window, cx));
        let editor_view = cx.new(|cx| EditorView::new(window, cx));

        // Initialize role selector
        let role_selector = cx.new(|cx| RoleSelector::new(window, cx));

        // Initialize tray menu
        let tray_menu = cx.new(|cx| TrayMenu::new(window, cx));

        // TODO: GPUI 0.2.2 - on_action API has changed
        // Subscribe to navigation actions
        // cx.on_action(|this: &mut Self, _: &NavigateToSearch, cx| {
        //     this.navigate_to(AppView::Search, cx);
        // });

        // cx.on_action(|this: &mut Self, _: &NavigateToChat, cx| {
        //     this.navigate_to(AppView::Chat, cx);
        // });

        // cx.on_action(|this: &mut Self, _: &NavigateToEditor, cx| {
        //     this.navigate_to(AppView::Editor, cx);
        // });

        log::info!("TerraphimApp initialized with view: {:?}", AppView::Search);

        Self {
            current_view: AppView::Search,
            search_view,
            chat_view,
            editor_view,
            role_selector,
            tray_menu,
            theme,
            show_tray_menu: false,
        }
    }

    pub fn navigate_to(&mut self, view: AppView, cx: &mut Context<Self>) {
        if self.current_view != view {
            log::info!("Navigating from {:?} to {:?}", self.current_view, view);
            self.current_view = view;
            cx.notify();
        }
    }

    /// Toggle tray menu visibility
    pub fn toggle_tray_menu(&mut self, cx: &mut Context<Self>) {
        self.show_tray_menu = !self.show_tray_menu;

        self.tray_menu.update(cx, |menu, cx| {
            if self.show_tray_menu {
                menu.show(cx);
            } else {
                menu.hide(cx);
            }
        });

        cx.notify();
    }

    /// Handle tray menu actions
    fn handle_tray_action(&mut self, action: TrayMenuAction, cx: &mut Context<Self>) {
        log::info!("Handling tray action: {:?}", action);

        match action {
            TrayMenuAction::ShowWindow => {
                // In a real app, this would show the window
                log::info!("Show window requested");
            }
            TrayMenuAction::HideWindow => {
                // In a real app, this would hide the window
                log::info!("Hide window requested");
            }
            TrayMenuAction::Search => {
                self.navigate_to(AppView::Search, cx);
            }
            TrayMenuAction::Chat => {
                self.navigate_to(AppView::Chat, cx);
            }
            TrayMenuAction::Settings => {
                log::info!("Settings view not yet implemented");
            }
            TrayMenuAction::About => {
                log::info!("About dialog not yet implemented");
            }
            TrayMenuAction::Quit => {
                log::info!("Quit requested - would close application");
                // In a real app: cx.quit();
            }
            TrayMenuAction::Custom(name) => {
                log::info!("Custom action: {}", name);
            }
        }

        self.show_tray_menu = false;
        cx.notify();
    }

    fn render_navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .bg(rgb(0xf5f5f5))
            .border_b_1()
            .border_color(rgb(0xdddddd))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        // Logo
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(0x333333))
                            .child("Terraphim AI"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(self.render_nav_button("Search", AppView::Search, cx))
                            .child(self.render_nav_button("Chat", AppView::Chat, cx))
                            .child(self.render_nav_button("Editor", AppView::Editor, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        // Role selector
                        self.role_selector.clone(),
                    )
                    .child(
                        // Tray menu button
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if self.show_tray_menu {
                                rgb(0x3273dc)
                            } else {
                                rgb(0xffffff)
                            })
                            .text_color(if self.show_tray_menu {
                                rgb(0xffffff)
                            } else {
                                rgb(0x363636)
                            })
                            .hover(|style| style.bg(rgb(0xf0f0f0)).cursor_pointer())
                            .child("☰"),
                    ),
            )
    }

    fn render_nav_button(
        &self,
        label: &str,
        view: AppView,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.current_view == view;
        let label = label.to_string();

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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .relative()
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
            .when(self.show_tray_menu, |this| {
                this.child(self.tray_menu.clone())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_view_variants() {
        assert_eq!(AppView::Search, AppView::Search);
        assert_ne!(AppView::Search, AppView::Chat);
        assert_ne!(AppView::Chat, AppView::Editor);
    }

    #[test]
    fn test_tray_menu_action_handling() {
        // Test that all TrayMenuAction variants are handled
        let actions = vec![
            TrayMenuAction::ShowWindow,
            TrayMenuAction::HideWindow,
            TrayMenuAction::Search,
            TrayMenuAction::Chat,
            TrayMenuAction::Settings,
            TrayMenuAction::About,
            TrayMenuAction::Quit,
        ];

        assert_eq!(actions.len(), 7);
    }
}
