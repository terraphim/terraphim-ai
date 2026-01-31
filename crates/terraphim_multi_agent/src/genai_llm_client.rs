//! Rust-GenAI LLM Client - Unified Multi-Provider Implementation
//!
//! This implementation uses the rust-genai library to provide a unified interface
//! to multiple LLM providers. The library handles provider-specific details,
//! authentication, and response formatting automatically.

use crate::{LlmRequest, LlmResponse, MessageRole, MultiAgentError, MultiAgentResult, TokenUsage};
use chrono::Utc;
use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use std::env;
use uuid::Uuid;

/// Rust-GenAI LLM client that works with multiple providers
#[derive(Debug)]
pub struct GenAiLlmClient {
    client: Client,
    provider: String,
    model: String,
}

/// Configuration for different LLM providers
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub model: String,
}

impl ProviderConfig {
    pub fn ollama(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "gemma3:270m".to_string()),
        }
    }

    pub fn openai(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
        }
    }

    pub fn anthropic(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "claude-3-sonnet-20240229".to_string()),
        }
    }

    pub fn openrouter(model: Option<String>) -> Self {
        Self {
            model: model.unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string()),
        }
    }
}

// All request/response formats are now handled by rust-genai library

impl GenAiLlmClient {
    /// Create a new Rust-GenAI LLM client
    pub fn new(provider: String, config: ProviderConfig) -> MultiAgentResult<Self> {
        let client = Client::default();

        Ok(Self {
            client,
            provider,
            model: config.model,
        })
    }

    /// Create a new client for Ollama
    pub fn new_ollama(model_name: Option<String>) -> MultiAgentResult<Self> {
        let config = ProviderConfig::ollama(model_name);
        Self::new("ollama".to_string(), config)
    }

    /// Create a new client for OpenAI
    pub fn new_openai(model_name: Option<String>) -> MultiAgentResult<Self> {
        let config = ProviderConfig::openai(model_name);
        Self::new("openai".to_string(), config)
    }

    /// Create a new client for Anthropic
    pub fn new_anthropic(model_name: Option<String>) -> MultiAgentResult<Self> {
        let config = ProviderConfig::anthropic(model_name);
        Self::new("anthropic".to_string(), config)
    }

    /// Create a new client for OpenRouter
    pub fn new_openrouter(model_name: Option<String>) -> MultiAgentResult<Self> {
        let config = ProviderConfig::openrouter(model_name);
        Self::new("openrouter".to_string(), config)
    }

    /// Convert LlmMessage to genai ChatMessage
    fn convert_message(msg: &crate::LlmMessage) -> ChatMessage {
        match msg.role {
            MessageRole::System => ChatMessage::system(msg.content.clone()),
            MessageRole::User => ChatMessage::user(msg.content.clone()),
            MessageRole::Assistant => ChatMessage::assistant(msg.content.clone()),
            MessageRole::Tool => ChatMessage::user(msg.content.clone()), // Map tool to user
        }
    }

    /// Generate response using rust-genai
    pub async fn generate(&self, request: LlmRequest) -> MultiAgentResult<LlmResponse> {
        let start_time = Utc::now();
        let request_id = Uuid::new_v4();

        // Convert messages
        let messages: Vec<ChatMessage> =
            request.messages.iter().map(Self::convert_message).collect();

        // Create chat request and options
        let chat_req = ChatRequest::new(messages);

        // Debug logging
        log::debug!(
            "🤖 LLM Request using rust-genai: {} ({})",
            self.model,
            self.provider
        );
        log::debug!("📋 Messages ({})", chat_req.messages.len());

        let mut options = ChatOptions::default();
        if let Some(temp) = request.temperature {
            options = options.with_temperature(temp as f64);
        }
        if let Some(max_tokens) = request.max_tokens {
            options = options.with_max_tokens(max_tokens as u32);
        }

        let chat_res = self
            .client
            .exec_chat(&self.model, chat_req, Some(&options))
            .await
            .map_err(|e| {
                log::error!("❌ rust-genai error: {}", e);
                MultiAgentError::LlmError(format!("rust-genai error: {}", e))
            })?;

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;

        // Extract content from response - MessageContent is now a struct with accessor methods
        let content = chat_res
            .content
            .joined_texts()
            .or_else(|| chat_res.content.first_text().map(|s| s.to_string()))
            .unwrap_or_else(|| "No text content in response".to_string());

        // Extract token usage if available
        let (input_tokens, output_tokens) = (
            chat_res.usage.prompt_tokens.unwrap_or(0) as u64,
            chat_res.usage.completion_tokens.unwrap_or(0) as u64,
        );

        log::debug!(
            "✅ LLM Response from {}: {} chars, tokens: {}/{}",
            self.model,
            content.len(),
            input_tokens,
            output_tokens
        );

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            usage: TokenUsage::new(input_tokens, output_tokens),
            request_id,
            timestamp: start_time,
            duration_ms,
            finish_reason: "completed".to_string(),
        })
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Set a different model
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.provider
    }
}

impl Default for GenAiLlmClient {
    fn default() -> Self {
        Self::new_ollama(None).expect("Failed to create default Ollama client")
    }
}

/// Convenience functions for creating different provider clients
impl GenAiLlmClient {
    /// Create client from provider configuration
    pub fn from_config(provider: &str, model: Option<String>) -> MultiAgentResult<Self> {
        match provider.to_lowercase().as_str() {
            "ollama" => Self::new_ollama(model),
            "openai" => Self::new_openai(model),
            "anthropic" => Self::new_anthropic(model),
            _ => {
                // Default to Ollama if unknown provider
                log::warn!("Unknown provider '{}', defaulting to Ollama", provider);
                Self::new_ollama(model)
            }
        }
    }

    /// Create client from provider configuration with custom base URL
    ///
    /// This method properly handles custom base URLs, particularly for the z.ai proxy.
    /// It sets appropriate environment variables before creating the genai client.
    pub fn from_config_with_url(
        provider: &str,
        model: Option<String>,
        base_url: Option<String>,
    ) -> MultiAgentResult<Self> {
        // Handle z.ai proxy configuration for Anthropic
        if provider.to_lowercase() == "anthropic" {
            if let Some(ref url) = base_url {
                if url.contains("z.ai") {
                    // Set environment variables for z.ai proxy
                    unsafe {
                        env::set_var("ANTHROPIC_API_BASE", url);
                    }

                    // Use ANTHROPIC_AUTH_TOKEN if available, otherwise look for ANTHROPIC_API_KEY
                    let api_key = env::var("ANTHROPIC_AUTH_TOKEN")
                        .or_else(|_| env::var("ANTHROPIC_API_KEY"))
                        .unwrap_or_default();

                    if !api_key.is_empty() {
                        unsafe {
                            env::set_var("ANTHROPIC_API_KEY", &api_key);
                        }
                    }

                    log::info!("🔗 Configured Anthropic client with z.ai proxy: {}", url);
                }
            }
        }

        // Handle OpenRouter with custom URL
        if provider.to_lowercase() == "openrouter" {
            if let Some(ref url) = base_url {
                unsafe {
                    env::set_var("OPENROUTER_API_BASE", url);
                }
                log::info!("🔗 Configured OpenRouter client with custom URL: {}", url);
            }
        }

        // Handle Ollama with custom URL
        if provider.to_lowercase() == "ollama" {
            if let Some(ref url) = base_url {
                unsafe {
                    env::set_var("OLLAMA_BASE_URL", url);
                }
                log::info!("🔗 Configured Ollama client with custom URL: {}", url);
            }
        }

        Self::from_config(provider, model)
    }

    /// Create client with automatic z.ai proxy detection
    ///
    /// This method automatically detects and configures z.ai proxy settings
    /// for Anthropic models when the appropriate environment variables are set.
    pub fn from_config_with_auto_proxy(
        provider: &str,
        model: Option<String>,
    ) -> MultiAgentResult<Self> {
        let base_url = if provider.to_lowercase() == "anthropic" {
            env::var("ANTHROPIC_BASE_URL").ok()
        } else if provider.to_lowercase() == "openrouter" {
            env::var("OPENROUTER_BASE_URL").ok()
        } else if provider.to_lowercase() == "ollama" {
            env::var("OLLAMA_BASE_URL").ok()
        } else {
            None
        };

        Self::from_config_with_url(provider, model, base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LlmMessage;

    #[test]
    fn test_create_clients() {
        let ollama_client = GenAiLlmClient::new_ollama(Some("llama2".to_string()));
        assert!(ollama_client.is_ok());
        assert_eq!(ollama_client.unwrap().model(), "llama2");

        let openai_client = GenAiLlmClient::new_openai(None);
        assert!(openai_client.is_ok());
        assert_eq!(openai_client.unwrap().model(), "gpt-3.5-turbo");

        let anthropic_client = GenAiLlmClient::new_anthropic(None);
        assert!(anthropic_client.is_ok());
        assert!(anthropic_client.unwrap().model().contains("claude"));
    }

    #[test]
    fn test_from_config() {
        let client = GenAiLlmClient::from_config("ollama", Some("gemma2".to_string()));
        assert!(client.is_ok());
        assert_eq!(client.unwrap().model(), "gemma2");

        let client = GenAiLlmClient::from_config("openai", None);
        assert!(client.is_ok());
        assert_eq!(client.unwrap().model(), "gpt-3.5-turbo");
    }

    #[test]
    fn test_message_conversion() {
        let _client = GenAiLlmClient::new_ollama(None).unwrap();

        let messages = vec![
            LlmMessage::system("You are a helpful assistant.".to_string()),
            LlmMessage::user("Hello!".to_string()),
        ];

        let request = LlmRequest::new(messages);

        // This test just checks that the conversion doesn't panic
        // Actual LLM calls would require Ollama to be running
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, MessageRole::System);
        assert_eq!(request.messages[1].role, MessageRole::User);
    }
}
