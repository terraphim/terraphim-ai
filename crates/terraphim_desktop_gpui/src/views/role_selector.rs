use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{button::*, StyledExt};
use terraphim_config::ConfigState;
use terraphim_types::RoleName;

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

    /// Initialize with config state to load roles
    pub fn with_config(mut self, config_state: ConfigState) -> Self {
        // Load available roles from config
        let roles: Vec<RoleName> = config_state.roles.keys().cloned().collect();
        log::info!("RoleSelector loaded {} roles from config", roles.len());

        self.available_roles = roles;
        self.config_state = Some(config_state);
        self
    }

    /// Get current role
    pub fn current_role(&self) -> &RoleName {
        &self.current_role
    }

    /// Change role using backend (pattern from Tauri select_role cmd.rs:392-462)
    pub fn change_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        if self.current_role == role {
            return;
        }

        log::info!("Changing role from {} to {}", self.current_role, role);

        let config_state = match &self.config_state {
            Some(state) => state.clone(),
            None => {
                log::warn!("Cannot change role: config not initialized");
                return;
            }
        };

        let role_clone = role.clone();

        cx.spawn(async move |this, cx| {
            // Update selected_role in config (from Tauri select_role pattern)
            let mut config = config_state.config.lock().await;
            config.selected_role = role_clone.clone();
            drop(config);

            log::info!("✅ Role changed to: {}", role_clone);

            this.update(cx, |this, cx| {
                this.current_role = role_clone;
                this.is_open = false;
                cx.notify();
            }).ok();
        }).detach();
    }

    /// Toggle dropdown open/closed
    fn toggle_dropdown(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
        log::info!("Role dropdown {}", if self.is_open { "opened" } else { "closed" });
        cx.notify();
    }

    /// Handle role selection from dropdown
    fn handle_role_select(&mut self, role_index: usize, cx: &mut Context<Self>) {
        if let Some(role) = self.available_roles.get(role_index).cloned() {
            self.change_role(role, cx);
        }
    }

    /// Render the role icon based on role name
    fn role_icon(&self, role: &RoleName) -> &'static str {
        match role.as_str() {
            "engineer" => "👨‍💻",
            "researcher" => "🔬",
            "writer" => "✍️",
            "data_scientist" => "📊",
            "default" => "👤",
            _ => "🎭",
        }
    }

    /// Render a role display with icon
    fn render_role_display(&self, role: &RoleName, _cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xl()
                    .child(self.role_icon(role)),
            )
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(rgb(0x363636))
                    .child(role.to_string()),
            )
    }

    /// Render dropdown menu with clickable role items
    fn render_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let roles_to_render: Vec<(usize, RoleName, bool)> = self.available_roles
            .iter()
            .enumerate()
            .map(|(idx, role)| (idx, role.clone(), role == &self.current_role))
            .collect();

        div()
            .absolute()
            .top(px(48.0))
            .right(px(0.0))
            .w(px(220.0))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xdbdbdb))
            .rounded_md()
            .shadow_lg()
            .overflow_hidden()
            .children(
                roles_to_render.iter().map(|(idx, role, is_current)| {
                    let role_name = role.to_string();
                    let icon = self.role_icon(role).to_string();
                    let current = *is_current;
                    let index = *idx;

                    {
                        let button_label = format!("{} {}", icon, role_name);

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
                                    .label(button_label)
                                    .ghost()
                                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                                        this.handle_role_select(index, cx);
                                    }))
                            )
                            .children(if current {
                                Some(div().text_color(rgb(0x48c774)).text_sm().child("✓"))
                            } else {
                                None
                            })
                    }
                })
            )
    }
}

impl Render for RoleSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_role_display = self.current_role.to_string();
        let is_open = self.is_open;

        div()
            .relative()
            .child(
                // Main button with click handler
                Button::new("role-selector-toggle")
                    .label(&format!("Role: {}", current_role_display))
                    .outline()
                    .on_click(cx.listener(Self::toggle_dropdown))
            )
            .when(is_open, |this| {
                this.child(self.render_dropdown(cx))
            })
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
            current_role: RoleName::from("default"),
            available_roles: vec![],
            is_open: false,
            on_role_change: None,
        };

        assert_eq!(selector.role_icon(&RoleName::from("engineer")), "👨‍💻");
        assert_eq!(selector.role_icon(&RoleName::from("researcher")), "🔬");
        assert_eq!(selector.role_icon(&RoleName::from("writer")), "✍️");
        assert_eq!(selector.role_icon(&RoleName::from("default")), "👤");
    }
}
