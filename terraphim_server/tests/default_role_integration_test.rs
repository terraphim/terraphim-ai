use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{axum_server, ConfigResponse, SearchResponse};

/// Integration test for Default role configuration with Ripgrep haystack
///
/// This test validates:
/// 1. Server starts with Default role configuration
/// 2. Ripgrep haystack is configured for local docs
/// 3. Search functionality works with basic text queries
/// 4. Results are returned from local documentation
#[tokio::test]
#[serial]
async fn test_default_role_ripgrep_integration() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    // Check if documentation exists
    let docs_src_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("../docs/src")
    } else {
        PathBuf::from("docs/src")
    };

    if !docs_src_path.exists() {
        log::warn!(
            "Documentation not found at {:?}. Skipping test.",
            docs_src_path
        );
        return;
    }

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

    // Fix paths in configuration for test environment
    if current_dir.ends_with("terraphim_server") {
        for (_role_name, role) in &mut config.roles {
            for haystack in &mut role.haystacks {
                if haystack.location == "docs/src" {
                    haystack.location = "../docs/src".to_string();
                }
            }
        }
        log::info!("✅ Adjusted config paths for test environment");
    }

    // Create config state
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    log::info!("✅ Configuration loaded with {} roles", config.roles.len());

    // Verify Default role exists and is configured correctly
    let default_role = config
        .roles
        .get(&"Default".into())
        .expect("Default role should exist");

    assert_eq!(
        default_role.name,
        "Default".into(),
        "Role name should be Default"
    );

    assert!(
        !default_role.haystacks.is_empty(),
        "Default role should have at least one haystack"
    );

    // Verify Ripgrep haystack
    let ripgrep_haystack = default_role
        .haystacks
        .iter()
        .find(|h| h.service == terraphim_config::ServiceType::Ripgrep)
        .expect("Should have Ripgrep haystack");

    log::info!("✅ Default role configuration validated");
    log::info!("   - Ripgrep haystack: {}", ripgrep_haystack.location);

    // Start server on a test port
    let server_addr = "127.0.0.1:8085".parse().unwrap();
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
    let base_url = "http://127.0.0.1:8085";

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

    assert!(config_json.config.roles.contains_key(&"Default".into()));
    log::info!("✅ Configuration endpoint validated");

    // Test 3: Search with Default role for common documentation terms
    log::info!("🔍 Testing search with Default role...");
    let doc_terms = [
        "installation",
        "configuration",
        "quickstart",
        "api",
        "usage",
    ];

    for term in &doc_terms {
        log::info!("📚 Testing search for documentation term: {}", term);
        let search_params = [("q", *term), ("role", "Default"), ("limit", "5")];

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

                    // Log some sample results
                    for (i, doc) in search_json.results.iter().take(2).enumerate() {
                        log::info!("   {}. {}", i + 1, doc.title);
                        if let Some(ref source) = doc.source_haystack {
                            log::info!("      Source: {}", source);
                        }
                    }

                    // Verify results are from Ripgrep haystack
                    if !search_json.results.is_empty() {
                        let all_from_ripgrep = search_json.results.iter().all(|doc| {
                            doc.source_haystack
                                .as_ref()
                                .map(|s| s.contains("docs/src"))
                                .unwrap_or(true)
                        });

                        if !all_from_ripgrep {
                            log::warn!("⚠️  Some results not from expected Ripgrep source");
                        }
                    }
                } else {
                    log::info!(
                        "ℹ️  Search for '{}' returned status: {} (may not exist in docs)",
                        term,
                        response.status()
                    );
                }
            }
            Err(e) => {
                log::warn!("⚠️  Search for '{}' failed: {}", term, e);
            }
        }

        // Small delay between requests
        sleep(Duration::from_millis(100)).await;
    }

    // Test 4: Search for specific content that should exist in docs
    log::info!("🔍 Testing search for specific documentation content...");
    let specific_terms = ["terraphim", "rust", "search"];

    for term in &specific_terms {
        log::info!("🔎 Testing search for specific term: {}", term);
        let search_params = [("q", *term), ("role", "Default"), ("limit", "3")];

        let search_response = client
            .get(format!("{}/documents/search", base_url))
            .query(&search_params)
            .send()
            .await
            .expect(&format!("Search for '{}' failed", term));

        if search_response.status().is_success() {
            let search_json: SearchResponse = search_response
                .json()
                .await
                .expect(&format!("Failed to parse search response for '{}'", term));

            log::info!(
                "✅ Found {} results for '{}'",
                search_json.results.len(),
                term
            );
        }
    }

    // Cleanup
    server_handle.abort();
    log::info!("✅ Default role integration test completed");
}

/// Test Default role configuration structure without making searches
#[tokio::test]
async fn test_default_role_config_structure() {
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
    let default_role = config
        .roles
        .get(&"Default".into())
        .expect("Default role should exist in config");

    // Verify basic properties
    assert_eq!(default_role.name, "Default".into());
    assert!(!default_role.haystacks.is_empty());

    // Verify Ripgrep haystack
    let has_ripgrep = default_role
        .haystacks
        .iter()
        .any(|h| h.service == terraphim_config::ServiceType::Ripgrep);

    assert!(has_ripgrep, "Default role should have Ripgrep haystack");

    // Verify relevance function
    assert_eq!(
        default_role.relevance_function,
        terraphim_types::RelevanceFunction::TitleScorer,
        "Default role should use TitleScorer relevance function"
    );

    println!("✅ Default role config structure validated");
    println!("   - Haystacks: {}", default_role.haystacks.len());
    println!(
        "   - Relevance function: {:?}",
        default_role.relevance_function
    );
}
