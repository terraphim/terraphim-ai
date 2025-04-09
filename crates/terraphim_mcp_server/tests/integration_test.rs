use anyhow::Result;
use mcp_core::{
    handler::{ResourceError, ToolError},
    Content,
    protocol::{JsonRpcResponse, JsonRpcRequest},
};
use mcp_server::router::Router;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{Document, RelevanceFunction, RoleName};
use terraphim_mcp_server::TerraphimMcpRouter;
use tokio::task;
use ahash::AHashMap;

/// Helper function to extract JsonRpcResponse from Content returned by a tool
fn extract_json_rpc_response(contents: &[Content]) -> Result<JsonRpcResponse> {
    if contents.is_empty() {
        return Err(anyhow::anyhow!("Empty content"));
    }
    
    match &contents[0] {
        Content::Text(text_content) => {
            serde_json::from_str::<JsonRpcResponse>(&text_content.text)
                .map_err(|e| anyhow::anyhow!("Failed to parse JSON-RPC response: {}", e))
        },
        _ => Err(anyhow::anyhow!("Expected TextContent as first result")),
    }
}

// Helper function to create a test document
fn create_test_document(id: &str, title: &str, content: &str) -> Document {
    Document {
        id: id.to_string(),
        url: format!("https://terraphim.ai/documents/{}", id),
        title: title.to_string(),
        body: content.to_string(),
        description: Some("Test document description".to_string()),
        stub: Some("Test stub".to_string()),
        tags: Some(vec!["test".to_string(), "integration".to_string()]),
        rank: None,
    }
}

// Create a test haystack directory with test files
async fn create_test_haystack(test_name: &str) -> Result<PathBuf> {
    // Create a temporary test directory in the project root with a unique name for the test
    let haystack_dir = PathBuf::from(format!("./test_haystack_{}", test_name));
    
    // Create the directory if it doesn't exist
    if !haystack_dir.exists() {
        fs::create_dir_all(&haystack_dir)?;
    }
    
    // Create some test markdown files in the haystack
    let test_docs = vec![
        (
            "doc1.md", 
            "# Test Document 1\n\nThis is a test document for MCP server integration testing. It contains information about Terraphim."
        ),
        (
            "doc2.md",
            "# Test Document 2\n\nAnother test document with different content. This one mentions knowledge graphs and search functionality."
        ),
        (
            "doc3.md",
            "# MCP Integration\n\nDocument about Model Context Protocol integration with Terraphim services. This shows how to use resources and tools."
        ),
    ];
    
    // Write the test files
    for (filename, content) in test_docs {
        let file_path = haystack_dir.join(filename);
        fs::write(file_path, content)?;
    }
    
    // Return the path to the test haystack directory
    Ok(haystack_dir.canonicalize()?)
}

// Test setup - initialize config state with test data
async fn setup_test_env(test_name: &str) -> Result<ConfigState> {
    // Create test haystack directory
    let haystack_path = create_test_haystack(test_name).await?;
    
    // Create a default configuration
    let mut config = ConfigBuilder::new()
        .build_default_server()
        .build()?;
    
    // Get the default role and update its haystack path
    let default_role_name = config.default_role.clone();
    if let Some(role) = config.roles.get_mut(&default_role_name) {
        // Update the first haystack's path or create one if none exists
        if role.haystacks.is_empty() {
            role.haystacks.push(terraphim_config::Haystack {
                path: haystack_path.clone(),
                service: terraphim_config::ServiceType::Ripgrep,
            });
        } else {
            role.haystacks[0].path = haystack_path.clone();
        }
    }
    
    // Add a specific test role that can be queried by name
    let test_role_name = "test_role".to_string();
    let test_role = terraphim_config::Role {
        name: test_role_name.clone().into(),
        shortname: Some("Test Role".to_string()),
        relevance_function: terraphim_types::RelevanceFunction::TitleScorer,
        theme: "default".to_string(),
        kg: None,
        haystacks: vec![
            terraphim_config::Haystack {
                path: haystack_path.clone(),
                service: terraphim_config::ServiceType::Ripgrep,
            }
        ],
        extra: ahash::AHashMap::new(),
    };
    
    // Add the test role to the config
    config.roles.insert(test_role_name.into(), test_role);
    
    // Create config state
    let config_state = ConfigState::new(&mut config).await?;

    Ok(config_state)
}

// Insert test documents into Terraphim service for testing
async fn insert_test_documents(service: &mut TerraphimService) -> Result<Vec<Document>> {
    // Create some test documents
    let documents = vec![
        create_test_document(
            "doc1",
            "Test Document 1",
            "This is a test document for MCP server integration testing. It contains information about Terraphim."
        ),
        create_test_document(
            "doc2",
            "Test Document 2",
            "Another test document with different content. This one mentions knowledge graphs and search functionality."
        ),
        create_test_document(
            "doc3",
            "MCP Integration",
            "Document about Model Context Protocol integration with Terraphim services. This shows how to use resources and tools."
        ),
    ];
    
    // Insert test documents into the service
    let mut result_docs = Vec::new();
    for doc in &documents {
        let created_doc = service.create_document(doc.clone()).await?;
        result_docs.push(created_doc);
    }
    
    Ok(result_docs)
}

// Clean up test resources
async fn cleanup_test_resources(test_name: &str) -> Result<()> {
    let haystack_dir = PathBuf::from(format!("./test_haystack_{}", test_name));
    if haystack_dir.exists() {
        task::spawn_blocking(move || {
            fs::remove_dir_all(haystack_dir)
        }).await??;
    }
    Ok(())
}

#[tokio::test]
async fn test_mcp_router_resources() -> Result<()> {
    let test_name = "resources_test";
    
    // Set up the test environment
    let config_state = setup_test_env(test_name).await?;
    
    // Create Terraphim service and insert test documents
    let mut service = TerraphimService::new(config_state.clone());
    let documents = insert_test_documents(&mut service).await?;
    
    // Create the MCP router
    let router = TerraphimMcpRouter::new(Arc::new(config_state));
    
    // Test 1: Search tool should return results
    let search_params = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "search",
        "params": {
            "query": "Terraphim"
        }
    });
    
    let search_result = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // The router now returns Vec<Content> where the first element is TextContent with JSON-RPC response
    assert!(!search_result.is_empty(), "Search result should not be empty");
    
    // Extract the JSON-RPC response from the TextContent
    let search_results = extract_json_rpc_response(&search_result)?;
    
    // Debug output
    println!("JSON-RPC Response: {:?}", search_results);
    
    // Verify we got a valid JSON-RPC response
    assert!(search_results.jsonrpc == "2.0", "Response should be JSON-RPC 2.0");
    assert!(search_results.id == Some(1), "Response should have matching ID");
    assert!(search_results.error.is_none(), "Response should not have an error");
    
    let result = search_results.result.expect("Response should have a result");
    let contents = result.get("contents").expect("Result should have contents").as_array().expect("Contents should be an array");
    
    // Debug output
    println!("Contents count: {}", contents.len());
    println!("Contents: {:?}", contents);
    
    // Verify we got content in the response
    assert!(!contents.is_empty(), "Search results should not be empty");
    
    // Extract resource URIs from search results
    let mut resource_uris = Vec::new();
    
    for content in contents {
        if let Some(resource) = content.get("resource") {
            if let Some(uri) = resource.get("uri") {
                if let Some(uri_str) = uri.as_str() {
                    println!("Found resource: {}", uri_str);
                    resource_uris.push(uri_str.to_string());
                }
            }
        }
    }
    
    assert!(!resource_uris.is_empty(), "Search should return resources");
    
    // We can't directly read resources by ID because the IDs are generated
    // Instead, we'll create terraphim:// URIs manually
    let test_uris: Vec<String> = documents.iter()
        .map(|doc| format!("terraphim://{}", doc.id))
        .collect();
    
    // Print test URIs for debugging
    println!("Test URIs: {:?}", test_uris);
    
    // Test invalid resource URIs should return errors
    let invalid_result = router.read_resource("terraphim://nonexistent").await;
    assert!(invalid_result.is_err(), "Reading nonexistent resource should fail");
    
    match invalid_result {
        Err(ResourceError::NotFound(_)) => {}
        _ => panic!("Expected ResourceError::NotFound for nonexistent resource"),
    }
    
    // Test 4: Invalid tool name should return error
    let invalid_tool_result = router.call_tool("nonexistent", json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "nonexistent",
        "params": {}
    })).await;
    assert!(invalid_tool_result.is_err(), "Calling nonexistent tool should fail");
    
    match invalid_tool_result {
        Err(ToolError::NotFound(_)) => {}
        _ => panic!("Expected ToolError::NotFound for nonexistent tool"),
    }
    
    // Clean up
    cleanup_test_resources(test_name).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_search_with_filters() -> Result<()> {
    let test_name = "filters_test";
    
    // Set up the test environment
    let config_state = setup_test_env(test_name).await?;
    
    // Create Terraphim service and insert test documents
    let mut service = TerraphimService::new(config_state.clone());
    let _documents = insert_test_documents(&mut service).await?;
    
    // Create the MCP router - we need to wrap config_state in an Arc
    let router = TerraphimMcpRouter::new(Arc::new(config_state));
    
    // Test searching with limit parameter
    let search_params = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "search",
        "params": {
            "query": "test",
            "limit": 1
        }
    });
    
    let search_result = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // Extract the JSON-RPC response from the TextContent
    let search_results = extract_json_rpc_response(&search_result)?;
    
    // Debug output
    println!("Filter test - JSON-RPC Response: {:?}", search_results);
    
    // Verify we got a valid JSON-RPC response
    assert!(search_results.jsonrpc == "2.0", "Response should be JSON-RPC 2.0");
    assert!(search_results.id == Some(1), "Response should have matching ID");
    assert!(search_results.error.is_none(), "Response should not have an error");
    
    let result = search_results.result.expect("Response should have a result");
    let contents = result.get("contents").expect("Result should have contents").as_array().expect("Contents should be an array");
    
    // Debug output
    println!("Filter test - Contents count: {}", contents.len());
    println!("Filter test - Contents: {:?}", contents);
    
    // Verify limit worked (1 result + summary = 2 items)
    assert_eq!(contents.len(), 2, "Search with limit=1 should return 2 results (1 summary + 1 document)");
    
    // Test searching with a role parameter that doesn't exist
    let nonexistent_role_params = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "search",
        "params": {
            "query": "test",
            "role": "nonexistent_role"
        }
    });
    
    let nonexistent_role_result = router.call_tool("search", nonexistent_role_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // Extract the JSON-RPC response
    let nonexistent_role_response = extract_json_rpc_response(&nonexistent_role_result)?;
    
    // Debug output
    println!("Nonexistent role test - JSON-RPC Response: {:?}", nonexistent_role_response);
    
    // Verify we got a valid JSON-RPC response
    assert!(nonexistent_role_response.jsonrpc == "2.0", "Response should be JSON-RPC 2.0");
    assert!(nonexistent_role_response.id == Some(3), "Response should have matching ID");
    
    // For a nonexistent role, we should get an error response
    assert!(nonexistent_role_response.error.is_some(), "Response should have an error for nonexistent role");
    assert!(nonexistent_role_response.result.is_none(), "Response should not have a result for nonexistent role");
    
    if let Some(error) = nonexistent_role_response.error {
        assert!(error.message.contains("Role `nonexistent_role` not found"), 
                "Error message should indicate the role was not found");
    }
    
    // Test searching with valid query
    let search_params = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "search",
        "params": {
            "query": "MCP"
        }
    });
    
    let doc2_result = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // Extract the JSON-RPC response
    let doc2_results = extract_json_rpc_response(&doc2_result)?;
    
    // Debug output
    println!("Regular search test - JSON-RPC Response: {:?}", doc2_results);
    
    // Verify we got a valid JSON-RPC response
    assert!(doc2_results.jsonrpc == "2.0", "Response should be JSON-RPC 2.0");
    assert!(doc2_results.id == Some(2), "Response should have matching ID");
    assert!(doc2_results.error.is_none(), "Response should not have an error");
    
    let doc2_result = doc2_results.result.expect("Response should have a result");
    let doc2_contents = doc2_result.get("contents").expect("Result should have contents").as_array().expect("Contents should be an array");
    
    // Debug output
    println!("Regular search test - Contents count: {}", doc2_contents.len());
    println!("Regular search test - Contents: {:?}", doc2_contents);
    
    // Verify we got at least one result for the search
    assert!(!doc2_contents.is_empty(), "Search should return results");
    
    // Clean up
    cleanup_test_resources(test_name).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_search_with_role() -> Result<()> {
    let test_name = "role_search_test";
    
    // Set up the test environment
    let config_state = setup_test_env(test_name).await?;
    
    // Create Terraphim service and insert test documents
    let mut service = TerraphimService::new(config_state.clone());
    let documents = insert_test_documents(&mut service).await?;
    
    // Create the MCP router with Arc<ConfigState>
    let router = TerraphimMcpRouter::new(Arc::new(config_state));
    
    // Test searching with the role parameter
    let search_params = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "search",
        "params": {
            "query": "MCP",
            "role": "test_role"
        }
    });
    
    let search_result = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // Extract the JSON-RPC response from the TextContent
    let search_results = extract_json_rpc_response(&search_result)?;
    
    // Debug output
    println!("Role search test - JSON-RPC Response: {:?}", search_results);
    
    // Verify we got a valid JSON-RPC response
    assert!(search_results.jsonrpc == "2.0", "Response should be JSON-RPC 2.0");
    assert!(search_results.id == Some(1), "Response should have matching ID");
    assert!(search_results.error.is_none(), "Response should not have an error");
    
    let result = search_results.result.expect("Response should have a result");
    let contents = result.get("contents").expect("Result should have contents").as_array().expect("Contents should be an array");
    
    // Debug output
    println!("Role search test - Contents count: {}", contents.len());
    println!("Role search test - Contents: {:?}", contents);
    
    // Verify we got content in the response (at least one result + summary)
    assert!(contents.len() >= 2, "Role-based search should return at least one result plus summary");
    
    // Extract resource URIs from search results
    let mut resource_uris = Vec::new();
    
    for content in contents.iter().skip(1) { // Skip the summary
        if let Some(resource) = content.get("resource") {
            if let Some(uri) = resource.get("uri") {
                if let Some(uri_str) = uri.as_str() {
                    println!("Found resource in role search: {}", uri_str);
                    resource_uris.push(uri_str.to_string());
                }
            }
        }
    }
    
    // Verify we got at least one resource
    assert!(!resource_uris.is_empty(), "Role-based search should return at least one resource");
    
    // Clean up
    cleanup_test_resources(test_name).await?;
    
    Ok(())
} 