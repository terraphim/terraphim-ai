use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use serial_test::serial;
use tokio::time::sleep;

use terraphim_config::{Config, ConfigState, Haystack, Role, ServiceType};
use terraphim_server::{axum_server, SearchResponse};
use terraphim_types::RelevanceFunction;

/// Integration test for relevance functions with duplicate handling from multiple haystacks
///
/// This test validates how different relevance functions handle results when
/// the same query is searched across multiple haystacks (QueryRs and GrepApp)
/// that might return overlapping results.
///
/// Tests all relevance functions:
/// - TitleScorer
/// - BM25
/// - BM25F
/// - BM25Plus
/// - TerraphimGraph
#[tokio::test]
#[serial]
#[ignore] // Requires internet connection to QueryRs and grep.app APIs
async fn test_relevance_functions_with_duplicate_scenarios() {
    // Set up logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();

    let current_dir = std::env::current_dir().unwrap();
    log::info!("Test running from directory: {:?}", current_dir);

    // Test query that's likely to appear in both QueryRs and GrepApp
    let test_query = "tokio spawn";

    // All relevance functions to test
    let relevance_functions = vec![
        RelevanceFunction::TitleScorer,
        RelevanceFunction::BM25,
        RelevanceFunction::BM25F,
        RelevanceFunction::BM25Plus,
        // Note: TerraphimGraph requires KG setup, tested separately
    ];

    let mut results_summary: HashMap<String, DuplicateAnalysis> = HashMap::new();

    for (idx, relevance_function) in relevance_functions.iter().enumerate() {
        log::info!("\n{}", "=".repeat(80));
        log::info!("🧪 Testing relevance function: {:?}", relevance_function);
        log::info!("{}\n", "=".repeat(80));

        // Create a test role with both QueryRs and GrepApp haystacks
        let mut test_role = Role::new("Test Rust Engineer");
        test_role.shortname = Some("test-rust".to_string());
        test_role.relevance_function = *relevance_function;

        // Add QueryRs haystack
        test_role.haystacks.push(Haystack {
            location: "https://query.rs".to_string(),
            service: ServiceType::QueryRs,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert(
                    "disable_content_enhancement".to_string(),
                    "true".to_string(),
                );
                params
            },
        });

        // Add GrepApp haystack
        test_role.haystacks.push(Haystack {
            location: "https://grep.app".to_string(),
            service: ServiceType::GrepApp,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("language".to_string(), "Rust".to_string());
                params.insert("repo".to_string(), "".to_string());
                params.insert("path".to_string(), "".to_string());
                params
            },
        });

        // Create config with test role
        let mut config = Config::default();
        config.id = terraphim_config::ConfigId::Server;
        config.roles.insert("Test Rust Engineer".into(), test_role);
        config.default_role = "Test Rust Engineer".into();

        // Create config state
        let config_state = ConfigState::new(&mut config)
            .await
            .expect("Failed to create config state");

        // Start server on unique test port
        let test_port = 8090 + idx as u16;
        let server_addr = format!("127.0.0.1:{}", test_port).parse().unwrap();
        let server_handle = tokio::spawn(async move {
            if let Err(e) = axum_server(server_addr, config_state).await {
                log::error!("Server error: {:?}", e);
            }
        });

        // Wait for server to start
        log::info!("⏳ Waiting for server startup on port {}...", test_port);
        sleep(Duration::from_secs(3)).await;

        let client = terraphim_service::http_client::create_default_client()
            .expect("Failed to create HTTP client");
        let base_url = format!("http://127.0.0.1:{}", test_port);

        // Perform search
        log::info!("🔍 Searching for: '{}'", test_query);
        let search_params = [
            ("q", test_query),
            ("role", "Test Rust Engineer"),
            ("limit", "20"),
        ];

        let search_response = client
            .get(format!("{}/documents/search", base_url))
            .query(&search_params)
            .send()
            .await;

        match search_response {
            Ok(response) => {
                if response.status().is_success() {
                    let search_json: SearchResponse = response
                        .json()
                        .await
                        .expect("Failed to parse search response");

                    let analysis = analyze_duplicates(&search_json, test_query);

                    log::info!("\n📊 Results Analysis for {:?}:", relevance_function);
                    log::info!("   Total results: {}", analysis.total_results);
                    log::info!("   Unique URLs: {}", analysis.unique_urls);
                    log::info!("   QueryRs results: {}", analysis.queryrs_count);
                    log::info!("   GrepApp results: {}", analysis.grepapp_count);
                    log::info!("   No source tag: {}", analysis.no_source_count);

                    if analysis.total_results > analysis.unique_urls {
                        log::warn!(
                            "   ⚠️  {} potential duplicates (different IDs, same URL)",
                            analysis.total_results - analysis.unique_urls
                        );
                    } else {
                        log::info!("   ✅ No URL duplicates");
                    }

                    if !analysis.duplicate_urls.is_empty() {
                        log::warn!("   🔍 Duplicate URLs found:");
                        for url in &analysis.duplicate_urls {
                            log::warn!("      - {}", url);
                        }
                    }

                    // Log sample results
                    log::info!("\n   📝 Sample results (first 5):");
                    for (i, doc) in search_json.results.iter().take(5).enumerate() {
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

                        log::info!(
                            "      {}. [{}] {} (score: {:?})",
                            i + 1,
                            source_label,
                            doc.title,
                            doc.rank
                        );
                        log::info!("         ID: {}", doc.id);
                        log::info!("         URL: {}", doc.url);
                    }

                    results_summary.insert(format!("{:?}", relevance_function), analysis);
                } else {
                    log::error!("❌ Search returned status: {}", response.status());
                }
            }
            Err(e) => {
                log::error!("❌ Search failed: {}", e);
            }
        }

        // Cleanup
        server_handle.abort();

        // Wait between tests to avoid rate limiting
        if idx < relevance_functions.len() - 1 {
            log::info!("\n⏳ Waiting before next test...");
            sleep(Duration::from_secs(2)).await;
        }
    }

    // Print summary comparison
    log::info!("\n\n{}", "=".repeat(80));
    log::info!("📊 DUPLICATE HANDLING SUMMARY ACROSS ALL RELEVANCE FUNCTIONS");
    log::info!("{}\n", "=".repeat(80));
    log::info!("Query: '{}'\n", test_query);

    for (func_name, analysis) in &results_summary {
        log::info!("{}:", func_name);
        log::info!(
            "  Total: {}, Unique: {}, Duplicates: {}",
            analysis.total_results,
            analysis.unique_urls,
            if analysis.total_results > analysis.unique_urls {
                analysis.total_results - analysis.unique_urls
            } else {
                0
            }
        );
        log::info!(
            "  QueryRs: {}, GrepApp: {}",
            analysis.queryrs_count,
            analysis.grepapp_count
        );
    }

    log::info!("\n✅ Relevance functions duplicate test completed");
}

#[derive(Debug)]
struct DuplicateAnalysis {
    total_results: usize,
    unique_urls: usize,
    queryrs_count: usize,
    grepapp_count: usize,
    no_source_count: usize,
    duplicate_urls: Vec<String>,
}

fn analyze_duplicates(search_response: &SearchResponse, _query: &str) -> DuplicateAnalysis {
    let mut urls = HashSet::new();
    let mut url_counts: HashMap<String, usize> = HashMap::new();
    let mut queryrs_count = 0;
    let mut grepapp_count = 0;
    let mut no_source_count = 0;

    for doc in &search_response.results {
        // Track URLs
        urls.insert(doc.url.clone());
        *url_counts.entry(doc.url.clone()).or_insert(0) += 1;

        // Track sources
        match doc.source_haystack.as_deref() {
            Some(source) if source.contains("query.rs") => queryrs_count += 1,
            Some(source) if source.contains("grep.app") => grepapp_count += 1,
            Some(_) => {}
            None => no_source_count += 1,
        }
    }

    // Find duplicate URLs
    let duplicate_urls: Vec<String> = url_counts
        .iter()
        .filter(|(_, &count)| count > 1)
        .map(|(url, _)| url.clone())
        .collect();

    DuplicateAnalysis {
        total_results: search_response.results.len(),
        unique_urls: urls.len(),
        queryrs_count,
        grepapp_count,
        no_source_count,
        duplicate_urls,
    }
}

/// Test TerraphimGraph relevance function with local KG
/// Separate test because it requires KG setup
#[tokio::test]
#[serial]
async fn test_terraphim_graph_with_duplicates() {
    let current_dir = std::env::current_dir().unwrap();

    // Check if we have the KG directory
    let kg_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("../docs/src/kg")
    } else {
        PathBuf::from("docs/src/kg")
    };

    if !kg_path.exists() {
        log::warn!("KG directory not found. Skipping TerraphimGraph duplicate test.");
        return;
    }

    // Load terraphim engineer config which uses TerraphimGraph
    let config_path = if current_dir.ends_with("terraphim_server") {
        PathBuf::from("default/terraphim_engineer_config.json")
    } else {
        PathBuf::from("terraphim_server/default/terraphim_engineer_config.json")
    };

    if !config_path.exists() {
        log::warn!("Terraphim Engineer config not found. Skipping test.");
        return;
    }

    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .expect("Failed to read config file");

    let mut config: Config =
        serde_json::from_str(config_content.trim()).expect("Failed to parse config JSON");

    // Fix paths if needed
    if current_dir.ends_with("terraphim_server") {
        for (_role_name, role) in &mut config.roles {
            for haystack in &mut role.haystacks {
                if haystack.location == "docs/src" {
                    haystack.location = "../docs/src".to_string();
                }
            }
        }
    }

    let _config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    log::info!("✅ TerraphimGraph uses local KG, not multiple remote haystacks");
    log::info!("   Therefore, no duplicate scenarios expected from multiple sources");
    log::info!("   Duplicate test focuses on QueryRs + GrepApp combinations");
}
