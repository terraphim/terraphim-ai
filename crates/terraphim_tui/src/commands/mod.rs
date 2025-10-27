//! Enhanced command system with markdown-based command definitions
//!
//! This module provides support for defining commands in markdown files with YAML frontmatter,
//! supporting multiple execution modes (local, firecracker, hybrid) and knowledge graph-based validation.

pub mod markdown_parser;
pub mod registry;

use serde::{Deserialize, Serialize};

/// Execution mode for commands
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Execute locally on the host machine (safe commands only)
    Local,
    /// Execute in isolated Firecracker microVM
    Firecracker,
    /// Smart hybrid mode based on risk assessment
    Hybrid,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Local
    }
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
