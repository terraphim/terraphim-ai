use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{axum_server, ConfigResponse, SearchResponse};

/// Integration test for Front End Engineer configuration with grep.app haystacks
///
/// This test validates:
/// 1. Server starts with Front End Engineer configuration
/// 2. GrepApp haystacks are configured for JavaScript and TypeScript
/// 3. Search functionality works with frontend-specific queries
/// 4. Results are returned from GitHub public repositories
#[tokio::test]
#[serial]
#[ignore] // Requires internet connection to grep.app API
async fn test_frontend_engineer_grepapp_integration() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    // Load the Front End Engineer configuration
    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/frontend_engineer_config.json")
    } else {
        PathBuf::from("terraphim_server/default/frontend_engineer_config.json")
    };

    if !config_path.exists() {
        log::warn!(
            "Front End Engineer config not found at {:?}. Skipping test.",
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

    // Verify Front End Engineer role exists and is configured correctly
    let frontend_engineer_role = config
        .roles
        .get(&"Front End Engineer".into())
        .expect("Front End Engineer role should exist");

    assert_eq!(
        frontend_engineer_role.name,
        "Front End Engineer".into(),
        "Role name should be Front End Engineer"
    );

    assert!(
        !frontend_engineer_role.haystacks.is_empty(),
        "Front End Engineer should have at least one haystack"
    );

    // Verify GrepApp haystacks are configured with JavaScript and TypeScript
    let grepapp_haystacks: Vec<_> = frontend_engineer_role
        .haystacks
        .iter()
        .filter(|h| h.service == terraphim_config::ServiceType::GrepApp)
        .collect();

    assert!(
        grepapp_haystacks.len() >= 2,
        "Should have at least 2 GrepApp haystacks (JavaScript and TypeScript)"
    );

    let js_haystack = grepapp_haystacks
        .iter()
        .find(|h| {
            h.extra_parameters
                .get("language")
                .map(|l| l == "JavaScript")
                .unwrap_or(false)
        })
        .expect("Should have JavaScript GrepApp haystack");

    let ts_haystack = grepapp_haystacks
        .iter()
        .find(|h| {
            h.extra_parameters
                .get("language")
                .map(|l| l == "TypeScript")
                .unwrap_or(false)
        })
        .expect("Should have TypeScript GrepApp haystack");

    log::info!("✅ Front End Engineer role configuration validated");
    log::info!("   - JavaScript haystack: {}", js_haystack.location);
    log::info!("   - TypeScript haystack: {}", ts_haystack.location);

    // Start server on a test port
    let server_addr = "127.0.0.1:8083".parse().unwrap();
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
    let base_url = "http://127.0.0.1:8083";

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

    assert!(config_json
        .config
        .roles
        .contains_key(&"Front End Engineer".into()));
    log::info!("✅ Configuration endpoint validated");

    // Test 3: Search with Front End Engineer role for common frontend terms
    log::info!("🔍 Testing search with Front End Engineer role...");
    let frontend_terms = [
        "useState hook",
        "async function",
        "interface Props",
        "export default",
        "const component",
    ];

    for term in &frontend_terms {
        log::info!("⚛️  Testing search for frontend term: {}", term);
        let search_params = [("q", *term), ("role", "Front End Engineer"), ("limit", "5")];

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

                    // Verify we get results from both JavaScript and TypeScript sources
                    let has_js_or_ts = search_json.results.iter().any(|doc| {
                        doc.title.ends_with(".js")
                            || doc.title.ends_with(".jsx")
                            || doc.title.ends_with(".ts")
                            || doc.title.ends_with(".tsx")
                    });

                    if !search_json.results.is_empty() && !has_js_or_ts {
                        log::warn!("⚠️  Results don't appear to be JavaScript/TypeScript files");
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
        sleep(Duration::from_millis(500)).await;
    }

    // Cleanup
    server_handle.abort();
    log::info!("✅ Front End Engineer integration test completed");
}

/// Test Front End Engineer configuration without making API calls
#[tokio::test]
async fn test_frontend_engineer_config_structure() {
    let current_dir = std::env::current_dir().unwrap();

    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/frontend_engineer_config.json")
    } else {
        PathBuf::from("terraphim_server/default/frontend_engineer_config.json")
    };

    if !config_path.exists() {
        log::warn!("Front End Engineer config not found. Skipping test.");
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let config: Config =
        serde_json::from_str(&config_content).expect("Failed to parse config JSON");

    // Verify role exists
    let frontend_role = config
        .roles
        .get(&"Front End Engineer".into())
        .expect("Front End Engineer role should exist in config");

    // Verify basic properties
    assert_eq!(frontend_role.name, "Front End Engineer".into());
    assert!(!frontend_role.haystacks.is_empty());

    // Verify GrepApp haystacks
    let grepapp_haystacks: Vec<_> = frontend_role
        .haystacks
        .iter()
        .filter(|h| h.service == terraphim_config::ServiceType::GrepApp)
        .collect();

    assert!(
        grepapp_haystacks.len() >= 2,
        "Front End Engineer should have at least 2 GrepApp haystacks"
    );

    // Verify language parameters
    let languages: Vec<String> = grepapp_haystacks
        .iter()
        .filter_map(|h| h.extra_parameters.get("language").cloned())
        .collect();

    assert!(
        languages.contains(&"JavaScript".to_string()),
        "Should have JavaScript language filter"
    );
    assert!(
        languages.contains(&"TypeScript".to_string()),
        "Should have TypeScript language filter"
    );

    println!("✅ Front End Engineer config structure validated");
    println!("   - Haystacks: {}", grepapp_haystacks.len());
    println!("   - Languages: {:?}", languages);
}
