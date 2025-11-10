//! Sessions panel
//!
//! This module provides the sessions panel UI for managing
//! conversation history and saved sessions.

use crate::state::AppState;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Saved session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub id: String,
    pub name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub conversation: Vec<crate::state::ChatMessage>,
    pub context: Vec<terraphim_types::Document>,
    pub search_query: Option<String>,
    pub current_role: String,
}

pub struct SessionsPanel {
    /// Saved sessions
    saved_sessions: Vec<SavedSession>,
    /// Selected session ID
    selected_session_id: Option<String>,
    /// New session name
    new_session_name: String,
    /// Sessions directory
    sessions_dir: PathBuf,
}

impl SessionsPanel {
    pub fn new(_state: &AppState) -> Self {
        let mut panel = Self {
            saved_sessions: Vec::new(),
            selected_session_id: None,
            new_session_name: String::new(),
            sessions_dir: PathBuf::from("sessions"),
        };

        // Create sessions directory if it doesn't exist
        if !panel.sessions_dir.exists() {
            fs::create_dir_all(&panel.sessions_dir).ok();
        }

        // Load existing sessions
        panel.load_sessions();
        panel
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.label(egui::RichText::new("💾 Sessions").heading().strong());
        ui.add_space(8.0);

        egui::ScrollArea::vertical()
            .max_height(600.0)
            .show(ui, |ui| {
                // Save current session
                self.render_save_section(ui, state);

                ui.add_space(12.0);

                // Load session
                self.render_load_section(ui, state);

                ui.add_space(12.0);

                // Session list
                self.render_session_list(ui, state);

                ui.add_space(12.0);

                // Auto-save settings
                self.render_auto_save(ui, state);
            });
    }

    fn render_save_section(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Save Current Session").heading());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Session Name:");
                ui.text_edit_singleline(&mut self.new_session_name);
            });

            ui.add_space(4.0);

            if ui.button("💾 Save Session").clicked() {
                self.save_current_session(state);
            }
        });
    }

    fn render_load_section(&mut self, ui: &mut egui::Ui, _state: &AppState) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Load Session").heading());
            ui.add_space(4.0);

            if self.saved_sessions.is_empty() {
                ui.label(egui::RichText::new("No saved sessions").weak().italics());
            } else {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for session in &self.saved_sessions {
                            let is_selected = self.selected_session_id == Some(session.id.clone());
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
                                        ui.label("📄");
                                        ui.label(egui::RichText::new(&session.name).strong());

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Load").clicked() {
                                                self.selected_session_id = Some(session.id.clone());
                                            }
                                        });
                                    });

                                    ui.add_space(4.0);

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} messages • {} context items",
                                            session.conversation.len(),
                                            session.context.len()
                                        ))
                                        .small()
                                        .weak(),
                                    );

                                    ui.label(
                                        egui::RichText::new(
                                            session.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
                                        )
                                        .small()
                                        .weak(),
                                    );
                                });
                        }
                    });
            }
        });
    }

    fn render_session_list(&mut self, ui: &mut egui::Ui, _state: &AppState) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Session Management").heading());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("🔄 Refresh").clicked() {
                    self.load_sessions();
                }

                if let Some(session_id) = &self.selected_session_id {
                    if ui.button("🗑️ Delete").clicked() {
                        let id_to_delete = session_id.clone();
                        self.delete_session(&id_to_delete);
                        self.selected_session_id = None;
                    }
                }
            });
        });
    }

    fn render_auto_save(&mut self, ui: &mut egui::Ui, _state: &AppState) {
        ui.group(|ui| {
            ui.label(egui::RichText::new("Auto-Save").heading());
            ui.add_space(4.0);

            ui.label(egui::RichText::new("Sessions are automatically saved every 5 minutes").small().weak());
            ui.add_space(8.0);

            ui.label(egui::RichText::new("📍 Session Storage Location:").small());
            ui.label(
                egui::RichText::new(self.sessions_dir.to_string_lossy().as_ref())
                    .font(egui::FontId::monospace(10.0))
                    .weak(),
            );
        });
    }

    fn save_current_session(&mut self, state: &AppState) {
        if self.new_session_name.trim().is_empty() {
            return;
        }

        let session = SavedSession {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.new_session_name.clone(),
            timestamp: chrono::Utc::now(),
            conversation: state.get_conversation_history().clone(),
            context: {
                let ctx = state.get_context_manager();
                ctx.selected_documents.clone()
            },
            search_query: None, // TODO: Store current search
            current_role: {
                let role = state.get_current_role();
                role.name.original.clone()
            },
        };

        // Save to file
        let filename = format!("{}.json", session.id);
        let filepath = self.sessions_dir.join(&filename);

        if let Ok(json) = serde_json::to_string_pretty(&session) {
            fs::write(&filepath, json).ok();
        }

        // Add to list and refresh
        self.saved_sessions.push(session);
        self.new_session_name.clear();
    }

    fn load_sessions(&mut self) {
        self.saved_sessions.clear();

        if let Ok(entries) = fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(json) = fs::read_to_string(&path) {
                        if let Ok(session) = serde_json::from_str::<SavedSession>(&json) {
                            self.saved_sessions.push(session);
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        self.saved_sessions
            .sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    }

    fn delete_session(&mut self, session_id: &str) {
        let filename = format!("{}.json", session_id);
        let filepath = self.sessions_dir.join(&filename);

        fs::remove_file(filepath).ok();

        self.saved_sessions.retain(|s| s.id != session_id);
    }
}
