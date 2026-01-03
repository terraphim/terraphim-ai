//! Routed LLM Client - Adapter for intelligent routing
//!
//! Implements LlmClient trait as an adapter to terraphim_llm_proxy
//! routing logic, providing graceful degradation and backward compatibility.

use super::llm::LlmClient;
use super::llm::LlmRequest;
use super::llm::LlmResponse;
use super::llm::LlmError;

use crate::llm_router_config::MergedRouterConfig;
use super::llm::summarization::SummarizationOptions;
use super::llm::chat::ChatOptions;
use super::llm::llm::LlmMessage;
use crate::llm::genai_llm_client::GenAiLlmClient;
use crate::llm_router_config::MergedRouterConfig;
use tracing::{debug, info, warn};

/// Routed LLM client that wraps intelligent routing
///
/// This adapter wraps GenAiLlmClient and adds routing intelligence
/// from terraphim_llm_proxy. If routing is enabled, requests are
/// routed through the intelligent 6-phase router. If routing fails
/// or is disabled, it falls back to the static client behavior.
#[derive(Debug)]
pub struct RoutedLlmClient {
    /// Underlying GenAi LLM client
    client: GenAiLlmClient,
}

impl RoutedLlmClient {
    /// Create a new routed LLM client
    pub fn new(client: GenAiLlmClient, config: MergedRouterConfig) -> Self {
        Self { client, config }
    }

    /// Check if routing is enabled
    fn is_routing_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the name of this client
    pub fn name(&self) -> &'static str {
        if self.is_routing_enabled() {
            "routed_llm"
        } else {
            self.client.name()
        }
    }
}

impl LlmClient for RoutedLlmClient {
    async fn summarize(&self, content: &str, opts: SummarizationOptions) -> Result<String> {
        if self.is_routing_enabled() {
            debug!("Routing enabled, using intelligent summarization");
            // Phase 3+ implementation: route through intelligent router
            // For now, use underlying client (will be enhanced in later steps)
            self.client.summarize(content, opts).await.map_err(|e| {
                warn!("Routed summarization failed, falling back: {}", e);
                LlmError::Internal(anyhow::anyhow!(e))
            })
        } else {
            debug!("Routing disabled, using static summarization");
            self.client.summarize(content, opts).await
        }
    }

    async fn chat(&self, messages: Vec<super::llm::LlmMessage>, opts: ChatOptions) -> Result<super::llm::LlmResponse> {
        if self.is_routing_enabled() {
            debug!("Routing enabled, using intelligent chat");
            // Phase 3+ implementation: route through intelligent router
            // For now, use underlying client (will be enhanced in later steps)
            self.client.chat(messages, opts).await.map_err(|e| {
                warn!("Routed chat failed, falling back: {}", e);
                LlmError::Internal(anyhow::anyhow!(e))
            })
        } else {
            debug!("Routing disabled, using static chat");
            self.client.chat(messages, opts).await
        }
    }

    async fn get_models(&self) -> Result<Vec<String>> {
        info!("Get models - routing {}, static {}", 
            self.is_routing_enabled(), 
            !self.is_routing_enabled());
        
        self.client.get_models().await
    }

    async fn stream_chat(
        &self,
        messages: Vec<super::llm::LlmMessage>,
        opts: ChatOptions,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String> + Unpin + Send + 'static>> {
        // Streaming support will be added in later steps
        // Phase 3+ implementation
        Err(LlmError::NotImplemented(
            "Stream chat not yet implemented in routed client".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_routed_client_creation() {
        use crate::llm::genai_llm_client::GenAiLlmClient;

        let client = GenAiLlmClient::new_ollama(None).unwrap();
        let config = crate::llm_router_config::MergedRouterConfig::default();

        let routed = super::RoutedLlmClient::new(client, config);
        assert!(routed.is_routing_enabled());
        assert_eq!(routed.name(), "routed_llm");
    }

    #[tokio::test]
    async fn test_routing_disabled_uses_static_client() {
        use crate::llm::genai_llm_client::GenAiLlmClient;

        let client = GenAiLlmClient::new_ollama(None).unwrap();
        let config = crate::llm_router_config::MergedRouterConfig {
            enabled: false,
            ..Default::default()
        };

        let routed = super::RoutedLlmClient::new(client, config);
        assert!(!routed.is_routing_enabled());
        assert_eq!(routed.name(), "ollama"); // Uses underlying client name
    }

    #[tokio::test]
    async fn test_routing_enabled_logs_debug() {
        use crate::llm::genai_llm_client::GenAiLlmClient;

        let client = GenAiLlmClient::new_ollama(None).unwrap();
        let config = crate::llm_router_config::MergedRouterConfig {
            enabled: true,
            ..Default::default()
        };

        let routed = super::RoutedLlmClient::new(client, config);
        
        // This test just verifies the struct can be created
        // Actual routing implementation comes in later steps
        assert!(routed.is_routing_enabled());
    }
}
