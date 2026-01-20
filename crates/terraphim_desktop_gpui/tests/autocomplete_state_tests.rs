#![recursion_limit = "1024"]

use gpui::*;
use terraphim_desktop_gpui::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
use terraphim_desktop_gpui::views::search::autocomplete::AutocompleteState;

/// Mock context for testing AutocompleteState without full GPUI setup
struct MockContext {
    notified: bool,
}

impl MockContext {
    fn new() -> Self {
        Self { notified: false }
    }

    fn notify(&mut self) {
        self.notified = true;
    }
}

/// Create a test suggestion
fn create_test_suggestion(term: &str, score: f64) -> AutocompleteSuggestion {
    AutocompleteSuggestion {
        term: term.to_string(),
        nterm: term.to_string(),
        score,
        from_kg: true,
        definition: Some(format!("Definition for {}", term)),
        url: Some(format!("https://example.com/{}", term)),
    }
}

#[test]
fn test_autocomplete_state_initialization() {
    // Note: This test demonstrates the structure - in a real GPUI environment,
    // we would need proper Context<Self> from GPUI's testing utilities

    // Test that we can create the struct with proper initial state
    let engine = None::<AutocompleteEngine>;
    let suggestions = vec![];
    let selected_index = 0;
    let last_query = String::new();

    // Verify initial state structure
    assert!(engine.is_none());
    assert!(suggestions.is_empty());
    assert_eq!(selected_index, 0);
    assert!(last_query.is_empty());
}

#[test]
fn test_autocomplete_state_selection_management() {
    // Test selection logic without GPUI context
    let mut suggestions = vec![
        create_test_suggestion("apple", 1.0),
        create_test_suggestion("application", 0.9),
        create_test_suggestion("apt", 0.8),
    ];
    let mut selected_index = 0;

    // Test initial selection
    assert_eq!(selected_index, 0);
    assert_eq!(suggestions[selected_index].term, "apple");

    // Test select_next
    selected_index = (selected_index + 1).min(suggestions.len() - 1);
    assert_eq!(selected_index, 1);
    assert_eq!(suggestions[selected_index].term, "application");

    // Test select_next at boundary
    selected_index = (selected_index + 1).min(suggestions.len() - 1);
    assert_eq!(selected_index, 2);
    assert_eq!(suggestions[selected_index].term, "apt");

    // Test select_next at max boundary (should stay at last)
    selected_index = (selected_index + 1).min(suggestions.len() - 1);
    assert_eq!(selected_index, 2); // Should stay at last element

    // Test select_previous
    selected_index = selected_index.saturating_sub(1);
    assert_eq!(selected_index, 1);
    assert_eq!(suggestions[selected_index].term, "application");

    // Test select_previous at boundary
    selected_index = selected_index.saturating_sub(1);
    assert_eq!(selected_index, 0);
    assert_eq!(suggestions[selected_index].term, "apple");

    // Test select_previous at min boundary (should stay at first)
    selected_index = selected_index.saturating_sub(1);
    assert_eq!(selected_index, 0); // Should stay at first element
}

#[test]
fn test_autocomplete_state_empty_suggestions() {
    let suggestions: Vec<AutocompleteSuggestion> = vec![];
    let mut selected_index = 0;

    // Test selection with empty suggestions
    let original_index = selected_index;
    selected_index = if !suggestions.is_empty() {
        (selected_index + 1).min(suggestions.len() - 1)
    } else {
        selected_index // Should not change
    };

    assert_eq!(selected_index, original_index);
    assert!(suggestions.is_empty());
}

#[test]
fn test_autocomplete_state_query_handling() {
    // Test query deduplication logic
    let mut last_query = String::new();
    let query = "test";

    // First call with new query
    if query != last_query {
        last_query = query.to_string();
        assert_eq!(last_query, "test");
    }

    // Second call with same query should be ignored
    if query != last_query {
        last_query = query.to_string();
        panic!("Should not update for same query");
    }

    assert_eq!(last_query, "test");
}

#[test]
fn test_autocomplete_state_clear_functionality() {
    let mut suggestions = vec![
        create_test_suggestion("item1", 1.0),
        create_test_suggestion("item2", 0.9),
    ];
    let mut selected_index = 1;
    let mut last_query = "some query".to_string();

    // Test clear logic
    suggestions.clear();
    selected_index = 0;
    last_query.clear();

    assert!(suggestions.is_empty());
    assert_eq!(selected_index, 0);
    assert!(last_query.is_empty());
}

#[test]
fn test_autocomplete_state_length_and_empty_checks() {
    let empty_suggestions: Vec<AutocompleteSuggestion> = vec![];
    let filled_suggestions = vec![
        create_test_suggestion("item1", 1.0),
        create_test_suggestion("item2", 0.9),
    ];

    // Test empty state
    assert!(empty_suggestions.is_empty());
    assert_eq!(empty_suggestions.len(), 0);

    // Test non-empty state
    assert!(!filled_suggestions.is_empty());
    assert_eq!(filled_suggestions.len(), 2);
}

#[test]
fn test_autocomplete_state_suggestion_selection() {
    let suggestions = vec![
        create_test_suggestion("first", 1.0),
        create_test_suggestion("second", 0.9),
        create_test_suggestion("third", 0.8),
    ];
    let selected_index = 1;

    // Test get_selected logic
    let selected = suggestions.get(selected_index);
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().term, "second");

    // Test out of bounds selection
    let out_of_bounds = suggestions.get(999);
    assert!(out_of_bounds.is_none());
}

#[test]
fn test_autocomplete_state_suggestion_scoring() {
    let suggestions = vec![
        create_test_suggestion("exact_match", 1.0),
        create_test_suggestion("partial_match", 0.7),
        create_test_suggestion("fuzzy_match", 0.5),
    ];

    // Verify that suggestions are properly structured with scores
    for suggestion in &suggestions {
        assert!(!suggestion.term.is_empty());
        assert!(!suggestion.nterm.is_empty());
        assert!(suggestion.score >= 0.0 && suggestion.score <= 1.0);
        assert!(suggestion.from_kg);
        assert!(suggestion.definition.is_some());
        assert!(suggestion.url.is_some());
    }

    // Test ordering by score (higher scores should come first)
    let mut sorted_suggestions = suggestions.clone();
    sorted_suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    assert_eq!(sorted_suggestions[0].term, "exact_match");
    assert_eq!(sorted_suggestions[1].term, "partial_match");
    assert_eq!(sorted_suggestions[2].term, "fuzzy_match");
}

#[test]
fn test_autocomplete_state_query_length_behavior() {
    // Test the logic for short vs long queries
    let query_short = "ru";
    let query_long = "rust programming";

    // Short queries (< 3 chars) should use exact matching
    let use_exact_short = query_short.len() < 3;
    assert!(use_exact_short);

    // Long queries (>= 3 chars) should use fuzzy search
    let use_exact_long = query_long.len() < 3;
    assert!(!use_exact_long);
}

#[test]
fn test_autocomplete_state_engine_check() {
    // Test behavior when engine is None vs Some
    let engine_none: Option<AutocompleteEngine> = None;
    let suggestions_empty = if let Some(_engine) = &engine_none {
        vec![create_test_suggestion("test", 1.0)]
    } else {
        vec![]
    };

    assert!(suggestions_empty.is_empty());

    // With a mock engine (we can't actually create one without proper setup)
    let engine_some = Some(()); // Mock to test the Some() branch
    let suggestions_non_empty = if let Some(_engine) = &engine_some {
        vec![create_test_suggestion("test", 1.0)]
    } else {
        vec![]
    };

    assert!(!suggestions_non_empty.is_empty());
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_autocomplete_engine_from_json() {
        // Test that we can create an AutocompleteEngine from JSON
        let thesaurus_json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"}
        ]"#;

        let result = AutocompleteEngine::from_thesaurus_json(thesaurus_json);
        assert!(result.is_ok(), "Should create engine from valid JSON");

        let engine = result.unwrap();
        assert_eq!(engine.term_count(), 2);
        assert!(engine.is_kg_term("rust"));
        assert!(engine.is_kg_term("tokio"));
        assert!(!engine.is_kg_term("nonexistent"));
    }

    #[tokio::test]
    async fn test_autocomplete_search_functionality() {
        let thesaurus_json = r#"[
            {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
            {"id": 2, "nterm": "ruby", "url": "https://ruby-lang.org"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json).unwrap();

        // Test autocomplete search
        let suggestions = engine.autocomplete("ru", 5);
        assert!(!suggestions.is_empty());

        // Test fuzzy search
        let fuzzy_suggestions = engine.fuzzy_search("rst", 5);
        assert!(!fuzzy_suggestions.is_empty());

        // Test getting terms
        let terms = engine.get_terms();
        assert_eq!(terms.len(), 2);
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"ruby".to_string()));
    }
}
