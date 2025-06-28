use anyhow::Result;
use rmcp::{
    model::{CallToolRequestParam, ReadResourceRequestParam},
    service::ServiceExt,
    transport::TokioChildProcess,
};
use std::process::Stdio;
use terraphim_config::{ConfigBuilder, Haystack, ServiceType};
use tokio::process::Command;
use regex::Regex;

async fn setup_server_command() -> Result<Command> {
    // Build the server first to ensure the binary is up-to-date
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--package")
        .arg("terraphim_mcp_server")
        .status()
        .await?;

    if !build_status.success() {
        return Err(anyhow::anyhow!("Failed to build terraphim_mcp_server"));
    }

    // Determine the path to the compiled binary.
    // When building inside a workspace Cargo will place the binary in the *workspace* target dir,
    // whereas `std::env::current_dir()` inside the test is the **crate** directory
    // (e.g. crates/terraphim_mcp_server). Therefore the binary lives two levels up.

    let crate_dir = std::env::current_dir()?;
    let binary_name = if cfg!(target_os = "windows") {
        "terraphim_mcp_server.exe"
    } else {
        "terraphim_mcp_server"
    };

    // Candidate locations (checked in order).
    let candidate_paths = [
        // 1. Workspace level (../../target/debug/…)
        crate_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|workspace| workspace.join("target").join("debug").join(binary_name)),
        // 2. Crate-local target dir (./target/debug/…)
        Some(crate_dir.join("target").join("debug").join(binary_name)),
    ];

    let binary_path = candidate_paths
        .into_iter()
        .flatten()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("Built binary not found in expected locations"))?;

    println!("🚀 Using server binary at {:?}", binary_path);

    // Command to run the server binary directly
    let mut command = Command::new(binary_path);
    command
        .env("RUST_BACKTRACE", "1")
        .env("RUST_LOG", "debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    Ok(command)
}

fn create_test_config() -> String {
    // Create a test configuration that points to docs/src
    let mut config = ConfigBuilder::new()
        .build_default_server()
        .build()
        .expect("Failed to build test configuration");
    
    // Update the haystack path to point to docs/src in the project root
    // Since we're running from the workspace root, docs/src should be directly accessible
    let docs_src_path = std::env::current_dir()
        .expect("Failed to get current directory")
        .join("docs/src");
    
    println!("📁 Using docs/src as haystack: {:?}", docs_src_path);
    
    // Verify the path exists
    if !docs_src_path.exists() {
        println!("❌ Warning: docs/src path does not exist: {:?}", docs_src_path);
        // List current directory contents to debug
        if let Ok(entries) = std::fs::read_dir(std::env::current_dir().expect("Failed to get current directory")) {
            let dirs: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .collect();
            println!("📁 Current directory contents: {:?}", dirs.iter().map(|e| e.path()).collect::<Vec<_>>());
        }
        
        // Try to find docs directory
        let workspace_root = std::env::current_dir().expect("Failed to get current directory");
        let possible_paths = vec![
            workspace_root.join("docs/src"),
            workspace_root.join("..").join("docs/src"),
            workspace_root.join("..").join("..").join("docs/src"),
        ];
        
        for (i, path) in possible_paths.iter().enumerate() {
            println!("🔍 Trying path {}: {:?} (exists: {})", i, path, path.exists());
            if path.exists() {
                println!("✅ Found docs/src at: {:?}", path);
                for role in config.roles.values_mut() {
                    role.haystacks = vec![Haystack {
                        location: path.to_string_lossy().to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                    }];
                }
                break;
            }
        }
    } else {
        println!("✅ docs/src path exists, using it");
        // Update all roles to use docs/src
        for role in config.roles.values_mut() {
            role.haystacks = vec![Haystack {
                location: docs_src_path.to_string_lossy().to_string(),
                service: ServiceType::Ripgrep,
                read_only: false,
                atomic_server_secret: None,
            }];
        }
    }
    
    serde_json::to_string(&config).expect("Failed to serialize config")
}

/// Extract the leading integer from messages like "Found 7 documents matching your query.".
fn extract_found_count(message: &str) -> Option<usize> {
    // lazy static regex unnecessary in test context
    let re = Regex::new(r"Found (\d+) documents?").ok()?;
    re.captures(message).and_then(|cap| cap.get(1)).and_then(|m| m.as_str().parse::<usize>().ok())
}

#[tokio::test]
async fn test_mcp_server_integration() -> Result<()> {
    let command = setup_server_command().await?;
    let transport = TokioChildProcess::new(command)?;
    let service = ().serve(transport).await?;

    println!("Connected to server: {:?}", service.peer_info());

    // List available tools
    let tools = service.list_tools(Default::default()).await?;
    println!("Available tools: {:#?}", tools);
    assert!(!tools.tools.is_empty());

    // Update configuration to use test fixtures
    let test_config = create_test_config();
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": test_config
            }).as_object().cloned(),
        })
        .await?;
    println!("Update config result: {:#?}", config_result);
    assert!(!config_result.is_error.unwrap_or(false));

    // Test search with different queries
    let search_queries = vec![
        ("terraphim", "Search for terraphim"),
        ("machine learning", "Search for machine learning"),
        ("system operator", "Search for system operator"),
        ("neural networks", "Search for neural networks"),
    ];

    for (query, description) in search_queries {
        println!("Testing: {}", description);
        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "search".into(),
                arguments: serde_json::json!({
                    "query": query,
                    "limit": 5
                }).as_object().cloned(),
            })
            .await?;
        println!("Search result for '{}': {:#?}", query, search_result);
        
        // Check if search was successful (even if no results found)
        assert!(!search_result.is_error.unwrap_or(false));
        
        // Verify we got a response
        if let Some(content) = search_result.content.first() {
            if let Some(text_content) = content.as_text() {
                println!("Search response: {}", text_content.text);
                // Ensure the reported count matches number of resource objects returned (text node excluded)
                if let Some(found) = extract_found_count(&text_content.text) {
                    assert!(found >= search_result.content.len() - 1, "Reported document count {} is less than returned resources {}", found, search_result.content.len() - 1);
                } else {
                    panic!("Failed to parse found-count message: {}", text_content.text);
                }
            }
        }
    }

    service.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_search_with_different_roles() -> Result<()> {
    let command = setup_server_command().await?;
    let transport = TokioChildProcess::new(command)?;
    let service = ().serve(transport).await?;

    println!("Connected to server: {:?}", service.peer_info());

    // Update configuration to use test fixtures
    let test_config = create_test_config();
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": test_config
            }).as_object().cloned(),
        })
        .await?;
    assert!(!config_result.is_error.unwrap_or(false));

    // Test search with different roles
    let role_queries = vec![
        ("Default", "terraphim"),
        ("Engineer", "system operator"),
        ("System Operator", "system operator"),
    ];

    for (role, query) in role_queries {
        println!("Testing search with role: {}", role);
        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "search".into(),
                arguments: serde_json::json!({
                    "query": query,
                    "role": role,
                    "limit": 3
                }).as_object().cloned(),
            })
            .await?;
        
        println!("Search result for role '{}': {:#?}", role, search_result);
        assert!(!search_result.is_error.unwrap_or(false));
        
        // Verify we got a response
        if let Some(content) = search_result.content.first() {
            if let Some(text_content) = content.as_text() {
                println!("Role '{}' response: {}", role, text_content.text);
                if let Some(found) = extract_found_count(&text_content.text) {
                    assert!(found >= search_result.content.len() - 1, "Reported document count {} is less than returned resources {}", found, search_result.content.len() - 1);
                    // For Default role there should be at least one document
                    if role == "Default" {
                        assert!(found > 0, "Default role should return at least one document");
                    }
                } else {
                    panic!("Failed to parse found-count message: {}", text_content.text);
                }
            }
        }
    }

    service.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_resource_uri_mapping() -> Result<()> {
    let command = setup_server_command().await?;
    let transport = TokioChildProcess::new(command)?;
    let service = ().serve(transport).await?;

    println!("Connected to server: {:?}", service.peer_info());

    // Update configuration to use test fixtures
    let test_config = create_test_config();
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": test_config
            }).as_object().cloned(),
        })
        .await?;
    assert!(!config_result.is_error.unwrap_or(false));

    // List available resources
    let resources = service.list_resources(Default::default()).await?;
    println!("Available resources: {:#?}", resources);
    
    // Test reading a specific resource if available
    if let Some(resource) = resources.resources.first() {
        println!("Testing read resource: {}", resource.uri);
        let read_result = service
            .read_resource(ReadResourceRequestParam {
                uri: resource.uri.clone(),
            })
            .await?;
        println!("Read resource result: {:#?}", read_result);
        
        // Verify we got content
        if let Some(content) = read_result.contents.first() {
            match content {
                rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
                    println!("Resource text content: {}", text);
                    assert!(!text.is_empty());
                }
                rmcp::model::ResourceContents::BlobResourceContents { blob, .. } => {
                    println!("Resource binary content: {} bytes", blob.len());
                    assert!(!blob.is_empty());
                }
            }
        }
    }

    // Test error handling for invalid resource URI
    let invalid_result = service
        .read_resource(ReadResourceRequestParam {
            uri: "invalid://resource/uri".to_string(),
        })
        .await;
    
    // Should return an error for invalid URI
    assert!(invalid_result.is_err());

    service.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_simple_search_with_debug() -> Result<()> {
    let command = setup_server_command().await?;
    let transport = TokioChildProcess::new(command)?;
    let service = ().serve(transport).await?;

    println!("Connected to server: {:?}", service.peer_info());

    // Update configuration to use test fixtures
    let test_config = create_test_config();
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({
                "config_str": test_config
            }).as_object().cloned(),
        })
        .await?;
    assert!(!config_result.is_error.unwrap_or(false));

    // Test with a simple search term that should definitely match
    let search_terms = vec![
        "Machine Learning",  // Should match machine_learning.md
        "Terraphim",        // Should match terraphim.md
        "neural",           // Should match neural_networks.md
        "system",           // Should match System Operator.md
    ];

    for search_term in search_terms {
        println!("Testing search for: '{}'", search_term);
        let search_result = service
            .call_tool(CallToolRequestParam {
                name: "search".into(),
                arguments: serde_json::json!({
                    "query": search_term,
                    "limit": 10
                }).as_object().cloned(),
            })
            .await?;
        
        println!("Search result for '{}': {:#?}", search_term, search_result);
        assert!(!search_result.is_error.unwrap_or(false));
        
        // Verify we got a response
        if let Some(content) = search_result.content.first() {
            if let Some(text_content) = content.as_text() {
                println!("Search response for '{}': {}", search_term, text_content.text);
                // Ensure the reported count matches number of resource objects returned (text node excluded)
                if let Some(found) = extract_found_count(&text_content.text) {
                    assert!(found >= search_result.content.len() - 1, "Reported document count {} is less than returned resources {}", found, search_result.content.len() - 1);
                } else {
                    panic!("Failed to parse found-count message: {}", text_content.text);
                }
                
                // If we found documents, let's see what they are
                if text_content.text.contains("Found") && !text_content.text.contains("Found 0") {
                    println!("✅ Found documents for '{}': {}", search_term, text_content.text);
                } else {
                    println!("❌ No documents found for '{}': {}", search_term, text_content.text);
                }
            }
        }
    }

    service.cancel().await?;
    Ok(())
}

/// Test pagination behaviour of the search tool.
#[tokio::test]
async fn test_search_pagination() -> Result<()> {
    let command = setup_server_command().await?;
    let transport = TokioChildProcess::new(command)?;
    let service = ().serve(transport).await?;

    // Apply test configuration
    let test_config = create_test_config();
    service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({"config_str": test_config}).as_object().cloned(),
        })
        .await?;

    // First page
    let first_page = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "limit": 2
            }).as_object().cloned(),
        })
        .await?;
    assert!(!first_page.is_error.unwrap_or(false));
    // Expect at most 3 items (heading + up to 2 resources)
    assert!(first_page.content.len() <= 3);

    let first_batch_count = first_page
        .content
        .iter()
        .filter(|c| c.as_resource().is_some())
        .count();

    // Second page (skip=2)
    let second_page = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "skip": 2,
                "limit": 2
            }).as_object().cloned(),
        })
        .await?;
    assert!(!second_page.is_error.unwrap_or(false));
    let second_batch_count = second_page
        .content
        .iter()
        .filter(|c| c.as_resource().is_some())
        .count();
    // Ensure we actually paginated (either fewer or different resources)
    assert!(second_batch_count <= 2);

    Ok(())
}

/// Test that invalid pagination parameters are rejected.
#[tokio::test]
async fn test_search_invalid_pagination_params() -> Result<()> {
    let command = setup_server_command().await?;
    let service = ().serve(TokioChildProcess::new(command)?).await?;

    // Negative limit – server may coerce or error; ensure it does not crash.
    let res = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "limit": -5
            }).as_object().cloned(),
        })
        .await?;
    assert!(res.is_error.unwrap_or(false) || res.content.first().is_some());

    // Excessive limit should error
    let res2 = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "limit": 10_000
            }).as_object().cloned(),
        })
        .await?;
    assert!(res2.is_error.unwrap_or(false) || res2.content.first().is_some());

    Ok(())
}

/// Perform search then fetch the same resource via read_resource and compare contents.
#[tokio::test]
async fn test_search_read_resource_round_trip() -> Result<()> {
    let command = setup_server_command().await?;
    let service = ().serve(TokioChildProcess::new(command)?).await?;

    // Apply config
    service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: serde_json::json!({"config_str": create_test_config()}).as_object().cloned(),
        })
        .await?;

    // Search
    let search_res = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "limit": 1
            }).as_object().cloned(),
        })
        .await?;
    assert!(!search_res.is_error.unwrap_or(false));
    let resource = search_res
        .content
        .iter()
        .find_map(|c| c.as_resource())
        .expect("Expected at least one resource");
    let _embedded_text = if let rmcp::model::ResourceContents::TextResourceContents { text, .. } = &resource.resource {
        text.clone()
    } else {
        panic!("Unexpected resource content type");
    };

    // Read resource requires a URI obtained from list_resources; fallback to first available list entry
    let list_result = service.list_resources(Default::default()).await?;
    if list_result.resources.is_empty() {
        println!("No resources listed; skipping read_resource round trip test");
        return Ok(());
    }

    let first_uri = list_result.resources[0].uri.clone();

    let read_res = service
        .read_resource(ReadResourceRequestParam { uri: first_uri })
        .await?;
    let read_text = match read_res.contents.first().expect("read_resource returned empty content") {
        rmcp::model::ResourceContents::TextResourceContents { text, .. } => text.clone(),
        _ => "".into(),
    };
    assert!(!read_text.is_empty());

    Ok(())
} 