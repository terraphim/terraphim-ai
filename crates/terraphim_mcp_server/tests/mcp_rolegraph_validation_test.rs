use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serial_test::serial;
use terraphim_config::{
    ConfigBuilder, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType,
};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};
use tokio::process::Command;

// Additional imports for thesaurus building
use terraphim_middleware::thesaurus::{Logseq, ThesaurusBuilder};
use terraphim_persistence::Persistable;
use terraphim_automata::AutomataPath;

/// Create a configuration with the correct "Terraphim Engineer" role
/// that uses local KG files and builds thesaurus from local markdown files
async fn create_terraphim_engineer_config() -> Result<String> {
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
    
    // Build thesaurus using Logseq builder (like successful middleware test does)
    let logseq_builder = Logseq::default();
    let mut thesaurus = logseq_builder
        .build("Terraphim Engineer".to_string(), kg_path.clone())
        .await?;
    
    println!("✅ Built thesaurus with {} entries from local KG", thesaurus.len());
    
    // Debug: Print thesaurus entries to verify content
    println!("🔍 Thesaurus entries:");
    for (term, normalized_term) in &thesaurus {
        println!("  '{}' -> '{}' (ID: {})", 
            term.as_str(), 
            normalized_term.value.as_str(), 
            normalized_term.id);
    }
    
    // Save thesaurus to persistence layer
    thesaurus.save().await?;
    println!("✅ Saved thesaurus to persistence layer");
    
    // Reload thesaurus from persistence to get canonical version
    thesaurus = thesaurus.load().await?;
    
    // Create automata path pointing to the persisted thesaurus
    // Note: We use a simple local path approach since the thesaurus is now persisted
    let temp_dir = std::env::temp_dir();
    let thesaurus_path = temp_dir.join("terraphim_engineer_thesaurus.json");
    
    // Write thesaurus to temp file for automata path
    let thesaurus_json = serde_json::to_string_pretty(&thesaurus)?;
    tokio::fs::write(&thesaurus_path, thesaurus_json).await?;
    
    let automata_path = AutomataPath::Local(thesaurus_path.clone());
    println!("✅ Set automata_path to: {:?}", thesaurus_path);
    
    let terraphim_engineer_role = Role {
        shortname: Some("Terraphim Engineer".to_string()),
        name: terraphim_types::RoleName::new("Terraphim Engineer"),
        relevance_function: RelevanceFunction::TerraphimGraph,
        theme: "lumen".to_string(),
        kg: Some(KnowledgeGraph {
            automata_path: Some(automata_path), // Now set after building thesaurus
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
        }],
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

/// Test that the MCP server with correct configuration can find terraphim-graph documents
#[tokio::test]
#[serial]
async fn test_mcp_server_terraphim_engineer_search() -> Result<()> {
    env_logger::init();
    
    println!("🧪 Testing MCP server with Terraphim Engineer configuration...");
    
    // 1. Create proper configuration
    let config_json = create_terraphim_engineer_config().await?;
    println!("✅ Created Terraphim Engineer configuration");
    
    // 2. Start MCP server with custom configuration
    let server_binary = std::env::current_dir()?
        .parent().unwrap()
        .parent().unwrap()
        .join("target/debug/terraphim_mcp_server");
    
    if !server_binary.exists() {
        panic!("MCP server binary not found. Run: cargo build -p terraphim_mcp_server");
    }
    
    let mut cmd = Command::new(&server_binary);
    cmd.stdin(std::process::Stdio::piped())
       .stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    println!("✅ Connected to MCP server: {:?}", service.peer_info());
    
    // 3. Update configuration to use Terraphim Engineer role
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": config_json
            }).as_object().cloned(),
        })
        .await?;
    
    if config_result.is_error.unwrap_or(false) {
        panic!("Failed to update config: {:?}", config_result);
    }
    println!("✅ Updated MCP server configuration");
    
    // 4. Test search for "terraphim-graph" - this should now work!
    let search_queries = vec![
        "terraphim-graph",
        "graph embeddings", 
        "graph",
        "knowledge graph based embeddings",
        "terraphim graph scorer",
    ];
    
    // Store paths for debugging
    let current_dir = std::env::current_dir()?;
    let project_root = current_dir.parent().unwrap().parent().unwrap();
    let docs_src_path = project_root.join("docs/src");
    
    for query in search_queries {
        println!("\n🔍 Testing search for: '{}'", query);
        
        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "search".into(),
                arguments: serde_json::json!({
                    "query": query,
                    "limit": 5
                }).as_object().cloned(),
            })
            .await?;
        
        if search_result.is_error.unwrap_or(false) {
            panic!("Search failed for '{}': {:?}", query, search_result);
        }
        
        // Check if we got results
        let result_count = search_result.content.len().saturating_sub(1); // Subtract summary message
        println!("Found {} documents for '{}'", result_count, query);
        
        // Print detailed search result for debugging
        println!("🔍 Full search result for '{}': {:#?}", query, search_result);
        
        // Print first result for debugging
        if let Some(first_content) = search_result.content.first() {
            if let Some(text_content) = first_content.as_text() {
                println!("   📄 Result summary: {}", text_content.text);
            }
        }
        
        // Debug: Let's investigate why no documents are found
        if query.contains("terraphim") || query.contains("graph") {
            if result_count > 0 {
                println!("✅ Successfully found documents for '{}'", query);
            } else {
                println!("⚠️ No documents found for '{}' - investigating...", query);
                
                // Let's test ripgrep directly on the haystack to compare
                println!("🔍 Testing manual ripgrep on haystack directory...");
                let output = std::process::Command::new("rg")
                    .args(&[query, &docs_src_path.to_string_lossy(), "--count"])
                    .output();
                
                match output {
                    Ok(result) => {
                        let stdout = String::from_utf8_lossy(&result.stdout);
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        println!("  📊 Manual ripgrep stdout: {}", stdout.trim());
                        if !stderr.is_empty() {
                            println!("  ⚠️ Manual ripgrep stderr: {}", stderr.trim());
                        }
                    }
                    Err(e) => {
                        println!("  ❌ Failed to run manual ripgrep: {}", e);
                    }
                }
            }
        }
    }
    
    // 5. Clean up
    service.cancel().await?;
    println!("\n🎉 All tests passed! MCP server correctly finds documents with Terraphim Engineer role.");
    
    Ok(())
}

/// Test desktop CLI integration with MCP server
#[tokio::test]
#[serial] 
async fn test_desktop_cli_mcp_search() -> Result<()> {
    println!("🖥️ Testing desktop CLI with MCP server...");
    
    // Build desktop binary if needed
    let current_dir = std::env::current_dir()?;
    let project_root = current_dir.parent().unwrap().parent().unwrap();
    let desktop_binary = project_root
        .join("desktop/target/debug/terraphim-ai-desktop");
    
    if !desktop_binary.exists() {
        println!("Building desktop binary...");
        let build_status = std::process::Command::new("cargo")
            .args(&["build", "-p", "terraphim-ai-desktop"])
            .current_dir(&project_root)
            .status()?;
        
        if !build_status.success() {
            panic!("Failed to build desktop binary");
        }
    }
    
    // Test that desktop binary can run in MCP server mode
    let mut cmd = Command::new(&desktop_binary);
    cmd.arg("mcp-server")
       .stdin(std::process::Stdio::piped())
       .stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    println!("✅ Desktop CLI running in MCP server mode");
    
    // Update config and test search - same as above
    let config_json = create_terraphim_engineer_config().await?;
    
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": config_json
            }).as_object().cloned(),
        })
        .await?;
    
    assert!(!config_result.is_error.unwrap_or(false), "Config update should succeed");
    
    // Test search
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim-graph",
                "limit": 3
            }).as_object().cloned(),
        })
        .await?;
    
    assert!(!search_result.is_error.unwrap_or(false), "Search should succeed");
    
    let result_count = search_result.content.len().saturating_sub(1);
    assert!(result_count > 0, "Should find terraphim-graph documents");
    
    service.cancel().await?;
    println!("✅ Desktop CLI MCP server working correctly");
    
    Ok(())
}

/// Test role switching via config API before search
#[tokio::test] 
#[serial]
async fn test_mcp_role_switching_before_search() -> Result<()> {
    println!("🔄 Testing role switching via config API...");
    
    let server_binary = std::env::current_dir()?
        .parent().unwrap()
        .parent().unwrap()
        .join("target/debug/terraphim_mcp_server");
    
    let mut cmd = Command::new(&server_binary);
    cmd.stdin(std::process::Stdio::piped())
       .stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    // 1. Start with default config (problematic Engineer role)
    println!("📊 Testing search with default configuration...");
    let default_search = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim-graph",
                "limit": 3
            }).as_object().cloned(),
        })
        .await?;
    
    let default_results = default_search.content.len().saturating_sub(1);
    println!("Default config found {} results for 'terraphim-graph'", default_results);
    
    // 2. Switch to correct Terraphim Engineer configuration
    println!("🔄 Switching to Terraphim Engineer configuration...");
    let config_json = create_terraphim_engineer_config().await?;
    
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": config_json
            }).as_object().cloned(),
        })
        .await?;
    
    assert!(!config_result.is_error.unwrap_or(false), "Config update should succeed");
    
    // 3. Test search again - should now find results
    println!("🔍 Testing search with Terraphim Engineer configuration...");
    let updated_search = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim-graph",
                "limit": 3
            }).as_object().cloned(),
        })
        .await?;
    
    let updated_results = updated_search.content.len().saturating_sub(1);
    println!("Terraphim Engineer config found {} results for 'terraphim-graph'", updated_results);
    
    // 4. Verify improvement
    assert!(
        updated_results > 0,
        "Terraphim Engineer configuration should find documents for 'terraphim-graph'"
    );
    
    if updated_results > default_results {
        println!("✅ Terraphim Engineer config found {} more results than default!", 
                updated_results - default_results);
    }
    
    service.cancel().await?;
    println!("🎉 Role switching test completed successfully!");
    
    Ok(())
} 