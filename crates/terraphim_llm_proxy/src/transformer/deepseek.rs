//! DeepSeek transformer
//!
//! Adapts Claude API format to DeepSeek's OpenAI-compatible format

use crate::{
    server::ChatResponse,
    token_counter::{ChatRequest, ContentBlock, Message, MessageContent, SystemPrompt},
    transformer::Transformer,
    Result,
};
use async_trait::async_trait;

/// DeepSeek transformer - converts to OpenAI format
pub struct DeepSeekTransformer;

#[async_trait]
impl Transformer for DeepSeekTransformer {
    fn name(&self) -> &str {
        "deepseek"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        // DeepSeek uses OpenAI-compatible format
        // Main differences:
        // 1. System prompt goes in messages array as {"role": "system", "content": "..."}
        // 2. Content blocks need to be flattened to text
        // 3. Tools need OpenAI format

        // Move system prompt into messages if present
        if let Some(system) = req.system.take() {
            let system_content = match system {
                SystemPrompt::Text(text) => text,
                SystemPrompt::Array(blocks) => {
                    // Concatenate text blocks
                    blocks
                        .into_iter()
                        .map(|block| match block {
                            crate::token_counter::SystemBlock::Text { text } => text,
                            crate::token_counter::SystemBlock::CacheControl { text, .. } => text,
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n")
                }
            };

            // Insert system message at the beginning
            req.messages.insert(
                0,
                Message {
                    role: "system".to_string(),
                    content: MessageContent::Text(system_content),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
            );
        }

        // Flatten content blocks to text for each message
        for message in &mut req.messages {
            message.content = match &message.content {
                MessageContent::Text(text) => MessageContent::Text(text.clone()),
                MessageContent::Array(blocks) => {
                    // Extract text from blocks
                    let text = blocks
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                            _ => None, // Skip images and tool_use
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    MessageContent::Text(text)
                }
                MessageContent::Null => MessageContent::Text(String::new()),
            };
        }

        // Remove thinking field (DeepSeek doesn't support it in the same way)
        req.thinking = None;

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        // DeepSeek responses are already in Claude-compatible format
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{ChatRequest, Message, MessageContent, SystemPrompt};

    #[tokio::test]
    async fn test_system_prompt_moved_to_messages() {
        let transformer = DeepSeekTransformer;

        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: Some(SystemPrompt::Text("You are helpful".to_string())),
            tools: None,
            max_tokens: None,
            temperature: None,
            stream: None,
            thinking: None,
            ..Default::default()
        };

        let transformed = transformer.transform_request(request).await.unwrap();

        assert_eq!(transformed.messages.len(), 2);
        assert_eq!(transformed.messages[0].role, "system");
        assert!(transformed.system.is_none());
    }

    #[tokio::test]
    async fn test_content_blocks_flattened() {
        let transformer = DeepSeekTransformer;

        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Array(vec![
                    ContentBlock::Text {
                        text: "Part 1".to_string(),
                    },
                    ContentBlock::Text {
                        text: "Part 2".to_string(),
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

        let transformed = transformer.transform_request(request).await.unwrap();

        match &transformed.messages[0].content {
            MessageContent::Text(text) => {
                assert!(text.contains("Part 1"));
                assert!(text.contains("Part 2"));
            }
            _ => panic!("Expected text content"),
        }
    }
}
