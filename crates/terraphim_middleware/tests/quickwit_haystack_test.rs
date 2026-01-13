use std::collections::HashMap;
use terraphim_config::{Haystack, ServiceType};
use terraphim_middleware::haystack::QuickwitHaystackIndexer;
use terraphim_middleware::indexer::IndexMiddleware;

#[tokio::test]
async fn test_explicit_index_configuration() {
    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("default_index".to_string(), "workers-logs".to_string());
    extra_params.insert("max_hits".to_string(), "10".to_string());

    let haystack = Haystack {
        location: "http://localhost:7280".to_string(),
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    // This will return empty since there's no running Quickwit server
    // But it should not crash or error
    let result = indexer.index("error", &haystack).await;
    assert!(result.is_ok());

    // Should return empty index gracefully when Quickwit unavailable
    let index = result.unwrap();
    assert_eq!(index.len(), 0);
}

#[tokio::test]
async fn test_auto_discovery_mode_no_default_index() {
    let indexer = QuickwitHaystackIndexer::default();
    // No default_index = auto-discovery mode
    let extra_params = HashMap::new();

    let haystack = Haystack {
        location: "http://localhost:7280".to_string(),
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    // Should attempt auto-discovery and return empty when Quickwit unavailable
    let result = indexer.index("test", &haystack).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_filtered_auto_discovery() {
    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    // No default_index, but has filter pattern
    extra_params.insert("index_filter".to_string(), "workers-*".to_string());

    let haystack = Haystack {
        location: "http://localhost:7280".to_string(),
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("test", &haystack).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_bearer_token_auth_configuration() {
    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("auth_token".to_string(), "Bearer test123".to_string());
    extra_params.insert("default_index".to_string(), "logs".to_string());

    let haystack = Haystack {
        location: "http://localhost:7280".to_string(),
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("test", &haystack).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_basic_auth_configuration() {
    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("auth_username".to_string(), "cloudflare".to_string());
    extra_params.insert("auth_password".to_string(), "secret".to_string());
    extra_params.insert("default_index".to_string(), "workers-logs".to_string());

    let haystack = Haystack {
        location: "https://logs.terraphim.cloud/api".to_string(),
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("test", &haystack).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_timeout_returns_empty() {
    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("default_index".to_string(), "logs".to_string());

    // Point to non-existent host - should timeout and return empty
    let haystack = Haystack {
        location: "http://127.0.0.1:9999".to_string(), // Unused port
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("test", &haystack).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
#[ignore] // Requires running Quickwit server
async fn test_quickwit_live_search_explicit() {
    // This test requires a running Quickwit instance
    // Run with: QUICKWIT_URL=http://localhost:7280 cargo test test_quickwit_live_search_explicit -- --ignored

    let quickwit_url =
        std::env::var("QUICKWIT_URL").unwrap_or_else(|_| "http://localhost:7280".to_string());

    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("default_index".to_string(), "workers-logs".to_string());
    extra_params.insert("max_hits".to_string(), "10".to_string());

    let haystack = Haystack {
        location: quickwit_url,
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("error", &haystack).await;
    assert!(result.is_ok());

    let index = result.unwrap();
    println!("Found {} documents", index.len());

    // Verify document structure
    if !index.is_empty() {
        let doc = index.values().next().unwrap();
        println!("Sample document: {:?}", doc.title);
        assert!(!doc.id.is_empty());
        assert!(!doc.title.is_empty());
        assert!(!doc.body.is_empty());
        assert!(doc.source_haystack.is_some());
        assert!(doc.tags.is_some());
    }
}

#[tokio::test]
#[ignore] // Requires running Quickwit server
async fn test_quickwit_live_autodiscovery() {
    // Test auto-discovery mode
    // Run with: QUICKWIT_URL=http://localhost:7280 cargo test test_quickwit_live_autodiscovery -- --ignored

    let quickwit_url =
        std::env::var("QUICKWIT_URL").unwrap_or_else(|_| "http://localhost:7280".to_string());

    let indexer = QuickwitHaystackIndexer::default();
    // No default_index = auto-discovery
    let extra_params = HashMap::new();

    let haystack = Haystack {
        location: quickwit_url,
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("*", &haystack).await;
    assert!(result.is_ok());

    let index = result.unwrap();
    println!(
        "Auto-discovery found {} documents across all indexes",
        index.len()
    );
}

#[tokio::test]
#[ignore] // Requires running Quickwit with authentication
async fn test_quickwit_live_with_basic_auth() {
    // Test with actual Quickwit instance using Basic Auth
    // Run with: QUICKWIT_URL=https://logs.terraphim.cloud/api QUICKWIT_USER=cloudflare QUICKWIT_PASS=xxx cargo test test_quickwit_live_with_basic_auth -- --ignored

    let quickwit_url = std::env::var("QUICKWIT_URL")
        .unwrap_or_else(|_| "https://logs.terraphim.cloud/api".to_string());
    let username = std::env::var("QUICKWIT_USER").unwrap_or_else(|_| "cloudflare".to_string());
    let password = std::env::var("QUICKWIT_PASS").expect("QUICKWIT_PASS must be set");

    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("auth_username".to_string(), username);
    extra_params.insert("auth_password".to_string(), password);
    extra_params.insert("default_index".to_string(), "workers-logs".to_string());
    extra_params.insert("max_hits".to_string(), "5".to_string());

    let haystack = Haystack {
        location: quickwit_url,
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("error", &haystack).await;
    assert!(result.is_ok());

    let index = result.unwrap();
    println!("Authenticated search found {} documents", index.len());

    // Should have results with authenticated access
    if !index.is_empty() {
        let doc = index.values().next().unwrap();
        assert!(!doc.id.is_empty());
        assert!(doc.id.starts_with("quickwit_"));
        assert!(doc.source_haystack.is_some());
    }
}

#[tokio::test]
#[ignore] // Requires running Quickwit
async fn test_quickwit_live_filtered_discovery() {
    // Test filtered auto-discovery
    // Run with: QUICKWIT_URL=http://localhost:7280 cargo test test_quickwit_live_filtered_discovery -- --ignored

    let quickwit_url =
        std::env::var("QUICKWIT_URL").unwrap_or_else(|_| "http://localhost:7280".to_string());

    let indexer = QuickwitHaystackIndexer::default();
    let mut extra_params = HashMap::new();
    extra_params.insert("index_filter".to_string(), "workers-*".to_string());
    extra_params.insert("max_hits".to_string(), "5".to_string());

    let haystack = Haystack {
        location: quickwit_url,
        service: ServiceType::Quickwit,
        read_only: true,
        fetch_content: false,
        atomic_server_secret: None,
        extra_parameters: extra_params,
    };

    let result = indexer.index("*", &haystack).await;
    assert!(result.is_ok());

    let index = result.unwrap();
    println!("Filtered discovery found {} documents", index.len());
}
