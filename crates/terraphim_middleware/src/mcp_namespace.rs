#[cfg(feature = "mcp-proxy")]
use terraphim_config::Role;
#[cfg(feature = "mcp-proxy")]
use terraphim_mcp_proxy::{McpProxy, Tool, ToolCallRequest, ToolCallResponse};

use crate::{Error, Result};

#[cfg(feature = "mcp-proxy")]
pub async fn list_namespace_tools(role: &Role) -> Result<Vec<Tool>> {
    let mut all_tools = Vec::new();

    for namespace in &role.mcp_namespaces {
        if !namespace.enabled {
            continue;
        }

        let proxy = McpProxy::with_namespace(namespace.clone())
            .await
            .map_err(|e| Error::Indexation(format!("Failed to create MCP proxy: {}", e)))?;

        let tools = proxy
            .list_tools()
            .await
            .map_err(|e| Error::Indexation(format!("Failed to list tools: {}", e)))?;

        all_tools.extend(tools);
    }

    Ok(all_tools)
}

#[cfg(feature = "mcp-proxy")]
pub async fn call_namespace_tool(
    role: &Role,
    tool_name: &str,
    arguments: Option<serde_json::Value>,
) -> Result<ToolCallResponse> {
    let (server_name, _tool) = terraphim_mcp_proxy::routing::parse_tool_name(tool_name)
        .map_err(|e| Error::Indexation(format!("Invalid tool name: {}", e)))?;

    for namespace in &role.mcp_namespaces {
        if !namespace.enabled {
            continue;
        }

        let has_server = namespace.servers.iter().any(|s| s.name == server_name);
        if !has_server {
            continue;
        }

        let proxy = McpProxy::with_namespace(namespace.clone())
            .await
            .map_err(|e| Error::Indexation(format!("Failed to create MCP proxy: {}", e)))?;

        let request = ToolCallRequest {
            name: tool_name.to_string(),
            arguments,
        };

        return proxy
            .call_tool(request)
            .await
            .map_err(|e| Error::Indexation(format!("Tool call failed: {}", e)));
    }

    Err(Error::Indexation(format!(
        "No namespace found for tool: {}",
        tool_name
    )))
}

#[cfg(not(feature = "mcp-proxy"))]
pub async fn list_namespace_tools(_role: &terraphim_config::Role) -> Result<Vec<()>> {
    Ok(Vec::new())
}

#[cfg(not(feature = "mcp-proxy"))]
pub async fn call_namespace_tool(
    _role: &terraphim_config::Role,
    tool_name: &str,
    _arguments: Option<serde_json::Value>,
) -> Result<()> {
    Err(Error::Indexation(format!(
        "MCP proxy feature not enabled. Cannot call tool: {}",
        tool_name
    )))
}
