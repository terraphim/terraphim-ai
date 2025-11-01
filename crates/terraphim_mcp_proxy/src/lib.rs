use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

#[cfg(not(feature = "json-schema"))]
use ahash::AHashMap as HashMap;

#[cfg(feature = "json-schema")]
use std::collections::HashMap;

pub mod middleware;
pub mod namespace;
pub mod pool;
pub mod proxy;
pub mod routing;

pub use middleware::*;
pub use namespace::*;
pub use pool::*;
pub use proxy::*;
pub use routing::*;

pub type Result<T> = std::result::Result<T, McpProxyError>;

#[derive(Error, Debug)]
pub enum McpProxyError {
    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid tool name format: {0}")]
    InvalidToolName(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("MCP protocol error: {0}")]
    Protocol(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
pub enum TransportType {
    Stdio,
    Sse,
    Http,
    OAuth,
}

impl Default for TransportType {
    fn default() -> Self {
        TransportType::Stdio
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(rename_all = "UPPERCASE")]
pub enum ToolStatus {
    Active,
    Inactive,
}

impl Default for ToolStatus {
    fn default() -> Self {
        ToolStatus::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct ToolOverride {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub status: ToolStatus,
}

impl Default for ToolOverride {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            status: ToolStatus::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct McpServerConfig {
    pub name: String,
    #[serde(default)]
    pub transport: TransportType,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub bearer_token: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

impl McpServerConfig {
    pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Stdio,
            command: Some(command.into()),
            args: Some(args),
            url: None,
            bearer_token: None,
            env: None,
        }
    }

    pub fn sse(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Sse,
            command: None,
            args: None,
            url: Some(url.into()),
            bearer_token: None,
            env: None,
        }
    }

    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: TransportType::Http,
            command: None,
            args: None,
            url: Some(url.into()),
            bearer_token: None,
            env: None,
        }
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn resolve_env_vars(&mut self) -> Result<()> {
        if let Some(env) = &mut self.env {
            let mut resolved = HashMap::default();
            for (key, value) in env.iter() {
                let resolved_value = resolve_env_string(value)?;
                resolved.insert(key.clone(), resolved_value);
            }
            self.env = Some(resolved);
        }

        if let Some(token) = &self.bearer_token {
            self.bearer_token = Some(resolve_env_string(token)?);
        }

        if let Some(url) = &self.url {
            self.url = Some(resolve_env_string(url)?);
        }

        Ok(())
    }
}

fn resolve_env_string(input: &str) -> Result<String> {
    let mut result = input.to_string();
    let re = regex::Regex::new(r"\$\{([^}]+)\}").map_err(|e| {
        McpProxyError::Configuration(format!("Invalid environment variable pattern: {}", e))
    })?;

    for cap in re.captures_iter(input) {
        let var_name = &cap[1];
        let value = std::env::var(var_name).map_err(|_| {
            McpProxyError::Configuration(format!("Environment variable not found: {}", var_name))
        })?;
        result = result.replace(&format!("${{{}}}", var_name), &value);
    }

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct McpNamespace {
    pub name: String,
    pub servers: Vec<McpServerConfig>,
    #[serde(default)]
    pub tool_overrides: HashMap<String, ToolOverride>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl McpNamespace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            servers: Vec::new(),
            tool_overrides: HashMap::default(),
            enabled: true,
        }
    }

    pub fn add_server(mut self, server: McpServerConfig) -> Self {
        self.servers.push(server);
        self
    }

    pub fn add_tool_override(
        mut self,
        tool_name: impl Into<String>,
        override_config: ToolOverride,
    ) -> Self {
        self.tool_overrides
            .insert(tool_name.into(), override_config);
        self
    }

    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        self.tool_overrides
            .get(tool_name)
            .map(|o| o.status == ToolStatus::Active)
            .unwrap_or(true) // Default to enabled if no override
    }

    pub fn get_tool_name(&self, original_name: &str) -> String {
        self.tool_overrides
            .get(original_name)
            .and_then(|o| o.name.as_ref())
            .cloned()
            .unwrap_or_else(|| original_name.to_string())
    }

    pub fn get_tool_description(&self, tool_name: &str, original_desc: &str) -> String {
        self.tool_overrides
            .get(tool_name)
            .and_then(|o| o.description.as_ref())
            .cloned()
            .unwrap_or_else(|| original_desc.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_creation() {
        let ns = McpNamespace::new("test-namespace").add_server(McpServerConfig::stdio(
            "filesystem",
            "npx",
            vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
            ],
        ));

        assert_eq!(ns.name, "test-namespace");
        assert_eq!(ns.servers.len(), 1);
        assert_eq!(ns.servers[0].name, "filesystem");
    }

    #[test]
    fn test_tool_override() {
        let ns = McpNamespace::new("test").add_tool_override(
            "filesystem__read_file",
            ToolOverride {
                name: Some("read_code".to_string()),
                description: Some("Read source code file".to_string()),
                status: ToolStatus::Active,
            },
        );

        assert_eq!(ns.get_tool_name("filesystem__read_file"), "read_code");
        assert_eq!(
            ns.get_tool_description("filesystem__read_file", "Original description"),
            "Read source code file"
        );
    }

    #[test]
    fn test_tool_status() {
        let ns = McpNamespace::new("test").add_tool_override(
            "filesystem__delete",
            ToolOverride {
                name: None,
                description: None,
                status: ToolStatus::Inactive,
            },
        );

        assert!(!ns.is_tool_enabled("filesystem__delete"));
        assert!(ns.is_tool_enabled("filesystem__read_file")); // Default enabled
    }
}
