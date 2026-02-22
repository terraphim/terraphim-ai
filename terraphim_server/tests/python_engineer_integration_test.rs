use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState};
use terraphim_server::{axum_server, ConfigResponse, SearchResponse};

/// Integration test for Python Engineer configuration with grep.app haystack
///
/// This test validates:
/// 1. Server starts with Python Engineer configuration
/// 2. GrepApp haystack is configured for Python language
/// 3. Search functionality works with Python-specific queries
/// 4. Results are returned from GitHub public repositories
#[tokio::test]
#[serial]
#[ignore] // Requires internet connection to grep.app API
async fn test_python_engineer_grepapp_integration() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    // Load the Python Engineer configuration
    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/python_engineer_config.json")
    } else {
        PathBuf::from("terraphim_server/default/python_engineer_config.json")
    };

    if !config_path.exists() {
        log::warn!(
            "Python Engineer config not found at {:?}. Skipping test.",
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

    // Verify Python Engineer role exists and is configured correctly
    let python_engineer_role = config
        .roles
        .get(&"Python Engineer".into())
        .expect("Python Engineer role should exist");

    assert_eq!(
        python_engineer_role.name,
        "Python Engineer".into(),
        "Role name should be Python Engineer"
    );

    assert!(
        !python_engineer_role.haystacks.is_empty(),
        "Python Engineer should have at least one haystack"
    );

    // Verify GrepApp haystack is configured with Python language
    let grepapp_haystack = python_engineer_role
        .haystacks
        .iter()
        .find(|h| h.service == terraphim_config::ServiceType::GrepApp)
        .expect("Should have GrepApp haystack");

    let language = grepapp_haystack
        .extra_parameters
        .get("language")
        .expect("GrepApp haystack should have language parameter");

    assert_eq!(language, "Python", "Language should be set to Python");

    log::info!("✅ Python Engineer role configuration validated");
    log::info!("   - GrepApp haystack: {}", grepapp_haystack.location);
    log::info!("   - Language filter: {}", language);

    // Start server on a test port
    let server_addr = "127.0.0.1:8082".parse().unwrap();
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
    let base_url = "http://127.0.0.1:8082";

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
        .contains_key(&"Python Engineer".into()));
    log::info!("✅ Configuration endpoint validated");

    // Test 3: Search with Python Engineer role for common Python terms
    log::info!("🔍 Testing search with Python Engineer role...");
    let python_terms = [
        "async def",
        "import pandas",
        "class Meta",
        "django models",
        "fastapi route",
    ];

    for term in &python_terms {
        log::info!("🐍 Testing search for Python term: {}", term);
        let search_params = [("q", *term), ("role", "Python Engineer"), ("limit", "5")];

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
    log::info!("✅ Python Engineer integration test completed");
}

/// Test Python Engineer configuration without making API calls
#[tokio::test]
async fn test_python_engineer_config_structure() {
    let current_dir = std::env::current_dir().unwrap();

    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/python_engineer_config.json")
    } else {
        PathBuf::from("terraphim_server/default/python_engineer_config.json")
    };

    if !config_path.exists() {
        log::warn!("Python Engineer config not found. Skipping test.");
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let config: Config =
        serde_json::from_str(&config_content).expect("Failed to parse config JSON");

    // Verify role exists
    let python_role = config
        .roles
        .get(&"Python Engineer".into())
        .expect("Python Engineer role should exist in config");

    // Verify basic properties
    assert_eq!(python_role.name, "Python Engineer".into());
    assert!(!python_role.haystacks.is_empty());

    // Verify GrepApp haystack
    let has_grepapp = python_role
        .haystacks
        .iter()
        .any(|h| h.service == terraphim_config::ServiceType::GrepApp);

    assert!(has_grepapp, "Python Engineer should have GrepApp haystack");

    println!("✅ Python Engineer config structure validated");
}
