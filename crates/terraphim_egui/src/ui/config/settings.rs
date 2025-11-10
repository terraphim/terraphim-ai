//! Configuration panel
//!
//! This module provides the configuration interface for application settings,
//! role management, and LLM provider configuration.

use crate::state::AppState;
use eframe::egui;

pub struct ConfigPanel {
    // Configuration state
    llm_provider: String,
    llm_model: String,
    llm_base_url: String,
    show_autocomplete: bool,
    max_autocomplete_results: usize,
}

impl ConfigPanel {
    pub fn new(state: &AppState) -> Self {
        // Load current settings
        let ui_state = state.get_ui_state();
        let settings = &ui_state.settings;

        let config_panel = Self {
            llm_provider: settings.llm_provider.clone(),
            llm_model: settings.llm_model.clone(),
            llm_base_url: settings.llm_base_url.clone().unwrap_or_default(),
            show_autocomplete: settings.show_autocomplete,
            max_autocomplete_results: settings.max_autocomplete_results,
        };

        drop(ui_state);
        config_panel
    }

    pub fn render(&mut self, ui: &mut egui::Ui, state: &AppState) {
        ui.label(egui::RichText::new("⚙️ Configuration").heading().strong());
        ui.add_space(12.0);

        egui::ScrollArea::vertical()
            .max_height(600.0)
            .show(ui, |ui| {
                // LLM Provider Settings
                ui.group(|ui| {
                    ui.label(egui::RichText::new("🤖 LLM Provider").heading());
                    ui.add_space(8.0);

                    // Provider selection
                    ui.horizontal(|ui| {
                        ui.label("Provider:");
                        let providers = ["ollama", "openrouter"];
                        egui::ComboBox::from_id_source("llm_provider")
                            .selected_text(&self.llm_provider)
                            .show_ui(ui, |ui| {
                                for provider in &providers {
                                    ui.selectable_value(&mut self.llm_provider, provider.to_string(), provider.to_string());
                                }
                            });
                    });

                    ui.add_space(8.0);

                    // Model selection
                    ui.horizontal(|ui| {
                        ui.label("Model:");
                        ui.text_edit_singleline(&mut self.llm_model);
                    });

                    ui.add_space(8.0);

                    // Base URL
                    ui.horizontal(|ui| {
                        ui.label("Base URL:");
                        ui.text_edit_singleline(&mut self.llm_base_url);
                    });

                    ui.add_space(8.0);

                    // Connection test
                    if ui.button("🔌 Test Connection").clicked() {
                        // TODO: Implement connection test
                    }
                });

                ui.add_space(16.0);

                // Search Settings
                ui.group(|ui| {
                    ui.label(egui::RichText::new("🔍 Search Settings").heading());
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_autocomplete, "Show Autocomplete");
                        ui.label("(Enable real-time search suggestions)");
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Max Autocomplete Results:");
                        ui.add(egui::Slider::new(&mut self.max_autocomplete_results, 1..=20));
                    });
                });

                ui.add_space(16.0);

                // Theme Settings
                ui.group(|ui| {
                    ui.label(egui::RichText::new("🎨 Theme").heading());
                    ui.add_space(8.0);

                    let ui_state = state.get_ui_state();
                    let current_theme = ui_state.theme.clone();
                    drop(ui_state);

                    let themes = vec!["Light", "Dark", "Nord", "Custom"];

                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        // TODO: Implement theme switching properly
                        ui.label(format!("{:?}", current_theme));
                    });
                });

                ui.add_space(16.0);

                // Save/Cancel buttons
                ui.separator();
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Save Settings").clicked() {
                        self.save_settings(state);
                    }

                    if ui.button("↺ Reset to Defaults").clicked() {
                        self.reset_to_defaults();
                    }
                });
            });
    }

    fn save_settings(&self, state: &AppState) {
        // Update settings in state
        state.update_ui_state(|ui_state| {
            ui_state.settings.llm_provider = self.llm_provider.clone();
            ui_state.settings.llm_model = self.llm_model.clone();
            ui_state.settings.llm_base_url = Some(self.llm_base_url.clone());
            ui_state.settings.show_autocomplete = self.show_autocomplete;
            ui_state.settings.max_autocomplete_results = self.max_autocomplete_results;
        });
    }

    fn reset_to_defaults(&mut self) {
        self.llm_provider = "ollama".to_string();
        self.llm_model = "llama3.2:3b".to_string();
        self.llm_base_url = "http://127.0.0.1:11434".to_string();
        self.show_autocomplete = true;
        self.max_autocomplete_results = 5;
    }
}

// Helper to convert string to Theme enum
impl crate::state::Theme {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "Light" => Some(crate::state::Theme::Light),
            "Dark" => Some(crate::state::Theme::Dark),
            "Nord" => Some(crate::state::Theme::Nord),
            "Custom" => Some(crate::state::Theme::Custom),
            _ => None,
        }
    }
}
