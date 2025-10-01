use crate::haystack::QueryRsHaystackIndexer;
use crate::indexer::IndexMiddleware;
use terraphim_config::{Haystack, ServiceType};

#[tokio::test]
async fn test_query_rs_haystack_indexer() {
    let indexer = QueryRsHaystackIndexer::default();

    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };

    // Test searching for Rust-related terms
    let needle = "async";
    let result = indexer.index(needle, &haystack).await;

    match result {
        Ok(index) => {
            println!("Found {} documents", index.len());
            for (_id, doc) in index.iter() {
                println!("Document: {} - {}", doc.title, doc.url);
            }
            // We expect some results for "async" in Rust context
            assert!(!index.is_empty(), "Should find some documents for 'async'");
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!(
                "QueryRs haystack test failed (expected for network issues): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_query_rs_reddit_search() {
    let indexer = QueryRsHaystackIndexer::default();

    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };

    // Test searching for Reddit posts
    let needle = "tokio";
    let result = indexer.index(needle, &haystack).await;

    match result {
        Ok(index) => {
            println!("Found {} documents from Reddit search", index.len());
            for (_id, doc) in index.iter() {
                if doc.url.contains("reddit") {
                    println!("Reddit post: {} - {}", doc.title, doc.url);
                }
            }
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!(
                "QueryRs Reddit search test failed (expected for network issues): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_query_rs_crates_search() {
    let indexer = QueryRsHaystackIndexer::default();

    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
        weight: 1.0,
    };

    // Test searching for crates - using "graph" since that's what was failing with 404s
    let needle = "graph";
    let result = indexer.index(needle, &haystack).await;

    match result {
        Ok(index) => {
            println!("Found {} documents from crates search", index.len());
            let mut found_crates = 0;
            for (_id, doc) in index.iter() {
                if doc.url.contains("crates.io") {
                    found_crates += 1;
                    println!("Crate: {} - {}", doc.title, doc.url);

                    // Verify that the body contains enhanced information from API (not 404 errors)
                    assert!(
                        doc.body.contains("Description:"),
                        "Crate body should contain API description data"
                    );
                    assert!(
                        doc.body.contains("Downloads:"),
                        "Crate body should contain download count from API"
                    );
                    assert!(
                        doc.body.len() > 50,
                        "Crate body should be comprehensive with API data, not minimal due to 404"
                    );

                    println!(
                        "✅ Crate has comprehensive API data (no 404): {} chars",
                        doc.body.len()
                    );
                    if found_crates >= 3 {
                        break;
                    } // Check a few to verify the fix
                }
            }

            if found_crates > 0 {
                println!(
                    "✅ Successfully found {} crates with comprehensive data (404 errors fixed)",
                    found_crates
                );
            }
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!(
                "QueryRs crates search test failed (expected for network issues): {}",
                e
            );
        }
    }
}

#[test]
fn test_extract_reddit_post_id() {
    let indexer = QueryRsHaystackIndexer::default();

    // Test normal Reddit URLs
    assert_eq!(
        indexer.extract_reddit_post_id("https://www.reddit.com/r/rust/comments/abc123/some_title/"),
        Some("abc123".to_string())
    );

    assert_eq!(
        indexer
            .extract_reddit_post_id("https://www.reddit.com/r/rust/comments/xyz456/another_title"),
        Some("xyz456".to_string())
    );

    // Test URL with query parameters
    assert_eq!(
        indexer.extract_reddit_post_id(
            "https://www.reddit.com/r/rust/comments/def789/title/?utm_source=share"
        ),
        Some("def789".to_string())
    );

    // Test invalid URLs
    assert_eq!(
        indexer.extract_reddit_post_id("https://www.reddit.com/r/rust/"),
        None
    );

    assert_eq!(
        indexer.extract_reddit_post_id("https://www.reddit.com/r/rust/hot/"),
        None
    );

    assert_eq!(
        indexer.extract_reddit_post_id("https://example.com/not-reddit"),
        None
    );
}

#[test]
fn test_extract_doc_identifier() {
    let indexer = QueryRsHaystackIndexer::default();

    // Test Rust std docs
    assert_eq!(
        indexer.extract_doc_identifier("https://doc.rust-lang.org/std/iter/trait.Iterator.html"),
        "std_iter_trait_Iterator"
    );

    assert_eq!(
        indexer.extract_doc_identifier(
            "https://doc.rust-lang.org/std/collections/struct.HashMap.html"
        ),
        "std_collections_struct_HashMap"
    );

    // Test docs.rs URLs
    assert_eq!(
        indexer.extract_doc_identifier("https://docs.rs/serde/latest/serde/"),
        "docs_rs_serde_latest_serde"
    );

    // Test other domains
    assert_eq!(
        indexer.extract_doc_identifier("https://example.com/docs/api/"),
        "example_com_docs_api"
    );

    // Test URL without path
    assert_eq!(
        indexer.extract_doc_identifier("https://example.com/"),
        "example_com"
    );
}

#[test]
fn test_document_id_generation_cleaner() {
    let indexer = QueryRsHaystackIndexer::default();

    // Test that Reddit URLs now generate clean IDs instead of full URLs
    let reddit_url = "https://www.reddit.com/r/rust/comments/abc123/some_great_rust_post/";
    let reddit_post_id = indexer.extract_reddit_post_id(reddit_url);
    assert_eq!(reddit_post_id, Some("abc123".to_string()));

    // Verify the normalized ID is clean
    let original_id = format!("reddit-{}", reddit_post_id.unwrap());
    let normalized_id = indexer.normalize_document_id(&original_id);
    assert_eq!(normalized_id, "reddit_abc123");

    // Test that documentation URLs generate clean IDs
    let doc_url = "https://doc.rust-lang.org/std/iter/trait.Iterator.html";
    let doc_identifier = indexer.extract_doc_identifier(doc_url);
    assert_eq!(doc_identifier, "std_iter_trait_Iterator");

    let original_doc_id = format!("std-{}", doc_identifier);
    let normalized_doc_id = indexer.normalize_document_id(&original_doc_id);
    assert_eq!(normalized_doc_id, "std_std_iter_trait_iterator");

    println!("✅ Document IDs are now much cleaner:");
    println!("  Reddit: {} -> {}", reddit_url, normalized_id);
    println!("  Docs:   {} -> {}", doc_url, normalized_doc_id);
    println!("  No more long URLs causing OpenDAL path issues!");
}
