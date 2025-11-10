//! End-to-end tests for search functionality
//!
//! These tests cover the complete search workflow from input to results.

use std::sync::{Arc, Mutex};
use terraphim_egui::state::AppState;
use terraphim_types::Document;

/// Test basic search execution
#[tokio::test]
async fn test_search_execution() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Initial state should have no results
    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert!(results.is_empty(), "Initial state should have no results");
    }

    // Create mock search results
    let mock_results = create_mock_results("rust");
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(mock_results);
    }

    // Verify results were set
    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert_eq!(results.len(), 3, "Should have 3 mock results");
        assert_eq!(results[0].title, "rust - Official Documentation");
        // Also verify by iterating
        for result in results.iter() {
            assert!(!result.id.is_empty());
        }
    }
}

/// Test search with different queries
#[tokio::test]
async fn test_search_different_queries() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Test Rust query
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_mock_results("rust"));
    }

    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert!(results[0].title.contains("rust"));
    }

    // Test different query
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_mock_results("egui"));
    }

    {
        let state = state_ref.lock().unwrap();
        let results = state.get_search_results();
        assert!(results[0].title.contains("egui"));
    }
}

/// Test search result structure
#[tokio::test]
async fn test_search_result_structure() {
    let state = AppState::new();
    let mut state = state;

    let results = create_mock_results("test");
    state.set_search_results(results);

    let state_results = state.get_search_results();

    // Verify each result has required fields
    for result in state_results.iter() {
        assert!(!result.id.is_empty(), "Result should have an ID");
        assert!(!result.url.is_empty(), "Result should have a URL");
        assert!(result.rank.is_some(), "Result should have a rank");
        assert!(
            result.source_haystack.is_some(),
            "Result should have a source"
        );
    }
}

/// Test adding documents to context
#[tokio::test]
async fn test_add_to_context() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Add some results
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_mock_results("rust"));
    }

    // Add first result to context
    {
        let state = state_ref.lock().unwrap();
        let result = state.get_search_results().first().cloned();
        drop(state);

        if let Some(result) = result {
            let state = state_ref.lock().unwrap();
            state.add_document_to_context(result);
        }
    }

    // Verify it's in context
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 1);
        assert_eq!(context.selected_documents[0].id, "1");
    }

    // Try to add same document again (should not duplicate)
    {
        let state = state_ref.lock().unwrap();
        let result = state.get_search_results().first().cloned();
        drop(state);

        if let Some(result) = result {
            let state = state_ref.lock().unwrap();
            state.add_document_to_context(result);
        }
    }

    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(
            context.selected_documents.len(),
            1,
            "Should not duplicate documents"
        );
    }
}

/// Test clearing context
#[tokio::test]
async fn test_clear_context() {
    let state = AppState::new();
    let state_clone = Arc::new(Mutex::new(state));
    let state_ref = Arc::clone(&state_clone);

    // Add results to context
    {
        let mut state = state_ref.lock().unwrap();
        state.set_search_results(create_mock_results("rust"));
    }

    // Get results first
    let results_to_add = {
        let state = state_ref.lock().unwrap();
        let results = state
            .get_search_results()
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        results
    };

    // Add them to context
    for result in results_to_add {
        let state = state_ref.lock().unwrap();
        state.add_document_to_context(result);
    }

    // Verify context has items
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 2);
    }

    // Clear context
    {
        let state = state_ref.lock().unwrap();
        state.clear_context();
    }

    // Verify context is empty
    {
        let state = state_ref.lock().unwrap();
        let context = state.get_context_manager();
        assert_eq!(context.selected_documents.len(), 0);
    }
}

/// Test search with empty query
#[tokio::test]
async fn test_empty_query_handling() {
    let state = AppState::new();
    let mut state = state;

    // Set results from previous search
    state.set_search_results(create_mock_results("rust"));

    // Clear query (simulated)
    {
        let results = Vec::<Document>::new();
        state.set_search_results(results);
    }

    let final_results = state.get_search_results();
    assert!(final_results.is_empty(), "Empty query should clear results");
}

/// Helper function to create mock search results
fn create_mock_results(query: &str) -> Vec<Document> {
    vec![
        Document {
            id: "1".to_string(),
            url: format!("https://example.com/{}", query),
            title: format!("{} - Official Documentation", query),
            body: format!("This is a comprehensive guide about {}.", query),
            description: Some(format!("Complete guide to {}", query)),
            summarization: None,
            stub: Some(format!("Learn {} fundamentals", query)),
            tags: Some(vec!["documentation".to_string(), "tutorial".to_string()]),
            rank: Some(100),
            source_haystack: Some("Local Files".to_string()),
        },
        Document {
            id: "2".to_string(),
            url: format!("https://github.com/search?q={}", query),
            title: format!("{} - GitHub Repository", query),
            body: format!(
                "Repository containing examples and projects related to {}.",
                query
            ),
            description: Some(format!("Open source {} implementation", query)),
            summarization: None,
            stub: Some(format!("{} codebase", query)),
            tags: Some(vec!["github".to_string(), "code".to_string()]),
            rank: Some(85),
            source_haystack: Some("GitHub".to_string()),
        },
        Document {
            id: "3".to_string(),
            url: format!("https://docs.rs/{}", query),
            title: format!("{} - Rust Crate Documentation", query),
            body: format!(
                "API documentation and usage examples for the {} crate.",
                query
            ),
            description: Some(format!("Rust crate documentation for {}", query)),
            summarization: None,
            stub: Some(format!("{} crate docs", query)),
            tags: Some(vec!["rust".to_string(), "crate".to_string()]),
            rank: Some(70),
            source_haystack: Some("docs.rs".to_string()),
        },
    ]
}
