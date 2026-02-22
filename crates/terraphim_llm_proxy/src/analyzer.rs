//! Request analysis module
//!
//! Analyzes incoming requests to generate routing hints for intelligent model selection.

use crate::{
    token_counter::{ChatRequest, ContentBlock, MessageContent, TokenCounter},
    Result,
};
use std::sync::Arc;
use tracing::{debug, trace};

/// Request analyzer that generates routing hints
pub struct RequestAnalyzer {
    token_counter: Arc<TokenCounter>,
}

/// Routing hints generated from request analysis
#[derive(Debug, Clone)]
pub struct RoutingHints {
    /// Whether this appears to be a background task (haiku model detected)
    pub is_background: bool,

    /// Whether the request includes a thinking field (reasoning mode)
    pub has_thinking: bool,

    /// Whether the request includes web_search tool
    pub has_web_search: bool,

    /// Whether the request contains image content blocks
    pub has_images: bool,

    /// Total token count for the request
    pub token_count: usize,

    /// Session ID extracted from metadata (if present)
    pub session_id: Option<String>,
}

impl RequestAnalyzer {
    /// Create a new request analyzer
    pub fn new(token_counter: Arc<TokenCounter>) -> Self {
        Self { token_counter }
    }

    /// Analyze a request and generate routing hints
    pub fn analyze(&self, req: &ChatRequest) -> Result<RoutingHints> {
        debug!(model = %req.model, "Analyzing request");

        // Detect background request (haiku model)
        let is_background = self.is_background_request(req);
        trace!(is_background, "Background detection");

        // Check for thinking field
        let has_thinking = req.thinking.is_some();
        trace!(has_thinking, "Thinking field check");

        // Detect web search tool
        let has_web_search = self.has_web_search_tool(req);
        trace!(has_web_search, "Web search detection");

        // Detect images in messages
        let has_images = self.detect_images(req);
        trace!(has_images, "Image detection");

        // Count tokens
        let token_count = self.token_counter.count_request(req)?;
        debug!(token_count, "Token count");

        // Extract session ID
        let session_id = self.extract_session_id(req);
        trace!(?session_id, "Session ID extraction");

        let hints = RoutingHints {
            is_background,
            has_thinking,
            has_web_search,
            has_images,
            token_count,
            session_id,
        };

        debug!(?hints, "Generated routing hints");

        Ok(hints)
    }

    /// Detect if this is a background request
    ///
    /// Background requests are identified by:
    /// - Model name contains "haiku"
    /// - Model name contains "background" in metadata
    fn is_background_request(&self, req: &ChatRequest) -> bool {
        // Check model name for haiku
        if req.model.to_lowercase().contains("haiku") {
            return true;
        }

        // Check metadata for background indicator (if we add metadata field later)
        // For now, just model name check

        false
    }

    /// Check if request includes web_search tool
    fn has_web_search_tool(&self, req: &ChatRequest) -> bool {
        if let Some(tools) = &req.tools {
            return tools.iter().any(|tool| {
                // Check both new format (function.name) and legacy format (name)
                let tool_name = tool
                    .function
                    .as_ref()
                    .map(|f| f.name.as_str())
                    .or(tool.name.as_deref())
                    .unwrap_or("");
                tool_name == "web_search"
                    || tool_name == "brave_web_search"
                    || tool_name == "google_search"
            });
        }

        false
    }

    /// Detect if request contains images
    fn detect_images(&self, req: &ChatRequest) -> bool {
        for message in &req.messages {
            if self.message_has_images(message) {
                return true;
            }
        }

        false
    }

    /// Check if a message contains image blocks
    fn message_has_images(&self, message: &crate::token_counter::Message) -> bool {
        match &message.content {
            MessageContent::Text(_) => false,
            MessageContent::Array(blocks) => blocks
                .iter()
                .any(|block| matches!(block, ContentBlock::Image { .. })),
            MessageContent::Null => false,
        }
    }

    /// Extract session ID from request metadata
    ///
    /// Session IDs can come from:
    /// - metadata.user_id field
    /// - metadata.session_id field
    /// - Custom headers (in future)
    fn extract_session_id(&self, _req: &ChatRequest) -> Option<String> {
        // TODO: Add metadata field to ChatRequest
        // For now, return None
        // In future, parse from metadata JSON

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt, Tool};
    use serde_json::json;

    fn create_test_analyzer() -> RequestAnalyzer {
        let token_counter = Arc::new(TokenCounter::new().unwrap());
        RequestAnalyzer::new(token_counter)
    }

    #[test]
    fn test_analyze_simple_request() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello!".to_string()),
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

        let hints = analyzer.analyze(&request).unwrap();

        assert!(!hints.is_background);
        assert!(!hints.has_thinking);
        assert!(!hints.has_web_search);
        assert!(!hints.has_images);
        assert!(hints.token_count > 0);
        assert!(hints.session_id.is_none());
    }

    #[test]
    fn test_detect_background_haiku_model() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-haiku-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Background task".to_string()),
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

        let hints = analyzer.analyze(&request).unwrap();

        assert!(
            hints.is_background,
            "Haiku model should be detected as background"
        );
    }

    #[test]
    fn test_detect_thinking_field() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Solve this problem".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: Some(json!({"enabled": true})),
            ..Default::default()
        };

        let hints = analyzer.analyze(&request).unwrap();

        assert!(hints.has_thinking, "Thinking field should be detected");
    }

    #[test]
    fn test_detect_web_search_tool() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Search for latest news".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: Some(vec![Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "web_search".to_string(),
                    description: Some("Search the web".to_string()),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "query": {"type": "string"}
                        }
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

        let hints = analyzer.analyze(&request).unwrap();

        assert!(hints.has_web_search, "web_search tool should be detected");
    }

    #[test]
    fn test_detect_brave_search_tool() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Search".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: Some(vec![Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "brave_web_search".to_string(),
                    description: Some("Brave search".to_string()),
                    parameters: json!({}),
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

        let hints = analyzer.analyze(&request).unwrap();

        assert!(
            hints.has_web_search,
            "brave_web_search tool should be detected"
        );
    }

    #[test]
    fn test_detect_images() {
        let analyzer = create_test_analyzer();

        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Array(vec![
                    ContentBlock::Text {
                        text: "What's in this image?".to_string(),
                    },
                    ContentBlock::Image {
                        source: crate::token_counter::ImageSource::Base64 {
                            media_type: "image/png".to_string(),
                            data: "iVBORw0KGgo=".to_string(),
                        },
                    },
                ]),
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

        let hints = analyzer.analyze(&request).unwrap();

        assert!(hints.has_images, "Images should be detected");
    }

    #[test]
    fn test_token_count_integration() {
        let analyzer = create_test_analyzer();

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

        let hints = analyzer.analyze(&request).unwrap();

        // Should have reasonable token count (20-40 tokens)
        assert!(
            hints.token_count > 10 && hints.token_count < 100,
            "Token count {} should be 10-100",
            hints.token_count
        );
    }

    #[test]
    fn test_combined_hints() {
        let analyzer = create_test_analyzer();

        // Request with thinking + web_search + images
        let request = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Array(vec![
                    ContentBlock::Text {
                        text: "Research this topic".to_string(),
                    },
                    ContentBlock::Image {
                        source: crate::token_counter::ImageSource::Url {
                            url: "https://example.com/image.png".to_string(),
                        },
                    },
                ]),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: None,
            tools: Some(vec![Tool {
                tool_type: Some("function".to_string()),
                function: Some(crate::token_counter::FunctionTool {
                    name: "web_search".to_string(),
                    description: Some("Search".to_string()),
                    parameters: json!({}),
                }),
                name: None,
                description: None,
                input_schema: None,
            }]),
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: Some(json!({"enabled": true})),
            ..Default::default()
        };

        let hints = analyzer.analyze(&request).unwrap();

        assert!(hints.has_thinking);
        assert!(hints.has_web_search);
        assert!(hints.has_images);
        assert!(hints.token_count > 0);
    }
}
