//! Tool registry and implementations for TinyClaw agent.

pub mod agent_spawn;
pub mod cron;
pub mod edit;
pub mod filesystem;
pub mod session_tools;
pub mod shell;
pub mod voice_transcribe;
pub mod web;

use crate::bus::OutboundMessage;
use crate::config::{CronConfig, SpawnerConfig, ToolsConfig};
use crate::session::SessionManager;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, mpsc};

/// A tool call request from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Response from an LLM that may include tool calls.
#[derive(Debug)]
pub struct LlmToolResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub model: String,
    pub stop_reason: String,
}

/// Errors that can occur during tool execution.
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{tool}' not found")]
    NotFound { tool: String },

    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArguments { tool: String, message: String },

    #[error("Tool '{tool}' execution failed: {message}")]
    ExecutionFailed { tool: String, message: String },

    #[error("Tool '{tool}' was blocked: {reason}")]
    Blocked { tool: String, reason: String },

    #[error("Tool '{tool}' timed out after {seconds}s")]
    Timeout { tool: String, seconds: u64 },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Tool interface for agent capabilities.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str;

    /// Get the JSON Schema for tool parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
}

/// Registry of available tools with JSON Schema export.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        log::info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Check if a tool exists.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Execute a tool call.
    pub async fn execute(&self, call: &ToolCall) -> Result<String, ToolError> {
        let tool = self.get(&call.name).ok_or_else(|| ToolError::NotFound {
            tool: call.name.clone(),
        })?;

        tool.execute(call.arguments.clone()).await
    }

    /// Export all tools as OpenAI/Anthropic format tool definitions.
    pub fn to_openai_tools(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.parameters_schema(),
                    }
                })
            })
            .collect()
    }

    /// List all registered tool names.
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get the number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a standard tool registry with all default tools.
pub fn create_default_registry() -> ToolRegistry {
    create_registry_from_config(&ToolsConfig::default())
}

/// Create a tool registry using tool-specific config overrides.
pub fn create_registry_from_config(tools_cfg: &ToolsConfig) -> ToolRegistry {
    create_registry_from_config_with_runtime_and_orchestration(
        tools_cfg,
        &SpawnerConfig::default(),
        &CronConfig::default(),
        None,
        None,
        None,
    )
}

/// Create a tool registry with optional runtime context for session-aware tools.
pub fn create_registry_from_config_with_runtime(
    tools_cfg: &ToolsConfig,
    sessions: Option<Arc<Mutex<SessionManager>>>,
    outbound_tx: Option<mpsc::Sender<OutboundMessage>>,
    workspace: Option<PathBuf>,
) -> ToolRegistry {
    create_registry_from_config_with_runtime_and_orchestration(
        tools_cfg,
        &SpawnerConfig::default(),
        &CronConfig::default(),
        sessions,
        outbound_tx,
        workspace,
    )
}

/// Create a tool registry with optional runtime context and orchestration config.
pub fn create_registry_from_config_with_runtime_and_orchestration(
    tools_cfg: &ToolsConfig,
    spawner_cfg: &SpawnerConfig,
    cron_cfg: &CronConfig,
    sessions: Option<Arc<Mutex<SessionManager>>>,
    outbound_tx: Option<mpsc::Sender<OutboundMessage>>,
    workspace: Option<PathBuf>,
) -> ToolRegistry {
    use crate::tools::agent_spawn::AgentSpawnTool;
    use crate::tools::cron::CronTool;
    use crate::tools::edit::EditTool;
    use crate::tools::filesystem::FilesystemTool;
    use crate::tools::session_tools::{SessionsHistoryTool, SessionsListTool, SessionsSendTool};
    use crate::tools::shell::ShellTool;
    use crate::tools::voice_transcribe::VoiceTranscribeTool;
    use crate::tools::web::{WebFetchTool, WebSearchTool};

    let shell_timeout = tools_cfg
        .shell
        .as_ref()
        .map(|cfg| cfg.timeout_seconds)
        .unwrap_or(120);

    let web_provider = tools_cfg
        .web
        .as_ref()
        .and_then(|cfg| cfg.search_provider.as_deref())
        .unwrap_or("brave")
        .to_string();
    let web_search_api_key = tools_cfg.web.as_ref().and_then(|cfg| cfg.api_key.clone());
    let web_base_url = tools_cfg.web.as_ref().and_then(|cfg| cfg.base_url.clone());

    let web_fetch_mode = tools_cfg
        .web
        .as_ref()
        .and_then(|cfg| cfg.fetch_mode.as_deref())
        .unwrap_or("raw")
        .to_string();

    let voice_config = tools_cfg.voice.clone().unwrap_or_default();
    let spawn_workdir =
        workspace.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FilesystemTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry.register(Box::new(ShellTool::with_timeout(shell_timeout)));
    registry.register(Box::new(WebSearchTool::with_config(
        web_provider,
        web_base_url,
        web_search_api_key,
    )));
    registry.register(Box::new(WebFetchTool::with_mode(web_fetch_mode)));
    registry.register(Box::new(VoiceTranscribeTool::with_config(voice_config)));
    registry.register(Box::new(AgentSpawnTool::with_config(
        spawn_workdir.clone(),
        spawner_cfg.clone(),
    )));

    if let Some(sessions) = sessions {
        registry.register(Box::new(SessionsListTool::new(sessions.clone())));
        registry.register(Box::new(SessionsHistoryTool::new(sessions.clone())));
        if let Some(outbound_tx) = outbound_tx {
            registry.register(Box::new(SessionsSendTool::new(
                sessions.clone(),
                outbound_tx.clone(),
            )));
            registry.register(Box::new(CronTool::new(
                cron_cfg.clone(),
                spawn_workdir,
                sessions,
                outbound_tx,
            )));
        }
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CronConfig, ShellToolConfig, SpawnerConfig, WebToolsConfig};
    use crate::session::SessionManager;
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }

        fn description(&self) -> &str {
            "A mock tool for testing"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "value": { "type": "string" }
                }
            })
        }

        async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
            Ok(format!("Mock result: {}", args))
        }
    }

    #[test]
    fn test_tool_registry_register_and_get() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool));

        assert!(registry.get("mock").is_some());
        assert!(registry.get("other").is_none());
    }

    #[test]
    fn test_tool_registry_schema_export() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool));

        let tools = registry.to_openai_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["function"]["name"], "mock");
    }

    #[tokio::test]
    async fn test_tool_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(MockTool));

        let call = ToolCall {
            id: "call_1".to_string(),
            name: "mock".to_string(),
            arguments: serde_json::json!({"value": "test"}),
        };

        let result = registry.execute(&call).await.unwrap();
        assert!(result.contains("Mock result"));
    }

    #[tokio::test]
    async fn test_tool_registry_not_found() {
        let registry = ToolRegistry::new();

        let call = ToolCall {
            id: "call_1".to_string(),
            name: "nonexistent".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = registry.execute(&call).await;
        assert!(matches!(result, Err(ToolError::NotFound { .. })));
    }

    #[test]
    fn test_create_registry_from_config_registers_expected_tools() {
        let config = ToolsConfig {
            shell: Some(ShellToolConfig {
                timeout_seconds: 30,
                deny_patterns: vec![],
            }),
            web: Some(WebToolsConfig {
                search_provider: Some("searxng".to_string()),
                fetch_mode: Some("readability".to_string()),
                api_key: Some("api-key".to_string()),
                base_url: Some("https://search.example.com".to_string()),
            }),
            voice: None,
        };

        let registry = create_registry_from_config(&config);
        assert!(registry.has("filesystem"));
        assert!(registry.has("edit"));
        assert!(registry.has("shell"));
        assert!(registry.has("web_search"));
        assert!(registry.has("web_fetch"));
        assert!(registry.has("voice_transcribe"));
        assert!(registry.has("agent_spawn"));
    }

    #[tokio::test]
    async fn test_create_registry_from_config_applies_shell_timeout() {
        let config = ToolsConfig {
            shell: Some(ShellToolConfig {
                timeout_seconds: 1,
                deny_patterns: vec![],
            }),
            web: None,
            voice: None,
        };
        let registry = create_registry_from_config(&config);

        let call = ToolCall {
            id: "call_1".to_string(),
            name: "shell".to_string(),
            arguments: serde_json::json!({
                "command": "sleep 2"
            }),
        };

        let result = registry.execute(&call).await;
        assert!(matches!(result, Err(ToolError::Timeout { seconds: 1, .. })));
    }

    #[tokio::test]
    async fn test_runtime_registry_registers_session_tools() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(
            temp_dir.path().to_path_buf(),
        )));
        let (outbound_tx, _outbound_rx) = mpsc::channel(1);

        let registry = create_registry_from_config_with_runtime(
            &ToolsConfig::default(),
            Some(sessions),
            Some(outbound_tx),
            Some(temp_dir.path().to_path_buf()),
        );

        assert!(registry.has("sessions_list"));
        assert!(registry.has("sessions_history"));
        assert!(registry.has("sessions_send"));
    }

    #[tokio::test]
    async fn test_runtime_registry_registers_cron_when_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(
            temp_dir.path().join("sessions"),
        )));
        let (outbound_tx, _outbound_rx) = mpsc::channel(1);

        let registry = create_registry_from_config_with_runtime_and_orchestration(
            &ToolsConfig::default(),
            &SpawnerConfig::default(),
            &CronConfig {
                enabled: true,
                ..Default::default()
            },
            Some(sessions),
            Some(outbound_tx),
            Some(temp_dir.path().to_path_buf()),
        );

        assert!(registry.has("cron"));
    }
}
