//! Chat functionality for REPL interface
//! Requires 'repl-chat' feature
//!
//! Integrates with ValidatedGenAiClient for LLM interactions with full validation pipeline

#[cfg(feature = "repl-chat")]
use anyhow::Result;

#[cfg(feature = "repl-chat")]
use terraphim_multi_agent::{LlmMessage, LlmRequest, MessageRole, ValidatedGenAiClient};

#[cfg(feature = "repl-chat")]
#[allow(dead_code)]
#[derive(Default)]
pub struct ChatHandler {
    client: Option<ValidatedGenAiClient>,
    conversation_history: Vec<LlmMessage>,
}

#[cfg(feature = "repl-chat")]
impl ChatHandler {
    pub fn new() -> Self {
        Self {
            client: None,
            conversation_history: Vec::new(),
        }
    }

    /// Initialize with Ollama (default for local use)
    pub fn with_ollama(model: Option<String>) -> Result<Self> {
        let client = ValidatedGenAiClient::new_ollama(model)?;
        Ok(Self {
            client: Some(client),
            conversation_history: Vec::new(),
        })
    }

    /// Initialize with OpenAI
    pub fn with_openai(model: Option<String>) -> Result<Self> {
        let client = ValidatedGenAiClient::new_openai(model)?;
        Ok(Self {
            client: Some(client),
            conversation_history: Vec::new(),
        })
    }

    /// Initialize with Anthropic
    pub fn with_anthropic(model: Option<String>) -> Result<Self> {
        let client = ValidatedGenAiClient::new_anthropic(model)?;
        Ok(Self {
            client: Some(client),
            conversation_history: Vec::new(),
        })
    }

    /// Send message with full validation pipeline
    pub async fn send_message(&mut self, message: &str) -> Result<String> {
        if let Some(client) = &self.client {
            // Add user message to history
            let user_msg = LlmMessage {
                role: MessageRole::User,
                content: message.to_string(),
            };
            self.conversation_history.push(user_msg);

            // Create request with full history for context
            let request = LlmRequest::new(self.conversation_history.clone());

            // Call LLM with validation pipeline (pre-LLM + post-LLM validation)
            match client.generate(request).await {
                Ok(response) => {
                    // Add assistant response to history
                    let assistant_msg = LlmMessage {
                        role: MessageRole::Assistant,
                        content: response.content.clone(),
                    };
                    self.conversation_history.push(assistant_msg);

                    Ok(response.content)
                }
                Err(e) => Err(anyhow::anyhow!("LLM error: {}", e)),
            }
        } else {
            // Fallback echo mode if no client configured
            Ok(format!("Echo (no LLM configured): {}", message))
        }
    }

    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Get conversation length
    pub fn history_len(&self) -> usize {
        self.conversation_history.len()
    }
}
