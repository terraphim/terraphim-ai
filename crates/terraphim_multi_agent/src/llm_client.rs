//! LLM client integration using Rig framework
//!
//! This module provides a professional LLM client that leverages the Rig framework
//! for token counting, cost tracking, and provider abstraction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// TODO: Add rig_core imports when available
// use rig_core::{
//     completion::{Completion, CompletionError, CompletionModel},
//     model::Model,
//     provider::{self, Provider},
// };

use crate::{
    AgentId, CostRecord, CostTracker, MultiAgentResult, TokenUsageRecord, TokenUsageTracker,
};

/// LLM client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClientConfig {
    /// Provider type (openai, anthropic, ollama, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// API key (optional for local models)
    pub api_key: Option<String>,
    /// Base URL (for custom endpoints)
    pub base_url: Option<String>,
    /// Default temperature
    pub temperature: f32,
    /// Max tokens per request
    pub max_tokens: u32,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Enable cost tracking
    pub track_costs: bool,
}

impl Default for LlmClientConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            api_key: None,
            base_url: None,
            temperature: 0.7,
            max_tokens: 4096,
            timeout_seconds: 30,
            track_costs: true,
        }
    }
}

/// Message role in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// LLM message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl LlmMessage {
    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            timestamp: Utc::now(),
        }
    }
}

/// LLM request parameters
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl LlmRequest {
    pub fn new(messages: Vec<LlmMessage>) -> Self {
        Self {
            messages,
            temperature: None,
            max_tokens: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// LLM response with detailed usage information
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub finish_reason: String,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl TokenUsage {
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// Professional LLM client using Rig framework
pub struct RigLlmClient {
    config: LlmClientConfig,
    agent_id: AgentId,
    token_tracker: Arc<RwLock<TokenUsageTracker>>,
    cost_tracker: Arc<RwLock<CostTracker>>,
    // TODO: Add actual Rig model when we implement provider-specific logic
}

impl RigLlmClient {
    /// Create a new LLM client
    pub async fn new(
        config: LlmClientConfig,
        agent_id: AgentId,
        token_tracker: Arc<RwLock<TokenUsageTracker>>,
        cost_tracker: Arc<RwLock<CostTracker>>,
    ) -> MultiAgentResult<Self> {
        // TODO: Initialize Rig provider based on config

        log::info!(
            "Created LLM client for agent {} using {} model {}",
            agent_id,
            config.provider,
            config.model
        );

        Ok(Self {
            config,
            agent_id,
            token_tracker,
            cost_tracker,
        })
    }

    /// Generate completion using Rig framework
    pub async fn complete(&self, request: LlmRequest) -> MultiAgentResult<LlmResponse> {
        let start_time = Utc::now();
        let request_id = Uuid::new_v4();

        log::debug!(
            "Starting LLM completion for agent {} (request: {})",
            self.agent_id,
            request_id
        );

        // TODO: Implement actual Rig completion
        let response = self
            .mock_completion(&request, request_id, start_time)
            .await?;

        // Track usage
        self.track_usage(&response).await?;

        log::debug!(
            "Completed LLM request {} in {}ms (tokens: {} input, {} output)",
            request_id,
            response.duration_ms,
            response.usage.input_tokens,
            response.usage.output_tokens
        );

        Ok(response)
    }

    /// Generate streaming completion
    pub async fn stream_complete(
        &self,
        _request: LlmRequest,
    ) -> MultiAgentResult<tokio::sync::mpsc::Receiver<String>> {
        // TODO: Implement streaming completion using Rig
        let (_tx, rx) = tokio::sync::mpsc::channel(100);

        log::debug!(
            "Started streaming completion for agent {} (not yet implemented)",
            self.agent_id
        );

        Ok(rx)
    }

    /// Check if model is available
    pub async fn check_availability(&self) -> MultiAgentResult<bool> {
        // TODO: Implement actual availability check
        log::debug!(
            "Checking availability for {} model {} (mock: available)",
            self.config.provider,
            self.config.model
        );

        Ok(true)
    }

    /// Get model capabilities
    pub fn get_capabilities(&self) -> ModelCapabilities {
        // TODO: Get actual capabilities from Rig model info
        ModelCapabilities {
            max_context_tokens: match self.config.model.as_str() {
                model if model.contains("gpt-4") => 128000,
                model if model.contains("gpt-3.5") => 16384,
                model if model.contains("claude-3") => 200000,
                _ => 8192,
            },
            supports_streaming: true,
            supports_function_calling: self.config.model.contains("gpt-4")
                || self.config.model.contains("gpt-3.5"),
            supports_vision: self.config.model.contains("vision")
                || self.config.model.contains("gpt-4"),
        }
    }

    // Private methods

    /// Mock completion for testing (TODO: replace with actual Rig implementation)
    async fn mock_completion(
        &self,
        request: &LlmRequest,
        request_id: Uuid,
        start_time: DateTime<Utc>,
    ) -> MultiAgentResult<LlmResponse> {
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let input_text: String = request
            .messages
            .iter()
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // Rough token estimation (4 chars = 1 token)
        let input_tokens = input_text.len() as u64 / 4;
        let output_content = format!(
            "This is a mock response from {} model for agent {}. \
             The request contained {} messages with approximately {} input tokens.",
            self.config.model,
            self.agent_id,
            request.messages.len(),
            input_tokens
        );
        let output_tokens = output_content.len() as u64 / 4;

        let duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;

        Ok(LlmResponse {
            content: output_content,
            model: self.config.model.clone(),
            usage: TokenUsage::new(input_tokens, output_tokens),
            request_id,
            timestamp: Utc::now(),
            duration_ms,
            finish_reason: "completed".to_string(),
        })
    }

    /// Track token usage and costs
    async fn track_usage(&self, response: &LlmResponse) -> MultiAgentResult<()> {
        // Track tokens
        {
            let mut tracker = self.token_tracker.write().await;
            let record = TokenUsageRecord {
                request_id: response.request_id,
                timestamp: response.timestamp,
                agent_id: self.agent_id,
                model: response.model.clone(),
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                total_tokens: response.usage.total_tokens,
                cost_usd: self.calculate_cost(&response.usage).await?,
                duration_ms: response.duration_ms,
                quality_score: None, // TODO: Implement quality scoring
            };
            tracker.add_record(record)?;
        }

        // Track costs if enabled
        if self.config.track_costs {
            let mut cost_tracker = self.cost_tracker.write().await;
            let cost = self.calculate_cost(&response.usage).await?;
            let cost_record = CostRecord {
                timestamp: response.timestamp,
                agent_id: self.agent_id,
                operation_type: "llm_completion".to_string(),
                cost_usd: cost,
                metadata: [
                    ("model".to_string(), response.model.clone()),
                    (
                        "input_tokens".to_string(),
                        response.usage.input_tokens.to_string(),
                    ),
                    (
                        "output_tokens".to_string(),
                        response.usage.output_tokens.to_string(),
                    ),
                ]
                .into(),
            };
            cost_tracker.add_record(cost_record)?;
        }

        Ok(())
    }

    /// Calculate cost based on token usage
    async fn calculate_cost(&self, usage: &TokenUsage) -> MultiAgentResult<f64> {
        // TODO: Get actual pricing from Rig or pricing database
        let (input_cost_per_1k, output_cost_per_1k) = match self.config.model.as_str() {
            "gpt-4" => (0.03, 0.06),
            "gpt-4-turbo" => (0.01, 0.03),
            "gpt-3.5-turbo" => (0.0015, 0.002),
            "claude-3-opus" => (0.015, 0.075),
            "claude-3-sonnet" => (0.003, 0.015),
            "claude-3-haiku" => (0.00025, 0.00125),
            _ => (0.001, 0.002), // Default fallback
        };

        let input_cost = (usage.input_tokens as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (usage.output_tokens as f64 / 1000.0) * output_cost_per_1k;

        Ok(input_cost + output_cost)
    }
}

/// Model capabilities information
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub max_context_tokens: u64,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
}

/// Extract LLM configuration from role extra parameters
use ahash::AHashMap;

pub fn extract_llm_config(extra: &AHashMap<String, serde_json::Value>) -> LlmClientConfig {
    let mut config = LlmClientConfig::default();

    if let Some(provider) = extra.get("llm_provider") {
        if let Some(provider_str) = provider.as_str() {
            config.provider = provider_str.to_string();
        }
    }

    if let Some(model) = extra.get("llm_model") {
        if let Some(model_str) = model.as_str() {
            config.model = model_str.to_string();
        }
    }

    if let Some(api_key) = extra.get("llm_api_key") {
        if let Some(key_str) = api_key.as_str() {
            config.api_key = Some(key_str.to_string());
        }
    }

    if let Some(base_url) = extra.get("llm_base_url") {
        if let Some(url_str) = base_url.as_str() {
            config.base_url = Some(url_str.to_string());
        }
    }

    if let Some(temperature) = extra.get("llm_temperature") {
        if let Some(temp_f64) = temperature.as_f64() {
            config.temperature = temp_f64 as f32;
        }
    }

    if let Some(max_tokens) = extra.get("llm_max_tokens") {
        if let Some(tokens_u64) = max_tokens.as_u64() {
            config.max_tokens = tokens_u64 as u32;
        }
    }

    // Handle Ollama-specific configuration
    if config.provider == "ollama" {
        if let Some(ollama_base_url) = extra.get("ollama_base_url") {
            if let Some(url_str) = ollama_base_url.as_str() {
                config.base_url = Some(url_str.to_string());
            }
        }
        if let Some(ollama_model) = extra.get("ollama_model") {
            if let Some(model_str) = ollama_model.as_str() {
                config.model = model_str.to_string();
            }
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    //use ahash::AHashMap;

    #[test]
    fn test_llm_message_creation() {
        let system_msg = LlmMessage::system("You are a helpful assistant".to_string());
        assert!(matches!(system_msg.role, MessageRole::System));
        assert_eq!(system_msg.content, "You are a helpful assistant");

        let user_msg = LlmMessage::user("Hello!".to_string());
        assert!(matches!(user_msg.role, MessageRole::User));
        assert_eq!(user_msg.content, "Hello!");
    }

    #[test]
    fn test_llm_request_builder() {
        let messages = vec![
            LlmMessage::system("System prompt".to_string()),
            LlmMessage::user("User message".to_string()),
        ];

        let request = LlmRequest::new(messages)
            .with_temperature(0.8)
            .with_max_tokens(2048)
            .with_metadata("test_key".to_string(), "test_value".to_string());

        assert_eq!(request.temperature, Some(0.8));
        assert_eq!(request.max_tokens, Some(2048));
        assert_eq!(
            request.metadata.get("test_key"),
            Some(&"test_value".to_string())
        );
    }

    #[test]
    fn test_extract_llm_config() {
        let mut extra = ahash::AHashMap::new();
        extra.insert(
            "llm_provider".to_string(),
            serde_json::Value::String("ollama".to_string()),
        );
        extra.insert(
            "ollama_model".to_string(),
            serde_json::Value::String("llama3.2:3b".to_string()),
        );
        extra.insert(
            "ollama_base_url".to_string(),
            serde_json::Value::String("http://127.0.0.1:11434".to_string()),
        );
        extra.insert(
            "llm_temperature".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(0.5).unwrap()),
        );

        let config = extract_llm_config(&extra);

        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "llama3.2:3b");
        assert_eq!(config.base_url, Some("http://127.0.0.1:11434".to_string()));
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_token_usage_calculation() {
        let usage = TokenUsage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }
}
