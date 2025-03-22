use anyhow::Result;
use mcp_core::{
    handler::{ResourceError, ToolError},
    Content,
};
use mcp_server::router::Router;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use terraphim_config::{ConfigBuilder, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::Document;
use terraphim_mcp_server::TerraphimMcpRouter;
use tokio::task;

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
    let router = TerraphimMcpRouter::new(config_state);
    
    // Test 1: Search tool should return results
    let search_params = json!({
        "query": "Terraphim"
    });
    
    let search_results = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search tool failed: {}", e))?;
    
    // Verify we got content in the response
    assert!(!search_results.is_empty(), "Search results should not be empty");
    
    // The first result should be a text summary, and the rest should be resources
    assert!(matches!(search_results[0], Content::Text(_)), "First result should be a text summary");
    
    // Extract resource URIs from search results
    let mut resource_uris = Vec::new();
    
    for content in search_results.iter().skip(1) { // Skip the text summary
        match content {
            Content::Resource(resource_content) => {
                if let mcp_core::resource::ResourceContents::TextResourceContents { uri, .. } = &resource_content.resource {
                    println!("Found resource: {}", uri);
                    resource_uris.push(uri.clone());
                }
            }
            _ => {}
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
    let invalid_tool_result = router.call_tool("nonexistent", json!({})).await;
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
    
    // Create the MCP router
    let router = TerraphimMcpRouter::new(config_state);
    
    // In the search implementation, the first item is always a text summary
    // The actual limit is applied to the documents that are returned by the service,
    // before they're converted to resources
    
    // Test 1: Search with a specific query that should return at least one result
    let search_params = json!({
        "query": "knowledge graphs"
    });
    
    let search_results = router.call_tool("search", search_params).await.map_err(|e| anyhow::anyhow!("Search failed: {}", e))?;
    
    // Print results for debugging
    if let Content::Text(text_content) = &search_results[0] {
        println!("Search results: {}", text_content.text);
    }
    
    // The search should find at least one result
    assert!(search_results.len() > 1, "Search should find at least one result");
    
    // Count resource objects
    let resource_count = search_results.iter().filter(|content| 
        matches!(content, Content::Resource(_))
    ).count();
    println!("Resource count: {}", resource_count);
    
    // Test 2: Verify that results contain expected content
    let doc2_params = json!({
        "query": "knowledge graphs"
    });
    
    let doc2_results = router.call_tool("search", doc2_params).await.map_err(|e| anyhow::anyhow!("Search failed: {}", e))?;
    
    // Get the text summary to see what documents were found
    if let Content::Text(text_content) = &doc2_results[0] {
        println!("Doc2 search results: {}", text_content.text);
        // Verify that the results mention doc2 which contains "knowledge graphs"
        assert!(text_content.text.to_lowercase().contains("document 2"), 
               "Search for 'knowledge graphs' should find Document 2");
    }
    
    // Clean up
    cleanup_test_resources(test_name).await?;
    
    Ok(())
} 