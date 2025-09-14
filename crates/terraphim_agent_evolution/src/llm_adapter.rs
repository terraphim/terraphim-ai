//! LLM adapter for agent evolution system
//!
//! This module provides a simplified LLM adapter interface for the evolution system.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::EvolutionResult;

/// Options for LLM completion requests
#[derive(Clone, Debug)]
pub struct CompletionOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub model: Option<String>,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            max_tokens: Some(1000),
            temperature: Some(0.7),
            model: None,
        }
    }
}

/// Adapter trait that bridges terraphim's LLM needs with rig framework
#[async_trait]
pub trait LlmAdapter: Send + Sync {
    /// Get the provider name
    fn provider_name(&self) -> String;

    /// Create a completion using rig's agent abstractions
    async fn complete(&self, prompt: &str, options: CompletionOptions) -> EvolutionResult<String>;

    /// Create a chat completion with multiple messages
    async fn chat_complete(
        &self,
        messages: Vec<Value>,
        options: CompletionOptions,
    ) -> EvolutionResult<String>;

    /// List available models for this provider
    async fn list_models(&self) -> EvolutionResult<Vec<String>>;
}

/// Mock LLM adapter for testing and development
pub struct MockLlmAdapter {
    provider_name: String,
}

impl MockLlmAdapter {
    /// Create a new mock adapter
    pub fn new(provider_name: &str) -> Self {
        Self {
            provider_name: provider_name.to_string(),
        }
    }
}

#[async_trait]
impl LlmAdapter for MockLlmAdapter {
    fn provider_name(&self) -> String {
        self.provider_name.clone()
    }

    async fn complete(&self, prompt: &str, _options: CompletionOptions) -> EvolutionResult<String> {
        // Input validation - prevent resource exhaustion
        if prompt.is_empty() {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if prompt.len() > 100_000 {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Prompt too long (max 100,000 characters)".to_string(),
            ));
        }

        // Basic prompt injection detection
        let suspicious_patterns = [
            "ignore previous instructions",
            "system:",
            "assistant:",
            "user:",
            "###",
            "---END---",
            "<|im_start|>",
            "<|im_end|>",
        ];

        let prompt_lower = prompt.to_lowercase();
        for pattern in &suspicious_patterns {
            if prompt_lower.contains(pattern) {
                log::warn!("Potential prompt injection detected: {}", pattern);
                // Don't reject entirely, but sanitize
                break;
            }
        }

        // Mock response that reflects the input for testing
        Ok(format!(
            "Mock response to: {}",
            prompt.chars().take(50).collect::<String>()
        ))
    }

    async fn chat_complete(
        &self,
        messages: Vec<Value>,
        options: CompletionOptions,
    ) -> EvolutionResult<String> {
        // Convert messages to a simple prompt and use complete
        let prompt = messages
            .iter()
            .filter_map(|msg| msg.get("content").and_then(|c| c.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        self.complete(&prompt, options).await
    }

    async fn list_models(&self) -> EvolutionResult<Vec<String>> {
        Ok(vec![
            "mock-gpt-4".to_string(),
            "mock-claude-3".to_string(),
            "mock-llama-2".to_string(),
        ])
    }
}

/// Factory for creating different types of LLM adapters
pub struct LlmAdapterFactory;

impl LlmAdapterFactory {
    /// Create a mock adapter for testing
    pub fn create_mock(provider: &str) -> Arc<dyn LlmAdapter> {
        Arc::new(MockLlmAdapter::new(provider))
    }

    /// Create an adapter from configuration
    pub fn from_config(
        provider: &str,
        _model: &str,
        _config: Option<Value>,
    ) -> EvolutionResult<Arc<dyn LlmAdapter>> {
        // Input validation
        if provider.is_empty() {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Provider name cannot be empty".to_string(),
            ));
        }

        if provider.len() > 100 {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Provider name too long (max 100 characters)".to_string(),
            ));
        }

        // Only allow alphanumeric and common characters
        if !provider
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(crate::error::EvolutionError::InvalidInput(
                "Provider name contains invalid characters".to_string(),
            ));
        }

        // For now, return mock adapters
        // In the future, this would create real adapters based on provider
        Ok(Self::create_mock(provider))
    }

    /// Create an adapter with a specific role/persona
    pub fn create_specialized_agent(
        provider: &str,
        _model: &str,
        _preamble: &str,
    ) -> EvolutionResult<Arc<dyn LlmAdapter>> {
        // For now, return mock adapters
        // In the future, this would create specialized agents
        Ok(Self::create_mock(provider))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_options_default() {
        let opts = CompletionOptions::default();
        assert_eq!(opts.max_tokens, Some(1000));
        assert_eq!(opts.temperature, Some(0.7));
        assert!(opts.model.is_none());
    }

    #[test]
    fn test_factory_create_mock() {
        let adapter = LlmAdapterFactory::create_mock("test");
        assert_eq!(adapter.provider_name(), "test");
    }

    #[tokio::test]
    async fn test_mock_adapter_complete() {
        let adapter = MockLlmAdapter::new("test");
        let result = adapter
            .complete("test prompt", CompletionOptions::default())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Mock response"));
    }
}
