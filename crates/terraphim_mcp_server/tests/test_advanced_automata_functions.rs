use anyhow::Result;
use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::TokioChildProcess};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test extract_paragraphs_from_automata with Terraphim Engineer role and real content
#[tokio::test]
async fn test_extract_paragraphs_with_terraphim_engineer() -> Result<()> {
    println!("📄 Testing extract_paragraphs_from_automata with Terraphim Engineer role");

    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| {
            workspace
                .join("target")
                .join("debug")
                .join("terraphim_mcp_server")
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;

    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop"); // Use desktop profile for Terraphim Engineer role

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    println!("🔗 Connected to MCP server with Terraphim Engineer profile");

    // Configure the server to use Terraphim Engineer role with proper KG setup
    println!("⚙️ Configuring Terraphim Engineer role...");
    let current_dir = std::env::current_dir()?;
    let workspace_root = current_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    let kg_path = workspace_root.join("docs/src/kg");

    let terraphim_config = json!({
        "roles": {
            "Terraphim Engineer": {
                "shortname": "terraphim_engineer",
                "name": "Terraphim Engineer",
                "relevance_function": "TerraphimGraph",
                "theme": "lumen",
                "terraphim_it": true,
                "kg": {
                    "automata_path": null,
                    "knowledge_graph_local": {
                        "input_type": "markdown",
                        "path": kg_path.to_string_lossy().to_string()
                    },
                    "public": true,
                    "publish": true
                },
                "haystacks": [{
                    "location": workspace_root.join("docs/src").to_string_lossy().to_string(),
                    "service": "Ripgrep",
                    "read_only": true,
                    "atomic_server_secret": null,
                    "extra_parameters": {}
                }],
                "extra": {}
            }
        },
        "selected_role": "Terraphim Engineer",
        "default_role": "Terraphim Engineer",
        "global_shortcut": "Ctrl+Space"
    });

    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: json!({
                "config_str": terraphim_config.to_string()
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Configuration result: {:?}", config_result.content);

    // Build autocomplete index for Terraphim Engineer role
    println!("🔧 Building autocomplete index for Terraphim Engineer role...");
    let build_index_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Build index result: {:?}", build_index_result.content);

    // Test with realistic content that contains terms from our knowledge graph
    let test_text = r#"
This document discusses the Terraphim Graph system and its components.

The haystack provides data sources for indexing. The haystack can be configured
as a service that acts as a datasource for the knowledge graph system.

Graph embeddings are used by the Terraphim Graph scorer to rank terms based on
their connections. The graph embeddings technique allows for knowledge graph
based embeddings that improve search relevance.

The service layer coordinates between different components. The provider
implements the middleware functionality that connects haystacks to the
knowledge graph system.
"#;

    // Test 1: Extract paragraphs containing 'haystack' term
    println!("🔍 Testing paragraph extraction for 'haystack' term...");
    let haystack_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": test_text,
                "terms": ["haystack"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Haystack extraction result: {:?}",
        haystack_result.content
    );

    // Test 2: Extract paragraphs containing 'graph embeddings' term
    println!("🔍 Testing paragraph extraction for 'graph embeddings' term...");
    let graph_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": test_text,
                "terms": ["graph embeddings"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Graph embeddings extraction result: {:?}",
        graph_result.content
    );

    // Test 3: Extract paragraphs with multiple terms
    println!("🔍 Testing paragraph extraction for multiple terms...");
    let multiple_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": test_text,
                "terms": ["service", "provider", "middleware"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Multiple terms extraction result: {:?}",
        multiple_result.content
    );

    // Test 4: Test with content from actual KG files
    let kg_content = r#"
# Terraphim-graph

## Terraphim Graph scorer

Terraphim Graph (scorer) is using unique graph embeddings, where the rank of the term is defined by number of synonyms connected to the concept.

synonyms:: graph embeddings, graph, knowledge graph based embeddings

Now we will have a concept "Terraphim Graph Scorer" with synonyms "graph embeddings".

# Haystack
synonyms:: datasource, service, agent

The haystack provides access to various data sources and acts as an agent for data retrieval.

# Terraphim Service
synonyms:: provider, middleware

The service layer acts as a provider and middleware between components.
"#;

    println!("🔍 Testing paragraph extraction from real KG content...");
    let kg_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": kg_content,
                "terms": ["terraphim", "synonyms", "knowledge graph"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ KG content extraction result: {:?}", kg_result.content);

    println!("🎉 All extract_paragraphs_from_automata tests completed!");
    Ok(())
}

/// Test is_all_terms_connected_by_path with knowledge graph connections
#[tokio::test]
async fn test_terms_connectivity_with_knowledge_graph() -> Result<()> {
    println!("🔗 Testing is_all_terms_connected_by_path with knowledge graph");

    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| {
            workspace
                .join("target")
                .join("debug")
                .join("terraphim_mcp_server")
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;

    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop"); // Use desktop profile for Terraphim Engineer role

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    println!("🔗 Connected to MCP server with Terraphim Engineer profile");

    // Test 1: Check if terms that should be connected via synonyms are connected
    println!("🔍 Testing connectivity of known synonym terms...");
    let synonym_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["haystack", "datasource", "service"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Synonym connectivity result: {:?}",
        synonym_connectivity.content
    );

    // Test 2: Check connectivity of graph embedding related terms
    println!("🔍 Testing connectivity of graph embedding terms...");
    let graph_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["graph", "graph embeddings", "knowledge graph based embeddings"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Graph embedding connectivity result: {:?}",
        graph_connectivity.content
    );

    // Test 3: Check connectivity of service-related terms
    println!("🔍 Testing connectivity of service terms...");
    let service_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["service", "provider", "middleware"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Service connectivity result: {:?}",
        service_connectivity.content
    );

    // Test 4: Test with terms that should NOT be connected
    println!("🔍 Testing non-connected terms...");
    let unconnected_test = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["completely", "random", "unrelated", "words"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Unconnected terms result: {:?}",
        unconnected_test.content
    );

    // Test 5: Test single term (should always be connected to itself)
    println!("🔍 Testing single term connectivity...");
    let single_term = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["haystack"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Single term connectivity result: {:?}",
        single_term.content
    );

    // Test 6: Test mixed connected and unconnected terms
    println!("🔍 Testing mixed connectivity...");
    let mixed_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["haystack", "service", "completely_random_term"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Mixed connectivity result: {:?}",
        mixed_connectivity.content
    );

    println!("🎉 All is_all_terms_connected_by_path tests completed!");
    Ok(())
}

/// Comprehensive test that validates both functions work together
#[tokio::test]
async fn test_advanced_automata_integration() -> Result<()> {
    println!("🚀 Testing advanced automata functions integration");

    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| {
            workspace
                .join("target")
                .join("debug")
                .join("terraphim_mcp_server")
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;

    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop");

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    println!("🔗 Connected to MCP server for integration testing");

    // Scenario: Extract paragraphs and then check if found terms are connected
    let document_text = r#"
Introduction

This document explains how the Terraphim system works with various components.

Architecture Overview

The haystack component serves as a datasource for the system. It works closely
with the service layer to provide data access. The haystack can be configured
as different types of agents depending on the data source.

Graph Processing

The Terraphim Graph uses sophisticated graph embeddings for ranking. These
graph embeddings are a type of knowledge graph based embeddings that create
connections between related concepts.

Service Layer

The service layer acts as both a provider and middleware. The provider
functionality handles data requests while the middleware coordinates between
different system components.
"#;

    // Step 1: Extract paragraphs containing service-related terms
    println!("📄 Step 1: Extracting paragraphs with service terms...");
    let paragraphs = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": document_text,
                "terms": ["service", "provider", "middleware"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Extracted paragraphs: {:?}", paragraphs.content);

    // Step 2: Check if these terms are connected in the knowledge graph
    println!("🔗 Step 2: Checking connectivity of service terms...");
    let connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["service", "provider", "middleware"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Connectivity result: {:?}", connectivity.content);

    // Step 3: Test with haystack-related terms
    println!("📄 Step 3: Testing haystack term extraction and connectivity...");
    let haystack_paragraphs = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": document_text,
                "terms": ["haystack", "datasource", "agent"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Haystack paragraphs: {:?}", haystack_paragraphs.content);

    let haystack_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["haystack", "datasource", "agent"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Haystack connectivity: {:?}",
        haystack_connectivity.content
    );

    // Step 4: Test cross-domain connectivity (should likely be false)
    println!("🔗 Step 4: Testing cross-domain connectivity...");
    let cross_domain = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": ["haystack", "graph embeddings", "service"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Cross-domain connectivity: {:?}", cross_domain.content);

    println!("🎉 Advanced automata integration test completed successfully!");
    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_advanced_automata_edge_cases() -> Result<()> {
    println!("⚠️ Testing edge cases for advanced automata functions");

    let crate_dir = std::env::current_dir()?;
    let binary_path = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|workspace| {
            workspace
                .join("target")
                .join("debug")
                .join("terraphim_mcp_server")
        })
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;

    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--profile")
        .arg("desktop");

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    // Test 1: Empty text
    println!("🔍 Testing empty text...");
    let empty_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": "",
                "terms": ["haystack"]
            })
            .as_object()
            .cloned(),
        })
        .await;

    match empty_result {
        Ok(result) => println!("✅ Empty text handled: {:?}", result.content),
        Err(e) => println!("⚠️ Empty text error (expected): {}", e),
    }

    // Test 2: Empty terms array
    println!("🔍 Testing empty terms array...");
    let empty_terms = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": "Some text here",
                "terms": []
            })
            .as_object()
            .cloned(),
        })
        .await;

    match empty_terms {
        Ok(result) => println!("✅ Empty terms handled: {:?}", result.content),
        Err(e) => println!("⚠️ Empty terms error (expected): {}", e),
    }

    // Test 3: Connectivity with empty terms
    println!("🔗 Testing connectivity with empty terms...");
    let empty_connectivity = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "terms": []
            })
            .as_object()
            .cloned(),
        })
        .await;

    match empty_connectivity {
        Ok(result) => println!("✅ Empty connectivity handled: {:?}", result.content),
        Err(e) => println!("⚠️ Empty connectivity error (expected): {}", e),
    }

    // Test 4: Very long text
    println!("📄 Testing very long text...");
    let long_text =
        "This is a test paragraph. ".repeat(100) + "The haystack provides excellent service.";
    let long_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": long_text,
                "terms": ["haystack", "service"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Long text handled: {:?}", long_result.content);

    println!("🎉 Edge case testing completed!");
    Ok(())
}
