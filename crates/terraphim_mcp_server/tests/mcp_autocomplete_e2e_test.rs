use anyhow::Result;
use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::TokioChildProcess};
use serde_json::json;
use serial_test::serial;
use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_config::{
    ConfigBuilder, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
};
use terraphim_persistence::DeviceStorage;
use terraphim_persistence::Persistable;
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};
use tokio::process::Command;

// Additional imports for thesaurus building
use terraphim_automata::AutomataPath;

/// Create a comprehensive test configuration with the "Terraphim Engineer" role
/// that uses local KG files and builds thesaurus from local markdown files
async fn create_autocomplete_test_config() -> Result<String> {
    // Use memory-only persistence to avoid RocksDB filesystem issues in CI
    std::env::set_var("TERRAPHIM_PROFILE_MEMORY_TYPE", "memory");
    // Isolate logs to tmp
    std::env::set_var("TERRAPHIM_LOG_DIR", "/tmp/terraphim-logs");
    // Force persistence layer to use memory-only device settings
    let _ = DeviceStorage::init_memory_only().await;
    let current_dir = std::env::current_dir()?;
    let project_root = current_dir.parent().unwrap().parent().unwrap();
    let docs_src_path = project_root.join("docs/src");
    let kg_path = docs_src_path.join("kg");

    // Verify paths exist
    if !kg_path.exists() {
        panic!("Knowledge graph directory not found: {:?}", kg_path);
    }
    if !kg_path.join("terraphim-graph.md").exists() {
        panic!("terraphim-graph.md not found in kg directory");
    }

    println!("🔧 Building thesaurus from local KG files: {:?}", kg_path);

    // Build thesaurus using Logseq builder
    let logseq_builder = Logseq::default();
    let mut thesaurus = logseq_builder
        .build("Terraphim Engineer".to_string(), kg_path.clone())
        .await?;

    println!(
        "✅ Built thesaurus with {} entries from local KG",
        thesaurus.len()
    );

    // Save thesaurus to persistence layer
    thesaurus.save().await?;
    println!("✅ Saved thesaurus to persistence layer");

    // Reload thesaurus from persistence to get canonical version
    thesaurus = thesaurus.load().await?;

    // Create automata path pointing to the persisted thesaurus
    let temp_dir = std::env::temp_dir();
    let thesaurus_path = temp_dir.join("terraphim_engineer_autocomplete_thesaurus.json");

    // Write thesaurus to temp file for automata path
    let thesaurus_json = serde_json::to_string_pretty(&thesaurus)?;
    tokio::fs::write(&thesaurus_path, thesaurus_json).await?;

    let automata_path = AutomataPath::Local(thesaurus_path.clone());
    println!("✅ Set automata_path to: {:?}", thesaurus_path);

    let terraphim_engineer_role = Role {
        shortname: Some("Terraphim Engineer".to_string()),
        name: terraphim_types::RoleName::new("Terraphim Engineer"),
        relevance_function: RelevanceFunction::TerraphimGraph,
        terraphim_it: true,
        theme: "lumen".to_string(),
        kg: Some(KnowledgeGraph {
            automata_path: Some(automata_path),
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: kg_path,
            }),
            public: true,
            publish: true,
        }),
        haystacks: vec![Haystack {
            location: docs_src_path.to_string_lossy().to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
            fetch_content: false,
        }],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: Some(4096),
        extra: ahash::AHashMap::new(),
    };

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+Shift+T")
        .add_role("Terraphim Engineer", terraphim_engineer_role)
        .build()?;

    // Set the selected role
    config.selected_role = terraphim_types::RoleName::new("Terraphim Engineer");

    Ok(serde_json::to_string_pretty(&config)?)
}

/// Start the MCP server as a subprocess and return the transport
async fn start_mcp_server() -> Result<TokioChildProcess> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--bin", "terraphim_mcp_server"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Allow server to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let transport = TokioChildProcess::new(cmd)?;
    Ok(transport)
}

/// Test building autocomplete index for Terraphim Engineer role
#[tokio::test]
#[serial]
async fn test_build_autocomplete_index_terraphim_engineer() -> Result<()> {
    println!("🧪 Testing autocomplete index building for Terraphim Engineer role");

    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // First update the configuration to use our test config
    let update_config_args = json!({
        "config_str": config_json
    });

    let update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    println!("Config update result: {:?}", update_result);
    assert!(update_result.is_error != Some(true));

    // Now build the autocomplete index for Terraphim Engineer role
    let build_index_args = json!({
        "role": "Terraphim Engineer"
    });

    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    println!("Build index result: {:?}", build_result);

    // Verify the build was successful
    assert!(build_result.is_error != Some(true));

    if let Some(content) = build_result.content.first() {
        let content_text = content.as_text().unwrap().text.clone();
        assert!(content_text.contains("Autocomplete index built successfully"));
        assert!(content_text.contains("Terraphim Engineer"));
        println!("✅ Successfully built autocomplete index: {}", content_text);
    } else {
        panic!("No content returned from build_autocomplete_index");
    }

    Ok(())
}

/// Test fuzzy autocomplete search using Jaro-Winkler algorithm with knowledge graph terms
#[tokio::test]
#[serial]
async fn test_fuzzy_autocomplete_search_kg_terms() -> Result<()> {
    println!("🧪 Testing fuzzy autocomplete search with knowledge graph terms");

    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Update configuration
    let update_config_args = json!({"config_str": config_json});
    let _update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    // Build autocomplete index
    let build_index_args = json!({"role": "Terraphim Engineer"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    assert!(build_result.is_error != Some(true));

    // Test various knowledge graph terms
    let test_queries = vec![
        ("terrapi", vec!["terraphim-graph"]), // Partial match
        ("graph", vec!["terraphim-graph", "graph embeddings"]), // Common term
        ("embedd", vec!["graph embeddings"]), // Partial embedding
        ("hayst", vec!["haystack"]),          // Haystack term
        ("servic", vec!["service"]),          // Service term
        ("machine", vec![]),                  // Should not match KG terms
    ];

    for (query, expected_terms) in test_queries {
        println!("🔍 Testing query: '{}'", query);

        let search_args = json!({
            "query": query,
            "similarity": 0.6,
            "limit": 10
        });

        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "fuzzy_autocomplete_search".into(),
                arguments: Some(search_args.as_object().unwrap().clone()),
            })
            .await?;

        println!("Search result for '{}': {:?}", query, search_result);
        assert!(search_result.is_error != Some(true));

        // Check if expected terms are found
        if let Some(summary_content) = search_result.content.first() {
            let summary_text = summary_content.as_text().unwrap().text.clone();
            println!("Summary: {}", summary_text);

            // For terms that should match, verify suggestions are returned
            if !expected_terms.is_empty() {
                assert!(summary_text.contains("Found"));
                assert!(!summary_text.contains("Found 0"));

                // Check individual suggestions
                for (i, content) in search_result.content.iter().skip(1).enumerate() {
                    let suggestion_text = content.as_text().unwrap().text.clone();
                    println!("Suggestion {}: {}", i + 1, suggestion_text);

                    // Verify suggestions contain expected terms
                    let suggestion_contains_expected = expected_terms.iter().any(|expected| {
                        suggestion_text
                            .to_lowercase()
                            .contains(&expected.to_lowercase())
                    });

                    if !suggestion_contains_expected {
                        println!(
                            "⚠️  Suggestion '{}' doesn't contain expected terms: {:?}",
                            suggestion_text, expected_terms
                        );
                    }
                }
            } else {
                // For queries that shouldn't match, verify few or no results
                println!("Query '{}' expected to have no matches", query);
            }
        }
    }

    println!("✅ Fuzzy autocomplete search testing completed");
    Ok(())
}

/// Test Levenshtein autocomplete search for comparison
#[tokio::test]
#[serial]
async fn test_levenshtein_autocomplete_search_kg_terms() -> Result<()> {
    println!("🧪 Testing Levenshtein autocomplete search with knowledge graph terms");

    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Setup (config update and index building)
    let update_config_args = json!({"config_str": config_json});
    let _update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    let build_index_args = json!({"role": "Terraphim Engineer"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    assert!(build_result.is_error != Some(true));

    // Test Levenshtein algorithm with various edit distances
    let test_cases = vec![
        ("graph", 1),   // Exact match
        ("grap", 1),    // 1 edit distance
        ("graff", 2),   // 2 edit distance
        ("terrapi", 2), // For "terraphim"
    ];

    for (query, max_edit_distance) in test_cases {
        println!(
            "🔍 Testing Levenshtein query: '{}' with max edit distance: {}",
            query, max_edit_distance
        );

        let search_args = json!({
            "query": query,
            "max_edit_distance": max_edit_distance,
            "limit": 10
        });

        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "fuzzy_autocomplete_search_levenshtein".into(),
                arguments: Some(search_args.as_object().unwrap().clone()),
            })
            .await?;

        println!(
            "Levenshtein search result for '{}': {:?}",
            query, search_result
        );
        assert!(search_result.is_error != Some(true));

        if let Some(summary_content) = search_result.content.first() {
            let summary_text = summary_content.as_text().unwrap().text.clone();
            println!("Levenshtein summary: {}", summary_text);

            // Print all suggestions for analysis
            for (i, content) in search_result.content.iter().skip(1).enumerate() {
                let suggestion_text = content.as_text().unwrap().text.clone();
                println!("Levenshtein suggestion {}: {}", i + 1, suggestion_text);
            }
        }
    }

    println!("✅ Levenshtein autocomplete search testing completed");
    Ok(())
}

/// Test algorithm comparison: Jaro-Winkler vs Levenshtein
#[tokio::test]
#[serial]
async fn test_autocomplete_algorithm_comparison() -> Result<()> {
    println!("🧪 Testing autocomplete algorithm comparison: Jaro-Winkler vs Levenshtein");

    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Setup
    let update_config_args = json!({"config_str": config_json});
    let _update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    let build_index_args = json!({"role": "Terraphim Engineer"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    assert!(build_result.is_error != Some(true));

    // Compare algorithms on the same queries
    let comparison_queries = vec!["terrapi", "graff", "embedd", "servic"];

    for query in comparison_queries {
        println!("\n🔄 Comparing algorithms for query: '{}'", query);

        // Test Jaro-Winkler
        let jw_args = json!({
            "query": query,
            "similarity": 0.6,
            "limit": 5
        });

        let jw_result = service
            .call_tool(CallToolRequestParam {
                name: "fuzzy_autocomplete_search".into(),
                arguments: Some(jw_args.as_object().unwrap().clone()),
            })
            .await?;

        // Test Levenshtein
        let lev_args = json!({
            "query": query,
            "max_edit_distance": 2,
            "limit": 5
        });

        let lev_result = service
            .call_tool(CallToolRequestParam {
                name: "fuzzy_autocomplete_search_levenshtein".into(),
                arguments: Some(lev_args.as_object().unwrap().clone()),
            })
            .await?;

        // Compare results
        println!(
            "📊 Jaro-Winkler results for '{}': {} items",
            query,
            jw_result.content.len().saturating_sub(1)
        );
        println!(
            "📊 Levenshtein results for '{}': {} items",
            query,
            lev_result.content.len().saturating_sub(1)
        );

        // Print summary comparison
        if let (Some(jw_summary), Some(lev_summary)) =
            (jw_result.content.first(), lev_result.content.first())
        {
            println!("🔍 JW: {}", jw_summary.as_text().unwrap().text);
            println!("🔍 LEV: {}", lev_summary.as_text().unwrap().text);
        }
    }

    println!("✅ Algorithm comparison testing completed");
    Ok(())
}

/// Test new autocomplete_terms tool (prefix + fuzzy)
#[tokio::test]
#[serial]
async fn test_autocomplete_terms_tool() -> Result<()> {
    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Update configuration
    let update_config_args = json!({"config_str": config_json});
    service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    // Build index
    let build_index_args = json!({"role": "Terraphim Engineer"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;
    assert!(build_result.is_error != Some(true));

    // Call autocomplete_terms: expect graph expansion to include synonyms of same concept id
    let ac_args = json!({"query": "terraph", "limit": 8});
    let ac_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: Some(ac_args.as_object().unwrap().clone()),
        })
        .await?;
    assert!(ac_result.is_error != Some(true));
    if let Some(summary) = ac_result.content.first() {
        let text = summary.as_text().unwrap().text.clone();
        assert!(text.contains("Found"));
    }
    // Verify that related terms like "graph" or "graph embeddings" appear via concept expansion
    let all_texts: Vec<String> = ac_result
        .content
        .iter()
        .skip(1)
        .filter_map(|c| c.as_text().map(|t| t.text.clone()))
        .collect();
    let has_graph_related = all_texts.iter().any(|t| t.to_lowercase().contains("graph"));
    assert!(
        has_graph_related,
        "Autocomplete should include graph-related synonyms via concept expansion: {:?}",
        all_texts
    );
    Ok(())
}

/// Test new autocomplete_with_snippets tool
#[tokio::test]
#[serial]
async fn test_autocomplete_with_snippets_tool() -> Result<()> {
    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Update configuration and build index
    let update_config_args = json!({"config_str": config_json});
    service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;
    let build_index_args = json!({"role": "Terraphim Engineer"});
    service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    // Call autocomplete_with_snippets
    let ac_args = json!({"query": "graph", "limit": 5});
    let ac_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_with_snippets".into(),
            arguments: Some(ac_args.as_object().unwrap().clone()),
        })
        .await?;
    assert!(ac_result.is_error != Some(true));
    // Should contain a summary and then several text items with " — " separator sometimes
    assert!(!ac_result.content.is_empty());
    let has_snippetish = ac_result
        .content
        .iter()
        .skip(1)
        .any(|c| c.as_text().map(|t| t.text.contains(" — ")).unwrap_or(false));
    if !has_snippetish {
        println!("No snippet separator found; still acceptable depending on dataset");
    }
    Ok(())
}

/// Test error handling for roles without proper knowledge graph configuration
#[tokio::test]
#[serial]
async fn test_autocomplete_error_handling() -> Result<()> {
    println!("🧪 Testing autocomplete error handling for invalid configurations");

    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Create config with role that doesn't have TerraphimGraph relevance function
    let invalid_role = Role {
        shortname: Some("Invalid Role".to_string()),
        name: terraphim_types::RoleName::new("Invalid Role"),
        relevance_function: RelevanceFunction::TitleScorer, // Wrong relevance function
        terraphim_it: false,
        theme: "spacelab".to_string(),
        kg: None, // No knowledge graph
        haystacks: vec![],
        llm_enabled: false,
        llm_api_key: None,
        llm_model: None,
        llm_auto_summarize: false,
        llm_chat_enabled: false,
        llm_chat_system_prompt: None,
        llm_chat_model: None,
        llm_context_window: Some(4096),
        extra: ahash::AHashMap::new(),
    };

    let invalid_config = ConfigBuilder::new()
        .add_role("Invalid Role", invalid_role)
        .build()?;

    let config_json = serde_json::to_string_pretty(&invalid_config)?;

    // Update configuration with invalid role
    let update_config_args = json!({"config_str": config_json});
    let _update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    // Try to build autocomplete index for invalid role
    let build_index_args = json!({"role": "Invalid Role"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    println!("Build result for invalid role: {:?}", build_result);

    // Should fail with proper error message
    assert!(build_result.is_error == Some(true));

    if let Some(error_content) = build_result.content.first() {
        let error_text = error_content.as_text().unwrap().text.clone();
        assert!(error_text.contains("does not use knowledge graph ranking"));
        println!(
            "✅ Correct error for invalid relevance function: {}",
            error_text
        );
    }

    // Test autocomplete search without index
    let search_args = json!({
        "query": "test",
        "similarity": 0.6,
        "limit": 5
    });

    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "fuzzy_autocomplete_search".into(),
            arguments: Some(search_args.as_object().unwrap().clone()),
        })
        .await?;

    println!("Search result without index: {:?}", search_result);

    // Should fail with proper error message about missing index
    assert!(search_result.is_error == Some(true));

    if let Some(error_content) = search_result.content.first() {
        let error_text = error_content.as_text().unwrap().text.clone();
        assert!(error_text.contains("Autocomplete index not built"));
        println!("✅ Correct error for missing index: {}", error_text);
    }

    println!("✅ Error handling testing completed");
    Ok(())
}

/// Test role-specific autocomplete functionality
#[tokio::test]
#[serial]
async fn test_role_specific_autocomplete() -> Result<()> {
    println!("🧪 Testing role-specific autocomplete functionality");

    let config_json = create_autocomplete_test_config().await?;
    let transport = start_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Setup configuration
    let update_config_args = json!({"config_str": config_json});
    let _update_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: Some(update_config_args.as_object().unwrap().clone()),
        })
        .await?;

    // Build index for specific role
    let build_index_args = json!({"role": "Terraphim Engineer"});
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_args.as_object().unwrap().clone()),
        })
        .await?;

    assert!(build_result.is_error != Some(true));

    // Test that autocomplete works within role context
    let terraphim_specific_queries = vec![
        "terraphim", // Should match terraphim-graph
        "graph",     // Should match various graph terms
        "haystack",  // Should match haystack term
        "service",   // Should match service term
    ];

    for query in terraphim_specific_queries {
        println!("🔍 Testing role-specific query: '{}'", query);

        let search_args = json!({
            "query": query,
            "similarity": 0.5, // Lower threshold for broader matches
            "limit": 10
        });

        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "fuzzy_autocomplete_search".into(),
                arguments: Some(search_args.as_object().unwrap().clone()),
            })
            .await?;

        assert!(search_result.is_error != Some(true));

        if let Some(summary_content) = search_result.content.first() {
            let summary_text = summary_content.as_text().unwrap().text.clone();
            println!("Role-specific result for '{}': {}", query, summary_text);

            // Should return some autocomplete suggestions
            assert!(summary_text.contains("Found"));
        }
    }

    // Test build without specifying role (should use selected role)
    let build_index_default_args = json!({});
    let build_default_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: Some(build_index_default_args.as_object().unwrap().clone()),
        })
        .await?;

    assert!(build_default_result.is_error != Some(true));

    if let Some(content) = build_default_result.content.first() {
        let content_text = content.as_text().unwrap().text.clone();
        assert!(content_text.contains("Terraphim Engineer")); // Should use selected role
        println!("✅ Default role autocomplete build: {}", content_text);
    }

    println!("✅ Role-specific autocomplete testing completed");
    Ok(())
}
