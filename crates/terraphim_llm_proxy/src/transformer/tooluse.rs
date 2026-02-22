//! Tool Use Transformer
//!
//! Handles tool call formatting and compatibility between different providers'
//! tool use formats. Ensures proper Claude-compatible tool call formatting.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;

use tracing::debug;

/// Tool use transformer for cross-provider tool call compatibility
pub struct ToolUseTransformer {
    /// Force Claude-style tool call format
    claude_format: bool,
    /// Enable tool choice auto-detection
    auto_tool_choice: bool,
}

impl ToolUseTransformer {
    pub fn new() -> Self {
        Self {
            claude_format: true,
            auto_tool_choice: true,
        }
    }

    pub fn with_claude_format(mut self, claude_format: bool) -> Self {
        self.claude_format = claude_format;
        self
    }

    pub fn with_auto_tool_choice(mut self, auto_tool_choice: bool) -> Self {
        self.auto_tool_choice = auto_tool_choice;
        self
    }
}

impl Default for ToolUseTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::transformer::Transformer for ToolUseTransformer {
    fn name(&self) -> &str {
        "tooluse"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        debug!("Applying ToolUse transformer - normalizing tool call format");

        // Process tools if present - tools are already in the correct format
        // The Tool struct already matches Claude's expected format
        // No conversion needed as the structure is already correct

        // Process messages for tool call formatting
        for message in req.messages.iter_mut() {
            self.normalize_message_tool_calls(message);
        }

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        debug!("ToolUse transformer - response pass-through");
        // Tool formatting is handled at the serialization layer
        Ok(resp)
    }
}

impl ToolUseTransformer {
    fn normalize_message_tool_calls(&self, message: &mut crate::token_counter::Message) {
        // For now, just pass through - tool call normalization
        // can be implemented when needed for specific provider compatibility
        let _ = message;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::Tool;
    use crate::transformer::Transformer;
    use serde_json::json;

    #[tokio::test]
    async fn test_processes_tools_without_modification() {
        let transformer = ToolUseTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            tools: Some(vec![Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "calculator".to_string(),
                    description: Some("Perform calculations".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "expression": {"type": "string"}
                        },
                        "required": ["expression"]
                    }),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        let tools = transformed.tools.unwrap();
        assert_eq!(tools.len(), 1);
        let func = tools[0].function.as_ref().unwrap();
        assert_eq!(func.name, "calculator");
        assert_eq!(func.description.as_ref().unwrap(), "Perform calculations");
    }

    #[tokio::test]
    async fn test_processes_request_with_tools() {
        let transformer = ToolUseTransformer::new();

        let req = ChatRequest {
            model: "test-model".to_string(),
            tools: Some(vec![]),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        assert!(transformed.tools.is_some());
    }

    #[tokio::test]
    async fn test_response_pass_through() {
        let transformer = ToolUseTransformer::new();

        // Create a basic ChatResponse - since ChatResponse doesn't have Default,
        // we'll create one with minimal required fields
        let resp = ChatResponse {
            id: "test".to_string(),
            message_type: "message".to_string(),
            model: "test-model".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            stop_reason: None,
            stop_sequence: None,
            usage: genai::chat::Usage {
                prompt_tokens_details: None,
                completion_tokens_details: None,
                total_tokens: None,
                prompt_tokens: Some(100),
                completion_tokens: Some(50),
            },
        };

        let transformed = transformer.transform_response(resp).await.unwrap();

        assert_eq!(transformed.model, "test-model");
    }
}
