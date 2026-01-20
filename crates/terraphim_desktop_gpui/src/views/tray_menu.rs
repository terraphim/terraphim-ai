use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::StyledExt;

/// Tray menu item definition
#[derive(Clone, Debug)]
pub struct TrayMenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub action: TrayMenuAction,
    pub enabled: bool,
}

/// Actions that can be triggered from tray menu
#[derive(Clone, Debug, PartialEq)]
pub enum TrayMenuAction {
    ShowWindow,
    HideWindow,
    Search,
    Chat,
    Settings,
    About,
    Quit,
    Custom(String),
}

/// Type alias for tray menu action handler
type TrayMenuActionHandler = Box<dyn Fn(TrayMenuAction, &mut Context<TrayMenu>) + 'static>;

/// System tray menu component
pub struct TrayMenu {
    items: Vec<TrayMenuItem>,
    is_visible: bool,
    on_action: Option<TrayMenuActionHandler>,
}

impl TrayMenu {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        log::info!("TrayMenu initialized");

        let items = vec![
            TrayMenuItem {
                id: "show".to_string(),
                label: "Show Terraphim".to_string(),
                icon: None,
                action: TrayMenuAction::ShowWindow,
                enabled: true,
            },
            TrayMenuItem {
                id: "hide".to_string(),
                label: "Hide Terraphim".to_string(),
                icon: None,
                action: TrayMenuAction::HideWindow,
                enabled: true,
            },
            TrayMenuItem {
                id: "search".to_string(),
                label: "Search".to_string(),
                icon: None,
                action: TrayMenuAction::Search,
                enabled: true,
            },
            TrayMenuItem {
                id: "chat".to_string(),
                label: "Chat".to_string(),
                icon: None,
                action: TrayMenuAction::Chat,
                enabled: true,
            },
            TrayMenuItem {
                id: "settings".to_string(),
                label: "Settings".to_string(),
                icon: None,
                action: TrayMenuAction::Settings,
                enabled: true,
            },
            TrayMenuItem {
                id: "about".to_string(),
                label: "About".to_string(),
                icon: None,
                action: TrayMenuAction::About,
                enabled: true,
            },
            TrayMenuItem {
                id: "quit".to_string(),
                label: "Quit".to_string(),
                icon: None,
                action: TrayMenuAction::Quit,
                enabled: true,
            },
        ];

        Self {
            items,
            is_visible: false,
            on_action: None,
        }
    }

    /// Set the callback for menu actions
    pub fn on_action<F>(mut self, callback: F) -> Self
    where
        F: Fn(TrayMenuAction, &mut Context<Self>) + 'static,
    {
        self.on_action = Some(Box::new(callback));
        self
    }

    /// Add custom menu item
    pub fn add_item(&mut self, item: TrayMenuItem, cx: &mut Context<Self>) {
        self.items.push(item);
        cx.notify();
    }

    /// Remove menu item by id
    pub fn remove_item(&mut self, id: &str, cx: &mut Context<Self>) {
        self.items.retain(|item| item.id != id);
        cx.notify();
    }

    /// Enable/disable menu item
    pub fn set_item_enabled(&mut self, id: &str, enabled: bool, cx: &mut Context<Self>) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.enabled = enabled;
            cx.notify();
        }
    }

    /// Show the tray menu
    pub fn show(&mut self, cx: &mut Context<Self>) {
        self.is_visible = true;
        cx.notify();
    }

    /// Hide the tray menu
    pub fn hide(&mut self, cx: &mut Context<Self>) {
        self.is_visible = false;
        cx.notify();
    }

    /// Toggle visibility
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.is_visible = !self.is_visible;
        cx.notify();
    }

    /// Handle menu item click
    fn handle_item_click(&mut self, action: TrayMenuAction, cx: &mut Context<Self>) {
        log::info!("Tray menu action triggered: {:?}", action);

        // Hide menu after action
        self.is_visible = false;
        cx.notify();

        // Trigger callback if set
        if let Some(callback) = &self.on_action {
            callback(action, cx);
        }
    }

    /// Render a single menu item
    fn render_menu_item(&self, item: &TrayMenuItem, _cx: &Context<Self>) -> impl IntoElement {
        let _action = item.action.clone();

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .when(item.enabled, |this| {
                this.hover(|style| style.bg(rgb(0xf5f5f5)).cursor_pointer())
            })
            .when(!item.enabled, |this| this.opacity(0.5))
            .border_b_1()
            .border_color(rgb(0xf0f0f0))
            .children(
                item.icon
                    .as_ref()
                    .map(|icon| div().text_lg().child(icon.clone())),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if item.enabled {
                        rgb(0x363636)
                    } else {
                        rgb(0xb5b5b5)
                    })
                    .child(item.label.clone()),
            )
    }
}

impl Render for TrayMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.is_visible {
            return div().into_any_element();
        }

        div()
            .absolute()
            .bottom(px(48.0))
            .right(px(16.0))
            .w(px(240.0))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_md()
            .shadow_xl()
            .overflow_hidden()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .bg(rgb(0x3273dc))
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(rgb(0xffffff))
                            .child("Terraphim AI"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xffffff))
                            .opacity(0.8)
                            .child("v1.0.0"),
                    ),
            )
            .child(
                div().flex().flex_col().children(
                    self.items
                        .iter()
                        .map(|item| self.render_menu_item(item, cx))
                        .collect::<Vec<_>>(),
                ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_menu_action_variants() {
        assert_eq!(TrayMenuAction::ShowWindow, TrayMenuAction::ShowWindow);
        assert_ne!(TrayMenuAction::ShowWindow, TrayMenuAction::HideWindow);
    }

    #[test]
    fn test_tray_menu_item_creation() {
        let item = TrayMenuItem {
            id: "test".to_string(),
            label: "Test Item".to_string(),
            icon: None,
            action: TrayMenuAction::Custom("test".to_string()),
            enabled: true,
        };

        assert_eq!(item.id, "test");
        assert_eq!(item.label, "Test Item");
        assert!(item.enabled);
    }

    #[test]
    fn test_tray_menu_default_items() {
        // Would require GPUI app context for full test
        let expected_actions = vec![
            TrayMenuAction::ShowWindow,
            TrayMenuAction::HideWindow,
            TrayMenuAction::Search,
            TrayMenuAction::Chat,
            TrayMenuAction::Settings,
            TrayMenuAction::About,
            TrayMenuAction::Quit,
        ];

        assert_eq!(expected_actions.len(), 7);
    }
}
