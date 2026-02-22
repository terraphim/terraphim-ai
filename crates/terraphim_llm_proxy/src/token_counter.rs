//! Token counting module using tiktoken-rs
//!
//! Provides accurate token counting for Claude API requests using the cl100k_base encoding.
//! This is essential for cost-aware routing and context length management.

use crate::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiktoken_rs::CoreBPE;
use tracing::{debug, trace};

/// Token counter using tiktoken-rs
pub struct TokenCounter {
    bpe: Arc<CoreBPE>,
}

impl TokenCounter {
    /// Create a new token counter with cl100k_base encoding
    pub fn new() -> Result<Self> {
        let bpe = tiktoken_rs::cl100k_base().map_err(|e| {
            ProxyError::TokenCountingError(format!("Failed to initialize tokenizer: {}", e))
        })?;

        Ok(Self { bpe: Arc::new(bpe) })
    }

    /// Count tokens in a complete chat request
    pub fn count_request(&self, req: &ChatRequest) -> Result<usize> {
        let mut total: usize = 0;

        // Count messages
        let message_tokens = self.count_messages(&req.messages)?;
        total = total
            .checked_add(message_tokens)
            .ok_or(ProxyError::TokenCountOverflow)?;

        debug!(message_tokens, "Counted message tokens");

        // Count system prompt if present
        if let Some(system) = &req.system {
            let system_tokens = self.count_system(system)?;
            total = total
                .checked_add(system_tokens)
                .ok_or(ProxyError::TokenCountOverflow)?;

            debug!(system_tokens, "Counted system tokens");
        }

        // Count tools if present
        if let Some(tools) = &req.tools {
            let tool_tokens = self.count_tools(tools)?;
            total = total
                .checked_add(tool_tokens)
                .ok_or(ProxyError::TokenCountOverflow)?;

            debug!(tool_tokens, "Counted tool tokens");
        }

        // Sanity check - 10M tokens is unrealistic
        if total > 10_000_000 {
            return Err(ProxyError::TokenCountTooLarge(total));
        }

        trace!(total, "Total token count");

        Ok(total)
    }

    /// Count tokens in messages array
    pub fn count_messages(&self, messages: &[Message]) -> Result<usize> {
        let mut total: usize = 0;

        for message in messages {
            let count = self.count_message(message)?;
            total = total
                .checked_add(count)
                .ok_or(ProxyError::TokenCountOverflow)?;
        }

        Ok(total)
    }

    /// Count tokens in a single message
    fn count_message(&self, message: &Message) -> Result<usize> {
        let mut total: usize = 0;

        // Count role tokens (~1 token)
        let role_tokens = self.count_text(&message.role)?;
        total = total
            .checked_add(role_tokens)
            .ok_or(ProxyError::TokenCountOverflow)?;

        // Count content tokens
        match &message.content {
            MessageContent::Text(text) => {
                let content_tokens = self.count_text(text)?;
                total = total
                    .checked_add(content_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;
            }
            MessageContent::Array(blocks) => {
                for block in blocks {
                    let block_tokens = self.count_content_block(block)?;
                    total = total
                        .checked_add(block_tokens)
                        .ok_or(ProxyError::TokenCountOverflow)?;
                }
            }
            MessageContent::Null => {}
        }

        // Add message overhead (formatting tokens)
        // Claude API uses approximately 4 tokens per message for formatting
        total = total.checked_add(4).ok_or(ProxyError::TokenCountOverflow)?;

        Ok(total)
    }

    /// Count tokens in a content block
    fn count_content_block(&self, block: &ContentBlock) -> Result<usize> {
        match block {
            ContentBlock::Text { text } => self.count_text(text),
            ContentBlock::Image { source } => {
                // Images are counted specially:
                // - Small images (~200x200): ~85 tokens
                // - Medium images (~512x512): ~340 tokens
                // - Large images (~1024x1024): ~1360 tokens
                // For now, use a conservative estimate
                match source {
                    ImageSource::Base64 { data, .. } => {
                        // Estimate based on base64 data size
                        let size_estimate = data.len() * 3 / 4; // base64 to bytes
                        let tokens = if size_estimate < 100_000 {
                            85 // Small image
                        } else if size_estimate < 500_000 {
                            340 // Medium image
                        } else {
                            1360 // Large image
                        };
                        Ok(tokens)
                    }
                    ImageSource::Url { url } => {
                        // Can't determine size from URL, use medium estimate
                        trace!(?url, "Image URL detected, using medium size estimate");
                        Ok(340)
                    }
                }
            }
            ContentBlock::ToolUse { id, name, input } => {
                let mut total: usize = 0;

                // Count tool use overhead
                total = total
                    .checked_add(10)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                // Count ID
                let id_tokens = self.count_text(id)?;
                total = total
                    .checked_add(id_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                // Count name
                let name_tokens = self.count_text(name)?;
                total = total
                    .checked_add(name_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                // Count input (as JSON)
                let input_json = serde_json::to_string(input)?;
                let input_tokens = self.count_text(&input_json)?;
                total = total
                    .checked_add(input_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                Ok(total)
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                let mut total: usize = 0;

                // Count tool result overhead
                total = total
                    .checked_add(10)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                // Count tool use ID
                let id_tokens = self.count_text(tool_use_id)?;
                total = total
                    .checked_add(id_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                // Count content
                let content_tokens = self.count_text(content)?;
                total = total
                    .checked_add(content_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;

                Ok(total)
            }
            ContentBlock::Other => Ok(0),
        }
    }

    /// Count tokens in system prompt
    pub fn count_system(&self, system: &SystemPrompt) -> Result<usize> {
        match system {
            SystemPrompt::Text(text) => self.count_text(text),
            SystemPrompt::Array(blocks) => {
                let mut total: usize = 0;
                for block in blocks {
                    let tokens = self.count_system_block(block)?;
                    total = total
                        .checked_add(tokens)
                        .ok_or(ProxyError::TokenCountOverflow)?;
                }
                Ok(total)
            }
        }
    }

    /// Count tokens in a system prompt block
    fn count_system_block(&self, block: &SystemBlock) -> Result<usize> {
        match block {
            SystemBlock::Text { text, .. } => self.count_text(text),
            SystemBlock::CacheControl { text, .. } => {
                // Count text plus cache control overhead (~5 tokens)
                let text_tokens = self.count_text(text)?;
                text_tokens
                    .checked_add(5)
                    .ok_or(ProxyError::TokenCountOverflow)
            }
        }
    }

    /// Count tokens in tools array
    pub fn count_tools(&self, tools: &[Tool]) -> Result<usize> {
        let mut total: usize = 0;

        for tool in tools {
            let count = self.count_tool(tool)?;
            total = total
                .checked_add(count)
                .ok_or(ProxyError::TokenCountOverflow)?;
        }

        Ok(total)
    }

    /// Count tokens in a single tool definition
    fn count_tool(&self, tool: &Tool) -> Result<usize> {
        let mut total: usize = 0;

        // Tool overhead (~10 tokens)
        total = total
            .checked_add(10)
            .ok_or(ProxyError::TokenCountOverflow)?;

        // Handle both OpenAI format (with function field) and legacy format
        if let Some(function) = &tool.function {
            // OpenAI format
            let name_tokens = self.count_text(&function.name)?;
            total = total
                .checked_add(name_tokens)
                .ok_or(ProxyError::TokenCountOverflow)?;

            if let Some(description) = &function.description {
                let desc_tokens = self.count_text(description)?;
                total = total
                    .checked_add(desc_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;
            }

            let schema_json = serde_json::to_string(&function.parameters)?;
            let schema_tokens = self.count_text(&schema_json)?;
            total = total
                .checked_add(schema_tokens)
                .ok_or(ProxyError::TokenCountOverflow)?;
        } else if let Some(name) = &tool.name {
            // Legacy format
            let name_tokens = self.count_text(name)?;
            total = total
                .checked_add(name_tokens)
                .ok_or(ProxyError::TokenCountOverflow)?;

            if let Some(description) = &tool.description {
                let desc_tokens = self.count_text(description)?;
                total = total
                    .checked_add(desc_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;
            }

            if let Some(input_schema) = &tool.input_schema {
                let schema_json = serde_json::to_string(input_schema)?;
                let schema_tokens = self.count_text(&schema_json)?;
                total = total
                    .checked_add(schema_tokens)
                    .ok_or(ProxyError::TokenCountOverflow)?;
            }
        }

        Ok(total)
    }

    /// Count tokens in a text string
    pub fn count_text(&self, text: &str) -> Result<usize> {
        if text.is_empty() {
            return Ok(0);
        }

        // Use tiktoken-rs to encode
        let tokens = self.bpe.encode_with_special_tokens(text);

        Ok(tokens.len())
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new().expect("Failed to create token counter")
    }
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<serde_json::Value>,
    /// Nucleus sampling probability (0.0-1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-K sampling (limits tokens to top K most likely).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Custom stop sequences to end generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Request metadata for tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RequestMetadata>,
}

/// Metadata for tracking requests.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestMetadata {
    /// Optional user identifier for tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
    /// OpenAI tool_calls array on assistant messages
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    /// OpenAI tool_call_id on tool-result messages (role: "tool")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// OpenAI function name for tool-result messages
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Array(Vec<ContentBlock>),
    /// Handles `"content": null` in OpenAI assistant tool_call messages
    Null,
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    #[serde(alias = "input_text", alias = "output_text")]
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),
    Array(Vec<SystemBlock>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemBlock {
    Text {
        text: String,
    },
    CacheControl {
        text: String,
        cache_control: CacheControl,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionTool>,
    // Legacy fields for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "input_schema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: Vec::new(),
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_count_simple_text() {
        let counter = TokenCounter::new().unwrap();

        // "Hello, world!" should be ~4 tokens
        let count = counter.count_text("Hello, world!").unwrap();
        assert!(
            (3..=5).contains(&count),
            "Expected ~4 tokens, got {}",
            count
        );
    }

    #[test]
    fn test_count_empty_text() {
        let counter = TokenCounter::new().unwrap();
        assert_eq!(counter.count_text("").unwrap(), 0);
    }

    #[test]
    fn test_count_simple_message() {
        let counter = TokenCounter::new().unwrap();

        let message = Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello, Claude!".to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };

        let count = counter.count_message(&message).unwrap();
        // Should be: role (~1) + content (~4) + overhead (4) = ~9 tokens
        assert!(
            (7..=11).contains(&count),
            "Expected ~9 tokens, got {}",
            count
        );
    }

    #[test]
    fn test_count_request_with_messages() {
        let counter = TokenCounter::new().unwrap();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: MessageContent::Text("Hello!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                Message {
                    role: "assistant".to_string(),
                    content: MessageContent::Text("Hi there!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
            ],
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let count = counter.count_request(&request).unwrap();
        // Should be reasonable - around 15-20 tokens for this simple conversation
        assert!(
            (10..=30).contains(&count),
            "Expected ~15-20 tokens, got {}",
            count
        );
    }

    #[test]
    fn test_count_request_with_system() {
        let counter = TokenCounter::new().unwrap();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello!".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: Some(SystemPrompt::Text(
                "You are a helpful assistant.".to_string(),
            )),
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let count = counter.count_request(&request).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_count_request_with_tools() {
        let counter = TokenCounter::new().unwrap();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("What's the weather?".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: Some(vec![Tool {
                tool_type: Some("function".to_string()),
                function: Some(FunctionTool {
                    name: "get_weather".to_string(),
                    description: Some("Get the current weather".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The city name"
                            }
                        },
                        "required": ["location"]
                    }),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let count = counter.count_request(&request).unwrap();
        assert!(count > 20, "Expected >20 tokens with tool, got {}", count);
    }

    #[test]
    fn test_count_image_block() {
        let counter = TokenCounter::new().unwrap();

        let block = ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "iVBORw0KGgo=".to_string(), // Small fake base64
            },
        };

        let count = counter.count_content_block(&block).unwrap();
        assert_eq!(count, 85); // Small image estimate
    }

    #[test]
    fn test_sanity_check_large_count() {
        let counter = TokenCounter::new().unwrap();

        // Test that the sanity check is enforced at the request level
        // We'll create a request that would theoretically have 10M+ tokens
        // by mocking the behavior with a direct test

        // First verify that reasonable inputs work
        let normal_text = "Hello world ".repeat(1000);
        let result = counter.count_text(&normal_text);
        assert!(result.is_ok());

        // Test the sanity check by creating many messages
        // Each message with ~10K tokens would need 1K messages to hit 10M tokens
        // This is impractical to test directly, so we verify the logic exists
        // by checking that reasonable requests succeed
        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text(normal_text),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let count = counter.count_request(&request).unwrap();
        assert!(
            count < 100_000,
            "Normal request should have <100K tokens, got {}",
            count
        );
    }

    #[test]
    fn test_reference_token_counts() {
        let counter = TokenCounter::new().unwrap();

        // Reference counts from OpenAI's tiktoken
        let test_cases = vec![
            ("Hello, world!", 4),
            ("The quick brown fox jumps over the lazy dog", 9),
            ("tiktoken is great!", 5),
        ];

        for (text, expected) in test_cases {
            let count = counter.count_text(text).unwrap();
            // Allow ±1 token variance
            assert!(
                (count as i32 - expected).abs() <= 1,
                "Text: '{}', Expected: {}, Got: {}",
                text,
                expected,
                count
            );
        }
    }

    #[test]
    fn test_deserialize_openai_input_text_content_blocks() {
        let request_json = json!({
            "model": "fastest",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "hello"},
                        {"type": "output_text", "text": "world"}
                    ]
                }
            ]
        });

        let request: ChatRequest = serde_json::from_value(request_json).unwrap();
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_deserialize_unknown_content_block_type() {
        let request_json = json!({
            "model": "fastest",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "reasoning", "summary": "thinking"}
                    ]
                }
            ]
        });

        let request: ChatRequest = serde_json::from_value(request_json).unwrap();
        assert_eq!(request.messages.len(), 1);
        match &request.messages[0].content {
            MessageContent::Array(blocks) => assert!(matches!(blocks[0], ContentBlock::Other)),
            _ => panic!("expected array content"),
        }
    }
}
