//! MCP client for connecting to external MCP servers via stdio.
//!
//! Wave 2 of the Hermes parity arc (epic #3160).

use super::McpError;
use rmcp::model::CallToolRequestParam;
use rmcp::service::ServiceExt;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;

/// MCP client connected to an external MCP server.
pub struct McpClient {
    service: rmcp::service::RunningService<rmcp::RoleClient, ()>,
}

impl McpClient {
    /// Connect to an external MCP server via stdio.
    pub async fn connect(command: &str, args: &[&str]) -> Result<Self, McpError> {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }

        let service =
            ().serve(TokioChildProcess::new(cmd)?)
                .await
                .map_err(|e| McpError::Client(e.to_string()))?;

        Ok(Self { service })
    }

    /// List tools available on the connected server.
    pub async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, McpError> {
        let result = self
            .service
            .list_tools(Default::default())
            .await
            .map_err(|e| McpError::Client(e.to_string()))?;
        Ok(result.tools)
    }

    /// Call a tool on the connected server.
    pub async fn call_tool(
        &self,
        name: impl Into<std::borrow::Cow<'static, str>>,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<rmcp::model::CallToolResult, McpError> {
        let result = self
            .service
            .call_tool(CallToolRequestParam {
                name: name.into(),
                arguments,
            })
            .await
            .map_err(|e| McpError::Client(e.to_string()))?;
        Ok(result)
    }

    /// Get server information.
    pub fn server_info(&self) -> Option<&rmcp::model::ServerInfo> {
        self.service.peer_info()
    }

    /// Gracefully disconnect.
    pub async fn disconnect(self) -> Result<(), McpError> {
        self.service
            .cancel()
            .await
            .map_err(|e| McpError::Client(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_connect_invalid_command() {
        let result = McpClient::connect("nonexistent-command-that-should-not-exist", &[]).await;
        assert!(result.is_err());
    }
}
