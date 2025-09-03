use terraphim_middleware::haystack::QueryRsHaystackIndexer;

/// Test to verify that the normalization fixes work for all problematic patterns
#[tokio::test]
async fn test_opendal_warning_fixes() {
    let indexer = QueryRsHaystackIndexer::default();

    println!("🧪 Testing OpenDAL Warning Fixes");
    println!("=================================");

    // Test the problematic IDs from the warnings
    let test_cases = vec![
        (
            "crate-gravity-db-0.1.0",
            "should not contain crategravitydb010md",
        ),
        ("crate-gqlite", "should be properly normalized"),
        ("crate-gqlite-0.0.0", "should not contain crategqlite000"),
        ("crate-caffe2-nomnigraph", "should handle hyphenated names"),
        ("docs-gravity-db", "should handle documentation IDs"),
        ("reddit-abc123", "should handle Reddit IDs"),
        // Test edge cases that might have caused the malformed IDs
        ("crate-gravity-db-0.1.0.md", "should remove .md extension"),
        ("crate/gqlite/0.0.0", "should handle path separators"),
    ];

    println!();
    for (original_id, description) in &test_cases {
        let normalized = indexer.normalize_document_id(original_id);
        let expected_key = format!("document_{}.json", normalized);

        println!("Testing: {} ({})", original_id, description);
        println!("  → Normalized: {}", normalized);
        println!("  → Key: {}", expected_key);

        // Validate that the normalized ID doesn't contain problematic patterns
        assert!(
            !normalized.contains("crategravitydb010md"),
            "ID should not contain crategravitydb010md pattern: {}",
            normalized
        );
        assert!(
            !normalized.contains("crategqlite000"),
            "ID should not contain crategqlite000 pattern: {}",
            normalized
        );
        assert!(
            !normalized.ends_with("md"),
            "ID should not end with 'md': {}",
            normalized
        );
        assert!(
            normalized.len() <= 50,
            "ID should not be excessively long: {} ({})",
            normalized,
            normalized.len()
        );
        assert!(
            !normalized.is_empty(),
            "ID should not be empty: {}",
            normalized
        );
        assert!(
            !normalized.contains("__"),
            "ID should not contain double underscores: {}",
            normalized
        );

        println!("  ✅ All validations passed");
        println!();
    }

    println!("🎯 All OpenDAL warning fix tests passed!");
}

/// Test the malformed ID detection and cleanup
#[test]
fn test_malformed_id_detection() {
    let indexer = QueryRsHaystackIndexer::default();

    println!("🧪 Testing Malformed ID Detection");
    println!("=================================");

    // Test IDs that should trigger the validation/cleanup
    let malformed_test_cases = vec![
        "crategravitydb010md", // Missing underscores, has md
        "crategqlite000",      // Missing underscores
        "some-very-long-document-id-that-exceeds-reasonable-limits-and-should-be-cleaned-up",
        "document.with.dots.md",
        "", // Empty ID
    ];

    for malformed_id in &malformed_test_cases {
        println!("Testing malformed ID: '{}'", malformed_id);
        let normalized = indexer.normalize_document_id(malformed_id);

        println!("  → Cleaned to: '{}'", normalized);
        println!(
            "  → Original ends with md: {}",
            malformed_id.ends_with("md")
        );
        println!(
            "  → Normalized ends with md: {}",
            normalized.ends_with("md")
        );
        println!("  → Length: {}", normalized.len());

        // All normalized IDs should be valid
        assert!(
            !normalized.is_empty(),
            "Should not produce empty ID from '{}'",
            malformed_id
        );
        assert!(
            !normalized.ends_with("md"),
            "Should not end with md: '{}' from '{}'",
            normalized,
            malformed_id
        );
        assert!(
            normalized.len() <= 50,
            "Should not be excessively long: {} ({})",
            normalized,
            normalized.len()
        );
        assert!(
            !normalized.contains("crategravitydb010md"),
            "Should not contain problematic patterns: '{}'",
            normalized
        );

        println!("  ✅ Cleanup successful");
    }
}

/// Test that normal IDs still work correctly
#[test]
fn test_normal_ids_still_work() {
    let indexer = QueryRsHaystackIndexer::default();

    println!("🧪 Testing Normal IDs Still Work");
    println!("=================================");

    let normal_test_cases = vec![
        ("crate-serde", "crate_serde"),
        ("reddit-abc123", "reddit_abc123"),
        ("docs-tokio", "docs_tokio"),
        ("simple-document", "simple_document"),
    ];

    for (input, expected) in &normal_test_cases {
        let normalized = indexer.normalize_document_id(input);
        assert_eq!(
            &normalized, expected,
            "Normal ID normalization should work: {} → {}",
            input, expected
        );
        println!("✅ {} → {} (expected {})", input, normalized, expected);
    }
}
