//! Main application structure for Terraphim AI Egui
//!
//! This module defines the EguiApp struct which implements eframe::App
//! and handles the main application lifecycle and rendering.

use eframe::egui;
use tracing::info;

use crate::state::AppState;
use crate::ui::{Panels, ThemeManager};

/// Main application struct
pub struct EguiApp {
    /// Application state (shared across UI components)
    pub state: AppState,

    /// UI panel manager
    pub panels: Panels,

    /// Theme and styling manager
    pub theme_manager: ThemeManager,

    /// Time since application start
    pub start_time: std::time::Instant,

    /// Frame counter for FPS tracking
    pub frame_count: u64,
}

impl EguiApp {
    /// Create a new EguiApp instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing Terraphim AI Egui application");

        // Configure egui context
        Self::configure_egui(&cc.egui_ctx);

        // Initialize application state
        let state = AppState::new();

        // Initialize panels
        let panels = Panels::new(&state);

        // Initialize theme manager
        let theme_manager = ThemeManager::new(&state);

        let app = Self {
            state,
            panels,
            theme_manager,
            start_time: std::time::Instant::now(),
            frame_count: 0,
        };

        info!("Terraphim AI Egui application initialized successfully");
        app
    }

    /// Configure egui context with default settings
    fn configure_egui(ctx: &egui::Context) {
        // Set up dark mode by default
        ctx.set_visuals(egui::Visuals::dark());

        // Configure fonts
        let mut fonts = egui::FontDefinitions::default();
        // Use default system fonts for now
        // TODO: Add custom font configuration
        ctx.set_fonts(fonts);

        // Set default style
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        ctx.set_style(style);
    }

    /// Handle global keyboard shortcuts
    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        // Global shortcuts can be handled here
        // For example: Ctrl+Shift+T to toggle visibility
        // Note: Implementation will be added in Task 1.4
    }

    /// Update application state
    fn update(&mut self, ctx: &egui::Context) {
        // Handle global shortcuts
        self.handle_global_shortcuts(ctx);

        // Set repaint interval for smoother UI
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Update frame count
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// Render the main application UI
    fn render_ui(&mut self, ctx: &egui::Context) {
        // Apply theme
        self.theme_manager.apply(ctx);

        // Create the main application layout
        egui::TopBottomPanel::top("top_panel")
            .min_height(50.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Terraphim AI")
                            .heading()
                            .strong()
                            .color(self.theme_manager.accent_color()),
                    );
                    ui.separator();
                    self.render_header(ui);
                });
            });

        // Render central panels
        self.panels.render(ctx, &self.state);

        // Render status bar
        self.render_status_bar(ctx);
    }

    /// Render header section
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Current Role Display
            let role = self.state.get_current_role();
            ui.label(egui::RichText::new("👤 Role:").small().weak());
            ui.label(
                egui::RichText::new(role.name.as_str())
                    .strong()
                    .color(self.theme_manager.accent_color()),
            );
            drop(role);

            ui.add_space(16.0);

            // Search Statistics
            let search_count = self.state.get_search_results().len();
            ui.label(egui::RichText::new("🔍").small());
            ui.label(egui::RichText::new(format!("Searches: {}", search_count)).small());

            ui.add_space(8.0);

            // Context Statistics
            let context = self.state.get_context_manager();
            let context_count = context.selected_documents.len();
            let concept_count = context.selected_concepts.len();
            let total_context = context_count + concept_count;
            drop(context);

            ui.label(egui::RichText::new("📋").small());
            ui.label(egui::RichText::new(format!("Context: {}", total_context)).small());

            ui.add_space(16.0);

            // Quick Action Buttons
            ui.add_space(8.0);
            if ui
                .button("🗑️ Clear Context")
                .on_hover_text("Clear all context items")
                .clicked()
            {
                self.state.clear_context();
            }

            if ui
                .button("💬 Clear Chat")
                .on_hover_text("Clear conversation history")
                .clicked()
            {
                self.state.clear_conversation();
            }

            // Right side - Theme toggle and shortcuts
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                // Theme toggle button
                let ui_state = self.state.get_ui_state();
                let current_theme = ui_state.theme.clone();
                drop(ui_state);

                if ui
                    .button("🎨 Theme")
                    .on_hover_text("Change theme (not implemented yet)")
                    .clicked()
                {
                    // TODO: Implement theme switching
                }

                // Keyboard shortcut hint
                ui.label(
                    egui::RichText::new("⌨️ Shortcuts: Ctrl+F (Search) | Ctrl+Enter (Send)")
                        .small()
                        .weak(),
                );
            });
        });
    }

    /// Render status bar
    fn render_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(32.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Application status with role
                    let role_name = self.state.get_current_role().name.to_string();
                    let app_status = "●";
                    ui.label(
                        egui::RichText::new(app_status)
                            .color(egui::Color32::from_rgb(45, 150, 90)) // Green
                            .small(),
                    );
                    ui.label(format!("Ready | {}", role_name));

                    ui.separator();

                    // LLM provider
                    let ui_state = self.state.get_ui_state();
                    let llm_provider = &ui_state.settings.llm_provider;
                    let llm_model = &ui_state.settings.llm_model;
                    ui.label(
                        egui::RichText::new(format!("🤖 {}:{}", llm_provider, llm_model)).small(),
                    );
                    drop(ui_state);

                    ui.separator();

                    // Search results count
                    let search_count = self.state.get_search_results().len();
                    ui.label(format!("🔍 Results: {}", search_count));

                    ui.separator();

                    // Context items count
                    let context_count = self.state.get_context_manager().selected_documents.len();
                    ui.label(format!("📋 Context: {}", context_count));

                    ui.separator();

                    // Connection status
                    // TODO: Implement actual connection monitoring
                    ui.label(egui::RichText::new("🟢 Connected").small().weak());

                    ui.separator();

                    // Frame rate display
                    let elapsed = self.start_time.elapsed().as_secs_f64();
                    if elapsed > 0.0 {
                        let fps = self.frame_count as f64 / elapsed;
                        ui.label(
                            egui::RichText::new(format!("FPS: {:.1}", fps))
                                .small()
                                .weak(),
                        );
                    }

                    // Right side - version and theme
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Show current theme
                        let ui_state = self.state.get_ui_state();
                        let theme = &ui_state.theme;
                        let theme_icon = match theme {
                            crate::state::Theme::Light => "☀",
                            crate::state::Theme::Dark => "🌙",
                            crate::state::Theme::Nord => "❄",
                            crate::state::Theme::Custom => "🎨",
                        };
                        drop(ui_state);

                        ui.label(
                            egui::RichText::new(format!("{} v0.1.0", theme_icon))
                                .small()
                                .weak(),
                        );
                    });
                });
            });
    }
}

impl eframe::App for EguiApp {
    /// Called each time the UI needs repainting
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update application state
        self.update(ctx);

        // Render UI
        self.render_ui(ctx);
    }

    /// Called when the application is being terminated
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("Terraphim AI Egui application shutting down");
    }

    /// Save application state
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // TODO: Implement persistence
        // Save user preferences, panel states, etc.
        info!("Saving application state");
    }
}
