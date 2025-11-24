use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;
use terraphim_types::RoleName;

/// Role selector dropdown component
pub struct RoleSelector {
    current_role: RoleName,
    available_roles: Vec<RoleName>,
    is_open: bool,
    on_role_change: Option<Box<dyn Fn(RoleName, &mut Context<Self>) + 'static>>,
}

impl RoleSelector {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        log::info!("RoleSelector initialized");

        // Default roles - in production, these would come from config
        let available_roles = vec![
            RoleName::from("default"),
            RoleName::from("engineer"),
            RoleName::from("researcher"),
            RoleName::from("writer"),
            RoleName::from("data_scientist"),
        ];

        Self {
            current_role: RoleName::from("default"),
            available_roles,
            is_open: false,
            on_role_change: None,
        }
    }

    /// Set the callback for role changes
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(RoleName, &mut Context<Self>) + 'static,
    {
        self.on_role_change = Some(Box::new(callback));
        self
    }

    /// Set available roles from config
    pub fn with_roles(mut self, roles: Vec<RoleName>) -> Self {
        self.available_roles = roles;
        self
    }

    /// Get current role
    pub fn current_role(&self) -> &RoleName {
        &self.current_role
    }

    /// Set current role
    pub fn set_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        if self.current_role != role {
            log::info!("Role changed from {} to {}", self.current_role, role);
            self.current_role = role.clone();
            cx.notify();

            // Trigger callback if set
            if let Some(callback) = &self.on_role_change {
                callback(role, cx);
            }
        }
    }

    /// Toggle dropdown open/closed
    fn toggle_dropdown(&mut self, cx: &mut Context<Self>) {
        self.is_open = !self.is_open;
        cx.notify();
    }

    /// Select a role from dropdown
    fn select_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        self.set_role(role, cx);
        self.is_open = false;
        cx.notify();
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

    /// Render dropdown menu
    fn render_dropdown(&self, _cx: &Context<Self>) -> impl IntoElement {
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
                self.available_roles
                    .iter()
                    .map(|role| {
                        let is_current = role == &self.current_role;
                        let role_clone = role.clone();

                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_3()
                            .when(is_current, |this| {
                                this.bg(rgb(0xf5f5f5))
                            })
                            .when(!is_current, |this| {
                                this.hover(|style| style.bg(rgb(0xf9f9f9)).cursor_pointer())
                            })
                            .border_b_1()
                            .border_color(rgb(0xf0f0f0))
                            .child(self.render_role_display(role, _cx))
                            .children(
                                if is_current {
                                    Some(div()
                                        .text_color(rgb(0x48c774))
                                        .text_sm()
                                        .child("✓"))
                                } else {
                                    None
                                },
                            )
                    })
                    .collect::<Vec<_>>(),
            )
    }
}

impl Render for RoleSelector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .child(
                // Main button
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .min_w(px(200.0))
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(if self.is_open {
                        rgb(0x3273dc)
                    } else {
                        rgb(0xdbdbdb)
                    })
                    .rounded_md()
                    .hover(|style| style.border_color(rgb(0xb5b5b5)).cursor_pointer())
                    .child(self.render_role_display(&self.current_role, cx))
                    .child(
                        div()
                            .text_color(rgb(0x7a7a7a))
                            .text_sm()
                            .child(if self.is_open { "▲" } else { "▼" }),
                    ),
            )
            .when(self.is_open, |this| {
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
