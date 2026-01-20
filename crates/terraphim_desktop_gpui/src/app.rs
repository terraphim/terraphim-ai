use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{StyledExt, button::*};
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use terraphim_config::ConfigState;
use terraphim_service::TerraphimService;
use terraphim_types::RoleName;

use crate::platform::hotkeys::HotkeyAction;
use crate::platform::tray::SystemTrayEvent;
use crate::platform::{GlobalHotkeys, SystemTray};
use crate::slash_command::CommandRegistry;
use crate::theme::colors::theme;
use crate::views::chat::ChatView;
use crate::views::editor::EditorView;
use crate::views::search::{AddToContextEvent, SearchView};
use crate::views::{RoleChangeEvent, RoleSelector, TrayMenu, TrayMenuAction};

/// Main application state with integrated backend services
pub struct TerraphimApp {
    current_view: AppView,
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
    editor_view: Entity<EditorView>,
    command_registry: Arc<CommandRegistry>,
    role_selector: Entity<RoleSelector>,
    tray_menu: Entity<TrayMenu>,
    show_tray_menu: bool,
    // Backend services
    config_state: ConfigState,
    // Platform features
    system_tray: Option<SystemTray>,
    global_hotkeys: Option<GlobalHotkeys>,
    // Channel for receiving hotkey events from background thread
    hotkey_receiver: Option<Receiver<HotkeyAction>>,
    // Channel for receiving tray events from background thread
    tray_event_receiver: Option<Receiver<SystemTrayEvent>>,
    // UI feedback
    notification: Option<String>,
    _subscriptions: Vec<Subscription>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Search,
    Chat,
    Editor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoleChangeSource {
    Tray,
    UiDropdown,
}

impl TerraphimApp {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        config_state: ConfigState,
        all_roles: Vec<RoleName>,
    ) -> Self {
        log::info!(
            "TerraphimApp initializing with backend services and {} roles...",
            all_roles.len()
        );

        let command_registry = Arc::new(CommandRegistry::with_builtin_commands());

        // Initialize views with service access (pass cloned config_state)
        let search_registry = command_registry.clone();
        let chat_registry = command_registry.clone();
        let editor_registry = command_registry.clone();

        let search_view =
            cx.new(|cx| SearchView::new(window, cx, config_state.clone(), search_registry));
        let chat_view =
            cx.new(|cx| ChatView::new(window, cx, chat_registry).with_config(config_state.clone()));
        let editor_view = cx.new(|cx| EditorView::new(window, cx, editor_registry));

        // Initialize role selector with ALL roles (pre-loaded in main.rs)
        let all_roles_for_tray = all_roles.clone(); // Clone for tray use later
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
        let search_sub = cx.subscribe(
            &search_view,
            move |this: &mut TerraphimApp, _search, event: &AddToContextEvent, cx| {
                log::info!(
                    "App received AddToContext for: {} (navigate_to_chat: {})",
                    event.document.title,
                    event.navigate_to_chat
                );

                // Show notification
                this.notification = Some(format!("Added '{}' to context", event.document.title));
                cx.notify();

                // Auto-hide notification after 3 seconds
                cx.spawn(async move |_this, _cx| {
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                })
                .detach();

                // Directly add to context (no modal from search results)
                chat_view_clone.update(cx, |chat, chat_cx| {
                    chat.add_document_as_context_direct(event.document.clone(), chat_cx);
                });

                // Navigate to chat only if requested (from "Chat with Document" button)
                if event.navigate_to_chat {
                    this.navigate_to(AppView::Chat, cx);
                }
            },
        );

        // Subscribe to RoleChangeEvent from RoleSelector (UI dropdown)
        let role_sub = cx.subscribe(
            &role_selector,
            |this: &mut TerraphimApp, _selector, event: &RoleChangeEvent, cx| {
                this.apply_role_change(event.new_role.clone(), RoleChangeSource::UiDropdown, cx);
            },
        );

        // Initialize platform features
        let mut system_tray = None;
        let mut global_hotkeys = None;
        let mut hotkey_receiver: Option<Receiver<HotkeyAction>> = None;
        let mut tray_event_receiver: Option<Receiver<SystemTrayEvent>> = None;

        // Initialize system tray if supported (with dynamic role list like Tauri)
        if SystemTray::is_supported() {
            log::info!("Initializing system tray with roles");

            // Get selected role from config_state
            let selected_role = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { config_state.get_selected_role().await })
            });

            log::info!(
                "System tray: roles count = {}, selected = {:?}",
                all_roles_for_tray.len(),
                selected_role
            );

            let mut tray = SystemTray::with_roles(all_roles_for_tray, selected_role);
            match tray.initialize() {
                Ok(()) => {
                    // Create channel for tray events (same pattern as hotkeys)
                    let (tray_tx, tray_rx) = mpsc::channel::<SystemTrayEvent>();
                    tray_event_receiver = Some(tray_rx);

                    // Set handler FIRST (before starting listener threads)
                    tray.on_event(move |event| {
                        log::info!("System tray event received: {:?}", event);
                        if let Err(e) = tray_tx.send(event) {
                            log::error!("Failed to send tray event: {}", e);
                        }

                        // Wake up the app event loop to process the event immediately
                        // This fixes the issue where events are only polled during render()
                        crate::platform::wake_app_event_loop();
                    });

                    // THEN start listener threads (handler is guaranteed to be set now)
                    // This fixes the race condition where threads would check for handler before it was set
                    tray.start_listening();

                    // Make the tray icon visible
                    if let Err(e) = tray.show() {
                        log::error!("Failed to show system tray icon: {}", e);
                    }

                    system_tray = Some(tray);
                    log::info!("System tray initialized with channel successfully");
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
            command_registry,
            role_selector,
            tray_menu,
            show_tray_menu: false,
            config_state,
            system_tray,
            global_hotkeys,
            hotkey_receiver,
            tray_event_receiver,
            notification: None,
            _subscriptions: vec![search_sub, role_sub],
        }
    }

    /// Poll hotkey channel and dispatch actions
    fn poll_hotkeys(&mut self, cx: &mut Context<Self>) {
        // Collect all pending actions first to avoid borrow conflicts
        let actions: Vec<HotkeyAction> = self
            .hotkey_receiver
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
        log::info!("=== HOTKEY ACTION HANDLER ===");
        log::info!("Action: {:?}", action);
        log::info!("Current view BEFORE: {:?}", self.current_view);

        match action {
            HotkeyAction::ShowHideWindow => {
                log::info!("ShowHideWindow hotkey - toggling window visibility");
                // In GPUI, we can't directly control window visibility without a window handle
                // For now, this is a placeholder
            }
            HotkeyAction::QuickSearch => {
                log::info!("QuickSearch hotkey - setting view to Search directly");
                self.current_view = AppView::Search;
                cx.notify();
                log::info!("View AFTER direct assignment: {:?}", self.current_view);
            }
            HotkeyAction::OpenChat => {
                log::info!("OpenChat hotkey - setting view to Chat directly");
                self.current_view = AppView::Chat;
                cx.notify();
                log::info!("View AFTER direct assignment: {:?}", self.current_view);
            }
            HotkeyAction::OpenEditor => {
                log::info!("OpenEditor hotkey - setting view to Editor directly");
                self.current_view = AppView::Editor;
                cx.notify();
                log::info!("View AFTER direct assignment: {:?}", self.current_view);
            }
            HotkeyAction::Custom(name) => {
                log::info!("Custom hotkey action: {}", name);
            }
        }

        log::info!("=== HOTKEY ACTION COMPLETE ===");
    }

    /// Poll tray event channel and dispatch actions (same pattern as poll_hotkeys)
    fn poll_tray_events(&mut self, cx: &mut Context<Self>) {
        // Collect all pending events first to avoid borrow conflicts
        let events: Vec<SystemTrayEvent> = self
            .tray_event_receiver
            .as_ref()
            .map(|rx| {
                let mut events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    events.push(event);
                }
                events
            })
            .unwrap_or_default();

        // Now dispatch all collected events
        for event in events {
            log::info!("Dispatching tray event: {:?}", event);
            self.handle_tray_event(event, cx);
        }
    }

    fn apply_role_change(
        &mut self,
        role: RoleName,
        source: RoleChangeSource,
        cx: &mut Context<Self>,
    ) {
        log::info!("Applying role change to {:?} (source: {:?})", role, source);

        // 1) Update config_state.selected_role
        let config_state = self.config_state.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut config = config_state.config.lock().await;
                config.selected_role = role.clone();
                log::info!("ConfigState.selected_role updated to '{}'", role);
            });
        });

        // 2) Update role selector UI
        self.role_selector.update(cx, |selector, selector_cx| {
            selector.set_selected_role(role.clone(), selector_cx);
        });

        // 3) Update search view
        self.search_view.update(cx, |search_view, search_cx| {
            search_view.update_role(role.to_string(), search_cx);
            log::info!("SearchView updated with new role: {}", role);
        });

        // 4) Update chat view
        self.chat_view.update(cx, |chat_view, chat_cx| {
            chat_view.update_role(role.to_string(), chat_cx);
            log::info!("ChatView updated with new role: {}", role);
        });

        // 5) Update editor view
        self.editor_view.update(cx, |editor_view, editor_cx| {
            editor_view.update_role(role.to_string(), editor_cx);
            log::info!("EditorView updated with new role: {}", role);
        });

        // 6) Update system tray menu checkmark (if supported / initialized)
        if let Some(ref mut tray) = self.system_tray {
            if let Err(e) = tray.update_selected_role(role.clone()) {
                log::error!("Failed to update tray menu role: {}", e);
            }
        }

        cx.notify();
    }

    /// Handle a system tray event
    fn handle_tray_event(&mut self, event: SystemTrayEvent, cx: &mut Context<Self>) {
        log::info!("=== TRAY EVENT HANDLER ===");
        log::info!("Event: {:?}", event);

        match event {
            SystemTrayEvent::Quit => {
                log::info!("Quit requested via tray - exiting application");
                cx.quit();
            }
            SystemTrayEvent::ChangeRole(role) => {
                log::info!("Role change requested via tray: {:?}", role);
                self.apply_role_change(role, RoleChangeSource::Tray, cx);
            }
            SystemTrayEvent::ToggleWindow => {
                log::info!("Toggle window requested via tray");
                // Window visibility control would go here
            }
            SystemTrayEvent::TrayIconClick => {
                // Ignore mouse move/click events - these are noisy
            }
        }

        log::info!("=== TRAY EVENT COMPLETE ===");
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
    fn navigate_to_search(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.navigate_to(AppView::Search, cx);
    }

    /// Navigate to chat view handler
    fn navigate_to_chat(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.navigate_to(AppView::Chat, cx);
    }

    /// Navigate to editor view handler
    fn navigate_to_editor(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            .bg(theme::surface())
            .border_b_1()
            .border_color(theme::border())
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
                            .text_color(theme::text_primary())
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
                    .relative()
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
                                theme::primary()
                            } else {
                                theme::background()
                            })
                            .text_color(if self.show_tray_menu {
                                theme::primary_text()
                            } else {
                                theme::text_primary()
                            })
                            .hover(|style| style.bg(theme::surface_hover()).cursor_pointer())
                            .child("Menu"),
                    ),
            )
    }

    /// Render role selector dropdown as overlay (appears above all content)
    fn render_role_dropdown_overlay(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let is_open = self.role_selector.read(cx).is_dropdown_open();

        if !is_open {
            return None;
        }

        // Render dropdown as absolute overlay at top-right (positioned relative to app container)
        // This ensures it appears above all other content including the search input
        Some(
            div()
                .absolute()
                .top(px(64.0)) // Below navigation bar (p_4 = 16px, button height ~32px, gap)
                .right(px(16.0)) // Right edge with padding
                .w(px(220.0))
                .max_h(px(300.0))
                .overflow_hidden()
                .bg(theme::background())
                .border_1()
                .border_color(theme::border())
                .rounded_md()
                .shadow_lg()
                .child(self.render_role_dropdown_content(cx)),
        )
    }

    /// Render the actual dropdown content
    fn render_role_dropdown_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let role_selector = self.role_selector.read(cx);
        let current_role = role_selector.current_role().clone();
        let available_roles = role_selector.available_roles().to_vec();
        let roles_to_render: Vec<(usize, RoleName, bool)> = available_roles
            .iter()
            .enumerate()
            .map(|(idx, role)| (idx, role.clone(), role == &current_role))
            .collect();

        div()
            .overflow_hidden()
            .children(roles_to_render.iter().map(|(idx, role, is_current)| {
                let role_name = role.to_string();
                let icon = role_selector.get_role_icon(role);
                let current = *is_current;
                let index = *idx;

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_2()
                    .border_b_1()
                    .border_color(theme::border_light())
                    .when(current, |this| this.bg(theme::surface()))
                    .child(
                        Button::new(("role-item", index))
                            .label(role_name)
                            .icon(icon)
                            .ghost()
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.role_selector.update(cx, |selector, selector_cx| {
                                    selector.select_role(index, selector_cx);
                                });
                            })),
                    )
                    .children(if current {
                        Some(div().text_color(theme::success()).text_sm().child("*"))
                    } else {
                        None
                    })
            }))
    }

    fn render_search_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Search;

        let btn = Button::new("nav-search").label("Search");

        if is_active {
            btn.primary()
        } else {
            btn.outline()
                .on_click(cx.listener(Self::navigate_to_search))
        }
    }

    fn render_chat_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Chat;

        let btn = Button::new("nav-chat").label("Chat");

        if is_active {
            btn.primary()
        } else {
            btn.outline().on_click(cx.listener(Self::navigate_to_chat))
        }
    }

    fn render_editor_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active = self.current_view == AppView::Editor;

        let btn = Button::new("nav-editor").label("Editor");

        if is_active {
            btn.primary()
        } else {
            btn.outline()
                .on_click(cx.listener(Self::navigate_to_editor))
        }
    }
}

impl Render for TerraphimApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll for hotkey events from background thread
        self.poll_hotkeys(cx);
        // Poll for tray events from background thread
        self.poll_tray_events(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::background())
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
            .children(self.render_role_dropdown_overlay(cx))
            // Notification banner
            .when(self.notification.is_some(), |this| {
                this.child(
                    div()
                        .absolute()
                        .top_4()
                        .right_4()
                        .px_4()
                        .py_2()
                        .bg(theme::success())
                        .text_color(theme::primary_text())
                        .rounded_md()
                        .shadow_lg()
                        .child(self.notification.as_ref().unwrap().clone()),
                )
            })
    }
}

// NOTE: No unit tests here.
//
// When this module is built for the binary (`src/main.rs`), it is included as a
// module in the `terraphim-gpui` crate. Keeping `#[cfg(test)]` tests here caused
// Rust's `#[test]` macro expansion to hit a recursion limit in this workspace.
//
// Place unit tests in `crates/terraphim_desktop_gpui/tests/*` instead.
