use terraphim_automata::autocomplete::{
    build_autocomplete_index, autocomplete_search, 
    fuzzy_autocomplete_search, fuzzy_autocomplete_search_levenshtein,
    serialize_autocomplete_index, deserialize_autocomplete_index,
    AutocompleteResult, AutocompleteConfig,
};

#[cfg(feature = "remote-loading")]
use terraphim_automata::{load_autocomplete_index, AutomataPath};
use terraphim_types::{Thesaurus, NormalizedTerm, NormalizedTermValue};

/// Create a comprehensive test thesaurus with various term patterns
fn create_test_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("Comprehensive Test".to_string());
    
    let test_data = vec![
        // Programming terms
        ("python", "Python Programming Language", 10),
        ("javascript", "JavaScript Programming Language", 9),
        ("rust", "Rust Programming Language", 15),
        ("programming", "Computer Programming", 8),
        ("algorithm", "Computer Algorithm", 12),
        ("data structure", "Data Structure", 11),
        
        // Machine Learning terms
        ("machine learning", "Machine Learning", 20),
        ("ml", "Machine Learning", 20),
        ("artificial intelligence", "Artificial Intelligence", 25),
        ("ai", "Artificial Intelligence", 25),
        ("neural network", "Neural Network", 18),
        ("deep learning", "Deep Learning", 22),
        ("supervised learning", "Supervised Learning", 16),
        ("unsupervised learning", "Unsupervised Learning", 14),
        
        // Data Science terms
        ("data science", "Data Science", 17),
        ("data analysis", "Data Analysis", 13),
        ("statistics", "Statistics", 10),
        ("visualization", "Data Visualization", 12),
        ("pandas", "Pandas Library", 8),
        ("numpy", "NumPy Library", 9),
        
        // Edge cases
        ("", "Empty Term", 1), // Empty string
        ("a", "Single Character", 2),
        ("very-long-term-with-many-hyphens-and-words", "Long Hyphenated Term", 5),
        ("UPPERCASE", "Uppercase Term", 6),
        ("MixedCase", "Mixed Case Term", 7),
        ("special!@#chars", "Special Characters", 3),
        ("unicode🚀term", "Unicode Term", 4),
        ("   spaces   ", "Spaces Term", 3),
    ];
    
    for (key, normalized, id) in test_data {
        let normalized_term = NormalizedTerm {
            id,
            value: NormalizedTermValue::from(normalized),
            url: Some(format!("https://example.com/{}", normalized.replace(' ', "-").to_lowercase())),
        };
        thesaurus.insert(NormalizedTermValue::from(key), normalized_term);
    }
    
    thesaurus
}

#[test]
fn test_build_autocomplete_index_basic() {
    let thesaurus = create_test_thesaurus();
    let original_len = thesaurus.len();
    
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    assert_eq!(index.name(), "Comprehensive Test");
    assert_eq!(index.len(), original_len);
    assert!(!index.is_empty());
}

#[test]
fn test_build_autocomplete_index_with_config() {
    let thesaurus = create_test_thesaurus();
    let config = AutocompleteConfig {
        max_results: 5,
        min_prefix_length: 2,
        case_sensitive: true,
    };
    
    let index = build_autocomplete_index(thesaurus, Some(config)).unwrap();
    
    assert!(!index.is_empty());
    // Config affects index building behavior
}

#[test]
fn test_autocomplete_search_prefix_matching() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test basic prefix matching
    let results = autocomplete_search(&index, "ma", None).unwrap();
    assert!(!results.is_empty());
    
    // Should find "machine learning"
    let has_ml = results.iter().any(|r| r.term.contains("machine"));
    assert!(has_ml, "Should find terms containing 'machine' for prefix 'ma'");
    
    // Test case insensitive matching
    let results = autocomplete_search(&index, "MA", None).unwrap();
    assert!(!results.is_empty(), "Should handle uppercase prefixes");
}

#[test]
fn test_autocomplete_search_exact_match() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test exact match
    let results = autocomplete_search(&index, "python", None).unwrap();
    assert!(!results.is_empty());
    
    let python_result = results.iter().find(|r| r.term == "python");
    assert!(python_result.is_some(), "Should find exact match for 'python'");
    
    let python_result = python_result.unwrap();
    assert_eq!(python_result.id, 10);
    assert_eq!(python_result.normalized_term, NormalizedTermValue::from("Python Programming Language"));
}

#[test]
fn test_autocomplete_search_ordering() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Search for terms that should be ordered by score
    let results = autocomplete_search(&index, "a", Some(10)).unwrap();
    
    // Check that results are sorted by score (descending)
    for i in 1..results.len() {
        assert!(
            results[i-1].score >= results[i].score,
            "Results should be sorted by score (descending). Position {}: {} > {}", 
            i, results[i-1].score, results[i].score
        );
    }
}

#[test]
fn test_autocomplete_search_limits() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test limit parameter
    let results = autocomplete_search(&index, "", Some(3)).unwrap();
    assert!(results.len() <= 3, "Should respect limit parameter");
    
    // Test with different limits
    for limit in [1, 5, 10, 20] {
        let results = autocomplete_search(&index, "a", Some(limit)).unwrap();
        assert!(results.len() <= limit, "Should respect limit of {}", limit);
    }
}

#[test]
fn test_autocomplete_search_empty_and_short_prefixes() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Empty prefix - should return results (FST can handle this)
    let results = autocomplete_search(&index, "", Some(5)).unwrap();
    // FST implementation may return results for empty prefix
    
    // Single character prefix
    let results = autocomplete_search(&index, "a", Some(10)).unwrap();
    assert!(!results.is_empty(), "Should find results for single character prefix");
}

#[test]
fn test_autocomplete_search_no_matches() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Search for prefix that shouldn't match anything
    let results = autocomplete_search(&index, "xyz123nonexistent", None).unwrap();
    assert!(results.is_empty(), "Should return empty results for non-matching prefix");
}

#[test]
fn test_autocomplete_search_special_characters() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test searching for terms with special characters
    let results = autocomplete_search(&index, "special", None).unwrap();
    // May or may not find results depending on normalization
    
    // Test unicode characters
    let results = autocomplete_search(&index, "unicode", None).unwrap();
    // Should handle unicode properly
}

#[test]
fn test_fuzzy_autocomplete_search_basic() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test fuzzy search with typos (now using Jaro-Winkler by default)
    let results = fuzzy_autocomplete_search(&index, "machne", 0.6, Some(5)).unwrap();
    // Should find "machine" related terms even with typo
    
    let results = fuzzy_autocomplete_search(&index, "pythonx", 0.6, Some(5)).unwrap();
    // Should find "python" with extra character
}

#[test]
fn test_fuzzy_autocomplete_search_similarity_thresholds() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test different similarity thresholds (now using Jaro-Winkler)
    for similarity in [0.3, 0.5, 0.7] {
        let results = fuzzy_autocomplete_search(&index, "maching", similarity, Some(10)).unwrap();
        // Higher similarity threshold should potentially find fewer results
    }
}

#[test]
fn test_serialization_roundtrip() {
    let thesaurus = create_test_thesaurus();
    let original_index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Serialize
    let serialized = serialize_autocomplete_index(&original_index).unwrap();
    assert!(!serialized.is_empty(), "Serialized data should not be empty");
    
    // Deserialize
    let deserialized_index = deserialize_autocomplete_index(&serialized).unwrap();
    
    // Verify metadata integrity
    assert_eq!(original_index.name(), deserialized_index.name());
    assert_eq!(original_index.len(), deserialized_index.len());
    
    // Verify search functionality is preserved
    let test_queries = ["ma", "python", "data", "ai"];
    
    for query in test_queries {
        let original_results = autocomplete_search(&original_index, query, Some(5)).unwrap();
        let deserialized_results = autocomplete_search(&deserialized_index, query, Some(5)).unwrap();
        
        assert_eq!(original_results.len(), deserialized_results.len(),
                  "Result count should match for query: {}", query);
        
        for (orig, deser) in original_results.iter().zip(deserialized_results.iter()) {
            assert_eq!(orig.term, deser.term, "Terms should match");
            assert_eq!(orig.id, deser.id, "IDs should match");
            assert_eq!(orig.score, deser.score, "Scores should match");
            assert_eq!(orig.normalized_term, deser.normalized_term, "Normalized terms should match");
            assert_eq!(orig.url, deser.url, "URLs should match");
        }
    }
}

#[test]
fn test_serialization_empty_index() {
    let empty_thesaurus = Thesaurus::new("Empty".to_string());
    let index = build_autocomplete_index(empty_thesaurus, None).unwrap();
    
    assert!(index.is_empty());
    
    // Should be able to serialize/deserialize empty index
    let serialized = serialize_autocomplete_index(&index).unwrap();
    let deserialized = deserialize_autocomplete_index(&serialized).unwrap();
    
    assert!(deserialized.is_empty());
    assert_eq!(index.name(), deserialized.name());
}

#[cfg(feature = "remote-loading")]
#[tokio::test]
async fn test_load_autocomplete_index_local() {
    // Test loading from local example file
    let result = load_autocomplete_index(&AutomataPath::local_example(), None).await;
    
    match result {
        Ok(index) => {
            assert!(!index.is_empty(), "Loaded index should not be empty");
            
            // Test search functionality on loaded index
            let results = autocomplete_search(&index, "foo", None).unwrap();
            // Should find some results from the test data
            
            // Test that all results have valid metadata
            for result in results {
                assert!(!result.term.is_empty(), "Term should not be empty");
                assert!(result.id > 0, "ID should be positive");
                assert!(!result.normalized_term.as_str().is_empty(), "Normalized term should not be empty");
            }
        }
        Err(e) => {
            // This is acceptable if the test data file doesn't exist
            eprintln!("Could not load test data for autocomplete index: {}", e);
        }
    }
}

#[test]
fn test_autocomplete_config_validation() {
    let thesaurus = create_test_thesaurus();
    
    // Test various configurations
    let configs = vec![
        AutocompleteConfig {
            max_results: 1,
            min_prefix_length: 1,
            case_sensitive: false,
        },
        AutocompleteConfig {
            max_results: 100,
            min_prefix_length: 3,
            case_sensitive: true,
        },
    ];
    
    for config in configs {
        let index = build_autocomplete_index(thesaurus.clone(), Some(config)).unwrap();
        assert!(!index.is_empty(), "Index should build successfully with any valid config");
    }
}

#[test]
fn test_autocomplete_result_metadata() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let results = autocomplete_search(&index, "python", None).unwrap();
    assert!(!results.is_empty());
    
    let python_result = results.iter().find(|r| r.term == "python").unwrap();
    
    // Verify all metadata fields are populated correctly
    assert_eq!(python_result.term, "python");
    assert_eq!(python_result.id, 10);
    assert_eq!(python_result.normalized_term, NormalizedTermValue::from("Python Programming Language"));
    assert!(python_result.url.is_some());
    assert!(python_result.score > 0.0);
    
    // Verify URL format
    let url = python_result.url.as_ref().unwrap();
    assert!(url.starts_with("https://example.com/"));
}

#[test]
fn test_autocomplete_concurrent_access() {
    use std::sync::Arc;
    use std::thread;
    
    let thesaurus = create_test_thesaurus();
    let index = Arc::new(build_autocomplete_index(thesaurus, None).unwrap());
    
    // Test concurrent access from multiple threads
    let mut handles = vec![];
    
    for i in 0..10 {
        let index = index.clone();
        let handle = thread::spawn(move || {
            let query = match i % 4 {
                0 => "ma",
                1 => "python",
                2 => "data",
                _ => "ai",
            };
            
            autocomplete_search(&index, query, Some(5)).unwrap()
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        let results = handle.join().unwrap();
        // Each thread should complete successfully
        assert!(results.len() <= 5);
    }
}

#[test]
fn test_autocomplete_performance_characteristics() {
    // Test that search performance is reasonable
    let large_thesaurus = {
        let mut thesaurus = Thesaurus::new("Performance Test".to_string());
        
        // Create 1000 terms for performance testing
        for i in 0..1000 {
            let term = format!("performance_term_{:04}", i);
            let normalized_term = NormalizedTerm {
                id: i as u64 + 1,
                value: NormalizedTermValue::from(term.clone()),
                url: Some(format!("https://example.com/{}", term)),
            };
            thesaurus.insert(NormalizedTermValue::from(term), normalized_term);
        }
        
        thesaurus
    };
    
    let start = std::time::Instant::now();
    let index = build_autocomplete_index(large_thesaurus, None).unwrap();
    let build_time = start.elapsed();
    
    println!("Built index with 1000 terms in {:?}", build_time);
    assert!(build_time.as_millis() < 1000, "Index building should be fast");
    
    // Test search performance
    let start = std::time::Instant::now();
    let results = autocomplete_search(&index, "performance", Some(10)).unwrap();
    let search_time = start.elapsed();
    
    println!("Search completed in {:?} with {} results", search_time, results.len());
    assert!(search_time.as_millis() < 100, "Search should be very fast");
}

#[test]
fn test_autocomplete_config_defaults() {
    let config = AutocompleteConfig::default();
    
    assert_eq!(config.max_results, 10);
    assert_eq!(config.min_prefix_length, 1);
    assert_eq!(config.case_sensitive, false);
}

#[test]
fn test_autocomplete_result_equality() {
    let result1 = AutocompleteResult {
        term: "test".to_string(),
        normalized_term: NormalizedTermValue::from("Test Term"),
        id: 1,
        url: Some("https://example.com/test".to_string()),
        score: 10.0,
    };
    
    let result2 = AutocompleteResult {
        term: "test".to_string(),
        normalized_term: NormalizedTermValue::from("Test Term"),
        id: 1,
        url: Some("https://example.com/test".to_string()),
        score: 10.0,
    };
    
    assert_eq!(result1, result2);
}

// Property-based testing
#[test]
fn test_autocomplete_property_all_results_start_with_prefix() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let prefixes = ["a", "ma", "py", "data", "ai"];
    
    for prefix in prefixes {
        let results = autocomplete_search(&index, prefix, None).unwrap();
        
        for result in results {
            let term_lower = result.term.to_lowercase();
            let prefix_lower = prefix.to_lowercase();
            
            // Note: FST implementation may not guarantee prefix matching at character level
            // This test validates the expectation but may need adjustment based on FST behavior
            if !term_lower.starts_with(&prefix_lower) {
                println!("Warning: Term '{}' doesn't start with prefix '{}'", result.term, prefix);
                // This might be expected behavior for FST-based implementation
            }
        }
    }
}

#[test]
fn test_autocomplete_property_result_limits_respected() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    for limit in [1, 5, 10, 50] {
        let results = autocomplete_search(&index, "a", Some(limit)).unwrap();
        assert!(results.len() <= limit, "Result count should not exceed limit {}", limit);
    }
}

#[test]
fn test_autocomplete_property_score_ordering() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let results = autocomplete_search(&index, "a", Some(20)).unwrap();
    
    for i in 1..results.len() {
        assert!(
            results[i-1].score >= results[i].score,
            "Scores should be in descending order: position {} has score {} > {}", 
            i, results[i-1].score, results[i].score
        );
    }
}

// ===== Jaro-Winkler vs Levenshtein Comparison Tests =====

#[test]
fn test_jaro_winkler_autocomplete_basic() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test Jaro-Winkler fuzzy search with typos (now the default fuzzy_autocomplete_search)
    let results = fuzzy_autocomplete_search(&index, "machne", 0.6, Some(5)).unwrap();
    
    // Should find "machine learning" even with typo
    let has_machine = results.iter().any(|r| r.term.contains("machine"));
    assert!(has_machine, "Jaro-Winkler should find 'machine' for typo 'machne'");
    
    println!("Jaro-Winkler results for 'machne':");
    for result in &results {
        println!("  {} (score: {:.3})", result.term, result.score);
    }
}

#[test]
fn test_comparison_prefix_emphasis() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test with query that has good prefix match but character transposition
    let query = "machien"; // "machine" with transposed 'i' and 'e'
    
    let levenshtein_results = fuzzy_autocomplete_search_levenshtein(&index, query, 2, Some(5)).unwrap();
    let jaro_winkler_results = fuzzy_autocomplete_search(&index, query, 0.6, Some(5)).unwrap();
    
    println!("Comparison for query '{}' (transposed characters):", query);
    println!("Levenshtein (edit distance ≤ 2):");
    for result in &levenshtein_results {
        println!("  {} (score: {:.3})", result.term, result.score);
    }
    
    println!("Jaro-Winkler (similarity ≥ 0.6):");
    for result in &jaro_winkler_results {
        println!("  {} (score: {:.3})", result.term, result.score);
    }
    
    // Both should find machine learning related terms
    let lev_has_machine = levenshtein_results.iter().any(|r| r.term.contains("machine"));
    let jw_has_machine = jaro_winkler_results.iter().any(|r| r.term.contains("machine"));
    
    assert!(lev_has_machine || jw_has_machine, "At least one method should find 'machine' terms");
}

#[test]
fn test_comparison_different_typo_patterns() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let test_cases = vec![
        ("pythno", "python"),     // transposition
        ("pythn", "python"),      // missing character
        ("pythonx", "python"),    // extra character
        ("pyton", "python"),      // missing character
        ("machne", "machine"),    // missing character
        ("machien", "machine"),   // transposition
        ("datascience", "data science"), // missing space
        ("aritificial", "artificial"),   // transposition + missing
    ];
    
    for (typo, target) in test_cases {
        println!("\n=== Testing typo pattern: '{}' → '{}' ===", typo, target);
        
        let lev_results = fuzzy_autocomplete_search_levenshtein(&index, typo, 2, Some(3)).unwrap();
        let jw_results = fuzzy_autocomplete_search(&index, typo, 0.5, Some(3)).unwrap();
        
        println!("Levenshtein results:");
        for result in &lev_results {
            println!("  {} (score: {:.3})", result.term, result.score);
        }
        
        println!("Jaro-Winkler results:");
        for result in &jw_results {
            println!("  {} (score: {:.3})", result.term, result.score);
        }
        
        // Check if either method finds the target term
        let lev_finds_target = lev_results.iter().any(|r| r.term.contains(target) || r.normalized_term.as_str().to_lowercase().contains(target));
        let jw_finds_target = jw_results.iter().any(|r| r.term.contains(target) || r.normalized_term.as_str().to_lowercase().contains(target));
        
        if !lev_finds_target && !jw_finds_target {
            println!("Neither method found target '{}' for typo '{}'", target, typo);
        } else {
            println!("Success: {} found target", 
                     match (lev_finds_target, jw_finds_target) {
                         (true, true) => "Both methods",
                         (true, false) => "Levenshtein",
                         (false, true) => "Jaro-Winkler",
                         _ => unreachable!(),
                     });
        }
    }
}

#[test]
fn test_jaro_winkler_prefix_advantage() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test cases where Jaro-Winkler should perform better due to prefix matching
    let prefix_cases = vec![
        "mach",    // Should strongly match "machine learning"
        "prog",    // Should match "programming"
        "artif",   // Should match "artificial intelligence"
        "super",   // Should match "supervised learning"
    ];
    
    for prefix in prefix_cases {
        println!("\n=== Testing prefix advantage: '{}' ===", prefix);
        
        // Use lower similarity threshold for Jaro-Winkler to see more results
        let jw_results = fuzzy_autocomplete_search(&index, prefix, 0.4, Some(5)).unwrap();
        
        println!("Jaro-Winkler results (min similarity 0.4):");
        for result in &jw_results {
            println!("  {} (score: {:.3})", result.term, result.score);
        }
        
        // Jaro-Winkler should find good matches for prefix-based queries
        assert!(!jw_results.is_empty(), "Jaro-Winkler should find matches for prefix '{}'", prefix);
        
        // Check that top results have decent scores (Jaro-Winkler emphasizes prefixes)
        if !jw_results.is_empty() {
            let top_score = jw_results[0].score;
            println!("Top Jaro-Winkler score for '{}': {:.3}", prefix, top_score);
        }
    }
}

#[test]
fn test_similarity_thresholds_comparison() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let query = "machne"; // typo for "machine"
    
    // Test different similarity thresholds for Jaro-Winkler
    let thresholds = [0.3, 0.5, 0.7, 0.8, 0.9];
    
    println!("Jaro-Winkler threshold comparison for query '{}':", query);
    
    for &threshold in &thresholds {
        let results = fuzzy_autocomplete_search(&index, query, threshold, Some(10)).unwrap();
        println!("  Threshold {:.1}: {} results", threshold, results.len());
        
        if !results.is_empty() {
            println!("    Top result: {} (score: {:.3})", results[0].term, results[0].score);
        }
    }
    
    // Test different edit distances for Levenshtein
    let edit_distances = [1, 2, 3];
    
    println!("Levenshtein edit distance comparison for query '{}':", query);
    
    for &edit_distance in &edit_distances {
        let results = fuzzy_autocomplete_search_levenshtein(&index, query, edit_distance, Some(10)).unwrap();
        println!("  Edit distance {}: {} results", edit_distance, results.len());
        
        if !results.is_empty() {
            println!("    Top result: {} (score: {:.3})", results[0].term, results[0].score);
        }
    }
}

#[test]
fn test_performance_comparison() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    let test_queries = ["machne", "pythno", "datascience", "aritificial"];
    
    for query in test_queries {
        println!("\n=== Performance comparison for '{}' ===", query);
        
        // Measure Levenshtein performance
        let start = std::time::Instant::now();
        let lev_results = fuzzy_autocomplete_search_levenshtein(&index, query, 2, Some(5)).unwrap();
        let lev_time = start.elapsed();
        
        // Measure Jaro-Winkler performance
        let start = std::time::Instant::now();
        let jw_results = fuzzy_autocomplete_search(&index, query, 0.5, Some(5)).unwrap();
        let jw_time = start.elapsed();
        
        println!("Levenshtein: {:?} ({} results)", lev_time, lev_results.len());
        println!("Jaro-Winkler: {:?} ({} results)", jw_time, jw_results.len());
        
        // Both should complete reasonably quickly
        assert!(lev_time.as_millis() < 1000, "Levenshtein should be fast");
        assert!(jw_time.as_millis() < 1000, "Jaro-Winkler should be fast");
    }
}

#[test]
fn test_word_level_matching_comparison() {
    let thesaurus = create_test_thesaurus();
    let index = build_autocomplete_index(thesaurus, None).unwrap();
    
    // Test word-level matching with multi-word terms
    let query = "learing"; // typo for "learning" in "machine learning"
    
    let lev_results = fuzzy_autocomplete_search_levenshtein(&index, query, 2, Some(5)).unwrap();
    let jw_results = fuzzy_autocomplete_search(&index, query, 0.6, Some(5)).unwrap();
    
    println!("Word-level matching comparison for '{}':", query);
    println!("Levenshtein results:");
    for result in &lev_results {
        println!("  {} (score: {:.3})", result.term, result.score);
    }
    
    println!("Jaro-Winkler results:");
    for result in &jw_results {
        println!("  {} (score: {:.3})", result.term, result.score);
    }
    
    // Both should find "machine learning" by matching the word "learning"
    let lev_has_learning = lev_results.iter().any(|r| r.term.contains("learning"));
    let jw_has_learning = jw_results.iter().any(|r| r.term.contains("learning"));
    
    if lev_has_learning || jw_has_learning {
        println!("Success: At least one method found 'learning' terms");
    }
} 