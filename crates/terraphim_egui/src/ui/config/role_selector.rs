//! Role selector component
//!
//! This module provides role selection functionality for switching
//! between different user roles and their configurations.

use crate::state::AppState;
use eframe::egui;
use terraphim_config::Role;

pub struct RoleSelector {
    // Selector state
    selected_role_index: usize,
}

impl RoleSelector {
    pub fn new(_state: &AppState) -> Self {
        Self {
            selected_role_index: 0,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.label(egui::RichText::new("👤 Role Selection").heading().strong());
        ui.add_space(8.0);

        // Get current role
        let current_role = state.get_current_role();
        let current_role_name_str = current_role.name.original.clone();
        drop(current_role);

        // Available roles (in a real implementation, this would come from terraphim_config)
        let available_roles = vec![
            "Default",
            "Rust Engineer",
            "Terraphim Engineer",
            "Researcher",
            "Writer",
            "System Administrator",
        ];

        // Find current role index
        if let Some(index) = available_roles.iter().position(|r| *r == current_role_name_str) {
            self.selected_role_index = index;
        }

        ui.label(egui::RichText::new("Current Role:").small().weak());
        ui.add_space(4.0);

        // Current role display
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.set_min_width(200.0);
                ui.horizontal(|ui| {
                    ui.label("🎭");
                    ui.label(
                        egui::RichText::new(&current_role_name_str)
                            .strong()
                            .heading(),
                    );
                });
            });

        ui.add_space(12.0);

        ui.label(egui::RichText::new("Available Roles:").small().weak());
        ui.add_space(4.0);

        // Role selection
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for (index, role_name) in available_roles.iter().enumerate() {
                    let is_selected = index == self.selected_role_index;
                    let bg_color = if is_selected {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().panel_fill
                    };

                    egui::Frame::default()
                        .fill(bg_color)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(if is_selected { "●" } else { "○" });
                                ui.label(*role_name);

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Select").clicked() {
                                        self.selected_role_index = index;
                                        // TODO: Actually switch roles using terraphim_config
                                    }
                                });
                            });
                        });
                }
            });

        ui.add_space(12.0);

        // Role description
        ui.label(egui::RichText::new("Role Description:").small().weak());
        ui.add_space(4.0);
        let description = match current_role_name_str.as_str() {
            "Default" => "General purpose role with balanced settings",
            "Rust Engineer" => "Specialized for Rust development and system programming",
            "Terraphim Engineer" => "Optimized for Terraphim AI development and maintenance",
            "Researcher" => "Focused on research, documentation, and knowledge management",
            "Writer" => "Designed for content creation and technical writing",
            "System Administrator" => "System administration and infrastructure management",
            _ => "Custom role configuration",
        };
        ui.label(description);
    }
}
