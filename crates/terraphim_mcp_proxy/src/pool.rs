use crate::{McpProxyError, McpServerConfig, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(not(feature = "json-schema"))]
use ahash::HashMap as HashMap;

#[cfg(feature = "json-schema")]
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerHealth {
    /// Server is healthy and responding
    Healthy,
    /// Server connection is being established
    Connecting,
    /// Server connection failed
    ConnectionFailed,
    /// Server is not responding
    Unresponsive,
    /// Server returned an error
    Error,
}

impl Default for ServerHealth {
    fn default() -> Self {
        ServerHealth::Connecting
    }
}

#[derive(Debug, Clone, Default)]
pub struct ServerStats {
    /// Total number of tool calls
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of failed calls
    pub failed_calls: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Last error message if any
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct McpServerConnection {
    /// Server configuration
    pub config: McpServerConfig,
    /// Current health status
    pub health: ServerHealth,
    /// Connection statistics
    pub stats: ServerStats,
    /// Last health check timestamp
    pub last_health_check: Option<std::time::Instant>,
    /// Number of active connections
    pub active_connections: usize,
}

impl McpServerConnection {
    /// Create a new server connection
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            health: ServerHealth::Connecting,
            stats: ServerStats::default(),
            last_health_check: None,
            active_connections: 0,
        }
    }

    /// Record a successful call
    pub fn record_success(&mut self, response_time_ms: f64) {
        self.stats.total_calls += 1;
        self.stats.successful_calls += 1;

        // Update rolling average
        let total = self.stats.successful_calls as f64;
        self.stats.avg_response_time_ms =
            (self.stats.avg_response_time_ms * (total - 1.0) + response_time_ms) / total;

        self.health = ServerHealth::Healthy;
    }

    /// Record a failed call
    pub fn record_failure(&mut self, error: String) {
        self.stats.total_calls += 1;
        self.stats.failed_calls += 1;
        self.stats.last_error = Some(error);
        self.health = ServerHealth::Error;
    }

    /// Check if server needs health check
    pub fn needs_health_check(&self, interval_secs: u64) -> bool {
        match self.last_health_check {
            None => true,
            Some(last_check) => last_check.elapsed().as_secs() >= interval_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpServerPool {
    /// Map of server name to connection
    connections: Arc<RwLock<HashMap<String, McpServerConnection>>>,
    /// Health check interval in seconds
    health_check_interval: u64,
    /// Maximum number of idle connections per server
    #[allow(dead_code)]
    max_idle_connections: usize,
}

impl Default for McpServerPool {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServerPool {
    /// Create a new server pool with default settings
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::default())),
            health_check_interval: 60, // 60 seconds
            max_idle_connections: 5,
        }
    }

    /// Create a server pool with custom settings
    pub fn with_config(health_check_interval: u64, max_idle_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::default())),
            health_check_interval,
            max_idle_connections,
        }
    }

    /// Add or update a server in the pool
    pub async fn add_server(&self, config: McpServerConfig) {
        let mut connections = self.connections.write().await;
        let server_name = config.name.clone();

        connections.insert(server_name, McpServerConnection::new(config));
    }

    /// Remove a server from the pool
    pub async fn remove_server(&self, server_name: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        connections
            .remove(server_name)
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))?;
        Ok(())
    }

    /// Get server connection info
    pub async fn get_server(&self, server_name: &str) -> Result<McpServerConnection> {
        let connections = self.connections.read().await;
        connections
            .get(server_name)
            .cloned()
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))
    }

    /// Update server health status
    pub async fn update_health(&self, server_name: &str, health: ServerHealth) -> Result<()> {
        let mut connections = self.connections.write().await;
        let conn = connections
            .get_mut(server_name)
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))?;

        conn.health = health;
        conn.last_health_check = Some(std::time::Instant::now());
        Ok(())
    }

    /// Record a successful call
    pub async fn record_success(&self, server_name: &str, response_time_ms: f64) -> Result<()> {
        let mut connections = self.connections.write().await;
        let conn = connections
            .get_mut(server_name)
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))?;

        conn.record_success(response_time_ms);
        Ok(())
    }

    /// Record a failed call
    pub async fn record_failure(&self, server_name: &str, error: impl Into<String>) -> Result<()> {
        let mut connections = self.connections.write().await;
        let conn = connections
            .get_mut(server_name)
            .ok_or_else(|| McpProxyError::ServerNotFound(server_name.to_string()))?;

        conn.record_failure(error.into());
        Ok(())
    }

    /// Get all server names
    pub async fn list_servers(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Get servers that need health checks
    pub async fn get_servers_needing_health_check(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, conn)| conn.needs_health_check(self.health_check_interval))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        let connections = self.connections.read().await;
        let total_servers = connections.len();
        let healthy_servers = connections
            .values()
            .filter(|c| c.health == ServerHealth::Healthy)
            .count();
        let total_calls: u64 = connections.values().map(|c| c.stats.total_calls).sum();
        let failed_calls: u64 = connections.values().map(|c| c.stats.failed_calls).sum();

        PoolStats {
            total_servers,
            healthy_servers,
            total_calls,
            failed_calls,
        }
    }

    /// Clear all connections
    pub async fn clear(&self) {
        let mut connections = self.connections.write().await;
        connections.clear();
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of servers
    pub total_servers: usize,
    /// Number of healthy servers
    pub healthy_servers: usize,
    /// Total number of calls across all servers
    pub total_calls: u64,
    /// Total number of failed calls
    pub failed_calls: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_pool() {
        let pool = McpServerPool::new();

        let config = McpServerConfig::stdio("test-server", "test-cmd", vec![]);
        pool.add_server(config).await;

        let servers = pool.list_servers().await;
        assert_eq!(servers.len(), 1);
        assert!(servers.contains(&"test-server".to_string()));

        let server = pool.get_server("test-server").await.unwrap();
        assert_eq!(server.config.name, "test-server");
        assert_eq!(server.health, ServerHealth::Connecting);
    }

    #[tokio::test]
    async fn test_record_stats() {
        let pool = McpServerPool::new();

        let config = McpServerConfig::stdio("test-server", "test-cmd", vec![]);
        pool.add_server(config).await;

        pool.record_success("test-server", 100.0).await.unwrap();
        pool.record_success("test-server", 200.0).await.unwrap();
        pool.record_failure("test-server", "test error")
            .await
            .unwrap();

        let server = pool.get_server("test-server").await.unwrap();
        assert_eq!(server.stats.total_calls, 3);
        assert_eq!(server.stats.successful_calls, 2);
        assert_eq!(server.stats.failed_calls, 1);
        assert_eq!(server.stats.avg_response_time_ms, 150.0);
        assert_eq!(server.health, ServerHealth::Error);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let pool = McpServerPool::new();

        pool.add_server(McpServerConfig::stdio("server1", "cmd1", vec![]))
            .await;
        pool.add_server(McpServerConfig::stdio("server2", "cmd2", vec![]))
            .await;

        pool.record_success("server1", 100.0).await.unwrap();
        pool.record_failure("server2", "error").await.unwrap();

        pool.update_health("server1", ServerHealth::Healthy)
            .await
            .unwrap();

        let stats = pool.get_pool_stats().await;
        assert_eq!(stats.total_servers, 2);
        assert_eq!(stats.healthy_servers, 1);
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.failed_calls, 1);
    }
}
