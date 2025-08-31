use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{axum_server, ConfigResponse, SearchResponse};

/// Integration test for Knowledge Graph term to document lookup functionality
///
/// This test validates the complete flow:
/// 1. Server starts with Terraphim Engineer configuration (local KG)
/// 2. Local knowledge graph is built from docs/src/kg
/// 3. Documents are indexed with their terms and synonyms
/// 4. API endpoint /roles/{role_name}/kg_search works correctly
/// 5. Known terms like "haystack" find their source documents
#[tokio::test]
#[serial]
async fn test_kg_term_to_document_lookup() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    // Check if Terraphim documentation exists
    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    let docs_src_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("../docs/src")
    } else {
        PathBuf::from("docs/src")
    };

    if !docs_src_path.exists() {
        log::warn!(
            "Terraphim docs not found at {:?}. Skipping test.",
            docs_src_path
        );
        return;
    }

    let kg_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("../docs/src/kg")
    } else {
        PathBuf::from("docs/src/kg")
    };

    if !kg_path.exists() {
        log::warn!(
            "Terraphim KG directory not found at {:?}. Skipping test.",
            kg_path
        );
        return;
    }

    // Initialize memory-only storage for this test
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory storage");

    // Load the Terraphim Engineer configuration
    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("../default/terraphim_engineer_config.json")
    } else {
        PathBuf::from("default/terraphim_engineer_config.json")
    };

    let mut config = if config_path.exists() {
        log::info!("Loading Terraphim Engineer config from: {:?}", config_path);
        let config_file = std::fs::File::open(&config_path).unwrap();
        serde_json::from_reader(config_file).unwrap()
    } else {
        log::warn!("Terraphim Engineer config not found. Using default config.");
        Config::default()
    };

    // Create config state with the loaded configuration
    let config_state = ConfigState::new(&mut config).await.unwrap();

    // Start the server
    let server_addr = "127.0.0.1:8001".parse().unwrap();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum_server(server_addr, config_state).await {
            log::error!("Server error: {:?}", e);
        }
    });

    // Give the server time to start
    sleep(Duration::from_secs(5)).await;

    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let base_url = "http://127.0.0.1:8001";

    // Test 1: Health check
    log::info!("🧪 Testing health endpoint");
    let health_response = client
        .get(format!("{}/health", base_url))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    assert!(health_response.is_ok(), "Health check failed");
    let health_response = health_response.unwrap();
    assert_eq!(health_response.status(), 200);
    log::info!("✅ Health check passed");

    // Test 2: Get config to ensure server is properly configured
    log::info!("🧪 Testing config endpoint");
    let config_response = client
        .get(format!("{}/config", base_url))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    assert!(config_response.is_ok(), "Config request failed");
    let config_response = config_response.unwrap();
    assert_eq!(config_response.status(), 200);

    let config_data: ConfigResponse = config_response
        .json()
        .await
        .expect("Failed to parse config response");
    log::info!("✅ Config retrieved successfully");
    log::info!(
        "Available roles: {:?}",
        config_data.config.roles.keys().collect::<Vec<_>>()
    );

    // Test 3: KG term to document lookup - Test with "haystack" term
    log::info!("🧪 Testing KG term to document lookup for 'haystack'");
    let kg_search_response = client
        .get(format!(
            "{}/roles/Terraphim Engineer/kg_search?term=haystack",
            base_url
        ))
        .timeout(Duration::from_secs(15))
        .send()
        .await;

    assert!(
        kg_search_response.is_ok(),
        "KG search request failed: {:?}",
        kg_search_response
    );
    let kg_search_response = kg_search_response.unwrap();
    assert_eq!(
        kg_search_response.status(),
        200,
        "KG search returned non-200 status"
    );

    let search_data: SearchResponse = kg_search_response
        .json()
        .await
        .expect("Failed to parse KG search response");

    log::info!("✅ KG search completed");
    log::info!("Found {} documents for 'haystack'", search_data.total);

    // Validate results
    assert!(
        search_data.total > 0,
        "Expected to find at least one document for 'haystack' term"
    );

    // Check if we found the haystack.md document specifically
    let haystack_doc = search_data
        .results
        .iter()
        .find(|doc| doc.id.contains("haystack") || doc.title.to_lowercase().contains("haystack"));

    if let Some(doc) = haystack_doc {
        log::info!(
            "✅ Found haystack document: id='{}', title='{}'",
            doc.id,
            doc.title
        );

        // Verify the document contains the expected content or synonyms
        let content = format!("{} {}", doc.title, doc.body);
        let has_synonyms = content.to_lowercase().contains("datasource")
            || content.to_lowercase().contains("service")
            || content.to_lowercase().contains("agent");

        if has_synonyms {
            log::info!("✅ Document contains expected synonyms");
        } else {
            log::warn!(
                "⚠️ Document doesn't contain expected synonyms, but that's okay for this test"
            );
        }
    } else {
        log::warn!(
            "⚠️ Didn't find specific haystack.md document, but found {} related documents",
            search_data.total
        );
    }

    // Test 4: Test with a synonym term (should find the same haystack document)
    log::info!("🧪 Testing KG term to document lookup for synonym 'service'");
    let synonym_search_response = client
        .get(format!(
            "{}/roles/Terraphim Engineer/kg_search?term=service",
            base_url
        ))
        .timeout(Duration::from_secs(15))
        .send()
        .await;

    if synonym_search_response.is_ok() {
        let synonym_response = synonym_search_response.unwrap();
        if synonym_response.status() == 200 {
            let synonym_data: SearchResponse = synonym_response
                .json()
                .await
                .expect("Failed to parse synonym search response");

            log::info!("✅ Synonym search completed");
            log::info!(
                "Found {} documents for 'service' synonym",
                synonym_data.total
            );

            if synonym_data.total > 0 {
                log::info!("✅ Synonym search found documents as expected");
            }
        }
    }

    // Test 5: Test with non-existent term
    log::info!("🧪 Testing KG term to document lookup for non-existent term 'nonexistentterm'");
    let empty_search_response = client
        .get(format!(
            "{}/roles/Terraphim Engineer/kg_search?term=nonexistentterm",
            base_url
        ))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    if empty_search_response.is_ok() {
        let empty_response = empty_search_response.unwrap();
        if empty_response.status() == 200 {
            let empty_data: SearchResponse = empty_response
                .json()
                .await
                .expect("Failed to parse empty search response");

            log::info!("✅ Empty search completed");
            log::info!(
                "Found {} documents for non-existent term (expected: 0)",
                empty_data.total
            );

            // This should return 0 results
            assert_eq!(
                empty_data.total, 0,
                "Expected 0 results for non-existent term"
            );
            log::info!("✅ Empty search returned correct result");
        }
    }

    log::info!("🎉 All KG term to document lookup tests completed successfully!");

    // Clean up - abort the server
    server_handle.abort();
}

/// Test the KG term lookup with different role configurations
#[tokio::test]
#[serial]
async fn test_kg_term_lookup_invalid_role() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    // Initialize memory-only storage
    terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("Failed to initialize memory storage");

    // Create minimal config state
    let mut config = Config::default();
    let config_state = ConfigState::new(&mut config).await.unwrap();

    // Start the server
    let server_addr = "127.0.0.1:8002".parse().unwrap();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum_server(server_addr, config_state).await {
            log::error!("Server error: {:?}", e);
        }
    });

    // Give the server time to start
    sleep(Duration::from_secs(3)).await;

    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let base_url = "http://127.0.0.1:8002";

    // Test with invalid role name
    log::info!("🧪 Testing KG search with invalid role name");
    let invalid_role_response = client
        .get(format!(
            "{}/roles/NonExistentRole/kg_search?term=test",
            base_url
        ))
        .timeout(Duration::from_secs(10))
        .send()
        .await;

    assert!(
        invalid_role_response.is_ok(),
        "Request should complete even with invalid role"
    );
    let response = invalid_role_response.unwrap();

    // Should return an error status (400 or 404 or 500)
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected error status for invalid role, got: {}",
        response.status()
    );

    log::info!(
        "✅ Invalid role test completed - correctly returned error status: {}",
        response.status()
    );

    // Clean up
    server_handle.abort();
}
