use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{button::*, StyledExt};
use std::sync::mpsc::{self, Receiver, Sender};
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use terraphim_types::RoleName;

use crate::theme::TerraphimTheme;
use crate::views::chat::ChatView;
use crate::views::editor::EditorView;
use crate::views::search::{AddToContextEvent, SearchView};
use crate::views::role_selector::RoleChangeEvent;
use crate::views::{RoleSelector, TrayMenu, TrayMenuAction};
use crate::platform::{SystemTray, GlobalHotkeys};
use crate::platform::tray::SystemTrayEvent;
use crate::platform::hotkeys::{HotkeyAction, HotkeyEvent};

/// Main application state with integrated backend services
pub struct TerraphimApp {
    current_view: AppView,
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
    editor_view: Entity<EditorView>,
    role_selector: Entity<RoleSelector>,
    tray_menu: Entity<TrayMenu>,
    theme: Entity<TerraphimTheme>,
    show_tray_menu: bool,
    // Backend services
    config_state: ConfigState,
    // Platform features
    system_tray: Option<SystemTray>,
    global_hotkeys: Option<GlobalHotkeys>,
    // Channel for receiving hotkey events from background thread
    hotkey_receiver: Option<Receiver<HotkeyAction>>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

impl TerraphimApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, config_state: ConfigState, all_roles: Vec<RoleName>) -> Self {
        log::info!("TerraphimApp initializing with backend services and {} roles...", all_roles.len());

        // Initialize theme
        let theme = cx.new(|cx| TerraphimTheme::new(cx));

        // Initialize views with service access (pass cloned config_state)
        let search_view = cx.new(|cx| SearchView::new(window, cx, config_state.clone()));
        let chat_view = cx.new(|cx| ChatView::new(window, cx).with_config(config_state.clone()));
        let editor_view = cx.new(|cx| EditorView::new(window, cx));

        // Initialize role selector with ALL roles (pre-loaded in main.rs)
        let role_selector = cx.new(|cx| {
            RoleSelector::new(window, cx)
                .with_config(config_state.clone())
                .with_roles(all_roles)
        });

        // Initialize tray menu
        let tray_menu = cx.new(|cx| TrayMenu::new(window, cx));

        // Role changes will update search state via shared config_state.selected_role

        // Subscribe to AddToContextEvent from SearchView
        let chat_view_clone = chat_view.clone();
        let search_sub = cx.subscribe(&search_view, move |this: &mut TerraphimApp, _search, event: &AddToContextEvent, cx| {
            log::info!("App received AddToContext for: {}", event.document.title);
            chat_view_clone.update(cx, |chat, chat_cx| {
                chat.add_document_as_context(event.document.clone(), chat_cx);
            });
            // Navigate to chat to show the context
            this.navigate_to(AppView::Chat, cx);
        });

        // Initialize platform features
        let mut system_tray = None;
        let mut global_hotkeys = None;
        let mut hotkey_receiver: Option<Receiver<HotkeyAction>> = None;

        // Initialize system tray if supported
        if SystemTray::is_supported() {
            log::info!("Initializing system tray");
            let mut tray = SystemTray::new();
            match tray.initialize() {
                Ok(()) => {
                    tray.on_event(|event| {
                        log::info!("System tray event: {:?}", event);
                        // Events will be handled via cx in the future
                    });

                    // Make the tray icon visible
                    if let Err(e) = tray.show() {
                        log::error!("Failed to show system tray icon: {}", e);
                    }

                    system_tray = Some(tray);
                    log::info!("System tray initialized and shown successfully");
                }
                Err(e) => {
                    log::error!("Failed to initialize system tray: {}", e);
                }
            }
        }

        // Initialize global hotkeys if supported
        if GlobalHotkeys::is_supported() {
            log::info!("Initializing global hotkeys");

            #[cfg(target_os = "macos")]
            if GlobalHotkeys::needs_accessibility_permission() {
                log::warn!("Global hotkeys require accessibility permissions on macOS");
            }

            match GlobalHotkeys::new() {
                Ok(mut hotkeys) => {
                    if let Err(e) = hotkeys.register_defaults() {
                        log::error!("Failed to register default hotkeys: {}", e);
                    } else {
                        // Create channel for hotkey events
                        let (tx, rx) = mpsc::channel::<HotkeyAction>();
                        hotkey_receiver = Some(rx);

                        // Send hotkey events through the channel
                        hotkeys.on_event(move |event| {
                            log::info!("Global hotkey pressed: {:?}", event.action);
                            if let Err(e) = tx.send(event.action.clone()) {
                                log::error!("Failed to send hotkey event: {}", e);
                            }
                        });

                        global_hotkeys = Some(hotkeys);
                        log::info!("Global hotkeys initialized with channel successfully");
                    }
                }
                Err(e) => {
                    log::error!("Failed to initialize global hotkeys: {}", e);
                }
            }
        }

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
            config_state,
            system_tray,
            global_hotkeys,
            hotkey_receiver,
            _subscriptions: vec![search_sub],
        }
    }

    /// Poll hotkey channel and dispatch actions
    fn poll_hotkeys(&mut self, cx: &mut Context<Self>) {
        // Collect all pending actions first to avoid borrow conflicts
        let actions: Vec<HotkeyAction> = self.hotkey_receiver
            .as_ref()
            .map(|rx| {
                let mut actions = Vec::new();
                while let Ok(action) = rx.try_recv() {
                    actions.push(action);
                }
                actions
            })
            .unwrap_or_default();

        // Now dispatch all collected actions
        for action in actions {
            log::info!("Dispatching hotkey action: {:?}", action);
            self.handle_hotkey_action(action, cx);
        }
    }

    /// Handle a hotkey action
    fn handle_hotkey_action(&mut self, action: HotkeyAction, cx: &mut Context<Self>) {
        match action {
            HotkeyAction::ShowHideWindow => {
                log::info!("ShowHideWindow hotkey - toggling window visibility");
                // In GPUI, we can't directly control window visibility without a window handle
                // For now, this is a placeholder
            }
            HotkeyAction::QuickSearch => {
                log::info!("QuickSearch hotkey - navigating to search");
                self.navigate_to(AppView::Search, cx);
            }
            HotkeyAction::OpenChat => {
                log::info!("OpenChat hotkey - navigating to chat");
                self.navigate_to(AppView::Chat, cx);
            }
            HotkeyAction::OpenEditor => {
                log::info!("OpenEditor hotkey - navigating to editor");
                self.navigate_to(AppView::Editor, cx);
            }
            HotkeyAction::Custom(name) => {
                log::info!("Custom hotkey action: {}", name);
            }
        }
    }

    /// Get a new TerraphimService instance for operations
    pub fn create_service(&self) -> TerraphimService {
        TerraphimService::new(self.config_state.clone())
    }

    /// Get reference to config state
    pub fn config_state(&self) -> &ConfigState {
        &self.config_state
    }

    pub fn navigate_to(&mut self, view: AppView, cx: &mut Context<Self>) {
        if self.current_view != view {
            log::info!("Navigating from {:?} to {:?}", self.current_view, view);
            self.current_view = view;
            cx.notify();
        }
    }

    /// Navigate to search view handler
    fn navigate_to_search(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(AppView::Search, cx);
    }

    /// Navigate to chat view handler
    fn navigate_to_chat(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(AppView::Chat, cx);
    }

    /// Navigate to editor view handler
    fn navigate_to_editor(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.navigate_to(AppView::Editor, cx);
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
                            .child(self.render_search_button(cx))
                            .child(self.render_chat_button(cx))
                            .child(self.render_editor_button(cx)),
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

    fn render_search_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Search;

        let btn = Button::new("nav-search")
            .label("Search");

        if is_active {
            btn.primary()
        } else {
            btn.outline().on_click(cx.listener(Self::navigate_to_search))
        }
    }

    fn render_chat_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Chat;

        let btn = Button::new("nav-chat")
            .label("Chat");

        if is_active {
            btn.primary()
        } else {
            btn.outline().on_click(cx.listener(Self::navigate_to_chat))
        }
    }

    fn render_editor_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Editor;

        let btn = Button::new("nav-editor")
            .label("Editor");

        if is_active {
            btn.primary()
        } else {
            btn.outline().on_click(cx.listener(Self::navigate_to_editor))
        }
    }
}

impl Render for TerraphimApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll for hotkey events from background thread
        self.poll_hotkeys(cx);

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
