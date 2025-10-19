//! Execution mode implementations for command execution
//!
//! This module provides different execution modes for commands:
//! - Local: Safe execution on host machine
//! - Firecracker: Isolated execution in microVMs
//! - Hybrid: Smart mode selection based on risk assessment

pub mod firecracker;
pub mod hybrid;
pub mod local;

pub use firecracker::FirecrackerExecutor;
pub use hybrid::HybridExecutor;
pub use local::LocalExecutor;

use super::{
    CommandDefinition, CommandExecutionError, CommandExecutionResult, ExecutionMode, ResourceUsage,
};

/// Trait for command executors
#[async_trait::async_trait]
pub trait CommandExecutor {
    /// Execute a command with the given definition and parameters
    async fn execute_command(
        &self,
        definition: &CommandDefinition,
        parameters: &std::collections::HashMap<String, String>,
    ) -> Result<CommandExecutionResult, CommandExecutionError>;

    /// Check if this executor supports the given execution mode
    fn supports_mode(&self, mode: &ExecutionMode) -> bool;

    /// Get executor capabilities
    fn capabilities(&self) -> ExecutorCapabilities;
}

/// Executor capabilities
#[derive(Debug, Clone)]
pub struct ExecutorCapabilities {
    /// Whether the executor supports resource limits
    pub supports_resource_limits: bool,
    /// Whether the executor supports network access
    pub supports_network_access: bool,
    /// Whether the executor supports file system access
    pub supports_file_system: bool,
    /// Maximum concurrent commands
    pub max_concurrent_commands: Option<usize>,
    /// Default timeout in seconds
    pub default_timeout: Option<u64>,
}

/// Create an executor for the given execution mode
pub fn create_executor(mode: ExecutionMode) -> Box<dyn CommandExecutor> {
    match mode {
        ExecutionMode::Local => Box::new(LocalExecutor::new()),
        ExecutionMode::Firecracker => Box::new(FirecrackerExecutor::new()),
        ExecutionMode::Hybrid => Box::new(HybridExecutor::new()),
    }
}

/// Default resource usage when no statistics are available
pub fn default_resource_usage() -> ResourceUsage {
    ResourceUsage {
        memory_mb: 0.0,
        cpu_time_seconds: 0.0,
        disk_mb: 0.0,
        network_bytes_sent: 0,
        network_bytes_received: 0,
    }
}
