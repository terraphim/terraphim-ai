use terraphim_middleware::haystack::QueryRsHaystackIndexer;

/// Test to prove that the OpenDAL persistence warnings fix works
/// This test validates that document IDs are now clean and predictable
#[tokio::test]
async fn test_document_id_fix_comprehensive() {
    let indexer = QueryRsHaystackIndexer::default();

    println!("🧪 Testing Document ID Generation Fix");
    println!("=====================================");

    // Test cases that were causing OpenDAL warnings before the fix
    let problematic_urls = vec![
        // Reddit URLs that used to create extremely long IDs
        ("https://www.reddit.com/r/rust/comments/abc123/some_title/", "reddit_abc123"),
        ("https://www.reddit.com/r/rust/comments/xyz789/rust_async_programming_guide/?utm_source=share", "reddit_xyz789"),

        // Documentation URLs that used to include full paths
        ("https://doc.rust-lang.org/std/iter/trait.Iterator.html", "std_std_iter_trait_iterator"),
        ("https://doc.rust-lang.org/std/collections/struct.HashMap.html", "std_std_collections_struct_hashmap"),

        // External documentation URLs
        ("https://docs.rs/serde/latest/serde/", "std_docs_rs_serde_latest_serde"),
        ("https://docs.rs/tokio/1.0.0/tokio/runtime/", "std_docs_rs_tokio_1_0_0_tokio_runtime"),
    ];

    for (url, expected_clean_id) in problematic_urls {
        println!("\n🔍 Testing URL: {}", url);

        // Test Reddit post ID extraction
        if url.contains("reddit.com") {
            let post_id = indexer.extract_reddit_post_id(url);
            assert!(
                post_id.is_some(),
                "Should extract Reddit post ID from: {}",
                url
            );

            let original_id = format!("reddit-{}", post_id.unwrap());
            let normalized_id = indexer.normalize_document_id(&original_id);

            assert_eq!(
                normalized_id, expected_clean_id,
                "Reddit ID should be clean for: {}",
                url
            );

            println!("  ✅ Reddit: {} -> {}", url, normalized_id);

            // Verify the normalized ID is filesystem-safe
            assert!(
                normalized_id.len() < 50,
                "ID should be reasonably short: {}",
                normalized_id
            );
            assert!(
                !normalized_id.contains('/'),
                "ID should not contain slashes: {}",
                normalized_id
            );
            assert!(
                !normalized_id.contains('\\'),
                "ID should not contain backslashes: {}",
                normalized_id
            );
            assert!(
                !normalized_id.contains(':'),
                "ID should not contain colons: {}",
                normalized_id
            );
        }

        // Test documentation identifier extraction
        if url.contains("doc.rust-lang.org") || url.contains("docs.rs") {
            let doc_identifier = indexer.extract_doc_identifier(url);

            let original_id = format!("std-{}", doc_identifier);
            let normalized_id = indexer.normalize_document_id(&original_id);

            assert_eq!(
                normalized_id, expected_clean_id,
                "Doc ID should be clean for: {}",
                url
            );

            println!("  ✅ Docs: {} -> {}", url, normalized_id);

            // Verify the normalized ID is filesystem-safe
            assert!(
                normalized_id.len() < 100,
                "Doc ID should be reasonably short: {}",
                normalized_id
            );
            assert!(
                !normalized_id.contains('/'),
                "Doc ID should not contain slashes: {}",
                normalized_id
            );
            assert!(
                !normalized_id.contains('.'),
                "Doc ID should not contain dots: {}",
                normalized_id
            );
        }
    }

    println!("\n✅ All document IDs are now clean and filesystem-safe!");
    println!("🎯 OpenDAL persistence warnings should be eliminated");
}

/// Test to validate that crate names with special characters are handled properly
#[test]
fn test_crate_name_normalization() {
    let indexer = QueryRsHaystackIndexer::default();

    // Test crate names that were causing issues
    let problematic_crate_names = vec![
        "caffe2-nomnigraph", // This was causing `document_crate_caffe2_nomnigraph.json` warnings
        "proc-macro2",
        "serde_json",
        "async-trait",
        "regex-lite",
    ];

    println!("🧪 Testing Crate Name Normalization");
    println!("==================================");

    for crate_name in problematic_crate_names {
        let original_id = format!("crate-{}", crate_name);
        let normalized_id = indexer.normalize_document_id(&original_id);

        println!("  Crate: {} -> {}", crate_name, normalized_id);

        // Verify normalization is working correctly
        assert!(
            !normalized_id.contains('-'),
            "Should not contain hyphens: {}",
            normalized_id
        );
        assert!(
            normalized_id.starts_with("crate_"),
            "Should start with crate_: {}",
            normalized_id
        );
        assert!(
            normalized_id.len() < 50,
            "Should be reasonably short: {}",
            normalized_id
        );

        // The normalized ID should be filesystem-safe
        let expected_safe_chars = normalized_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_');
        assert!(
            expected_safe_chars,
            "Should only contain safe characters: {}",
            normalized_id
        );
    }

    println!("✅ All crate names properly normalized");
}

/// Test edge cases and fallback mechanisms
#[test]
fn test_edge_cases_and_fallbacks() {
    let indexer = QueryRsHaystackIndexer::default();

    println!("🧪 Testing Edge Cases and Fallbacks");
    println!("==================================");

    // Test malformed Reddit URLs (should fallback to hash)
    let malformed_reddit_url = "https://www.reddit.com/r/rust/malformed/url/structure/";
    let post_id = indexer.extract_reddit_post_id(malformed_reddit_url);

    if post_id.is_none() {
        println!("  ✅ Malformed Reddit URL correctly returns None, will use hash fallback");
    }

    // Test very long URLs that would cause issues
    let very_long_url = format!("https://example.com/{}", "very_long_path".repeat(20));
    let doc_identifier = indexer.extract_doc_identifier(&very_long_url);

    println!(
        "  Long URL: {} chars -> {} chars",
        very_long_url.len(),
        doc_identifier.len()
    );
    assert!(
        doc_identifier.len() < very_long_url.len(),
        "Should be shorter than original"
    );

    // Test URL with special characters
    let special_url = "https://example.com/path/with-special@chars#section?param=value";
    let special_identifier = indexer.extract_doc_identifier(special_url);

    println!("  Special chars: {} -> {}", special_url, special_identifier);
    assert!(
        !special_identifier.contains('@'),
        "Should not contain @ symbols"
    );
    assert!(
        !special_identifier.contains('#'),
        "Should not contain # symbols"
    );
    assert!(
        !special_identifier.contains('?'),
        "Should not contain ? symbols"
    );

    println!("✅ Edge cases handled properly with fallbacks");
}

/// Benchmark test to ensure the new ID generation is performant
#[test]
fn test_performance_of_id_generation() {
    let indexer = QueryRsHaystackIndexer::default();

    let test_urls = vec![
        "https://www.reddit.com/r/rust/comments/test123/benchmark_post/",
        "https://doc.rust-lang.org/std/collections/HashMap.html",
        "https://docs.rs/serde/latest/serde/",
    ];

    let start_time = std::time::Instant::now();

    // Generate 100 document IDs to test performance
    for _ in 0..100 {
        for url in &test_urls {
            if url.contains("reddit.com") {
                if let Some(post_id) = indexer.extract_reddit_post_id(url) {
                    let original_id = format!("reddit-{}", post_id);
                    let _normalized = indexer.normalize_document_id(&original_id);
                }
            } else {
                let doc_id = indexer.extract_doc_identifier(url);
                let original_id = format!("doc-{}", doc_id);
                let _normalized = indexer.normalize_document_id(&original_id);
            }
        }
    }

    let duration = start_time.elapsed();
    println!("🏁 Generated 300 document IDs in {:?}", duration);

    // Should be very fast (< 1000ms for 300 IDs)
    assert!(
        duration.as_millis() < 1000,
        "ID generation should be fast: {:?}",
        duration
    );

    println!("✅ Document ID generation is performant");
}
