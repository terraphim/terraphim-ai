//! Tabbed panel system for the application
//!
//! This module implements the central docking panel system that manages
//! all major UI panels (Search, Chat, KnowledgeGraph, Context, etc.)

use eframe::egui;
use tracing::{info, warn};

use crate::state::AppState;
use crate::state::{ActiveTab, PanelVisibility};
use crate::ui::chat::ChatWidget;
use crate::ui::config::ConfigPanel;
use crate::ui::context::ContextPanel;
use crate::ui::kg::KnowledgeGraphViewer;
use crate::ui::search::{SearchInput, SearchResults};
use crate::ui::sessions::SessionsPanel;

/// Manages all application panels
pub struct Panels {
    /// Currently active tab
    active_tab: ActiveTab,

    /// Panel visibility states
    visibility: PanelVisibility,

    /// Search components
    search_input: SearchInput,
    search_results: SearchResults,

    /// Chat component
    chat_widget: ChatWidget,

    /// Context manager
    context_panel: ContextPanel,

    /// Knowledge graph viewer
    knowledge_graph: KnowledgeGraphViewer,

    /// Configuration panel
    config_panel: ConfigPanel,

    /// Sessions panel
    sessions_panel: SessionsPanel,

    /// Tab button states
    tab_states: TabStates,
}

/// Tab button states for tracking which tabs are active/hovered
#[derive(Debug, Default)]
struct TabStates {
    search_hovered: bool,
    chat_hovered: bool,
    context_hovered: bool,
    kg_hovered: bool,
    config_hovered: bool,
    sessions_hovered: bool,
}

impl Panels {
    /// Create a new Panels instance
    pub fn new(state: &AppState) -> Self {
        info!("Initializing UI panels");

        Self {
            active_tab: ActiveTab::Search,
            visibility: PanelVisibility::default(),
            search_input: SearchInput::new(state),
            search_results: SearchResults::new(state),
            chat_widget: ChatWidget::new(state),
            context_panel: ContextPanel::new(state),
            knowledge_graph: KnowledgeGraphViewer::new(state),
            config_panel: ConfigPanel::new(state),
            sessions_panel: SessionsPanel::new(state),
            tab_states: TabStates::default(),
        }
    }

    /// Handle keyboard shortcuts for panel navigation
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) && ctx.input(|i| i.modifiers.ctrl) {
            // Ctrl+Tab: Next tab
            self.next_tab();
        } else if ctx.input(|i| i.key_pressed(egui::Key::Tab))
            && ctx.input(|i| i.modifiers.shift)
            && ctx.input(|i| i.modifiers.ctrl)
        {
            // Ctrl+Shift+Tab: Previous tab
            self.previous_tab();
        }
        // F1-F6: Direct tab access
        else if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.active_tab = ActiveTab::Search;
        } else if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            self.active_tab = ActiveTab::Chat;
        } else if ctx.input(|i| i.key_pressed(egui::Key::F3)) {
            self.active_tab = ActiveTab::KnowledgeGraph;
        } else if ctx.input(|i| i.key_pressed(egui::Key::F4)) {
            self.active_tab = ActiveTab::Context;
        } else if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.active_tab = ActiveTab::Configuration;
        } else if ctx.input(|i| i.key_pressed(egui::Key::F6)) {
            self.active_tab = ActiveTab::Sessions;
        }
    }

    /// Switch to next tab
    fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Search => ActiveTab::Chat,
            ActiveTab::Chat => ActiveTab::KnowledgeGraph,
            ActiveTab::KnowledgeGraph => ActiveTab::Context,
            ActiveTab::Context => ActiveTab::Configuration,
            ActiveTab::Configuration => ActiveTab::Sessions,
            ActiveTab::Sessions => ActiveTab::Search,
        };
    }

    /// Switch to previous tab
    fn previous_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Search => ActiveTab::Sessions,
            ActiveTab::Chat => ActiveTab::Search,
            ActiveTab::KnowledgeGraph => ActiveTab::Chat,
            ActiveTab::Context => ActiveTab::KnowledgeGraph,
            ActiveTab::Configuration => ActiveTab::Context,
            ActiveTab::Sessions => ActiveTab::Configuration,
        };
    }

    /// Render all panels
    pub fn render(&mut self, ctx: &egui::Context, state: &AppState) {
        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Render tab bar
        self.render_tab_bar(ctx);

        // Render active panel content
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            ActiveTab::Search => {
                if self.visibility.search_panel {
                    self.render_search_panel(ui, state);
                }
            }
            ActiveTab::Chat => {
                if self.visibility.chat_panel {
                    self.render_chat_panel(ui, state);
                }
            }
            ActiveTab::KnowledgeGraph => {
                if self.visibility.knowledge_graph_panel {
                    self.render_kg_panel(ui, state);
                }
            }
            ActiveTab::Context => {
                if self.visibility.context_panel {
                    self.render_context_panel(ui, state);
                }
            }
            ActiveTab::Configuration => {
                self.render_config_panel(ui, state);
            }
            ActiveTab::Sessions => {
                if self.visibility.sessions_panel {
                    self.render_sessions_panel(ui, state);
                }
            }
        });
    }

    /// Render the tab bar
    fn render_tab_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("tab_bar")
            .min_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Render tab buttons
                    ui.add_space(8.0);

                    // Search tab
                    if ui
                        .selectable_label(matches!(self.active_tab, ActiveTab::Search), "🔍 Search")
                        .on_hover_text("Search articles and knowledge")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Search;
                    }

                    // Chat tab
                    if ui
                        .selectable_label(matches!(self.active_tab, ActiveTab::Chat), "💬 Chat")
                        .on_hover_text("Chat with AI assistant")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Chat;
                    }

                    // Knowledge Graph tab
                    if ui
                        .selectable_label(
                            matches!(self.active_tab, ActiveTab::KnowledgeGraph),
                            "🕸️ Knowledge Graph",
                        )
                        .on_hover_text("Explore knowledge connections")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::KnowledgeGraph;
                    }

                    // Context tab
                    if ui
                        .selectable_label(
                            matches!(self.active_tab, ActiveTab::Context),
                            "📋 Context",
                        )
                        .on_hover_text("Manage context for LLM")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Context;
                    }

                    ui.add_space(16.0);

                    // Configuration tab
                    if ui
                        .selectable_label(
                            matches!(self.active_tab, ActiveTab::Configuration),
                            "⚙️ Configure",
                        )
                        .on_hover_text("Application settings")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Configuration;
                    }

                    // Sessions tab
                    if ui
                        .selectable_label(
                            matches!(self.active_tab, ActiveTab::Sessions),
                            "📚 Sessions",
                        )
                        .on_hover_text("Conversation history")
                        .clicked()
                    {
                        self.active_tab = ActiveTab::Sessions;
                    }

                    // Spacer
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Egui Prototype").small().weak());
                    });
                });
            });
    }

    /// Render search panel
    fn render_search_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        egui::ScrollArea::vertical()
            .id_source("search_panel")
            .show(ui, |ui| {
                ui.add_space(8.0);

                // Search input
                self.search_input.render(ui, state);

                ui.add_space(16.0);

                // Update and render search results
                self.search_results.update_results(state);
                self.search_results.render(ui, state);
            });
    }

    /// Render chat panel
    fn render_chat_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        self.chat_widget.render(ui, state);
    }

    /// Render context panel
    fn render_context_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        self.context_panel.render(ui, state);
    }

    /// Render knowledge graph panel
    fn render_kg_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        self.knowledge_graph.render(ui, state);
    }

    /// Render configuration panel
    fn render_config_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        self.config_panel.render(ui, state);
    }

    /// Render sessions panel
    fn render_sessions_panel(&mut self, ui: &mut egui::Ui, state: &AppState) {
        self.sessions_panel.render(ui, state);
    }

    /// Get the currently active tab
    pub fn active_tab(&self) -> &ActiveTab {
        &self.active_tab
    }

    /// Set the active tab
    pub fn set_active_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
    }

    /// Toggle panel visibility
    pub fn toggle_panel_visibility(&mut self, panel: &str) {
        match panel {
            "search" => self.visibility.search_panel = !self.visibility.search_panel,
            "chat" => self.visibility.chat_panel = !self.visibility.chat_panel,
            "context" => self.visibility.context_panel = !self.visibility.context_panel,
            "kg" | "knowledge_graph" => {
                self.visibility.knowledge_graph_panel = !self.visibility.knowledge_graph_panel
            }
            "sessions" => self.visibility.sessions_panel = !self.visibility.sessions_panel,
            _ => warn!("Unknown panel: {}", panel),
        }
    }
}
