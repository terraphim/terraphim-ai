//! Enhanced command system with markdown-based command definitions
//!
//! This module provides support for defining commands in markdown files with YAML frontmatter,
//! supporting multiple execution modes (local, firecracker, hybrid) and knowledge graph-based validation.

pub mod executor;
pub mod hooks;
pub mod markdown_parser;
pub mod registry;
pub mod validator;

// Re-export main types for easier access
pub use executor::CommandExecutor;
pub use registry::CommandRegistry;
pub use validator::CommandValidator;

#[cfg(test)]
mod tests;

#[cfg(feature = "repl")]
pub mod modes;

use serde::{Deserialize, Serialize};

/// Execution mode for commands
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Execute locally on the host machine (safe commands only)
    #[default]
    Local,
    /// Execute in isolated Firecracker microVM
    Firecracker,
    /// Smart hybrid mode based on risk assessment
    Hybrid,
}

/// Command parameter definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, etc.)
    #[serde(rename = "type")]
    pub param_type: String,
    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,
    /// Parameter description
    pub description: Option<String>,
    /// Default value if not provided
    #[serde(default)]
    pub default_value: Option<serde_json::Value>,
    /// Validation rules
    #[serde(default)]
    pub validation: Option<ParameterValidation>,
}

/// Parameter validation rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParameterValidation {
    /// Minimum value for numbers
    pub min: Option<f64>,
    /// Maximum value for numbers
    pub max: Option<f64>,
    /// Allowed values (enum)
    pub allowed_values: Option<Vec<String>>,
    /// Regex pattern for string validation
    pub pattern: Option<String>,
}

/// Command definition from markdown frontmatter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CommandDefinition {
    /// Command name (slug)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Command usage example
    pub usage: Option<String>,
    /// Command parameters
    #[serde(default)]
    pub parameters: Vec<CommandParameter>,
    /// Execution mode requirement
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    /// Required permissions
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Required knowledge graph concepts
    #[serde(default)]
    pub knowledge_graph_required: Vec<String>,
    /// Command category
    pub category: Option<String>,
    /// Command version
    #[serde(default = "default_version")]
    pub version: String,
    /// Command namespace (for organization)
    pub namespace: Option<String>,
    /// Aliases for this command
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Risk level (low, medium, high)
    #[serde(default = "default_risk_level")]
    pub risk_level: RiskLevel,
    /// Timeout in seconds
    #[serde(default)]
    pub timeout: Option<u64>,
    /// Resource limits
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_risk_level() -> RiskLevel {
    RiskLevel::Low
}

/// Risk assessment level
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Resource limits for command execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    /// Maximum memory in MB
    pub max_memory_mb: Option<u64>,
    /// Maximum CPU time in seconds
    pub max_cpu_time: Option<u64>,
    /// Maximum disk usage in MB
    pub max_disk_mb: Option<u64>,
    /// Network access allowed
    #[serde(default)]
    pub network_access: bool,
}

/// Parsed command with both definition and content
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// Command metadata from frontmatter
    pub definition: CommandDefinition,
    /// Command description/content from markdown
    pub content: String,
    /// File path where command was defined
    pub source_path: std::path::PathBuf,
    /// Last modified timestamp
    pub modified: std::time::SystemTime,
}

/// Command validation error
#[derive(Debug, thiserror::Error)]
pub enum CommandValidationError {
    #[error("Command '{0}' not found")]
    CommandNotFound(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter '{0}': {1}")]
    InvalidParameter(String, String),

    #[error("Insufficient permissions for command '{0}'")]
    InsufficientPermissions(String),

    #[error("Command '{0}' requires knowledge graph concepts: {1:?}")]
    MissingKnowledgeGraphConcepts(String, Vec<String>),

    #[error("Execution mode '{0}' not available for command '{1}'")]
    ExecutionModeUnavailable(String, String),

    #[error("Command '{0}' exceeds risk level for current role")]
    RiskLevelExceeded(String),

    #[error("Command validation failed: {0}")]
    ValidationFailed(String),
}

/// Command execution error
#[derive(Debug, thiserror::Error)]
pub enum CommandExecutionError {
    #[error("Command '{0}' execution failed: {1}")]
    ExecutionFailed(String, String),

    #[error("VM execution error: {0}")]
    VmExecutionError(String),

    #[error("Local execution error: {0}")]
    LocalExecutionError(String),

    #[error("Command timeout after {0} seconds")]
    Timeout(u64),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Command execution was cancelled")]
    Cancelled,

    #[error("Pre-command hook failed: {0}")]
    PreHookFailed(String),

    #[error("Post-command hook failed: {0}")]
    PostHookFailed(String),
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandExecutionResult {
    /// Command that was executed
    pub command: String,
    /// Execution mode used
    pub execution_mode: ExecutionMode,
    /// Exit code (0 for success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Resource usage statistics
    pub resource_usage: Option<ResourceUsage>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU time in seconds
    pub cpu_time_seconds: f64,
    /// Disk usage in MB
    pub disk_mb: f64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
}

/// Command registry error
#[derive(Debug, thiserror::Error)]
pub enum CommandRegistryError {
    #[error("Failed to parse command file '{0}': {1}")]
    ParseError(String, String),

    #[error("Invalid frontmatter in '{0}': {1}")]
    InvalidFrontmatter(String, String),

    #[error("Duplicate command definition: {0}")]
    DuplicateCommand(String),

    #[error("Command file not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

/// Hook execution context
#[derive(Debug, Clone)]
pub struct HookContext {
    pub command: String,
    pub parameters: std::collections::HashMap<String, String>,
    pub user: String,
    pub role: String,
    pub execution_mode: ExecutionMode,
    pub working_directory: std::path::PathBuf,
}

/// Hook execution result
#[derive(Debug, Clone)]
pub struct HookResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub should_continue: bool, // Whether main command should continue
}

/// Command hook interface
#[async_trait::async_trait]
pub trait CommandHook {
    /// Hook name
    fn name(&self) -> &str;

    /// Hook priority (higher numbers run first)
    fn priority(&self) -> i32 {
        0
    }

    /// Execute the hook
    async fn execute(&self, context: &HookContext) -> Result<HookResult, CommandExecutionError>;
}

/// Hook manager for organizing and executing hooks
pub struct HookManager {
    pre_hooks: Vec<Box<dyn CommandHook + Send + Sync>>,
    post_hooks: Vec<Box<dyn CommandHook + Send + Sync>>,
}

impl HookManager {
    /// Create a new hook manager
    pub fn new() -> Self {
        Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
        }
    }

    /// Add a pre-command hook
    pub fn add_pre_hook(&mut self, hook: Box<dyn CommandHook + Send + Sync>) {
        self.pre_hooks.push(hook);
        self.pre_hooks
            .sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// Add a post-command hook
    pub fn add_post_hook(&mut self, hook: Box<dyn CommandHook + Send + Sync>) {
        self.post_hooks.push(hook);
        self.post_hooks
            .sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// Execute all pre-command hooks
    pub async fn execute_pre_hooks(
        &self,
        context: &HookContext,
    ) -> Result<(), CommandExecutionError> {
        for hook in &self.pre_hooks {
            match hook.execute(context).await {
                Ok(result) => {
                    if !result.should_continue {
                        return Err(CommandExecutionError::PreHookFailed(format!(
                            "Pre-hook '{}' blocked execution: {}",
                            hook.name(),
                            result.message
                        )));
                    }
                }
                Err(e) => {
                    return Err(CommandExecutionError::PreHookFailed(format!(
                        "Pre-hook '{}' failed: {}",
                        hook.name(),
                        e
                    )));
                }
            }
        }
        Ok(())
    }

    /// Execute all post-command hooks
    pub async fn execute_post_hooks(
        &self,
        context: &HookContext,
        _result: &CommandExecutionResult,
    ) -> Result<(), CommandExecutionError> {
        for hook in &self.post_hooks {
            match hook.execute(context).await {
                Ok(_) => {
                    // Post hooks can't stop execution but we log failures
                }
                Err(e) => {
                    return Err(CommandExecutionError::PostHookFailed(format!(
                        "Post-hook '{}' failed: {}",
                        hook.name(),
                        e
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistryError {
    pub fn parse_error(path: impl AsRef<std::path::Path>, error: impl Into<String>) -> Self {
        CommandRegistryError::ParseError(path.as_ref().to_string_lossy().to_string(), error.into())
    }

    pub fn invalid_frontmatter(
        path: impl AsRef<std::path::Path>,
        error: impl Into<String>,
    ) -> Self {
        CommandRegistryError::InvalidFrontmatter(
            path.as_ref().to_string_lossy().to_string(),
            error.into(),
        )
    }
}
