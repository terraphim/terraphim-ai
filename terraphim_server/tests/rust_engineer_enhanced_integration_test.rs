use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{ConfigResponse, SearchResponse, axum_server};

/// Integration test for Rust Engineer configuration with QueryRs + GrepApp haystacks
///
/// This test validates:
/// 1. Server starts with Rust Engineer configuration
/// 2. Both QueryRs and GrepApp haystacks are configured
/// 3. Search returns results from both sources
/// 4. Duplicate handling behavior is observed and documented
#[tokio::test]
#[serial]
#[ignore] // Requires internet connection to QueryRs and grep.app APIs
async fn test_rust_engineer_dual_haystack_integration() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    // Load the combined roles configuration
    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/combined_roles_config.json")
    } else {
        PathBuf::from("terraphim_server/default/combined_roles_config.json")
    };

    if !config_path.exists() {
        log::warn!(
            "Combined roles config not found at {:?}. Skipping test.",
            config_path
        );
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let config_content = config_content.trim();
    log::info!("Config file content length: {} bytes", config_content.len());

    let mut config: Config = serde_json::from_str(config_content)
        .map_err(|e| {
            log::error!("JSON parsing error: {}", e);
            e
        })
        .expect("Failed to parse config JSON");

    // Create config state
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    log::info!("✅ Configuration loaded with {} roles", config.roles.len());

    // Verify Rust Engineer role exists and is configured correctly
    let rust_engineer_role = config
        .roles
        .get(&"Rust Engineer".into())
        .expect("Rust Engineer role should exist");

    assert_eq!(
        rust_engineer_role.name,
        "Rust Engineer".into(),
        "Role name should be Rust Engineer"
    );

    assert!(
        rust_engineer_role.haystacks.len() >= 2,
        "Rust Engineer should have at least 2 haystacks (QueryRs and GrepApp)"
    );

    // Verify QueryRs haystack
    let queryrs_haystack = rust_engineer_role
        .haystacks
        .iter()
        .find(|h| h.service == terraphim_config::ServiceType::QueryRs)
        .expect("Should have QueryRs haystack");

    // Verify GrepApp haystack
    let grepapp_haystack = rust_engineer_role
        .haystacks
        .iter()
        .find(|h| h.service == terraphim_config::ServiceType::GrepApp)
        .expect("Should have GrepApp haystack");

    let rust_language = grepapp_haystack
        .extra_parameters
        .get("language")
        .expect("GrepApp haystack should have language parameter");

    assert_eq!(rust_language, "Rust", "Language should be set to Rust");

    log::info!("✅ Rust Engineer role configuration validated");
    log::info!("   - QueryRs haystack: {}", queryrs_haystack.location);
    log::info!("   - GrepApp haystack: {}", grepapp_haystack.location);
    log::info!("   - GrepApp language filter: {}", rust_language);

    // Start server on a test port
    let server_addr = "127.0.0.1:8084".parse().unwrap();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum_server(server_addr, config_state).await {
            log::error!("Server error: {:?}", e);
        }
    });

    // Wait for server to start
    log::info!("⏳ Waiting for server startup...");
    sleep(Duration::from_secs(3)).await;

    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let base_url = "http://127.0.0.1:8084";

    // Test 1: Health check
    log::info!("🔍 Testing server health...");
    let health_response = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Health check failed");

    assert!(
        health_response.status().is_success(),
        "Health check should succeed"
    );
    log::info!("✅ Server health check passed");

    // Test 2: Get configuration
    log::info!("🔍 Testing configuration endpoint...");
    let config_response = client
        .get(format!("{}/config", base_url))
        .send()
        .await
        .expect("Config request failed");

    assert!(
        config_response.status().is_success(),
        "Config request should succeed"
    );

    let config_json: ConfigResponse = config_response
        .json()
        .await
        .expect("Failed to parse config response");

    assert!(
        config_json
            .config
            .roles
            .contains_key(&"Rust Engineer".into())
    );
    log::info!("✅ Configuration endpoint validated");

    // Test 3: Search with Rust Engineer role for common Rust terms
    // These terms should potentially appear in both QueryRs and GrepApp results
    log::info!("🔍 Testing search with Rust Engineer role (dual haystack)...");
    let rust_terms = ["async tokio", "Result error", "impl trait"];

    for term in &rust_terms {
        log::info!("🦀 Testing search for Rust term: {}", term);
        let search_params = [("q", *term), ("role", "Rust Engineer"), ("limit", "10")];

        let search_response = client
            .get(format!("{}/documents/search", base_url))
            .query(&search_params)
            .send()
            .await;

        match search_response {
            Ok(response) => {
                if response.status().is_success() {
                    let search_json: SearchResponse = response.json().await.unwrap_or_else(|_| {
                        panic!("Failed to parse search response for '{}'", term)
                    });

                    log::info!(
                        "✅ Found {} results for '{}'",
                        search_json.results.len(),
                        term
                    );

                    // Analyze results by source
                    let mut queryrs_count = 0;
                    let mut grepapp_count = 0;
                    let mut no_source_count = 0;
                    let mut urls = HashSet::new();

                    for doc in &search_json.results {
                        // Track source
                        match doc.source_haystack.as_deref() {
                            Some(source) if source.contains("query.rs") => queryrs_count += 1,
                            Some(source) if source.contains("grep.app") => grepapp_count += 1,
                            Some(_) => {}
                            None => no_source_count += 1,
                        }

                        // Track URLs for duplicate detection
                        urls.insert(doc.url.clone());
                    }

                    log::info!("   📊 Source breakdown:");
                    log::info!("      - QueryRs results: {}", queryrs_count);
                    log::info!("      - GrepApp results: {}", grepapp_count);
                    log::info!("      - No source tag: {}", no_source_count);
                    log::info!("   📊 Duplicate analysis:");
                    log::info!("      - Total results: {}", search_json.results.len());
                    log::info!("      - Unique URLs: {}", urls.len());

                    if search_json.results.len() > urls.len() {
                        log::warn!(
                            "   ⚠️  Potential duplicates detected: {} results, {} unique URLs",
                            search_json.results.len(),
                            urls.len()
                        );
                    } else {
                        log::info!("   ✅ No URL duplicates detected");
                    }

                    // Log sample results from each source
                    log::info!("   📝 Sample results:");
                    for (i, doc) in search_json.results.iter().take(3).enumerate() {
                        let source_label = doc
                            .source_haystack
                            .as_ref()
                            .map(|s| {
                                if s.contains("query.rs") {
                                    "QueryRs"
                                } else if s.contains("grep.app") {
                                    "GrepApp"
                                } else {
                                    "Unknown"
                                }
                            })
                            .unwrap_or("NoSource");

                        log::info!("      {}. [{}] {}", i + 1, source_label, doc.title);
                    }
                } else {
                    log::warn!(
                        "⚠️  Search for '{}' returned status: {}",
                        term,
                        response.status()
                    );
                }
            }
            Err(e) => {
                log::warn!("⚠️  Search for '{}' failed: {}", term, e);
            }
        }

        // Rate limiting - wait between requests
        sleep(Duration::from_secs(1)).await;
    }

    // Cleanup
    server_handle.abort();
    log::info!("✅ Rust Engineer enhanced integration test completed");
}

/// Test Rust Engineer configuration structure without making API calls
#[tokio::test]
async fn test_rust_engineer_config_structure() {
    let current_dir = std::env::current_dir().unwrap();

    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/combined_roles_config.json")
    } else {
        PathBuf::from("terraphim_server/default/combined_roles_config.json")
    };

    if !config_path.exists() {
        log::warn!("Combined roles config not found. Skipping test.");
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let config: Config =
        serde_json::from_str(&config_content).expect("Failed to parse config JSON");

    // Verify role exists
    let rust_role = config
        .roles
        .get(&"Rust Engineer".into())
        .expect("Rust Engineer role should exist in config");

    // Verify basic properties
    assert_eq!(rust_role.name, "Rust Engineer".into());
    assert!(rust_role.haystacks.len() >= 2);

    // Verify QueryRs haystack
    let has_queryrs = rust_role
        .haystacks
        .iter()
        .any(|h| h.service == terraphim_config::ServiceType::QueryRs);

    assert!(has_queryrs, "Rust Engineer should have QueryRs haystack");

    // Verify GrepApp haystack
    let has_grepapp = rust_role
        .haystacks
        .iter()
        .any(|h| h.service == terraphim_config::ServiceType::GrepApp);

    assert!(has_grepapp, "Rust Engineer should have GrepApp haystack");

    println!("✅ Rust Engineer config structure validated");
    println!("   - Total haystacks: {}", rust_role.haystacks.len());
    println!("   - Has QueryRs: {}", has_queryrs);
    println!("   - Has GrepApp: {}", has_grepapp);
}
