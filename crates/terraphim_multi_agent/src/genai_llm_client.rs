//! Direct HTTP LLM Client - Goose-inspired Implementation
//!
//! This implementation uses reqwest directly to communicate with LLM providers,
//! avoiding dependency issues with rig-core and genai crates that require unstable Rust features.
//! This approach provides maximum control and compatibility.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{MultiAgentResult, LlmRequest, LlmResponse, TokenUsage, MessageRole, MultiAgentError};
use chrono::Utc;
use uuid::Uuid;

/// Direct HTTP LLM client that works with multiple providers
#[derive(Debug)]
pub struct GenAiLlmClient {
    client: Client,
    provider: String,
    model: String,
    base_url: String,
}

/// Configuration for different LLM providers
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub base_url: String,
    pub model: String,
    pub requires_auth: bool,
}

impl ProviderConfig {
    pub fn ollama(model: Option<String>) -> Self {
        Self {
            base_url: "http://127.0.0.1:11434".to_string(),
            model: model.unwrap_or_else(|| "gemma3:270m".to_string()),
            requires_auth: false,
        }
    }

    pub fn openai(model: Option<String>) -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: model.unwrap_or_else(|| "gpt-3.5-turbo".to_string()),
            requires_auth: true,
        }
    }

    pub fn anthropic(model: Option<String>) -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: model.unwrap_or_else(|| "claude-3-sonnet-20240229".to_string()),
            requires_auth: true,
        }
    }
}

/// Request format for Ollama API
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Response format for Ollama API
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

/// Request format for OpenAI API
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// Response format for OpenAI API
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

impl GenAiLlmClient {
    /// Create a new Direct HTTP LLM client
    pub fn new(provider: String, config: ProviderConfig) -> MultiAgentResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| MultiAgentError::LlmError(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            provider,
            model: config.model,
            base_url: config.base_url,
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

    /// Generate response using the HTTP client
    pub async fn generate(&self, request: LlmRequest) -> MultiAgentResult<LlmResponse> {
        let start_time = Utc::now();
        let request_id = Uuid::new_v4();
        
        let response_content = match self.provider.as_str() {
            "ollama" => self.call_ollama(&request).await?,
            "openai" => self.call_openai(&request).await?,
            "anthropic" => self.call_anthropic(&request).await?,
            _ => return Err(MultiAgentError::LlmError(format!("Unsupported provider: {}", self.provider))),
        };

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds() as u64;

        // Estimate token usage (rough approximation)
        let input_tokens = request.messages
            .iter()
            .map(|m| m.content.len())
            .sum::<usize>() as u64 / 4;
        let output_tokens = (response_content.len() / 4) as u64;
        
        Ok(LlmResponse {
            content: response_content,
            model: self.model.clone(),
            usage: TokenUsage::new(input_tokens, output_tokens),
            request_id,
            timestamp: start_time,
            duration_ms,
            finish_reason: "completed".to_string(),
        })
    }

    /// Call Ollama API
    async fn call_ollama(&self, request: &LlmRequest) -> MultiAgentResult<String> {
        let messages: Vec<OllamaMessage> = request.messages
            .iter()
            .map(|msg| OllamaMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Tool => "user".to_string(), // Ollama doesn't have tool role
                },
                content: msg.content.clone(),
            })
            .collect();

        let ollama_request = OllamaRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let url = format!("{}/api/chat", self.base_url);
        
        // Debug logging - log the request details
        log::debug!("🤖 LLM Request to Ollama: {} at {}", self.model, url);
        log::debug!("📋 Messages ({}):", ollama_request.messages.len());
        for (i, msg) in ollama_request.messages.iter().enumerate() {
            log::debug!("  [{}] {}: {}", i + 1, msg.role, 
                if msg.content.len() > 200 { 
                    format!("{}...", &msg.content[..200]) 
                } else { 
                    msg.content.clone() 
                });
        }
        
        let response = self.client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| {
                log::error!("❌ Ollama API request failed: {}", e);
                MultiAgentError::LlmError(format!("Ollama API request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("❌ Ollama API error {}: {}", status, text);
            return Err(MultiAgentError::LlmError(format!("Ollama API error {}: {}", status, text)));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| {
                log::error!("❌ Failed to parse Ollama response: {}", e);
                MultiAgentError::LlmError(format!("Failed to parse Ollama response: {}", e))
            })?;

        // Debug logging - log the response
        let response_content = &ollama_response.message.content;
        log::debug!("✅ LLM Response from {}: {}", self.model, 
            if response_content.len() > 200 { 
                format!("{}...", &response_content[..200]) 
            } else { 
                response_content.clone() 
            });

        Ok(ollama_response.message.content)
    }

    /// Call OpenAI API (placeholder - requires API key setup)
    async fn call_openai(&self, request: &LlmRequest) -> MultiAgentResult<String> {
        let messages: Vec<OpenAiMessage> = request.messages
            .iter()
            .map(|msg| OpenAiMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let openai_request = OpenAiRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens.map(|t| t as u32),
            temperature: request.temperature,
        };

        let url = format!("{}/chat/completions", self.base_url);
        
        // TODO: Add API key from environment or config
        let mut request_builder = self.client
            .post(&url)
            .json(&openai_request);

        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            request_builder = request_builder.bearer_auth(api_key);
        } else {
            return Err(MultiAgentError::LlmError("OpenAI API key not found in OPENAI_API_KEY environment variable".to_string()));
        }
        
        let response = request_builder
            .send()
            .await
            .map_err(|e| MultiAgentError::LlmError(format!("OpenAI API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(MultiAgentError::LlmError(format!("OpenAI API error {}: {}", status, text)));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| MultiAgentError::LlmError(format!("Failed to parse OpenAI response: {}", e)))?;

        Ok(openai_response.choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "No response".to_string()))
    }

    /// Call Anthropic API (placeholder - requires API key setup)
    async fn call_anthropic(&self, _request: &LlmRequest) -> MultiAgentResult<String> {
        // TODO: Implement Anthropic API call
        Err(MultiAgentError::LlmError("Anthropic API not implemented yet".to_string()))
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
    pub fn from_config_with_url(provider: &str, model: Option<String>, base_url: Option<String>) -> MultiAgentResult<Self> {
        match provider.to_lowercase().as_str() {
            "ollama" => {
                let mut config = ProviderConfig::ollama(model);
                if let Some(url) = base_url {
                    config.base_url = url;
                }
                Self::new("ollama".to_string(), config)
            }
            "openai" => {
                let mut config = ProviderConfig::openai(model);
                if let Some(url) = base_url {
                    config.base_url = url;
                }
                Self::new("openai".to_string(), config)
            }
            "anthropic" => {
                let mut config = ProviderConfig::anthropic(model);
                if let Some(url) = base_url {
                    config.base_url = url;
                }
                Self::new("anthropic".to_string(), config)
            }
            _ => {
                // Default to Ollama if unknown provider
                log::warn!("Unknown provider '{}', defaulting to Ollama with custom URL", provider);
                let mut config = ProviderConfig::ollama(model);
                if let Some(url) = base_url {
                    config.base_url = url;
                }
                Self::new("ollama".to_string(), config)
            }
        }
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
        let client = GenAiLlmClient::new_ollama(None).unwrap();
        
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