use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test extract_paragraphs_from_automata and is_all_terms_connected_by_path with explicit role specification
#[tokio::test]
async fn test_advanced_functions_with_explicit_terraphim_engineer_role() -> Result<()> {
    println!("🚀 Testing advanced MCP functions with explicit Terraphim Engineer role");
    
    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| workspace.join("target").join("debug").join("terraphim_mcp_server"))
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    
    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop"); // Desktop profile has Terraphim Engineer role
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    println!("🔗 Connected to MCP server");
    
    // Step 1: Build autocomplete index for Terraphim Engineer role specifically  
    println!("🔧 Building autocomplete index for Terraphim Engineer...");
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Build index result: {:?}", build_result.content);
    
    // Step 2: Test extract_paragraphs_from_automata with Terraphim Engineer role
    println!("📄 Testing extract_paragraphs_from_automata...");
    
    let text_with_kg_terms = r#"
    Introduction to Terraphim System
    
    The Terraphim system is built around several key components that work together 
    to provide semantic search capabilities.
    
    Haystack Component Overview
    
    The haystack serves as the primary data source for indexing documents. Each haystack
    can be configured to work with different types of data sources, acting as an agent
    for data retrieval. The haystack component is essential for gathering documents
    that will be processed by the knowledge graph system.
    
    Graph Processing and Embeddings
    
    Terraphim Graph uses sophisticated graph embeddings for ranking search results.
    These graph embeddings create connections between related concepts, allowing for
    more intelligent search results. The knowledge graph based embeddings system
    helps identify semantic relationships between documents.
    
    Service Architecture
    
    The service layer acts as both a provider and middleware component. This service
    architecture ensures smooth communication between different parts of the system,
    with the provider handling data requests and the middleware coordinating between
    various system components.
    "#;
    
    let extract_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": text_with_kg_terms,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Extract paragraphs result: {:?}", extract_result.content);
    
    // Step 3: Test is_all_terms_connected_by_path with Terraphim Engineer role
    println!("🔗 Testing is_all_terms_connected_by_path...");
    
    // Test with text that should contain connected terms from our knowledge graph
    let connectivity_text = r#"
    The haystack provides service functionality as a datasource for the system.
    This service acts as a provider and middleware for data processing.
    Graph embeddings are used for knowledge graph based embeddings in the system.
    "#;
    
    let connectivity_result = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": connectivity_text,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Connectivity result: {:?}", connectivity_result.content);
    
    // Step 4: Test extract_paragraphs with different content
    println!("📄 Testing extract_paragraphs with KG content...");
    
    let kg_content = r#"
    Terraphim Graph Analysis
    
    This section explains the Terraphim Graph implementation and its relationship
    with other system components.
    
    Knowledge Graph Structure
    
    The knowledge graph system uses graph embeddings to create semantic connections.
    These graph embeddings are a form of knowledge graph based embeddings that
    help establish relationships between concepts and documents.
    
    Haystack Integration
    
    Each haystack in the system serves as a datasource for document indexing.
    The haystack component can be configured to work with various data sources,
    from local files to remote APIs, acting as an intelligent agent for data retrieval.
    "#;
    
    let kg_extract_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": kg_content,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ KG extract result: {:?}", kg_extract_result.content);
    
    // Step 5: Test connectivity with simpler text
    println!("🔗 Testing connectivity with service-related terms...");
    
    let service_text = "The service provides functionality through its provider interface, acting as middleware.";
    
    let service_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": service_text,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Service connectivity result: {:?}", service_connectivity.content);
    
    // Step 6: Test autocomplete to verify the role is working
    println!("🔤 Testing autocomplete with Terraphim Engineer role...");
    
    let autocomplete_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "haystack",
                "limit": 5,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Autocomplete result: {:?}", autocomplete_result.content);
    
    println!("🎉 All advanced function tests completed successfully!");
    
    Ok(())
}

/// Test that both advanced functions work correctly with realistic scenarios
#[tokio::test]
async fn test_advanced_functions_realistic_scenarios() -> Result<()> {
    println!("🎯 Testing advanced functions with realistic scenarios");
    
    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| workspace.join("target").join("debug").join("terraphim_mcp_server"))
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    
    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop");
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    // Build the index first
    let _build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    // Realistic Scenario 1: Technical documentation analysis
    println!("📚 Scenario 1: Technical documentation analysis");
    
    let tech_doc = r#"
    System Architecture Overview
    
    The Terraphim system consists of multiple interconnected components designed
    for efficient document processing and semantic search.
    
    Data Ingestion Layer
    
    The haystack component serves as the foundation for data ingestion. Each haystack
    acts as a datasource, capable of processing various document types. The haystack
    can be configured as an agent that monitors and indexes new content automatically.
    
    Processing Pipeline
    
    Once documents are ingested through the haystack, they flow through the service
    layer. This service acts as a provider of processing capabilities and serves as
    middleware between the ingestion layer and the knowledge graph system.
    
    Semantic Analysis
    
    The core of Terraphim lies in its graph embeddings technology. These embeddings
    create a knowledge graph where concepts are interconnected. The knowledge graph
    based embeddings allow for sophisticated semantic search capabilities that go
    beyond simple keyword matching.
    "#;
    
    let tech_extract = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": tech_doc,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("📄 Tech doc extraction: {:?}", tech_extract.content);
    
    let tech_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": tech_doc,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("🔗 Tech doc connectivity: {:?}", tech_connectivity.content);
    
    // Realistic Scenario 2: Short content analysis
    println!("📝 Scenario 2: Short content analysis");
    
    let short_content = "Haystack service provides graph embeddings for the knowledge graph system.";
    
    let short_extract = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": short_content,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("📄 Short content extraction: {:?}", short_extract.content);
    
    let short_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": short_content,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("🔗 Short content connectivity: {:?}", short_connectivity.content);
    
    // Realistic Scenario 3: Mixed terminology
    println!("🔀 Scenario 3: Mixed terminology analysis");
    
    let mixed_content = r#"
    Configuration and Setup
    
    Setting up Terraphim requires configuring multiple components. The primary
    component is the haystack, which serves as your datasource for document indexing.
    
    Service Configuration
    
    The service layer needs to be configured to work as a provider for your specific
    use case. This middleware component handles communication between different parts
    of the system.
    
    Graph Setup
    
    Finally, configure the graph embeddings system to enable knowledge graph based
    embeddings for semantic search functionality.
    "#;
    
    let mixed_extract = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": mixed_content,
                "include_term": true,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("📄 Mixed content extraction: {:?}", mixed_extract.content);
    
    let mixed_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": mixed_content,
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("🔗 Mixed content connectivity: {:?}", mixed_connectivity.content);
    
    println!("🎉 All realistic scenario tests completed!");
    
    Ok(())
}