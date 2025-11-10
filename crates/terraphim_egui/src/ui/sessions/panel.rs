//! Sessions panel
//!
//! This module provides the sessions panel UI for managing
//! conversation history and saved sessions.

use crate::state::AppState;
use eframe::egui;

pub struct SessionsPanel {
    // Panel state
}

impl SessionsPanel {
    pub fn new(state: &AppState) -> Self {
        Self {}
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.label(egui::RichText::new("Sessions").heading().strong());
        ui.add_space(8.0);
        ui.label("Sessions will be implemented in Phase 9");
    }
}
