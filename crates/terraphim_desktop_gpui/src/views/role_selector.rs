use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{IconName, StyledExt, button::*};
use terraphim_config::ConfigState;
use terraphim_types::RoleName;

use crate::theme::colors::theme;

/// Event emitted when role changes
pub struct RoleChangeEvent {
    pub new_role: RoleName,
}

impl EventEmitter<RoleChangeEvent> for RoleSelector {}

/// Role selector dropdown with real backend integration
pub struct RoleSelector {
    config_state: Option<ConfigState>,
    current_role: RoleName,
    available_roles: Vec<RoleName>,
    is_open: bool,
}

impl RoleSelector {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        log::info!("RoleSelector initialized");

        Self {
            config_state: None,
            current_role: RoleName::from("Terraphim Engineer"),
            available_roles: vec![],
            is_open: false,
        }
    }

    /// Check if dropdown is open (for app-level overlay rendering)
    pub fn is_dropdown_open(&self) -> bool {
        self.is_open
    }

    /// Get available roles (for app-level dropdown rendering)
    pub fn available_roles(&self) -> &[RoleName] {
        &self.available_roles
    }

    /// Get role icon (public for app-level rendering)
    pub fn get_role_icon(&self, role: &RoleName) -> IconName {
        self.role_icon(role)
    }

    /// Handle role selection (public for app-level rendering)
    pub fn select_role(&mut self, role_index: usize, cx: &mut Context<Self>) {
        self.handle_role_select(role_index, cx)
    }

    /// Initialize with config state
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        self.config_state = Some(config_state);
        self
    }

    /// Set available roles (loaded from config in App)
    pub fn with_roles(mut self, roles: Vec<RoleName>) -> Self {
        log::info!(
            "RoleSelector loaded {} roles from config (Tauri pattern)",
            roles.len()
        );
        self.available_roles = roles;
        self
    }

    /// Get current role
    pub fn current_role(&self) -> &RoleName {
        &self.current_role
    }

    /// Set selected role directly (called from tray menu)
    /// Unlike change_role, this doesn't update config_state (already done by caller)
    pub fn set_selected_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        log::info!("RoleSelector: setting selected role to {}", role);
        self.current_role = role;
        self.is_open = false;
        cx.notify();
    }

    /// Request a role change (App will apply via RoleChangeEvent subscription)
    pub fn change_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        if self.current_role == role {
            self.is_open = false;
            cx.notify();
            return;
        }

        log::info!(
            "RoleSelector: requesting role change from {} to {}",
            self.current_role,
            role
        );

        // Close immediately; App will update selected role + views + tray.
        self.is_open = false;
        cx.emit(RoleChangeEvent { new_role: role });
        cx.notify();
    }

    /// Toggle dropdown open/closed
    pub fn toggle_dropdown(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_open = !self.is_open;
        log::info!(
            "Role dropdown {}",
            if self.is_open { "opened" } else { "closed" }
        );
        cx.notify();
    }

    /// Close dropdown (public for app-level handling)
    pub fn close_dropdown(&mut self, cx: &mut Context<Self>) {
        if self.is_open {
            self.is_open = false;
            cx.notify();
        }
    }

    /// Handle role selection from dropdown
    fn handle_role_select(&mut self, role_index: usize, cx: &mut Context<Self>) {
        if let Some(role) = self.available_roles.get(role_index).cloned() {
            self.change_role(role, cx);
        }
    }

    /// Get lucide icon for role
    fn role_icon(&self, role: &RoleName) -> IconName {
        let role_lower = role.as_str().to_lowercase();
        if role_lower.contains("rust") {
            IconName::GitHub // Rust (open source/code)
        } else if role_lower.contains("python") {
            IconName::SquareTerminal // Python (terminal/scripting)
        } else if role_lower.contains("frontend") || role_lower.contains("front-end") {
            IconName::Palette // Frontend (design/colors)
        } else if role_lower.contains("terraphim") {
            IconName::Settings2 // Terraphim (system/config)
        } else if role_lower.contains("engineer") {
            IconName::SquareTerminal // Generic engineer
        } else if role_lower.contains("researcher") {
            IconName::BookOpen // Researcher
        } else if role_lower.contains("writer") {
            IconName::File // Writer
        } else if role_lower.contains("data") {
            IconName::ChartPie // Data scientist
        } else if role_lower.contains("default") {
            IconName::CircleUser // Default user
        } else {
            IconName::User // Fallback
        }
    }

    /// Render a role display with icon
    fn render_role_display(&self, role: &RoleName, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_xl().child(self.role_icon(role)))
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(theme::text_primary())
                    .child(role.to_string()),
            )
    }

    /// Render dropdown menu with clickable role items
    #[allow(dead_code)]
    fn render_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let roles_to_render: Vec<(usize, RoleName, bool)> = self
            .available_roles
            .iter()
            .enumerate()
            .map(|(idx, role)| (idx, role.clone(), role == &self.current_role))
            .collect();

        div()
            .absolute()
            .top(px(48.0))
            .right(px(0.0))
            .w(px(220.0))
            .max_h(px(300.0)) // Limit height to prevent extending too far down
            .overflow_hidden() // Clip content that exceeds max height
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_md()
            .shadow_lg()
            .overflow_hidden()
            .children(roles_to_render.iter().map(|(idx, role, is_current)| {
                let role_name = role.to_string();
                let icon = self.role_icon(role);
                let current = *is_current;
                let index = *idx;

                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0xf0f0f0))
                    .when(current, |this| this.bg(rgb(0xf5f5f5)))
                    .child(
                        Button::new(("role-item", index))
                            .label(role_name)
                            .icon(icon)
                            .ghost()
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.handle_role_select(index, cx);
                            })),
                    )
                    .children(if current {
                        Some(div().text_color(rgb(0x48c774)).text_sm().child("*"))
                    } else {
                        None
                    })
            }))
    }
}

impl Render for RoleSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_role_display = self.current_role.to_string();
        let current_icon = self.role_icon(&self.current_role);

        // Only render the button - dropdown will be rendered as overlay in app
        div().relative().child(
            // Main button with lucide icon
            Button::new("role-selector-toggle")
                .label(&format!("Role: {}", current_role_display))
                .icon(current_icon)
                .outline()
                .on_click(cx.listener(Self::toggle_dropdown)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_selector_creation() {
        // Would require GPUI app context
        assert_eq!(RoleName::from("engineer").as_str(), "engineer");
    }

    #[test]
    fn test_role_icon_mapping() {
        let selector = RoleSelector {
            config_state: None,
            current_role: RoleName::from("default"),
            available_roles: vec![],
            is_open: false,
        };

        assert_eq!(
            selector.get_role_icon(&RoleName::from("engineer")),
            IconName::SquareTerminal
        );
        assert_eq!(
            selector.get_role_icon(&RoleName::from("researcher")),
            IconName::BookOpen
        );
        assert_eq!(
            selector.get_role_icon(&RoleName::from("writer")),
            IconName::File
        );
        assert_eq!(
            selector.get_role_icon(&RoleName::from("default")),
            IconName::CircleUser
        );
    }
}
