//! End-to-end tests for search results management
//!
//! These tests cover sorting, filtering, selection, and batch operations.

use std::sync::{Arc, Mutex};
use terraphim_egui::state::AppState;
use terraphim_egui::ui::search::results::{FilterOptions, SearchResults, SortOption};
use terraphim_types::Document;

/// Test sorting by relevance
#[tokio::test]
async fn test_sort_by_relevance() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Set mock results
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    // Create SearchResults widget
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Set sort to relevance
    search_results.sort_option = SortOption::Relevance;
    search_results.apply_filters_and_sort();

    // Verify results are sorted by rank (descending)
    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 3);
    assert_eq!(filtered[0].rank, Some(100));
    assert_eq!(filtered[1].rank, Some(85));
    assert_eq!(filtered[2].rank, Some(70));
}

/// Test sorting by title
#[tokio::test]
async fn test_sort_by_title() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.sort_option = SortOption::Title;
    search_results.apply_filters_and_sort();

    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 3);

    // Results should be alphabetically sorted by title
    // (Even though our test data isn't perfectly sorted, we can check the logic)
    for i in 0..filtered.len() - 1 {
        assert!(filtered[i].title <= filtered[i + 1].title);
    }
}

/// Test filtering by source
#[tokio::test]
async fn test_filter_by_source() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Filter by "GitHub" source
    search_results.filter_options.source_filter = Some("GitHub".to_string());
    search_results.apply_filters_and_sort();

    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].source_haystack, Some("GitHub".to_string()));
}

/// Test filtering by tag
#[tokio::test]
async fn test_filter_by_tag() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Filter by "documentation" tag
    search_results.filter_options.tag_filter = Some("documentation".to_string());
    search_results.apply_filters_and_sort();

    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0]
        .tags
        .as_ref()
        .unwrap()
        .contains(&"documentation".to_string()));
}

/// Test filtering by minimum rank
#[tokio::test]
async fn test_filter_by_rank() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Filter for rank >= 90
    search_results.filter_options.min_rank = Some(90);
    search_results.apply_filters_and_sort();

    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].rank, Some(100));
}

/// Test multiple filters combined
#[tokio::test]
async fn test_multiple_filters() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Apply multiple filters
    search_results.filter_options.source_filter = Some("Local".to_string());
    search_results.filter_options.min_rank = Some(80);
    search_results.apply_filters_and_sort();

    let filtered = search_results.get_filtered_results();
    // Should match "Local Files" source with rank >= 80
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0]
        .source_haystack
        .as_ref()
        .unwrap()
        .contains("Local"));
    assert!(filtered[0].rank.unwrap() >= 80);
}

/// Test clearing filters
#[tokio::test]
async fn test_clear_filters() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Apply filters
    search_results.filter_options.source_filter = Some("GitHub".to_string());
    search_results.filter_options.tag_filter = Some("code".to_string());
    search_results.sort_option = SortOption::Title;
    search_results.apply_filters_and_sort();

    // Verify filtering works
    assert_eq!(search_results.get_filtered_results().len(), 1);

    // Clear filters
    search_results.clear_filters();

    // Verify all results are back
    assert_eq!(search_results.get_filtered_results().len(), 3);
    assert_eq!(search_results.sort_option, SortOption::Relevance);
}

/// Test selection management
#[tokio::test]
async fn test_selection_management() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Initially no selection
    assert_eq!(search_results.selected_indices.len(), 0);

    // Select all
    search_results.select_all();
    assert_eq!(search_results.selected_indices.len(), 3);

    // Check is_selected
    assert!(search_results.is_selected(0));
    assert!(search_results.is_selected(1));
    assert!(search_results.is_selected(2));

    // Clear selection
    search_results.clear_selection();
    assert_eq!(search_results.selected_indices.len(), 0);
    assert!(!search_results.is_selected(0));
}

/// Test toggling individual selections
#[tokio::test]
async fn test_toggle_selection() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Toggle select index 0
    search_results.toggle_selection(0);
    assert_eq!(search_results.selected_indices.len(), 1);
    assert!(search_results.is_selected(0));

    // Toggle again to deselect
    search_results.toggle_selection(0);
    assert_eq!(search_results.selected_indices.len(), 0);
    assert!(!search_results.is_selected(0));

    // Select multiple
    search_results.toggle_selection(0);
    search_results.toggle_selection(2);
    assert_eq!(search_results.selected_indices.len(), 2);
    assert!(search_results.is_selected(0));
    assert!(search_results.is_selected(2));
    assert!(!search_results.is_selected(1));
}

/// Test adding selected to context
#[tokio::test]
async fn test_add_selected_to_context() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let state_for_results = state_ref.clone();
    let mut search_results = SearchResults::new(&state_for_results.lock().unwrap());

    // Select some results
    search_results.toggle_selection(0);
    search_results.toggle_selection(2);

    // Add selected to context
    search_results.add_selected_to_context(&state_ref.lock().unwrap());

    // Verify context has the selected items
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 2);
        assert_eq!(context.selected_documents[0].id, "1");
        assert_eq!(context.selected_documents[1].id, "3");
    }
}

/// Test get selected results
#[tokio::test]
async fn test_get_selected_results() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_test_documents());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());

    // Select first and third results
    search_results.toggle_selection(0);
    search_results.toggle_selection(2);

    let selected = search_results.get_selected_results();
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].id, "1");
    assert_eq!(selected[1].id, "3");
}

/// Test empty results handling
#[tokio::test]
async fn test_empty_results() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Set empty results
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(Vec::new());
    }

    let search_results = SearchResults::new(&state_ref.lock().unwrap());

    assert_eq!(search_results.get_filtered_results().len(), 0);
    assert_eq!(search_results.get_selected_results().len(), 0);
}

/// Helper function to create test documents
fn create_test_documents() -> Vec<Document> {
    vec![
        Document {
            id: "1".to_string(),
            url: "https://example.com/rust".to_string(),
            title: "Rust Documentation".to_string(),
            body: "Complete Rust guide".to_string(),
            description: Some("Official Rust documentation".to_string()),
            summarization: None,
            stub: Some("Rust fundamentals".to_string()),
            tags: Some(vec!["documentation".to_string(), "tutorial".to_string()]),
            rank: Some(100),
            source_haystack: Some("Local Files".to_string()),
        },
        Document {
            id: "2".to_string(),
            url: "https://github.com/rust".to_string(),
            title: "Rust GitHub".to_string(),
            body: "Rust repository".to_string(),
            description: Some("Open source Rust".to_string()),
            summarization: None,
            stub: Some("Rust codebase".to_string()),
            tags: Some(vec!["github".to_string(), "code".to_string()]),
            rank: Some(85),
            source_haystack: Some("GitHub".to_string()),
        },
        Document {
            id: "3".to_string(),
            url: "https://docs.rs/rust".to_string(),
            title: "Rust Crate".to_string(),
            body: "Rust crate docs".to_string(),
            description: Some("Rust crate documentation".to_string()),
            summarization: None,
            stub: Some("Rust API".to_string()),
            tags: Some(vec!["rust".to_string(), "crate".to_string()]),
            rank: Some(70),
            source_haystack: Some("docs.rs".to_string()),
        },
    ]
}
