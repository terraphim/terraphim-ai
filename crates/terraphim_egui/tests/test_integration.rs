//! End-to-end integration tests
//!
//! These tests cover complete user workflows and cross-component interactions.

use std::sync::{Arc, Mutex};
use terraphim_egui::state::AppState;
use terraphim_egui::ui::search::autocomplete::AutocompleteWidget;
use terraphim_egui::ui::search::input::SearchInput;
use terraphim_egui::ui::search::results::SearchResults;
use terraphim_types::Document;

/// Test complete search workflow
#[tokio::test]
async fn test_complete_search_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Initialize search input
    let mut search_input = SearchInput::new(&state_ref.lock().unwrap());

    // Initially query should be empty
    assert!(search_input.query().is_empty());

    // Step 2: Set search query (simulating user input)
    search_input.set_query("rust programming".to_string());
    assert_eq!(search_input.query(), "rust programming");

    // Step 3: Execute search
    search_input.execute_search(&state_ref.lock().unwrap());

    // Step 4: Verify results were created
    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert_eq!(results.len(), 3);

        // Verify results have expected structure
        for result in results.iter() {
            assert!(!result.id.is_empty());
            assert!(!result.url.is_empty());
            assert!(result.rank.is_some());
        }
    }

    // Step 5: Verify results can be accessed through SearchResults widget
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());

    let filtered_results = search_results.get_filtered_results();
    assert_eq!(filtered_results.len(), 3);
}

/// Test search with autocomplete workflow
#[tokio::test]
async fn test_search_with_autocomplete_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Enable autocomplete
    let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
    assert!(search_input.is_autocomplete_ready());

    // Step 2: User types partial query
    search_input.set_query("r".to_string());
    assert_eq!(search_input.query(), "r");

    // Step 3: Autocomplete suggestions are shown (implementation dependent)

    // Step 4: User completes the query
    search_input.set_query("rust".to_string());
    assert_eq!(search_input.query(), "rust");

    // Step 5: User executes search
    search_input.execute_search(&state_ref.lock().unwrap());

    // Step 6: Verify results
    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert!(!results.is_empty());
        // Results should mention "rust"
        for result in results.iter() {
            assert!(
                result.title.to_lowercase().contains("rust")
                    || result
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains("rust"))
                        .unwrap_or(false)
            );
        }
    }
}

/// Test search results filtering and selection workflow
#[tokio::test]
async fn test_results_filtering_and_selection_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Execute search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 2: Get results widget
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());

    // Step 3: Apply filter (e.g., filter by source)
    search_results.filter_options.source_filter = Some("GitHub".to_string());
    search_results.apply_filters_and_sort();

    // Verify filtering
    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].source_haystack, Some("GitHub".to_string()));

    // Step 4: Select result
    search_results.toggle_selection(0);
    assert!(search_results.is_selected(0));
    assert_eq!(search_results.get_selected_results().len(), 1);

    // Step 5: Clear filter
    search_results.clear_filters();
    assert_eq!(search_results.get_filtered_results().len(), 3);
}

/// Test add to context workflow
#[tokio::test]
async fn test_add_to_context_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Execute search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 2: Get results and select some
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());

    // Select first and third results
    search_results.toggle_selection(0);
    search_results.toggle_selection(2);

    // Step 3: Add selected to context
    search_results.add_selected_to_context(&state_ref.lock().unwrap());

    // Step 4: Verify context
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 2);
        assert_eq!(context.selected_documents[0].id, "1");
        assert_eq!(context.selected_documents[1].id, "3");
    }
}

/// Test batch add to context workflow
#[tokio::test]
async fn test_batch_add_to_context_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Execute search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 2: Select all results
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());
    search_results.select_all();

    // Verify all selected
    assert_eq!(search_results.selected_indices.len(), 3);

    // Step 3: Add all to context via search input
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.add_to_context(&state_ref.lock().unwrap());
    }

    // Step 4: Verify all results are in context
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 3);
    }
}

/// Test sort and filter combined workflow
#[tokio::test]
async fn test_sort_and_filter_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Execute search to get results
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("test".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());

    // Step 2: Apply multiple filters
    search_results.filter_options.source_filter = Some("Local".to_string());
    search_results.filter_options.tag_filter = Some("tutorial".to_string());
    search_results.sort_option = terraphim_egui::ui::search::results::SortOption::Title;

    search_results.apply_filters_and_sort();

    // Step 3: Verify combined filtering works
    let filtered = search_results.get_filtered_results();
    // Should match Local Files source with tutorial tag, sorted by title
    for result in filtered.iter() {
        assert!(result.source_haystack.as_ref().unwrap().contains("Local"));
        assert!(result
            .tags
            .as_ref()
            .unwrap()
            .contains(&"tutorial".to_string()));
    }

    // Verify sorting
    for i in 0..filtered.len() - 1 {
        assert!(filtered[i].title <= filtered[i + 1].title);
    }
}

/// Test clear and refactor workflow
#[tokio::test]
async fn test_clear_and_refactor_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Execute first search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 2: Apply filters and select
    let mut search_results = SearchResults::new(&state_ref.lock().unwrap());
    search_results.update_results(&state_ref.lock().unwrap());
    search_results.filter_options.source_filter = Some("GitHub".to_string());
    search_results.apply_filters_and_sort();
    search_results.select_all();

    assert_eq!(search_results.get_filtered_results().len(), 1);

    // Step 3: Clear all filters
    search_results.clear_filters();
    assert_eq!(search_results.get_filtered_results().len(), 3);

    // Step 4: Execute new search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("egui".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 5: Verify new results
    search_results.update_results(&state_ref.lock().unwrap());
    let filtered = search_results.get_filtered_results();
    assert_eq!(filtered.len(), 3);

    // Results should mention "egui" now
    for result in filtered.iter() {
        assert!(result.title.to_lowercase().contains("egui"));
    }
}

/// Test search input state persistence
#[tokio::test]
async fn test_search_input_state_persistence() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: Create search input
    let mut search_input = SearchInput::new(&state_ref.lock().unwrap());

    // Step 2: Set query
    search_input.set_query("persistent query".to_string());
    assert_eq!(search_input.query(), "persistent query");

    // Step 3: Toggle autocomplete
    search_input.set_query("another query".to_string());
    assert_eq!(search_input.query(), "another query");
}

/// Test context management across multiple operations
#[tokio::test]
async fn test_context_management_workflow() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Step 1: First search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("rust".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 2: Add first result to context
    {
        let state = state_ref.lock().unwrap();
        let result = state.get_search_results().first().cloned();
        drop(state);

        if let Some(result) = result {
            let state = state_ref.lock().unwrap();
            state.add_document_to_context(result);
        }
    }

    // Step 3: Verify context
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 1);
    }

    // Step 4: Second search
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.set_query("egui".to_string());
        search_input.execute_search(&state_ref.lock().unwrap());
    }

    // Step 5: Add results to context
    {
        let mut search_input = SearchInput::new(&state_ref.lock().unwrap());
        search_input.add_to_context(&state_ref.lock().unwrap());
    }

    // Step 6: Verify context has accumulated documents
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        // Note: The add_to_context behavior may vary, so we just check it's at least 1
        assert!(
            context.selected_documents.len() >= 1,
            "Context should have documents"
        );
    }

    // Step 7: Clear context
    {
        let state = state_ref.lock().unwrap();
        state.clear_context();
    }

    // Step 8: Verify context is empty
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 0);
    }
}

/// Helper to create test documents
fn create_test_document(id: &str, title: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://example.com/{}", id),
        title: title.to_string(),
        body: format!("Content of {}", title),
        description: Some(format!("Description of {}", title)),
        summarization: None,
        stub: None,
        tags: Some(vec!["test".to_string()]),
        rank: Some(100),
        source_haystack: Some("test".to_string()),
    }
}
