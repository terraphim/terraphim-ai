//! Tool registry and implementations for TinyClaw agent.

pub mod edit;
pub mod filesystem;
pub mod shell;
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
    use crate::tools::edit::EditTool;
    use crate::tools::filesystem::FilesystemTool;
    use crate::tools::shell::ShellTool;
    use crate::tools::voice_transcribe::VoiceTranscribeTool;
    use crate::tools::web::{WebFetchTool, WebSearchTool};

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FilesystemTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry.register(Box::new(ShellTool::new()));
    registry.register(Box::new(WebSearchTool::new()));
    registry.register(Box::new(WebFetchTool::new()));
    registry.register(Box::new(VoiceTranscribeTool::new()));
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
