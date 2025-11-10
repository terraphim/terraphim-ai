//! Panel integration tests
//!
//! These tests verify basic panel initialization.

use std::sync::{Arc, Mutex};
use terraphim_egui::state::AppState;
use terraphim_egui::ui::search::input::SearchInput;
use terraphim_egui::ui::search::results::SearchResults;

/// Test search panel initialization
#[tokio::test]
async fn test_search_panel_initialization() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Create search input widget
    let mut search_input = SearchInput::new(&state_ref.lock().unwrap());

    // Set a query
    search_input.set_query("test query".to_string());

    // Execute search
    search_input.execute_search(&state_ref.lock().unwrap());
}

/// Test panel widget initialization
#[tokio::test]
async fn test_panel_widget_initialization() {
    let state = AppState::new();
    let state_arc = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_arc);

    // Create widgets to verify they initialize without panicking
    let _search_input = SearchInput::new(&state_ref.lock().unwrap());
    let _search_results = SearchResults::new(&state_ref.lock().unwrap());
}
