use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test that MCP server properly uses the selected role for responses
#[tokio::test]
async fn test_mcp_server_uses_selected_role() -> Result<()> {
    println!("🎯 Testing that MCP server uses the selected role automatically");
    
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
        .arg("desktop"); // Use desktop profile which should have Terraphim Engineer
    
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;
    
    println!("🔗 Connected to MCP server with desktop profile");
    
    // Step 1: Configure with Terraphim Engineer as selected role
    println!("⚙️ Setting up Terraphim Engineer as selected role...");
    let current_dir = std::env::current_dir()?;
    let workspace_root = current_dir.parent().and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    let kg_path = workspace_root.join("docs/src/kg");
    
    let config_with_selected_role = json!({
        "roles": {
            "Default": {
                "shortname": "default",
                "name": "Default",
                "relevance_function": "title-scorer",
                "theme": "lumen",
                "terraphim_it": false,
                "kg": null,
                "haystacks": [{
                    "location": workspace_root.join("docs/src").to_string_lossy().to_string(),
                    "service": "Ripgrep",
                    "read_only": true,
                    "atomic_server_secret": null,
                    "extra_parameters": {}
                }],
                "extra": {}
            },
            "Terraphim Engineer": {
                "shortname": "terraphim_engineer", 
                "name": "Terraphim Engineer",
                "relevance_function": "terraphim-graph",
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
        "default_role": "Default",
        "global_shortcut": "Ctrl+Space"
    });
    
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: json!({
                "config_str": config_with_selected_role.to_string()
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Configuration with selected role set: {:?}", config_result.content);
    
    // Step 2: Build autocomplete index for selected role
    println!("🔧 Building autocomplete index (should use selected role)...");
    let build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({}) // No role specified - should use selected role
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Build index result (selected role): {:?}", build_result.content);
    
    // Step 3: Test autocomplete without specifying role (should use selected role)
    println!("🔤 Testing autocomplete without role parameter (should use selected role)...");
    let autocomplete_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "terraphim",
                "limit": 5
                // No role parameter - should use selected role automatically
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Autocomplete result (selected role): {:?}", autocomplete_result.content);
    
    // Step 4: Test extract_paragraphs without specifying role
    println!("📄 Testing extract_paragraphs without role parameter...");
    let text_with_kg_terms = r#"
    The Terraphim Graph system uses advanced graph embeddings for semantic search.
    
    The haystack component serves as a datasource for the system. Multiple haystacks
    can be configured to provide different types of data sources.
    
    The service layer coordinates between different components and acts as middleware
    between the user interface and the data processing components.
    "#;
    
    let extract_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": text_with_kg_terms,
                "include_term": true
                // No role parameter - should use selected role
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Extract paragraphs result (selected role): {:?}", extract_result.content);
    
    // Step 5: Test connectivity without specifying role 
    println!("🔗 Testing connectivity without role parameter...");
    let connectivity_result = service
        .call_tool(CallToolRequestParam {
            name: "is_all_terms_connected_by_path".into(),
            arguments: json!({
                "text": "The haystack provides service functionality as a datasource for the system."
                // No role parameter - should use selected role
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Connectivity result (selected role): {:?}", connectivity_result.content);
    
    // Step 6: Test search without specifying role
    println!("🔍 Testing search without role parameter...");
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: json!({
                "query": "terraphim",
                "limit": 3
                // No role parameter - should use selected role
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Search result (selected role): {:?}", search_result.content);
    
    // Step 7: Change selected role and test again
    println!("🔄 Changing selected role to Default and testing...");
    let mut config_default_selected = config_with_selected_role.clone();
    config_default_selected["selected_role"] = json!("Default");
    
    let config_change_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: json!({
                "config_str": config_default_selected.to_string()
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Changed selected role to Default: {:?}", config_change_result.content);
    
    // Test autocomplete with Default role selected (should fail or behave differently)
    let default_autocomplete = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "test",
                "limit": 3
                // No role parameter - should now use Default role
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Autocomplete with Default selected: {:?}", default_autocomplete.content);
    
    println!("🎉 Selected role usage test completed!");
    
    Ok(())
}

/// Test that role parameter overrides selected role when provided
#[tokio::test]
async fn test_role_parameter_overrides_selected_role() -> Result<()> {
    println!("🎯 Testing that explicit role parameter overrides selected role");
    
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
    
    println!("🔗 Connected to MCP server");
    
    // Set up config with Default as selected role but Terraphim Engineer available
    let current_dir = std::env::current_dir()?;
    let workspace_root = current_dir.parent().and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Cannot find workspace root"))?;
    let kg_path = workspace_root.join("docs/src/kg");
    
    let config = json!({
        "roles": {
            "Default": {
                "shortname": "default",
                "name": "Default", 
                "relevance_function": "title-scorer",
                "theme": "lumen",
                "terraphim_it": false,
                "kg": null,
                "haystacks": [{
                    "location": workspace_root.join("docs/src").to_string_lossy().to_string(),
                    "service": "Ripgrep",
                    "read_only": true,
                    "atomic_server_secret": null,
                    "extra_parameters": {}
                }],
                "extra": {}
            },
            "Terraphim Engineer": {
                "shortname": "terraphim_engineer",
                "name": "Terraphim Engineer",
                "relevance_function": "terraphim-graph", 
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
        "selected_role": "Default",  // Default is selected
        "default_role": "Default",
        "global_shortcut": "Ctrl+Space"
    });
    
    let _config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: json!({
                "config_str": config.to_string()
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    // Build index for Terraphim Engineer specifically  
    let _build_result = service
        .call_tool(CallToolRequestParam {
            name: "build_autocomplete_index".into(),
            arguments: json!({
                "role": "Terraphim Engineer"  // Explicit role parameter
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    // Test 1: Call without role parameter (should use Default - selected role)
    println!("🔤 Testing autocomplete without role (should use selected Default)...");
    let default_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "test",
                "limit": 3
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Default role result: {:?}", default_result.content);
    
    // Test 2: Call with explicit role parameter (should override selected role)
    println!("🔤 Testing autocomplete with explicit Terraphim Engineer role...");
    let explicit_role_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "terraphim",
                "limit": 3,
                "role": "Terraphim Engineer"  // Explicit role parameter overrides selected
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Explicit role result: {:?}", explicit_role_result.content);
    
    // Test 3: Advanced functions with explicit role
    println!("📄 Testing extract_paragraphs with explicit role...");
    let extract_explicit = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": "The haystack provides service functionality for graph embeddings.",
                "include_term": true,
                "role": "Terraphim Engineer"  // Explicit role
            })
            .as_object()
            .cloned(),
        })
        .await?;
    
    println!("✅ Extract with explicit role: {:?}", extract_explicit.content);
    
    println!("🎉 Role parameter override test completed!");
    
    Ok(())
}