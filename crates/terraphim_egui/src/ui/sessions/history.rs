//! Session history management
//!
//! This module handles the display and management of conversation
//! sessions and their history.

use crate::state::AppState;
use eframe::egui;

pub struct SessionHistory {
    // History state
}

impl SessionHistory {
    pub fn new(_state: &AppState) -> Self {
        Self {}
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.label(egui::RichText::new("📜 Session History").heading().strong());
        ui.add_space(8.0);

        let history = {
            let hist = state.get_conversation_history();
            hist.clone()
        };

        if history.is_empty() {
            ui.label(egui::RichText::new("No conversation history").weak().italics());
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for message in &history {
                    self.render_message(ui, message);
                }
            });
    }

    fn render_message(&self, ui: &mut egui::Ui, message: &crate::state::ChatMessage) {
        let is_user = matches!(message.role, crate::state::ChatMessageRole::User);

        let align = if is_user {
            egui::Align::RIGHT
        } else {
            egui::Align::LEFT
        };

        ui.with_layout(egui::Layout::top_down(align), |ui| {
            egui::Frame::default()
                .inner_margin(8.0)
                .show(ui, |ui| {
                    // Message header
                    ui.horizontal(|ui| {
                        let role_text = match message.role {
                            crate::state::ChatMessageRole::User => "👤 You",
                            crate::state::ChatMessageRole::Assistant => "🤖 Assistant",
                            crate::state::ChatMessageRole::System => "⚙️ System",
                        };
                        ui.label(egui::RichText::new(role_text).small().weak());
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                message.timestamp.format("%H:%M:%S").to_string()
                            )
                            .small()
                            .weak(),
                        );
                    });

                    ui.add_space(4.0);

                    // Message content (first 100 chars)
                    let content = if message.content.len() > 100 {
                        format!("{}...", &message.content[..100])
                    } else {
                        message.content.clone()
                    };
                    ui.label(content);
                });
        });
    }
}
