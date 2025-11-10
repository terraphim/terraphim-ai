//! Terraphim AI - Egui Desktop Application
//!
//! This crate provides a native desktop UI for Terraphim AI using egui.
//! It integrates with existing Terraphim crates for search, knowledge graphs,
//! LLM interaction, and role-based context management.

pub mod app;
pub mod state;
pub mod ui;

pub use app::EguiApp;
pub use state::AppState;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        // Test that AppState can be created
        let _state = AppState::new();
    }
}
