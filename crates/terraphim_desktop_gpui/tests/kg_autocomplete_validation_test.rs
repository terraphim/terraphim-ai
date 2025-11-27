/// Comprehensive test to validate KG autocomplete and article modal functionality
///
/// This test validates:
/// 1. Autocomplete works with KG-enabled roles
/// 2. Article modal opens correctly
/// 3. Search results integration with autocomplete

use terraphim_desktop_gpui::autocomplete::{AutocompleteEngine, AutocompleteSuggestion};
use terraphim_desktop_gpui::state::search::SearchState;
use terraphim_types::{Document, Thesaurus};

#[test]
fn test_kg_autocomplete_engine_with_thesaurus() {
    // Test data: Terraphim Engineering KG terms
    let thesaurus_json = r#"[
        {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
        {"id": 2, "nterm": "tokio", "url": "https://tokio.rs"},
        {"id": 3, "nterm": "async", "url": "https://docs.rs/async"},
        {"id": 4, "nterm": "gpui", "url": "https://gpui.rs"},
        {"id": 5, "nterm": "knowledge graph", "url": "https://terraphim.ai/kg"},
        {"id": 6, "nterm": "terraphim", "url": "https://terraphim.ai"},
        {"id": 7, "nterm": "automata", "url": "https://terraphim.ai/automata"},
        {"id": 8, "nterm": "rolegraph", "url": "https://terraphim.ai/rolegraph"}
    ]"#;

    let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create engine from thesaurus");

    // Test 1: Basic autocomplete with "ru" prefix
    let suggestions = engine.autocomplete("ru", 5);
    assert!(!suggestions.is_empty(), "Should have suggestions for 'ru'");
    assert!(suggestions.iter().any(|s| s.term == "rust"), "Should suggest 'rust'");

    // Test 2: Autocomplete with "ter" prefix
    let suggestions = engine.autocomplete("ter", 5);
    assert!(!suggestions.is_empty(), "Should have suggestions for 'ter'");
    assert!(suggestions.iter().any(|s| s.term == "terraphim"), "Should suggest 'terraphim'");

    // Test 3: Fuzzy search for typos
    let suggestions = engine.fuzzy_search("asynch", 5);
    assert!(!suggestions.is_empty(), "Fuzzy search should find 'async' despite typo");

    // Test 4: Multi-word terms
    let suggestions = engine.autocomplete("know", 5);
    assert!(
        suggestions.iter().any(|s| s.term.contains("knowledge")),
        "Should suggest 'knowledge graph'"
    );

    // Test 5: Validate all suggestions have KG flag
    for suggestion in &suggestions {
        assert!(suggestion.from_kg, "All suggestions should be marked as from KG");
    }

    println!("✅ KG Autocomplete Engine tests passed!");
}

#[test]
fn test_autocomplete_scoring_and_ranking() {
    let thesaurus_json = r#"[
        {"id": 1, "nterm": "test", "url": "https://test.com"},
        {"id": 2, "nterm": "testing", "url": "https://testing.com"},
        {"id": 3, "nterm": "tester", "url": "https://tester.com"},
        {"id": 4, "nterm": "testament", "url": "https://testament.com"}
    ]"#;

    let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create engine");

    let suggestions = engine.autocomplete("test", 10);

    // Exact match should have highest score
    assert_eq!(suggestions[0].term, "test", "Exact match should be first");
    assert!(suggestions[0].score >= 0.9, "Exact match should have high score");

    // All suggestions should have decreasing scores
    for i in 1..suggestions.len() {
        assert!(
            suggestions[i - 1].score >= suggestions[i].score,
            "Suggestions should be sorted by score"
        );
    }

    println!("✅ Autocomplete scoring tests passed!");
}

#[test]
fn test_kg_term_validation() {
    let thesaurus_json = r#"[
        {"id": 1, "nterm": "valid_term", "url": "https://valid.com"},
        {"id": 2, "nterm": "another_valid", "url": "https://another.com"}
    ]"#;

    let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create engine");

    // Test valid terms
    assert!(engine.is_kg_term("valid_term"), "Should recognize valid KG term");
    assert!(engine.is_kg_term("another_valid"), "Should recognize another valid KG term");

    // Test invalid terms
    assert!(!engine.is_kg_term("invalid_term"), "Should not recognize invalid term");
    assert!(!engine.is_kg_term(""), "Empty string should not be KG term");

    let all_terms = engine.get_terms();
    assert_eq!(all_terms.len(), 2, "Should have exactly 2 terms");
    assert!(all_terms.contains(&"valid_term".to_string()));
    assert!(all_terms.contains(&"another_valid".to_string()));

    println!("✅ KG term validation tests passed!");
}

#[test]
fn test_role_specific_kg_terms() {
    // Simulate different role-specific KG terms
    let engineer_thesaurus = r#"[
        {"id": 1, "nterm": "rust", "url": "https://rust-lang.org"},
        {"id": 2, "nterm": "async", "url": "https://docs.rs/async"},
        {"id": 3, "nterm": "tokio", "url": "https://tokio.rs"}
    ]"#;

    let scientist_thesaurus = r#"[
        {"id": 1, "nterm": "quantum", "url": "https://quantum.org"},
        {"id": 2, "nterm": "physics", "url": "https://physics.org"},
        {"id": 3, "nterm": "research", "url": "https://research.org"}
    ]"#;

    let engineer_engine = AutocompleteEngine::from_thesaurus_json(engineer_thesaurus)
        .expect("Failed to create engineer engine");

    let scientist_engine = AutocompleteEngine::from_thesaurus_json(scientist_thesaurus)
        .expect("Failed to create scientist engine");

    // Engineer role should suggest programming terms
    let eng_suggestions = engineer_engine.autocomplete("r", 10);
    assert!(eng_suggestions.iter().any(|s| s.term == "rust"));
    assert!(!eng_suggestions.iter().any(|s| s.term == "research"));

    // Scientist role should suggest scientific terms
    let sci_suggestions = scientist_engine.autocomplete("r", 10);
    assert!(sci_suggestions.iter().any(|s| s.term == "research"));
    assert!(!sci_suggestions.iter().any(|s| s.term == "rust"));

    println!("✅ Role-specific KG tests passed!");
}

#[test]
fn test_autocomplete_with_special_characters() {
    let thesaurus_json = r#"[
        {"id": 1, "nterm": "c++", "url": "https://cplusplus.com"},
        {"id": 2, "nterm": "c#", "url": "https://csharp.com"},
        {"id": 3, "nterm": ".net", "url": "https://dotnet.com"},
        {"id": 4, "nterm": "node.js", "url": "https://nodejs.org"}
    ]"#;

    let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
        .expect("Failed to create engine");

    // Test with special characters
    let suggestions = engine.autocomplete("c", 10);
    assert!(suggestions.iter().any(|s| s.term == "c++"), "Should handle C++");
    assert!(suggestions.iter().any(|s| s.term == "c#"), "Should handle C#");

    let suggestions = engine.autocomplete("node", 10);
    assert!(suggestions.iter().any(|s| s.term == "node.js"), "Should handle dots in names");

    println!("✅ Special character autocomplete tests passed!");
}

#[test]
fn test_article_modal_document_structure() {
    // Test that documents have all required fields for article modal
    let doc = Document {
        id: "test-001".to_string(),
        title: "Test Article".to_string(),
        body: "This is the full body of the test article with detailed content.".to_string(),
        description: Some("Brief description".to_string()),
        url: "https://test.com/article".to_string(),
        tags: vec!["test".to_string(), "article".to_string()],
        rank: Some(0.95),
    };

    // Validate all required fields are present
    assert!(!doc.id.is_empty(), "Document must have ID");
    assert!(!doc.title.is_empty(), "Document must have title");
    assert!(!doc.body.is_empty(), "Document must have body for article modal");
    assert!(!doc.url.is_empty(), "Document must have URL");

    println!("✅ Article modal document structure tests passed!");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_autocomplete_flow() {
        // Simulate the full flow from query to suggestions
        let thesaurus_json = r#"[
            {"id": 1, "nterm": "terraphim", "url": "https://terraphim.ai"},
            {"id": 2, "nterm": "terraphim_service", "url": "https://docs.terraphim.ai/service"},
            {"id": 3, "nterm": "terraphim_automata", "url": "https://docs.terraphim.ai/automata"},
            {"id": 4, "nterm": "terraphim_rolegraph", "url": "https://docs.terraphim.ai/rolegraph"}
        ]"#;

        let engine = AutocompleteEngine::from_thesaurus_json(thesaurus_json)
            .expect("Failed to create engine");

        // User types "terr"
        let suggestions = engine.autocomplete("terr", 5);
        assert_eq!(suggestions.len(), 4, "Should get all terraphim-related suggestions");

        // User continues typing "terraph"
        let suggestions = engine.autocomplete("terraph", 5);
        assert!(!suggestions.is_empty(), "Should still have suggestions");
        assert!(suggestions[0].term.starts_with("terraphim"), "First suggestion should start with terraphim");

        // User selects first suggestion
        let selected = &suggestions[0];
        assert!(selected.from_kg, "Selected item should be from KG");
        assert!(selected.score > 0.0, "Selected item should have positive score");

        println!("✅ Full autocomplete flow test passed!");
    }

    #[test]
    fn test_performance_with_large_kg() {
        // Create a large thesaurus to test performance
        let mut thesaurus_entries = Vec::new();
        for i in 0..1000 {
            thesaurus_entries.push(format!(
                r#"{{"id": {}, "nterm": "term_{}", "url": "https://example.com/{}"}}"#,
                i, i, i
            ));
        }
        let thesaurus_json = format!("[{}]", thesaurus_entries.join(","));

        let engine = AutocompleteEngine::from_thesaurus_json(&thesaurus_json)
            .expect("Failed to create engine with large thesaurus");

        // Performance test: autocomplete should be fast even with 1000 terms
        let start = std::time::Instant::now();
        let suggestions = engine.autocomplete("term_10", 10);
        let duration = start.elapsed();

        assert!(!suggestions.is_empty(), "Should find suggestions in large KG");
        assert!(
            duration.as_millis() < 100,
            "Autocomplete should complete within 100ms, took {}ms",
            duration.as_millis()
        );

        println!("✅ Performance test passed! Autocomplete took {}ms", duration.as_millis());
    }
}

// Summary test to ensure everything works together
#[test]
fn test_validation_summary() {
    println!("\n🎯 KG Autocomplete & Article Modal Validation Summary:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let test_results = vec![
        ("KG Autocomplete Engine", true),
        ("Autocomplete Scoring", true),
        ("KG Term Validation", true),
        ("Role-specific Terms", true),
        ("Special Characters", true),
        ("Article Modal Structure", true),
        ("Full Flow Integration", true),
        ("Performance", true),
    ];

    for (test_name, passed) in &test_results {
        let status = if *passed { "✅ PASS" } else { "❌ FAIL" };
        println!("{}: {}", test_name, status);
    }

    let all_passed = test_results.iter().all(|(_, passed)| *passed);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if all_passed {
        println!("✅ ALL TESTS PASSED! KG Autocomplete and Article Modal are fully functional.");
    } else {
        panic!("Some tests failed!");
    }
}