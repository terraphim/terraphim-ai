//! Tool registry and implementations for TinyClaw agent.

pub mod agent_memory;
pub mod approval;
pub mod browser;
pub mod clarify;
pub mod clipboard;
pub mod debug_helpers;
pub mod edit;
pub mod filesystem;
pub mod fuzzy_match;
pub mod homeassistant;
pub mod image_generation;
pub mod interrupt;
pub mod moa;
pub mod patch_parser;
pub mod process_registry;
pub mod sandbox;
pub mod scheduler;
pub mod session_tools;
pub mod shell;
pub mod subagent;
pub mod todo;
pub mod tts;
pub mod vision;
pub mod voice_transcribe;
pub mod web;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

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

    #[error("Tool '{tool}' backend unavailable: {message}")]
    BackendUnavailable { tool: String, message: String },

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
///
/// # Arguments
/// * `sessions` - Optional session manager for session-aware tools
/// * `web_tools_config` - Optional web tools configuration
/// * `memory_config` - Optional memory bridge configuration
pub async fn create_default_registry(
    sessions: Option<std::sync::Arc<tokio::sync::Mutex<crate::session::SessionManager>>>,
    web_tools_config: Option<&crate::config::WebToolsConfig>,
    memory_config: Option<&crate::config::MemoryConfig>,
) -> ToolRegistry {
    create_default_registry_with_parity(
        sessions,
        web_tools_config,
        memory_config,
        ParityConfig::default(),
    )
    .await
}

/// Bundled Hermes-parity tool configuration.
#[derive(Default)]
pub struct ParityConfig<'a> {
    pub sandbox: Option<&'a crate::config::SandboxConfig>,
    pub subagent: Option<&'a crate::config::SubagentConfig>,
    pub browser: Option<&'a crate::config::BrowserConfig>,
    pub scheduler: Option<&'a crate::config::SchedulerConfig>,
    pub homeassistant: Option<&'a crate::config::HomeAssistantConfig>,
    pub vision: Option<&'a crate::config::VisionConfig>,
    pub image_gen: Option<&'a crate::config::ImageGenConfig>,
    pub tts: Option<&'a crate::config::TtsConfig>,
    pub moa: Option<&'a crate::config::MoaConfig>,
}

/// Create a standard tool registry including the Hermes-parity tools
/// (sandbox / subagent / browser / scheduler / homeassistant / vision /
/// image_gen / tts / moa) when their configs are enabled.
pub async fn create_default_registry_with_parity(
    sessions: Option<std::sync::Arc<tokio::sync::Mutex<crate::session::SessionManager>>>,
    web_tools_config: Option<&crate::config::WebToolsConfig>,
    memory_config: Option<&crate::config::MemoryConfig>,
    parity: ParityConfig<'_>,
) -> ToolRegistry {
    use crate::tools::agent_memory::{
        AgentMemoryConfig, LearnCaptureTool, MemoryApplyTool, MemoryCaptureTool, MemoryRetrieveTool,
    };
    use crate::tools::clarify::ClarifyTool;
    use crate::tools::clipboard::ClipboardTool;
    use crate::tools::edit::EditTool;
    use crate::tools::filesystem::FilesystemTool;
    use crate::tools::patch_parser::PatchParseTool;
    use crate::tools::process_registry::{ProcessRegistry, ProcessTool};
    use crate::tools::session_tools::{SessionHistoryTool, SessionListTool, SessionSendTool};
    use crate::tools::shell::ShellTool;
    use crate::tools::todo::{TodoStore, TodoTool};
    use crate::tools::voice_transcribe::VoiceTranscribeTool;
    use crate::tools::web::{WebFetchTool, WebSearchTool};

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FilesystemTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry.register(Box::new(ShellTool::new()));
    registry.register(Box::new(WebSearchTool::from_config(web_tools_config)));
    registry.register(Box::new(WebFetchTool::from_config(web_tools_config)));
    registry.register(Box::new(VoiceTranscribeTool::new()));
    registry.register(Box::new(TodoTool::new(std::sync::Arc::new(
        TodoStore::new(),
    ))));
    registry.register(Box::new(PatchParseTool::new()));
    registry.register(Box::new(ClarifyTool::new()));
    registry.register(Box::new(ClipboardTool::new()));
    registry.register(Box::new(ProcessTool::new(std::sync::Arc::new(
        ProcessRegistry::new(),
    ))));

    // Register session tools if SessionManager is provided
    if let Some(sessions) = sessions {
        registry.register(Box::new(SessionListTool::new(sessions.clone())));
        registry.register(Box::new(SessionHistoryTool::new(sessions.clone())));
        registry.register(Box::new(SessionSendTool::new(sessions)));
    }

    // Register agent memory tools if memory bridge is enabled
    if let Some(mem_cfg) = memory_config
        && mem_cfg.enabled
    {
        let agent_mem_cfg = std::sync::Arc::new(AgentMemoryConfig::from(mem_cfg));
        registry.register(Box::new(MemoryCaptureTool::new(agent_mem_cfg.clone())));
        registry.register(Box::new(MemoryRetrieveTool::new(agent_mem_cfg.clone())));
        registry.register(Box::new(MemoryApplyTool::new(agent_mem_cfg.clone())));
        registry.register(Box::new(LearnCaptureTool::new(agent_mem_cfg)));
    }

    // Register Hermes-parity tools (sandbox / subagent / browser / scheduler /
    // homeassistant / vision / image_gen / tts / moa). Each is off by default;
    // enabled via tinyclaw.toml sections.
    if let Some(cfg) = parity.sandbox
        && cfg.enabled
    {
        match crate::tools::sandbox::SandboxTool::from_config(cfg).await {
            Ok(tool) => registry.register(Box::new(tool)),
            Err(e) => log::warn!("sandbox tool disabled: {}", e),
        }
    }
    if let Some(cfg) = parity.subagent
        && cfg.enabled
    {
        registry.register(Box::new(crate::tools::subagent::SubagentTool::from_config(
            cfg,
        )));
    }
    if let Some(cfg) = parity.browser
        && cfg.enabled
    {
        match crate::tools::browser::BrowserTool::from_config(cfg) {
            Ok(tool) => registry.register(Box::new(tool)),
            Err(e) => log::warn!("browser tool disabled: {}", e),
        }
    }
    if let Some(cfg) = parity.scheduler
        && cfg.enabled
    {
        match crate::tools::scheduler::ScheduleTool::from_config(cfg).await {
            Ok(tool) => registry.register(Box::new(tool)),
            Err(e) => log::warn!("schedule tool disabled: {}", e),
        }
    }
    if let Some(cfg) = parity.homeassistant
        && cfg.available()
    {
        for tool in crate::tools::homeassistant::build_tools(cfg) {
            registry.register(tool);
        }
    }
    if let Some(cfg) = parity.vision
        && cfg.available()
    {
        registry.register(Box::new(crate::tools::vision::VisionTool::from_config(cfg)));
    }
    if let Some(cfg) = parity.image_gen
        && cfg.available()
    {
        registry.register(Box::new(
            crate::tools::image_generation::ImageGenerateTool::from_config(cfg),
        ));
    }
    if let Some(cfg) = parity.tts
        && cfg.available()
    {
        registry.register(Box::new(crate::tools::tts::TextToSpeechTool::from_config(
            cfg,
        )));
    }
    if let Some(cfg) = parity.moa
        && cfg.available()
    {
        registry.register(Box::new(
            crate::tools::moa::MixtureOfAgentsTool::from_config(cfg),
        ));
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
