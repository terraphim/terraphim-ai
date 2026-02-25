use anyhow::Result;
use rmcp::{ServiceExt, model::CallToolRequestParam, transport::TokioChildProcess};
use std::{path::PathBuf, process::Stdio};
use tokio::process::Command;

/// Launch desktop binary in MCP server mode and return a transport for the client.
///
/// This repo no longer builds desktop; integration is exercised when an external
/// desktop binary is provided via TERRAPHIM_DESKTOP_BINARY.
async fn launch_desktop_mcp_server() -> Result<Option<TokioChildProcess>> {
    let binary_path = resolve_desktop_binary()?;
    let Some(binary_path) = binary_path else {
        eprintln!(
            "Skipping desktop MCP integration test: set TERRAPHIM_DESKTOP_BINARY to external terraphim-ai-desktop binary"
        );
        return Ok(None);
    };

    let mut cmd = Command::new(binary_path);
    cmd.arg("mcp-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    Ok(Some(TokioChildProcess::new(cmd)?))
}

fn resolve_desktop_binary() -> Result<Option<PathBuf>> {
    if let Ok(path) = std::env::var("TERRAPHIM_DESKTOP_BINARY") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(Some(path));
        }
        anyhow::bail!(
            "TERRAPHIM_DESKTOP_BINARY is set but file does not exist: {}",
            path.display()
        );
    }

    let crate_dir = std::env::current_dir()?;
    let binary_name = if cfg!(target_os = "windows") {
        "terraphim-ai-desktop.exe"
    } else {
        "terraphim-ai-desktop"
    };

    let candidate_paths = [
        crate_dir
            .parent()
            .and_then(|p| p.parent())
            .map(|workspace| workspace.join("target").join("debug").join(binary_name)),
        Some(crate_dir.join("target").join("debug").join(binary_name)),
    ];

    Ok(candidate_paths.into_iter().flatten().find(|p| p.exists()))
}

#[tokio::test]
async fn test_desktop_mcp_server_basic_search() -> Result<()> {
    let Some(transport) = launch_desktop_mcp_server().await? else {
        return Ok(());
    };
    let service = ().serve(transport).await?;

    let tools = service.list_tools(Default::default()).await?;
    assert!(!tools.tools.is_empty(), "No tools exposed by server");

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

    assert!(
        !search_result.is_error.unwrap_or(false),
        "Search reported error"
    );

    service.cancel().await?;
    Ok(())
}
