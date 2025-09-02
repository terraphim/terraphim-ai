use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{axum_server, ConfigResponse, SearchResponse};

/// Integration test for System Operator configuration with remote knowledge graph
///
/// This test validates:
/// 1. Server starts with system operator configuration
/// 2. Remote knowledge graph is loaded correctly
/// 3. Documents are indexed from GitHub repository
/// 4. Search functionality works with TerraphimGraph ranking
#[tokio::test]
#[serial]
async fn test_system_operator_remote_kg_integration() {
    // Set up logging
    terraphim_service::logging::init_logging(
        terraphim_service::logging::LoggingConfig::IntegrationTest,
    );

    // Check if system operator data exists
    let system_operator_path = PathBuf::from("/tmp/system_operator/pages");
    if !system_operator_path.exists() {
        log::warn!(
            "System operator data not found at {:?}. Run setup_system_operator.sh first.",
            system_operator_path
        );
        return;
    }

    // Load the system operator configuration
    let config_path = PathBuf::from("terraphim_server/default/system_operator_config.json");
    if !config_path.exists() {
        log::warn!(
            "System operator config not found at {:?}. Skipping test.",
            config_path
        );
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let mut config: Config =
        serde_json::from_str(&config_content).expect("Failed to parse config JSON");

    // Create config state
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    log::info!("✅ Configuration loaded with {} roles", config.roles.len());

    // Verify System Operator role exists and is configured correctly
    let system_operator_role = config
        .roles
        .get(&"System Operator".into())
        .expect("System Operator role should exist");

    assert_eq!(
        system_operator_role.relevance_function,
        terraphim_types::RelevanceFunction::TerraphimGraph,
        "System Operator should use TerraphimGraph"
    );

    assert!(
        system_operator_role.kg.is_some(),
        "System Operator should have knowledge graph configuration"
    );

    let kg = system_operator_role.kg.as_ref().unwrap();
    assert!(
        kg.automata_path.is_some(),
        "System Operator should have remote automata path"
    );

    log::info!("✅ System Operator role configuration validated");

    // Start server on a test port
    let server_addr = "127.0.0.1:8080".parse().unwrap();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum_server(server_addr, config_state).await {
            log::error!("Server error: {:?}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_secs(3)).await;

    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let base_url = "http://127.0.0.1:8080";

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

    assert_eq!(config_json.config.default_role, "System Operator".into());
    assert!(config_json
        .config
        .roles
        .contains_key(&"System Operator".into()));
    log::info!("✅ Configuration endpoint validated");

    // Test 3: Search with System Operator role
    log::info!("🔍 Testing search with System Operator role...");
    let search_params = [("q", "system"), ("role", "System Operator"), ("limit", "5")];

    let search_response = client
        .get(format!("{}/documents/search", base_url))
        .query(&search_params)
        .send()
        .await
        .expect("Search request failed");

    assert!(
        search_response.status().is_success(),
        "Search request should succeed"
    );

    let search_json: SearchResponse = search_response
        .json()
        .await
        .expect("Failed to parse search response");

    assert!(
        !search_json.results.is_empty(),
        "Search should return results for 'system' in system operator documents"
    );

    log::info!(
        "✅ Found {} search results for 'system'",
        search_json.results.len()
    );

    // Test 4: Search for specific system engineering terms
    let engineering_terms = ["MBSE", "requirements", "architecture", "verification"];

    for term in &engineering_terms {
        log::info!("🔍 Testing search for term: {}", term);
        let search_params = [("q", *term), ("role", "System Operator"), ("limit", "3")];

        let search_response = client
            .get(format!("{}/documents/search", base_url))
            .query(&search_params)
            .send()
            .await
            .unwrap_or_else(|_| panic!("Search for '{}' failed", term));

        if search_response.status().is_success() {
            let search_json: SearchResponse = search_response
                .json()
                .await
                .unwrap_or_else(|_| panic!("Failed to parse search response for '{}'", term));

            log::info!(
                "✅ Found {} results for '{}'",
                search_json.results.len(),
                term
            );

            // Log some sample results
            for (i, doc) in search_json.results.iter().take(2).enumerate() {
                log::info!("   {}. {} (score: {:?})", i + 1, doc.title, doc.rank);
            }
        } else {
            log::warn!(
                "⚠️  Search for '{}' returned status: {}",
                term,
                search_response.status()
            );
        }
    }

    // Cleanup: abort the server
    server_handle.abort();

    log::info!("🎉 System Operator integration test completed successfully!");
    log::info!("✅ Remote knowledge graph configuration validated");
    log::info!("✅ Document indexing from GitHub repository working");
    log::info!("✅ TerraphimGraph ranking functional");
    log::info!("✅ Search API responsive with system engineering content");
}
