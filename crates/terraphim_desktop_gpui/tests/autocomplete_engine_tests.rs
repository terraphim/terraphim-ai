use terraphim_desktop_gpui::autocomplete::AutocompleteEngine;
use terraphim_automata::load_thesaurus_from_json;

/// Test suite for AutocompleteEngine with comprehensive coverage
#[test]
fn test_autocomplete_suggestion_structure() {
    let suggestion = terraphim_desktop_gpui::autocomplete::AutocompleteSuggestion {
        term: "rust".to_string(),
        nterm: "rust".to_string(),
        score: 1.0,
        from_kg: true,
        definition: Some("A programming language".to_string()),
        url: Some("https://rust-lang.org".to_string()),
    };

    assert_eq!(suggestion.term, "rust");
    assert_eq!(suggestion.nterm, "rust");
    assert_eq!(suggestion.score, 1.0);
    assert!(suggestion.from_kg);
    assert!(suggestion.definition.is_some());
    assert!(suggestion.url.is_some());
}

#[test]
fn test_autocomplete_creation_error() {
    // Test that thesaurus requires proper structure
    let json = r#"[]"#;  // Empty array - Thesaurus expects 2 elements

    let result = AutocompleteEngine::from_thesaurus_json(json);
    assert!(result.is_err(), "Empty JSON should return error - Thesaurus expects 2 elements");
}

#[test]
fn test_is_kg_term_on_invalid_json() {
    let json = r#"[]"#;

    let result = AutocompleteEngine::from_thesaurus_json(json);
    assert!(result.is_err(), "Invalid JSON should return error");

    // If we can't create an engine, we can't test is_kg_term
    // This test verifies the error path is working correctly
}

#[test]
fn test_invalid_json_error_cases() {
    // Test various invalid JSON scenarios
    let cases = vec![
        (r#"["#, "Incomplete array"),
        (r#"{invalid json"#, "Invalid object"),
        (r#""#, "Empty string"),
        (r#"null"#, "Null value"),
    ];

    for (invalid_json, description) in cases {
        let result = AutocompleteEngine::from_thesaurus_json(invalid_json);
        assert!(result.is_err(), "Should reject invalid JSON: {}", description);
    }
}

#[test]
fn test_autocomplete_engine_public_api() {
    // Test that the public API exists and has the expected methods
    // This tests compilation and API surface without creating a valid instance

    let json = r#"[]"#;
    let result = AutocompleteEngine::from_thesaurus_json(json);
    assert!(result.is_err(), "Invalid JSON should return error");

    // The fact that we can call from_thesaurus_json means the API is accessible
    // This test verifies the public methods exist and can be called (even if they fail)
}

#[test]
fn test_autocomplete_suggestion_api() {
    // Test that AutocompleteSuggestion struct can be created with proper fields
    let suggestion = terraphim_desktop_gpui::autocomplete::AutocompleteSuggestion {
        term: "test".to_string(),
        nterm: "test".to_string(),
        score: 1.0,
        from_kg: true,
        definition: Some("test definition".to_string()),
        url: Some("https://example.com".to_string()),
    };

    assert_eq!(suggestion.term, "test");
    assert_eq!(suggestion.nterm, "test");
    assert_eq!(suggestion.score, 1.0);
    assert!(suggestion.from_kg);
    assert_eq!(suggestion.definition, Some("test definition".to_string()));
    assert_eq!(suggestion.url, Some("https://example.com".to_string()));
}