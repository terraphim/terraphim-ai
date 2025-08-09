use anyhow::Result;
use rmcp::{ServiceExt, model::CallToolRequestParam, transport::TokioChildProcess};
use tokio::process::Command;
use std::process::Stdio;

/// Build and launch the desktop binary in MCP server mode and return a transport for the client.
async fn launch_desktop_mcp_server() -> Result<TokioChildProcess> {
    // Ensure the desktop crate is built
    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--package")
        .arg("terraphim-ai-desktop")
        .status()
        .await?;

    if !build_status.success() {
        anyhow::bail!("Failed to build terraphim-ai-desktop");
    }

    // Locate the compiled binary: same logic as other integration tests
    let crate_dir = std::env::current_dir()?;
    let binary_name = if cfg!(target_os = "windows") {
        if cfg!(debug_assertions) { "terraphim-ai-desktop.exe" } else { "terraphim-ai-desktop.exe" }
    } else {
        "terraphim-ai-desktop"
    };

    let candidate_paths = [
        crate_dir.parent().and_then(|p| p.parent()).map(|workspace| workspace.join("target").join("debug").join(binary_name)),
        Some(crate_dir.join("target").join("debug").join(binary_name)),
    ];

    let binary_path = candidate_paths.into_iter().flatten().find(|p| p.exists()).ok_or_else(|| anyhow::anyhow!("Desktop binary not found in expected locations"))?;

    // Spawn the desktop binary with the `mcp-server` subcommand
    let mut cmd = Command::new(binary_path);
    cmd.arg("mcp-server").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());

    Ok(TokioChildProcess::new(cmd)?)
}

#[tokio::test]
#[ignore]
async fn test_desktop_mcp_server_basic_search() -> Result<()> {
    let transport = launch_desktop_mcp_server().await?;
    let service = ().serve(transport).await?;

    // Basic sanity: list tools
    let tools = service.list_tools(Default::default()).await?;
    assert!(!tools.tools.is_empty(), "No tools exposed by server");

    // Perform a simple search
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: serde_json::json!({
                "query": "terraphim",
                "limit": 3
            })
            .as_object()
            .cloned(),
        })
        .await?;

    assert!(!search_result.is_error.unwrap_or(false), "Search reported error");

    // Terminate server
    service.cancel().await?;
    Ok(())
} 