//! UI module for Terraphim AI Egui application
//!
//! This module contains all UI-related code including panels, widgets,
//! and component rendering.

pub mod panels;
pub mod theme;

pub use panels::Panels;
pub use theme::ThemeManager;

pub mod search {
    pub mod autocomplete;
    pub mod input;
    pub mod results;

    pub use autocomplete::AutocompleteWidget;
    pub use input::SearchInput;
    pub use results::SearchResults;
}

pub mod chat {
    pub mod history;
    pub mod widget;

    pub use history::ChatHistory;
    pub use widget::ChatWidget;
}

pub mod context {
    pub mod manager;
    pub mod panel;

    pub use manager::ContextPanel;
    pub use panel::ContextManagerWidget;
}

pub mod kg {
    pub mod painter;
    pub mod viewer;

    pub use painter::KnowledgeGraphPainter;
    pub use viewer::KnowledgeGraphViewer;
}

pub mod config {
    pub mod role_selector;
    pub mod settings;

    pub use role_selector::RoleSelector;
    pub use settings::ConfigPanel;
}

pub mod sessions {
    pub mod history;
    pub mod panel;

    pub use history::SessionHistory;
    pub use panel::SessionsPanel;
}
