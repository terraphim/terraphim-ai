//! Ollama transformer
//!
//! Adapts Claude API format to Ollama's OpenAI-compatible format

use crate::{
    server::ChatResponse,
    token_counter::{ChatRequest, Message, MessageContent, SystemPrompt},
    transformer::Transformer,
    Result,
};
use async_trait::async_trait;

/// Ollama transformer - converts to OpenAI format (similar to DeepSeek)
pub struct OllamaTransformer;

#[async_trait]
impl Transformer for OllamaTransformer {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn transform_request(&self, mut req: ChatRequest) -> Result<ChatRequest> {
        // Ollama uses OpenAI-compatible format
        // Move system prompt into messages

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

        // Flatten content blocks to text
        for message in &mut req.messages {
            if let MessageContent::Array(blocks) = &message.content {
                let text = blocks
                    .iter()
                    .filter_map(|block| match block {
                        crate::token_counter::ContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");

                message.content = MessageContent::Text(text);
            }
        }

        // Remove unsupported fields
        req.thinking = None;
        req.tools = None; // Ollama may not support tools in all models

        Ok(req)
    }

    async fn transform_response(&self, resp: ChatResponse) -> Result<ChatResponse> {
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_counter::{ChatRequest, Message, MessageContent, SystemPrompt};

    #[tokio::test]
    async fn test_ollama_system_prompt_transformation() {
        let transformer = OllamaTransformer;

        let request = ChatRequest {
            model: "qwen2.5-coder".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
            }],
            system: Some(SystemPrompt::Text("You are a coding assistant".to_string())),
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
        assert!(transformed.tools.is_none());
    }
}
