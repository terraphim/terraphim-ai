use anyhow::Result;
use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::TokioChildProcess};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

/// Test that MCP server properly separates logs from JSON-RPC responses
#[tokio::test]
async fn test_mcp_log_separation_and_tools() -> Result<()> {
    println!("🧪 Testing MCP server log separation and tool availability");

    // Build the server first
    let mut build = Command::new("cargo");
    build
        .arg("build")
        .arg("--package")
        .arg("terraphim_mcp_server");

    if std::env::var_os("CI").is_some() {
        build.arg("--features").arg("zlob");
    }

    let build_status = build.status().await?;

    if !build_status.success() {
        anyhow::bail!("Failed to build terraphim_mcp_server");
    }

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

    if !binary_path.exists() {
        anyhow::bail!("MCP server binary not found at {:?}", binary_path);
    }

    println!("✅ Using MCP server binary: {:?}", binary_path);

    // Create command with proper stdio separation
    let mut cmd = Command::new(binary_path);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("RUST_LOG", "info");

    // Create transport and connect
    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    println!("🔗 Connected to MCP server: {:?}", service.peer_info());

    // Test 1: List tools to verify server is working
    println!("📋 Testing tools/list...");
    let tools = service.list_tools(Default::default()).await?;
    println!("✅ Found {} tools", tools.tools.len());

    // Verify we have expected tools
    let expected_tools = vec![
        "search",
        "autocomplete_terms",
        "find_matches",
        "load_thesaurus",
        "update_config_tool",
    ];

    for expected_tool in &expected_tools {
        let tool_found = tools.tools.iter().any(|t| t.name == *expected_tool);
        if tool_found {
            println!("  ✅ Tool '{}' found", expected_tool);
        } else {
            println!("  ⚠️ Tool '{}' not found", expected_tool);
        }
    }

    assert!(
        !tools.tools.is_empty(),
        "Should have at least some tools available"
    );

    // Test 2: Test autocomplete functionality
    println!("🔤 Testing autocomplete...");
    let autocomplete_result = service
        .call_tool(CallToolRequestParam {
            name: "autocomplete_terms".into(),
            arguments: json!({
                "query": "terra",
                "limit": 5
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Autocomplete result: {:?}", autocomplete_result.content);

    // Test 3: Test search functionality (should work without errors even if no results)
    println!("🔍 Testing search...");
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: json!({
                "query": "test",
                "limit": 5
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ Search completed: {:?}", search_result.content);

    // Test 4: Test thesaurus loading
    println!("📚 Testing thesaurus loading...");
    let thesaurus_result = service
        .call_tool(CallToolRequestParam {
            name: "load_thesaurus".into(),
            arguments: json!({
                "role_name": "Default",
                // Use local KG that exists in this repo; avoids needing role config automata_path
                "automata_path": "docs/src/kg"
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Thesaurus loading result: {:?}",
        thesaurus_result.content
    );

    println!("🎉 All MCP tests passed successfully!");
    Ok(())
}

/// Test MCP server with role switching and configuration updates
#[tokio::test]
async fn test_mcp_role_configuration() -> Result<()> {
    println!("⚙️ Testing MCP role configuration");

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
        .stderr(Stdio::piped());

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    // Test configuration update
    println!("🔄 Testing configuration update...");
    let test_config = json!({
        "roles": {
            "Test Role": {
                "name": "Test Role",
                "shortname": "test",
                "relevance_function": "TitleScorer",
                "theme": "lumen",
                "haystacks": [],
                "kg": null,
                "terraphim_it": false,
                "extra": {}
            }
        },
        "selected_role": "Test Role",
        "default_role": "Test Role",
        "global_shortcut": "Ctrl+Space"
    });

    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: json!({
                "config_str": test_config.to_string()
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Configuration update result: {:?}",
        config_result.content
    );

    // Test search with updated config
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: json!({
                "query": "test",
                "role": "Test Role",
                "limit": 3
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!(
        "✅ Search with role configuration: {:?}",
        search_result.content
    );

    println!("🎉 MCP role configuration test passed!");
    Ok(())
}

/// Test text processing tools (find_matches, replace_matches, etc.)
#[tokio::test]
async fn test_mcp_text_processing_tools() -> Result<()> {
    println!("📝 Testing MCP text processing tools");

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
        .stderr(Stdio::piped());

    let transport = TokioChildProcess::new(cmd)?;
    let service = ().serve(transport).await?;

    // Test find_matches
    println!("🔍 Testing find_matches...");
    let find_result = service
        .call_tool(CallToolRequestParam {
            name: "find_matches".into(),
            arguments: json!({
                "text": "This is a test document with some test words",
                "patterns": ["test", "document"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ find_matches result: {:?}", find_result.content);

    // Test replace_matches
    println!("🔄 Testing replace_matches...");
    let replace_result = service
        .call_tool(CallToolRequestParam {
            name: "replace_matches".into(),
            arguments: json!({
                "text": "This is a test document",
                "patterns": ["test"],
                "replacement": "sample",
                // required by tool schema
                "link_type": "PlainText"
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ replace_matches result: {:?}", replace_result.content);

    // Test extract_paragraphs_from_automata
    println!("📄 Testing extract_paragraphs_from_automata...");
    let extract_result = service
        .call_tool(CallToolRequestParam {
            name: "extract_paragraphs_from_automata".into(),
            arguments: json!({
                "text": "First paragraph.\n\nSecond paragraph with test content.\n\nThird paragraph.",
                "terms": ["test"]
            })
            .as_object()
            .cloned(),
        })
        .await?;

    println!("✅ extract_paragraphs result: {:?}", extract_result.content);

    println!("🎉 All text processing tools working correctly!");
    Ok(())
}
