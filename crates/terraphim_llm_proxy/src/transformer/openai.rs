//! OpenAI transformer
//!
//! Adapts Claude API format to OpenAI's format (similar to DeepSeek)

use crate::{
    server::ChatResponse,
    token_counter::{ChatRequest, ContentBlock, Message, MessageContent, SystemPrompt},
    transformer::Transformer,
    Result,
};
use async_trait::async_trait;

/// OpenAI transformer - converts to OpenAI format
pub struct OpenAITransformer;

#[async_trait]
impl Transformer for OpenAITransformer {
    fn name(&self) -> &str {
        "openai"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        // OpenAI format differences from Claude:
        // 1. System prompt goes in messages array as {"role": "system", "content": "..."}
        // 2. Content blocks need to be flattened to text (or structured for vision)
        // 3. Tools use OpenAI format

        // Move system prompt into messages if present
        if let Some(system) = req.system.take() {
            let system_content = match system {
                SystemPrompt::Text(text) => text,
                SystemPrompt::Array(blocks) => blocks
                    .into_iter()
                    .map(|block| match block {
                        crate::token_counter::SystemBlock::Text { text } => text,
                        crate::token_counter::SystemBlock::CacheControl { text, .. } => text,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            };

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
        // TODO: Handle images properly for vision models
        for message in &mut req.messages {
            message.content = match &message.content {
                MessageContent::Text(text) => MessageContent::Text(text.clone()),
                MessageContent::Array(blocks) => {
                    let text = blocks
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.clone()),
                            ContentBlock::ToolResult { content, .. } => Some(content.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n\n");

                    MessageContent::Text(text)
                }
                MessageContent::Null => MessageContent::Text(String::new()),
            };
        }

        // Remove provider prefix from model name (e.g., "groq:llama-3.1-8b-instant" -> "llama-3.1-8b-instant")
        if let Some(colon_pos) = req.model.find(':') {
            req.model = req.model[colon_pos + 1..].to_string();
        }

        // Remove Claude-specific fields
        req.thinking = None;

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        // OpenAI responses need minimal transformation
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{ChatRequest, Message, MessageContent, SystemPrompt};

    #[tokio::test]
    async fn test_openai_system_prompt_moved() {
        let transformer = OpenAITransformer;

        let request = ChatRequest {
            model: "gpt-4".to_string(),
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
}
