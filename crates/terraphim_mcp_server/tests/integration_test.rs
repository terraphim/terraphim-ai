use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
};
use std::process::Stdio;
use terraphim_config::ConfigBuilder;
use tokio::process::Command;

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

    // Command to run the server
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("--package")
        .arg("terraphim_mcp_server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()); // Also pipe stderr to see logs from the server
    
    Ok(command)
}

#[tokio::test]
async fn test_mcp_server_integration() -> Result<()> {
    let server_command = setup_server_command().await?;
    let transport = TokioChildProcess::new(server_command)?;

    let service = ().serve(transport).await?;
    println!("Connected to server: {:#?}", service.peer_info());

    // List tools
    let tools_result = service.list_tools(Default::default()).await?;
    println!("Available tools: {:#?}", tools_result);
    assert!(tools_result.tools.iter().any(|t| t.name == "search"));
    assert!(tools_result
        .tools
        .iter()
        .any(|t| t.name == "update_config_tool"));

    // Call search tool
    let search_params = serde_json::json!({
        "query": "test"
    });
    let search_result = service
        .call_tool(CallToolRequestParam {
            name: "search".into(),
            arguments: search_params.as_object().cloned(),
        })
        .await?;
    println!("Search result: {:#?}", search_result);
    assert!(!search_result.is_error.unwrap_or(false));
    if let Some(content) = search_result.content.first() {
         if let Some(text_content) = content.as_text() {
            assert!(text_content.text.contains("Found 0 documents"));
        }
    }

    // Call update_config_tool with a default server config
    let default_config = ConfigBuilder::new()
        .build_default_server()
        .build()
        .expect("Failed to build default server configuration");
    let config_str = serde_json::to_string(&default_config)?;
    let config_params = serde_json::json!({
        "config_str": config_str
    });
    let config_result = service
        .call_tool(CallToolRequestParam {
            name: "update_config_tool".into(),
            arguments: config_params.as_object().cloned(),
        })
        .await?;
    println!("Update config result: {:#?}", config_result);
    assert!(!config_result.is_error.unwrap_or(false));

    service.cancel().await?;
    Ok(())
} 