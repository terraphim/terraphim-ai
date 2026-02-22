//! Reasoning Transformer
//!
//! Processes reasoning_content field from models with extended thinking capabilities
//! like DeepSeek-Reasoner and Qwen3-235B-Thinking.

use crate::{server::ChatResponse, token_counter::ChatRequest, Result};
use async_trait::async_trait;
use tracing::debug;

/// Reasoning transformer for processing thinking/reasoning content
pub struct ReasoningTransformer;

impl ReasoningTransformer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReasoningTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::transformer::Transformer for ReasoningTransformer {
    fn name(&self) -> &str {
        "reasoning"
    }

    async fn transform_request(&self, req: ChatRequest) -> Result<ChatRequest> {
        debug!("Reasoning transformer - request pass-through");
        // Request typically doesn't need modification for reasoning models
        Ok(req)
    }

    async fn transform_response(&self, mut resp: ChatResponse) -> Result<ChatResponse> {
        debug!("Applying Reasoning transformer - processing reasoning content");

        // Look for reasoning content in the first content block
        if let Some(block) = resp.content.first_mut() {
            if block.block_type == "text" {
                if let Some(text) = &mut block.text {
                    // Check if text contains reasoning markers
                    if text.contains("<reasoning>") || text.contains("reasoning_content:") {
                        // Extract and format reasoning content
                        let formatted_text = if let Some(start) = text.find("<reasoning>") {
                            if let Some(end) = text.find("</reasoning>") {
                                let reasoning = &text[start + 11..end];
                                let answer = &text[end + 12..];
                                format!(
                                    "🤔 **Reasoning:**\n\n{}\n\n💡 **Answer:**\n\n{}",
                                    reasoning, answer
                                )
                            } else {
                                text.clone()
                            }
                        } else {
                            text.clone()
                        };

                        *text = formatted_text;
                    }
                }
            }
        }

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transformer::Transformer;

    #[tokio::test]
    async fn test_processes_reasoning_content() {
        let transformer = ReasoningTransformer::new();

        let resp = ChatResponse {
            id: "test".to_string(),
            message_type: "message".to_string(),
            model: "test-model".to_string(),
            role: "assistant".to_string(),
            content: vec![
                crate::server::ContentBlock {
                    block_type: "text".to_string(),
                    text: Some("<reasoning>Let me think step by step...\nFirst, I need to understand the question...</reasoning>The final answer is 42.".to_string()),
                    id: None,
                    name: None,
                    input: None,
                }
            ],
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: genai::chat::Usage { prompt_tokens_details: None, completion_tokens_details: None, total_tokens: None,
                prompt_tokens: Some(100),
                completion_tokens: Some(50),
            },
        };

        let transformed = transformer.transform_response(resp).await.unwrap();

        assert_eq!(transformed.content.len(), 1);

        let content_block = &transformed.content[0];
        let text = content_block.text.as_ref().unwrap();
        assert!(text.contains("🤔 **Reasoning:**"));
        assert!(text.contains("Let me think step by step"));
        assert!(text.contains("💡 **Answer:**"));
        assert!(text.contains("The final answer is 42"));
    }

    #[tokio::test]
    async fn test_pass_through_without_reasoning() {
        let transformer = ReasoningTransformer::new();

        let resp = ChatResponse {
            id: "test".to_string(),
            message_type: "message".to_string(),
            model: "test-model".to_string(),
            role: "assistant".to_string(),
            content: vec![crate::server::ContentBlock {
                block_type: "text".to_string(),
                text: Some("Simple answer without reasoning".to_string()),
                id: None,
                name: None,
                input: None,
            }],
            stop_reason: Some("end_turn".to_string()),
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

        // Should remain unchanged
        assert_eq!(transformed.content.len(), 1);
        assert_eq!(
            transformed.content[0].text.as_ref().unwrap(),
            "Simple answer without reasoning"
        );
    }
}
