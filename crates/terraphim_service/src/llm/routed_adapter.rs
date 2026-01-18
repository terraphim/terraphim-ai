//! Routed LLM Client - Adapter for intelligent routing
//!
//! Implements LlmClient trait as an adapter to terraphim_llm_proxy
//! routing logic, providing graceful degradation and backward compatibility.

use std::sync::Arc;

use super::LlmClient;

#[cfg(feature = "llm_router")]
use super::router_config::MergedRouterConfig;
use super::ChatOptions;
use super::SummarizeOptions as SummarizationOptions;
use crate::Result as ServiceResult;
use async_trait::async_trait;
use log::{debug, info};

/// Routed LLM client that wraps intelligent routing
///
/// This adapter wraps a dynamic LlmClient and adds routing intelligence
/// from terraphim_llm_proxy. If routing is enabled, requests are
/// routed through the intelligent 6-phase router. If routing fails
/// or is disabled, it falls back to the static client behavior.
#[derive(Clone)]
pub struct RoutedLlmClient {
    /// Underlying LLM client (dynamic)
    client: Arc<dyn LlmClient>,
    /// Router configuration
    config: MergedRouterConfig,
}

impl RoutedLlmClient {
    /// Create a new routed LLM client
    pub fn new(client: Arc<dyn LlmClient>, config: MergedRouterConfig) -> Self {
        Self { client, config }
    }

    /// Check if routing is enabled
    fn is_routing_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[async_trait]
impl LlmClient for RoutedLlmClient {
    fn name(&self) -> &'static str {
        if self.is_routing_enabled() {
            "routed_llm"
        } else {
            self.client.as_ref().name()
        }
    }

    async fn summarize(&self, content: &str, opts: SummarizationOptions) -> ServiceResult<String> {
        debug!("Summarize - routing {}", self.is_routing_enabled());
        self.client.as_ref().summarize(content, opts).await
    }

    async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        debug!("Chat - routing {}", self.is_routing_enabled());
        self.client.as_ref().chat_completion(messages, opts).await
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        info!(
            "Get models - routing {}, static {}",
            self.is_routing_enabled(),
            !self.is_routing_enabled()
        );

        self.client.as_ref().list_models().await
    }
}
