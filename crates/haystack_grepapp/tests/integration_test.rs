use grepapp_haystack::{GrepAppClient, GrepAppHaystack, SearchParams};
use haystack_core::HaystackProvider;
use terraphim_types::SearchQuery;

/// Live test against grep.app API
/// Run with: cargo test -p grepapp_haystack live_search_test -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn live_search_test() {
    let client = GrepAppClient::new().expect("Failed to create client");

    let params = SearchParams {
        query: "tokio spawn".to_string(),
        language: Some("Rust".to_string()),
        repo: None,
        path: None,
    };

    let results = client.search(&params).await.expect("Search failed");

    println!("Found {} results", results.len());
    assert!(!results.is_empty(), "Expected to find results");

    for (i, hit) in results.iter().take(5).enumerate() {
        println!("\n--- Result {} ---", i + 1);
        println!("Repo: {}", hit.source.repo.raw);
        println!("File: {}", hit.source.path.raw);
        println!("Branch: {}", hit.source.branch.raw);
        println!("Snippet: {}", hit.source.content.snippet);
    }
}

/// Live test with HaystackProvider trait
/// Run with: cargo test -p grepapp_haystack live_haystack_test -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn live_haystack_test() {
    let haystack = GrepAppHaystack::with_filters(
        Some("Rust".to_string()),
        Some("tokio-rs/tokio".to_string()),
        None,
    )
    .expect("Failed to create haystack");

    let query = SearchQuery {
        search_term: "JoinHandle".into(),
        ..Default::default()
    };

    let documents = haystack.search(&query).await.expect("Search failed");

    println!("Found {} documents", documents.len());
    assert!(!documents.is_empty(), "Expected to find documents");

    for (i, doc) in documents.iter().take(5).enumerate() {
        println!("\n--- Document {} ---", i + 1);
        println!("Title: {}", doc.title);
        println!("URL: {}", doc.url);
        println!("Body: {}", doc.body);
        if let Some(ref tags) = doc.tags {
            println!("Tags: {:?}", tags);
        }
    }
}

/// Test search across multiple languages
/// Run with: cargo test -p grepapp_haystack live_multi_language_test -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn live_multi_language_test() {
    let languages = vec!["Rust", "Python", "JavaScript"];

    for lang in languages {
        println!("\n=== Testing {} ===", lang);

        let haystack = GrepAppHaystack::with_filters(Some(lang.to_string()), None, None)
            .expect("Failed to create haystack");

        let query = SearchQuery {
            search_term: "async function".into(),
            ..Default::default()
        };

        match haystack.search(&query).await {
            Ok(documents) => {
                println!("Found {} documents in {}", documents.len(), lang);

                if let Some(doc) = documents.first() {
                    println!("Sample: {} - {}", doc.title, doc.url);
                }
            }
            Err(e) => {
                println!("Error searching {}: {}", lang, e);
            }
        }
    }
}

/// Test search with path filtering
/// Run with: cargo test -p grepapp_haystack live_path_filter_test -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn live_path_filter_test() {
    let haystack =
        GrepAppHaystack::with_filters(Some("Rust".to_string()), None, Some("src/".to_string()))
            .expect("Failed to create haystack");

    let query = SearchQuery {
        search_term: "impl Default".into(),
        ..Default::default()
    };

    let documents = haystack.search(&query).await.expect("Search failed");

    println!("Found {} documents in src/ directories", documents.len());

    for doc in documents.iter().take(3) {
        println!("\n{}", doc.title);
        println!("URL: {}", doc.url);
        assert!(doc.url.contains("/src/"), "Expected URL to contain /src/");
    }
}

/// Test error handling with invalid query
#[tokio::test]
async fn test_empty_query_error() {
    let client = GrepAppClient::new().expect("Failed to create client");

    let params = SearchParams {
        query: "".to_string(),
        ..Default::default()
    };

    let result = client.search(&params).await;
    assert!(result.is_err(), "Expected error for empty query");
}

/// Test query length validation
#[tokio::test]
async fn test_query_too_long_error() {
    let client = GrepAppClient::new().expect("Failed to create client");

    let params = SearchParams {
        query: "a".repeat(1001),
        ..Default::default()
    };

    let result = client.search(&params).await;
    assert!(result.is_err(), "Expected error for query too long");
}
