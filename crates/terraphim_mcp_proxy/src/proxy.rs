use crate::{
    namespace::NamespaceManager, pool::McpServerPool, routing::ToolRouter, ContentItem,
    McpNamespace, McpProxyError, Result, Tool, ToolCallRequest, ToolCallResponse,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct McpProxy {
    /// Namespace manager
    namespace_manager: Arc<RwLock<NamespaceManager>>,
    /// Server connection pool
    server_pool: Arc<McpServerPool>,
    /// Tool router for directing calls
    #[allow(dead_code)]
    router: Arc<RwLock<ToolRouter>>,
}

impl Default for McpProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl McpProxy {
    /// Create a new MCP proxy
    pub fn new() -> Self {
        Self {
            namespace_manager: Arc::new(RwLock::new(NamespaceManager::new())),
            server_pool: Arc::new(McpServerPool::new()),
            router: Arc::new(RwLock::new(ToolRouter::new())),
        }
    }

    /// Create a proxy with a specific namespace
    pub async fn with_namespace(namespace: McpNamespace) -> Result<Self> {
        let proxy = Self::new();
        proxy.add_namespace(namespace).await?;
        Ok(proxy)
    }

    /// Add a namespace to the proxy
    pub async fn add_namespace(&self, mut namespace: McpNamespace) -> Result<()> {
        // Resolve environment variables in all server configs
        for server in &mut namespace.servers {
            server.resolve_env_vars()?;
        }

        // Add all servers to the pool
        for server in &namespace.servers {
            self.server_pool.add_server(server.clone()).await;
        }

        // Add namespace to manager
        let mut manager = self.namespace_manager.write().await;
        manager.add_namespace(namespace);

        Ok(())
    }

    /// Remove a namespace
    pub async fn remove_namespace(&self, name: &str) -> Result<()> {
        let mut manager = self.namespace_manager.write().await;
        let namespace = manager.remove_namespace(name).ok_or_else(|| {
            McpProxyError::Configuration(format!("Namespace not found: {}", name))
        })?;

        // Remove servers from pool
        for server in &namespace.servers {
            let _ = self.server_pool.remove_server(&server.name).await;
        }

        Ok(())
    }

    /// List all available tools from all namespaces
    ///
    /// Tools are prefixed with their server name (ServerName__toolName)
    /// and filtered based on namespace tool_overrides
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let manager = self.namespace_manager.read().await;
        let all_tools = Vec::new();

        // For now, return placeholder tools until we implement actual MCP client integration
        // TODO: Implement actual tool discovery from MCP servers
        log::info!("list_tools called - MCP client integration pending");

        for namespace in manager.get_enabled_namespaces() {
            for server in &namespace.servers {
                // Placeholder - in real implementation, we would call the MCP server here
                log::debug!(
                    "Would list tools from server '{}' in namespace '{}'",
                    server.name,
                    namespace.name
                );
            }
        }

        Ok(all_tools)
    }

    /// Call a tool by its prefixed name
    ///
    /// The tool name should be in the format: ServerName__toolName
    pub async fn call_tool(&self, request: ToolCallRequest) -> Result<ToolCallResponse> {
        // Parse the prefixed tool name to get server and tool
        let (server_name, tool_name) = crate::routing::parse_tool_name(&request.name)?;

        log::info!("Calling tool '{}' on server '{}'", tool_name, server_name);

        // Get server from pool
        let _server = self.server_pool.get_server(&server_name).await?;

        // TODO: Implement actual MCP client call here
        // For now, return a placeholder response
        log::warn!("MCP client integration pending - returning placeholder response");

        Ok(ToolCallResponse {
            content: vec![ContentItem::Text {
                text: format!(
                    "Placeholder response for tool '{}' on server '{}'",
                    tool_name, server_name
                ),
            }],
            is_error: false,
        })
    }

    /// Get proxy statistics
    pub async fn get_stats(&self) -> ProxyStats {
        let pool_stats = self.server_pool.get_pool_stats().await;
        let manager = self.namespace_manager.read().await;

        ProxyStats {
            total_namespaces: manager.count(),
            enabled_namespaces: manager.get_enabled_namespaces().len(),
            total_servers: pool_stats.total_servers,
            healthy_servers: pool_stats.healthy_servers,
            total_tool_calls: pool_stats.total_calls,
            failed_tool_calls: pool_stats.failed_calls,
        }
    }

    /// Get server pool reference
    pub fn server_pool(&self) -> &McpServerPool {
        &self.server_pool
    }

    /// List all namespaces
    pub async fn list_namespaces(&self) -> Vec<String> {
        let manager = self.namespace_manager.read().await;
        manager.list_namespaces()
    }

    /// Get a specific namespace
    pub async fn get_namespace(&self, name: &str) -> Option<McpNamespace> {
        let manager = self.namespace_manager.read().await;
        manager.get_namespace(name).cloned()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProxyStats {
    /// Total number of namespaces
    pub total_namespaces: usize,
    /// Number of enabled namespaces
    pub enabled_namespaces: usize,
    /// Total number of servers
    pub total_servers: usize,
    /// Number of healthy servers
    pub healthy_servers: usize,
    /// Total number of tool calls
    pub total_tool_calls: u64,
    /// Number of failed tool calls
    pub failed_tool_calls: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{McpNamespace, McpServerConfig};

    #[tokio::test]
    async fn test_proxy_creation() {
        let proxy = McpProxy::new();
        let stats = proxy.get_stats().await;
        assert_eq!(stats.total_namespaces, 0);
        assert_eq!(stats.total_servers, 0);
    }

    #[tokio::test]
    async fn test_add_namespace() {
        let proxy = McpProxy::new();

        let namespace = McpNamespace::new("test-namespace").add_server(McpServerConfig::stdio(
            "test-server",
            "test-cmd",
            vec![],
        ));

        proxy.add_namespace(namespace).await.unwrap();

        let stats = proxy.get_stats().await;
        assert_eq!(stats.total_namespaces, 1);
        assert_eq!(stats.total_servers, 1);

        let namespaces = proxy.list_namespaces().await;
        assert_eq!(namespaces.len(), 1);
        assert!(namespaces.contains(&"test-namespace".to_string()));
    }

    #[tokio::test]
    async fn test_remove_namespace() {
        let proxy = McpProxy::new();

        let namespace = McpNamespace::new("test-namespace").add_server(McpServerConfig::stdio(
            "test-server",
            "test-cmd",
            vec![],
        ));

        proxy.add_namespace(namespace).await.unwrap();
        assert_eq!(proxy.get_stats().await.total_namespaces, 1);

        proxy.remove_namespace("test-namespace").await.unwrap();
        assert_eq!(proxy.get_stats().await.total_namespaces, 0);
    }

    #[tokio::test]
    async fn test_tool_call_routing() {
        let proxy = McpProxy::new();

        let namespace = McpNamespace::new("test").add_server(McpServerConfig::stdio(
            "filesystem",
            "test-cmd",
            vec![],
        ));

        proxy.add_namespace(namespace).await.unwrap();

        let request = ToolCallRequest {
            name: "filesystem__read_file".to_string(),
            arguments: Some(serde_json::json!({"path": "/test/file.txt"})),
        };

        // This will return a placeholder response until MCP client is integrated
        let response = proxy.call_tool(request).await.unwrap();
        assert!(!response.is_error);
    }
}
