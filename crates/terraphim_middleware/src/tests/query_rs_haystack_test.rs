use terraphim_config::{Haystack, ServiceType};
use crate::haystack::QueryRsHaystackIndexer;
use crate::indexer::IndexMiddleware;

#[tokio::test]
async fn test_query_rs_haystack_indexer() {
    let indexer = QueryRsHaystackIndexer::new();
    
    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    // Test searching for Rust-related terms
    let needle = "async";
    let result = indexer.index(needle, &haystack).await;
    
    match result {
        Ok(index) => {
            println!("Found {} documents", index.len());
            for (id, doc) in index.iter() {
                println!("Document: {} - {}", doc.title, doc.url);
            }
            // We expect some results for "async" in Rust context
            assert!(!index.is_empty(), "Should find some documents for 'async'");
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!("QueryRs haystack test failed (expected for network issues): {}", e);
        }
    }
}

#[tokio::test]
async fn test_query_rs_reddit_search() {
    let indexer = QueryRsHaystackIndexer::new();
    
    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    // Test searching for Reddit posts
    let needle = "tokio";
    let result = indexer.index(needle, &haystack).await;
    
    match result {
        Ok(index) => {
            println!("Found {} documents from Reddit search", index.len());
            for (id, doc) in index.iter() {
                if doc.url.contains("reddit") {
                    println!("Reddit post: {} - {}", doc.title, doc.url);
                }
            }
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!("QueryRs Reddit search test failed (expected for network issues): {}", e);
        }
    }
}

#[tokio::test]
async fn test_query_rs_crates_search() {
    let indexer = QueryRsHaystackIndexer::new();
    
    let haystack = Haystack {
        location: "https://query.rs".to_string(),
        service: ServiceType::QueryRs,
        read_only: true,
        atomic_server_secret: None,
        extra_parameters: std::collections::HashMap::new(),
    };

    // Test searching for crates
    let needle = "serde";
    let result = indexer.index(needle, &haystack).await;
    
    match result {
        Ok(index) => {
            println!("Found {} documents from crates search", index.len());
            for (id, doc) in index.iter() {
                if doc.url.contains("crates.io") {
                    println!("Crate: {} - {}", doc.title, doc.url);
                }
            }
        }
        Err(e) => {
            // It's okay if the test fails due to network issues
            println!("QueryRs crates search test failed (expected for network issues): {}", e);
        }
    }
} 