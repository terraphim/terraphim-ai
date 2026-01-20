/// Simple KG autocomplete test to validate basic functionality
use terraphim_desktop_gpui::AutocompleteEngine;

#[test]
fn test_basic_kg_autocomplete() {
    // Minimal test data
    let thesaurus_json = r#"[
        {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
        {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"}
    ]"#;

    let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create engine from thesaurus");

    // Basic autocomplete test
    let suggestions = engine.autocomplete("ru", 5);
    assert!(!suggestions.is_empty(), "Should have suggestions for 'ru'");
    assert!(
        suggestions.iter().any(|s| s.term == "rust"),
        "Should suggest 'rust'"
    );

    println!("✅ Basic KG Autocomplete test passed!");
}
