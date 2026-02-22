//! MaxToken Transformer
//!
//! Manages token limits and context window optimization. Automatically adjusts
//! max_tokens based on model context limits and input size.

use crate::{server::ChatResponse, token_counter::ChatRequest, ProxyError, Result};
use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use tiktoken_rs::CoreBPE;
use tracing::debug;

/// MaxToken transformer for intelligent token limit management
pub struct MaxTokenTransformer {
    /// Model context limits
    model_limits: HashMap<String, u32>,
    /// Default max_tokens if not specified
    default_max_tokens: u32,
    /// Safety margin to leave for response
    safety_margin: u32,
    /// Enable automatic adjustment based on input size
    auto_adjust: bool,
    /// Tiktoken encoder
    encoder: CoreBPE,
}

impl MaxTokenTransformer {
    pub fn new() -> Result<Self> {
        let mut model_limits = HashMap::new();

        // Common model context limits
        model_limits.insert("claude-3-5-sonnet-20241022".to_string(), 200000);
        model_limits.insert("claude-3-5-haiku-20241022".to_string(), 200000);
        model_limits.insert("claude-3-opus-20240229".to_string(), 200000);
        model_limits.insert("claude-3-sonnet-20240229".to_string(), 200000);
        model_limits.insert("claude-3-haiku-20240307".to_string(), 200000);
        model_limits.insert("gpt-4-turbo".to_string(), 128000);
        model_limits.insert("gpt-4".to_string(), 8192);
        model_limits.insert("gpt-3.5-turbo".to_string(), 16385);
        model_limits.insert("deepseek-chat".to_string(), 32768);
        model_limits.insert("deepseek-coder".to_string(), 16384);
        model_limits.insert("gemini-pro".to_string(), 32768);
        model_limits.insert("gemini-pro-vision".to_string(), 16384);

        let encoder = tiktoken_rs::cl100k_base().map_err(|e| {
            ProxyError::TokenCountingError(format!("Failed to initialize tokenizer: {}", e))
        })?;

        Ok(Self {
            model_limits,
            default_max_tokens: 4096,
            safety_margin: 1024,
            auto_adjust: true,
            encoder,
        })
    }

    pub fn with_default_max_tokens(mut self, max_tokens: u32) -> Self {
        self.default_max_tokens = max_tokens;
        self
    }

    pub fn with_safety_margin(mut self, margin: u32) -> Self {
        self.safety_margin = margin;
        self
    }

    pub fn with_auto_adjust(mut self, auto_adjust: bool) -> Self {
        self.auto_adjust = auto_adjust;
        self
    }

    pub fn add_model_limit(mut self, model: &str, limit: u32) -> Self {
        self.model_limits.insert(model.to_string(), limit);
        self
    }

    fn get_model_limit(&self, model: &str) -> Option<u32> {
        // Try exact match first
        if let Some(limit) = self.model_limits.get(model) {
            return Some(*limit);
        }

        // Try partial match for model families
        for (model_pattern, limit) in &self.model_limits {
            if model.contains(model_pattern) || model_pattern.contains(model) {
                return Some(*limit);
            }
        }

        None
    }

    fn estimate_input_tokens(&self, req: &ChatRequest) -> u32 {
        let mut total_tokens = 0u32;

        // Count system prompt
        if let Some(system) = &req.system {
            if let Ok(system_json) = serde_json::to_string(system) {
                total_tokens += self.encoder.encode_with_special_tokens(&system_json).len() as u32;
            }
        }

        // Count messages
        for message in &req.messages {
            if let Ok(message_json) = serde_json::to_string(message) {
                total_tokens += self.encoder.encode_with_special_tokens(&message_json).len() as u32;
            }
        }

        // Count tools
        if let Some(tools) = &req.tools {
            for tool in tools {
                if let Ok(tool_json) = serde_json::to_string(tool) {
                    total_tokens +=
                        self.encoder.encode_with_special_tokens(&tool_json).len() as u32;
                }
            }
        }

        total_tokens
    }
}

impl Default for MaxTokenTransformer {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait]
impl crate::transformer::Transformer for MaxTokenTransformer {
    fn name(&self) -> &str {
        "maxtoken"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        debug!("Applying MaxToken transformer - managing token limits");

        // Get model from request
        let model = &req.model;

        let input_tokens = self.estimate_input_tokens(&req);
        debug!(
            input_tokens = input_tokens,
            model = model,
            "Estimated input tokens"
        );

        // Determine optimal max_tokens
        let optimal_max_tokens = if let Some(context_limit) = self.get_model_limit(model) {
            if self.auto_adjust {
                // Calculate available tokens for response
                let available = context_limit.saturating_sub(input_tokens + self.safety_margin);

                // Use the smaller of available tokens and default
                std::cmp::min(available, self.default_max_tokens)
            } else {
                self.default_max_tokens
            }
        } else {
            // Unknown model, use default
            self.default_max_tokens
        };

        // Set max_tokens in request if not already set or if auto_adjust is enabled
        if req.max_tokens.is_none() || self.auto_adjust {
            req.max_tokens = Some(optimal_max_tokens as u64);
            debug!(max_tokens = optimal_max_tokens, "Set max_tokens");
        }

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        debug!("MaxToken transformer - response pass-through");
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{Message, MessageContent, SystemPrompt};
    use crate::transformer::Transformer;

    #[tokio::test]
    async fn test_sets_default_max_tokens() {
        let transformer = MaxTokenTransformer::new().unwrap();

        let req = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        assert_eq!(transformed.max_tokens.unwrap(), 4096);
    }

    #[tokio::test]
    async fn test_adjusts_for_known_model() {
        let transformer = MaxTokenTransformer::new()
            .unwrap()
            .with_safety_margin(100)
            .with_auto_adjust(true);

        let req = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            // Add some content to simulate input
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("A".repeat(10000)), // ~2500 tokens
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        let max_tokens = transformed.max_tokens.unwrap();
        // Should be equal to default since available tokens > default
        assert_eq!(max_tokens, 4096);
    }

    #[tokio::test]
    async fn test_preserves_existing_max_tokens() {
        let transformer = MaxTokenTransformer::new().unwrap().with_auto_adjust(false);

        let req = ChatRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: Some(8192),
            ..Default::default()
        };

        let transformed = transformer.transform_request(req).await.unwrap();

        // Should preserve existing value when auto_adjust is false
        assert_eq!(transformed.max_tokens.unwrap(), 8192);
    }

    #[tokio::test]
    async fn test_estimates_input_tokens() {
        let transformer = MaxTokenTransformer::new().unwrap();

        let req = ChatRequest {
            system: Some(SystemPrompt::Text(
                "System prompt with about 10 tokens".to_string(),
            )),
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: MessageContent::Text("User message with about 8 tokens".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                Message {
                    role: "assistant".to_string(),
                    content: MessageContent::Text(
                        "Assistant response with about 9 tokens".to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
            ],
            ..Default::default()
        };

        let input_tokens = transformer.estimate_input_tokens(&req);

        // Should be roughly (10 + 8 + 9) / 4 = ~7 tokens, but we'll just check it's reasonable
        assert!(input_tokens > 0);
        assert!(input_tokens < 100);
    }
}
