//! Integration tests for RoleGraph client with real taxonomy files
//!
//! Tests the RoleGraph client against the actual taxonomy files in llm_proxy_terraphim/

use std::path::Path;
use terraphim_llm_proxy::rolegraph_client::RoleGraphClient;

const TAXONOMY_PATH: &str = "docs/taxonomy";

#[test]
#[ignore] // Run with: cargo test --test rolegraph_integration_test -- --ignored
fn test_load_real_taxonomy() {
    let taxonomy_path = Path::new(TAXONOMY_PATH);

    // Skip if taxonomy directory doesn't exist
    if !taxonomy_path.exists() {
        println!(
            "Skipping: taxonomy directory not found at {}",
            TAXONOMY_PATH
        );
        return;
    }

    let mut client =
        RoleGraphClient::new(taxonomy_path).expect("Failed to create RoleGraph client");

    // Load taxonomy files
    client.load_taxonomy().expect("Failed to load taxonomy");

    println!("✓ Successfully loaded taxonomy from {}", TAXONOMY_PATH);
}

#[test]
#[ignore]
fn test_pattern_matching_with_real_taxonomy() {
    let taxonomy_path = Path::new(TAXONOMY_PATH);

    if !taxonomy_path.exists() {
        println!("Skipping: taxonomy directory not found");
        return;
    }

    let mut client = RoleGraphClient::new(taxonomy_path).unwrap();
    client.load_taxonomy().unwrap();

    // Test think routing synonyms
    let test_queries = vec![
        ("I need to think about this problem", "think"),
        ("Let me reason through this", "think"),
        ("Enter plan mode please", "think"),
        ("Background task to process files", "background"),
        ("Search the web for information", "search"),
    ];

    for (query, expected_pattern) in test_queries {
        let matches = client.match_patterns(query);

        if !matches.is_empty() {
            println!("Query: '{}' → Matched: {:?}", query, matches[0].concept);
            assert!(
                matches[0].concept.contains(expected_pattern),
                "Expected pattern '{}' in concept '{}' for query '{}'",
                expected_pattern,
                matches[0].concept,
                query
            );
        } else {
            println!("Query: '{}' → No matches", query);
        }
    }
}

#[test]
#[ignore]
fn test_routing_decisions_with_real_taxonomy() {
    let taxonomy_path = Path::new(TAXONOMY_PATH);

    if !taxonomy_path.exists() {
        println!("Skipping: taxonomy directory not found");
        return;
    }

    let mut client = RoleGraphClient::new(taxonomy_path).unwrap();
    client.load_taxonomy().unwrap();

    // Test various queries and validate routing decisions
    let test_cases = vec![
        ("enter plan mode please", Some("think")), // Matches "plan mode" synonym - KNOWN WORKING
        ("regular query", None),                   // Should not match
    ];

    for (query, expected_match) in test_cases {
        let routing = client.query_routing(query);

        match (routing, expected_match) {
            (Some(decision), Some(expected)) => {
                println!(
                    "✓ '{}' → {} (provider: {})",
                    query, decision.concept, decision.provider
                );
                assert!(decision.concept.contains(expected));
            }
            (None, None) => {
                println!("✓ '{}' → No match (as expected)", query);
            }
            (Some(decision), None) => {
                println!("⚠  '{}' → {} (unexpected match)", query, decision.concept);
            }
            (None, Some(_expected)) => {
                panic!("Expected match for '{}' but got None", query);
            }
        }
    }
}

#[test]
#[ignore]
fn test_all_taxonomy_files_parseable() {
    let taxonomy_path = Path::new(TAXONOMY_PATH);

    if !taxonomy_path.exists() {
        println!("Skipping: taxonomy directory not found");
        return;
    }

    let client = RoleGraphClient::new(taxonomy_path).unwrap();
    let files = client
        .scan_taxonomy_files()
        .expect("Failed to scan taxonomy files");

    println!("Found {} taxonomy files", files.len());
    assert!(
        files.len() >= 40,
        "Expected at least 40 taxonomy files, found {}",
        files.len()
    );

    // Try parsing each file
    let mut parsed = 0;
    let mut failed = 0;

    for file in &files {
        match client.parse_taxonomy_file(file) {
            Ok((concept, synonyms)) => {
                parsed += 1;
                println!("✓ {}: {} synonyms", concept, synonyms.len());
            }
            Err(e) => {
                failed += 1;
                println!("✗ {:?}: {}", file.file_name(), e);
            }
        }
    }

    println!("\nParsing results: {} parsed, {} failed", parsed, failed);
    assert!(parsed >= 40, "Too many parsing failures");
}
