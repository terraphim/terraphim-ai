//! Router Bridge -- connects terraphim_router provider selection to LlmClient execution.
//!
//! This module provides `RouterBridgeLlmClient`, which implements `LlmClient`
//! and uses `terraphim_router::Router` for capability-based routing. Each registered
//! provider maps to a concrete `Arc<dyn LlmClient>`.

use std::sync::Arc;

use ahash::AHashMap;
use async_trait::async_trait;
use terraphim_router::{
    Capability, CostLevel, Latency, Provider, ProviderType, Router, RouterMetrics, RoutingContext,
    strategy::{CapabilityFirst, CostOptimized, RoundRobin, RoutingStrategy},
};

use super::router_config::{MergedRouterConfig, RouterStrategy};
use super::{ChatOptions, LlmClient, SummarizeOptions};
use crate::Result as ServiceResult;

/// Associates a Router Provider with its executable LLM client.
pub struct LlmProviderDescriptor {
    /// Router-level provider metadata (capabilities, cost, latency, keywords).
    pub provider: Provider,
    /// The LlmClient that executes requests for this provider.
    pub client: Arc<dyn LlmClient>,
}

/// Router-based LLM client that selects the best provider for each request.
///
/// On each call, extracts capabilities from the prompt, routes via
/// `terraphim_router::Router`, then executes against the matched `LlmClient`.
pub struct RouterBridgeLlmClient {
    router: Router,
    clients: AHashMap<String, Arc<dyn LlmClient>>,
    fallback: Arc<dyn LlmClient>,
    #[allow(dead_code)]
    metrics: Arc<RouterMetrics>,
    #[allow(dead_code)]
    config: MergedRouterConfig,
}

impl RouterBridgeLlmClient {
    /// Create a new bridge with a fallback client and configuration.
    pub fn new(fallback: Arc<dyn LlmClient>, config: MergedRouterConfig) -> Self {
        let strategy = strategy_from_config(&config.strategy);
        let router = Router::new().with_strategy(strategy);

        Self {
            router,
            clients: AHashMap::new(),
            fallback,
            metrics: Arc::new(RouterMetrics::new()),
            config,
        }
    }

    /// Register a provider + client pair.
    pub fn register_provider(&mut self, descriptor: LlmProviderDescriptor) {
        let provider_id = descriptor.provider.id.clone();
        self.router.add_provider(descriptor.provider);
        self.clients.insert(provider_id, descriptor.client);
    }

    /// Resolve the best client for a given prompt.
    fn resolve_client(&self, prompt: &str) -> Arc<dyn LlmClient> {
        // Short-circuit: single provider -> skip routing
        if self.clients.len() <= 1 {
            if let Some(client) = self.clients.values().next() {
                return Arc::clone(client);
            }
            return Arc::clone(&self.fallback);
        }

        let context = RoutingContext::default();
        match self.router.route(prompt, &context) {
            Ok(decision) => {
                if let Some(client) = self.clients.get(&decision.provider.id) {
                    log::info!(
                        "Routed to provider '{}' (confidence={:.2}, reason={:?})",
                        decision.provider.id,
                        decision.confidence,
                        decision.reason
                    );
                    Arc::clone(client)
                } else {
                    log::warn!(
                        "Router selected provider '{}' but no client registered, using fallback",
                        decision.provider.id
                    );
                    Arc::clone(&self.fallback)
                }
            }
            Err(e) => {
                log::debug!("Routing failed: {:?}, using fallback", e);
                Arc::clone(&self.fallback)
            }
        }
    }
}

#[async_trait]
impl LlmClient for RouterBridgeLlmClient {
    fn name(&self) -> &'static str {
        "routed_llm"
    }

    async fn summarize(&self, content: &str, opts: SummarizeOptions) -> ServiceResult<String> {
        let client = self.resolve_client(content);
        client.summarize(content, opts).await
    }

    async fn chat_completion(
        &self,
        messages: Vec<serde_json::Value>,
        opts: ChatOptions,
    ) -> ServiceResult<String> {
        // Extract text from last user message for routing
        let prompt = extract_routing_text(&messages);
        let client = self.resolve_client(&prompt);
        client.chat_completion(messages, opts).await
    }

    async fn list_models(&self) -> ServiceResult<Vec<String>> {
        let mut all_models = Vec::new();
        for client in self.clients.values() {
            if let Ok(models) = client.list_models().await {
                all_models.extend(models);
            }
        }
        Ok(all_models)
    }
}

/// Map `RouterStrategy` from config to a `Box<dyn RoutingStrategy>`.
fn strategy_from_config(strategy: &RouterStrategy) -> Box<dyn RoutingStrategy> {
    match strategy {
        RouterStrategy::CostFirst => Box::new(CostOptimized),
        RouterStrategy::QualityFirst => Box::new(CapabilityFirst),
        RouterStrategy::Balanced => Box::new(CostOptimized),
        RouterStrategy::Static => Box::new(RoundRobin::new()),
    }
}

/// Build a `Provider` from an LLM client and role configuration.
pub fn provider_from_llm_client(client: &dyn LlmClient, role: &terraphim_config::Role) -> Provider {
    match client.name() {
        "ollama" => {
            let model = super::get_string_extra(&role.extra, "llm_model")
                .unwrap_or_else(|| "llama3.1".to_string());
            let base_url = super::get_string_extra(&role.extra, "ollama_base_url")
                .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

            Provider::new(
                "ollama",
                "Ollama Local",
                ProviderType::Llm {
                    model_id: model,
                    api_endpoint: base_url,
                },
                vec![
                    Capability::CodeGeneration,
                    Capability::Explanation,
                    Capability::FastThinking,
                    Capability::Documentation,
                ],
            )
            .with_cost(CostLevel::Cheap)
            .with_latency(Latency::Fast)
        }
        "openrouter" => {
            let model = role.llm_model.clone().unwrap_or_default();

            Provider::new(
                "openrouter",
                "OpenRouter Cloud",
                ProviderType::Llm {
                    model_id: model,
                    api_endpoint: "https://openrouter.ai/api/v1".to_string(),
                },
                vec![
                    Capability::DeepThinking,
                    Capability::CodeGeneration,
                    Capability::CodeReview,
                    Capability::Architecture,
                ],
            )
            .with_cost(CostLevel::Expensive)
            .with_latency(Latency::Medium)
        }
        other => Provider::new(
            other,
            other,
            ProviderType::Llm {
                model_id: "unknown".to_string(),
                api_endpoint: "unknown".to_string(),
            },
            vec![
                Capability::CodeGeneration,
                Capability::Explanation,
                Capability::FastThinking,
            ],
        )
        .with_cost(CostLevel::Moderate)
        .with_latency(Latency::Medium),
    }
}

/// Extract text from chat messages for routing purposes.
fn extract_routing_text(messages: &[serde_json::Value]) -> String {
    // Find the last user message
    for msg in messages.iter().rev() {
        if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
            if role == "user" {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    return content.to_string();
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_mapping_cost_first() {
        let strategy = strategy_from_config(&RouterStrategy::CostFirst);
        assert_eq!(strategy.name(), "cost_optimized");
    }

    #[test]
    fn test_strategy_mapping_quality_first() {
        let strategy = strategy_from_config(&RouterStrategy::QualityFirst);
        assert_eq!(strategy.name(), "capability_first");
    }

    #[test]
    fn test_strategy_mapping_balanced() {
        let strategy = strategy_from_config(&RouterStrategy::Balanced);
        assert_eq!(strategy.name(), "cost_optimized");
    }

    #[test]
    fn test_strategy_mapping_static() {
        let strategy = strategy_from_config(&RouterStrategy::Static);
        assert_eq!(strategy.name(), "round_robin");
    }

    #[test]
    fn test_extract_routing_text_last_user_message() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are helpful"}),
            serde_json::json!({"role": "user", "content": "Implement a function"}),
            serde_json::json!({"role": "assistant", "content": "Sure!"}),
            serde_json::json!({"role": "user", "content": "Now review the code"}),
        ];
        assert_eq!(extract_routing_text(&messages), "Now review the code");
    }

    #[test]
    fn test_extract_routing_text_empty() {
        let messages: Vec<serde_json::Value> = vec![];
        assert_eq!(extract_routing_text(&messages), "");
    }

    /// Minimal LlmClient for testing
    struct TestClient {
        name: &'static str,
    }

    #[async_trait]
    impl LlmClient for TestClient {
        fn name(&self) -> &'static str {
            self.name
        }
        async fn summarize(
            &self,
            _content: &str,
            _opts: SummarizeOptions,
        ) -> ServiceResult<String> {
            Ok(format!("summary from {}", self.name))
        }
    }

    #[test]
    fn test_bridge_name() {
        let fallback: Arc<dyn LlmClient> = Arc::new(TestClient { name: "test" });
        let bridge = RouterBridgeLlmClient::new(fallback, MergedRouterConfig::default());
        assert_eq!(bridge.name(), "routed_llm");
    }

    #[test]
    fn test_bridge_single_provider_shortcircuit() {
        let fallback: Arc<dyn LlmClient> = Arc::new(TestClient { name: "fallback" });
        let client: Arc<dyn LlmClient> = Arc::new(TestClient { name: "ollama" });

        let mut bridge = RouterBridgeLlmClient::new(fallback, MergedRouterConfig::default());
        bridge.register_provider(LlmProviderDescriptor {
            provider: Provider::new(
                "ollama",
                "Ollama",
                ProviderType::Llm {
                    model_id: "test".to_string(),
                    api_endpoint: "http://localhost".to_string(),
                },
                vec![Capability::CodeGeneration],
            ),
            client: client.clone(),
        });

        // With only one provider, should skip routing and use it directly
        let resolved = bridge.resolve_client("anything");
        assert_eq!(resolved.name(), "ollama");
    }

    #[test]
    fn test_bridge_routes_to_correct_provider() {
        let fallback: Arc<dyn LlmClient> = Arc::new(TestClient { name: "fallback" });
        let cheap: Arc<dyn LlmClient> = Arc::new(TestClient { name: "cheap" });
        let expensive: Arc<dyn LlmClient> = Arc::new(TestClient { name: "expensive" });

        let config = MergedRouterConfig {
            strategy: RouterStrategy::CostFirst,
            ..Default::default()
        };
        let mut bridge = RouterBridgeLlmClient::new(fallback, config);

        bridge.register_provider(LlmProviderDescriptor {
            provider: Provider::new(
                "cheap-coder",
                "Cheap Coder",
                ProviderType::Llm {
                    model_id: "cheap".to_string(),
                    api_endpoint: "http://localhost".to_string(),
                },
                vec![Capability::CodeGeneration],
            )
            .with_cost(CostLevel::Cheap),
            client: cheap,
        });

        bridge.register_provider(LlmProviderDescriptor {
            provider: Provider::new(
                "expensive-thinker",
                "Expensive Thinker",
                ProviderType::Llm {
                    model_id: "expensive".to_string(),
                    api_endpoint: "http://localhost".to_string(),
                },
                vec![Capability::DeepThinking],
            )
            .with_cost(CostLevel::Expensive),
            client: expensive,
        });

        // "Implement a function" -> CodeGeneration -> cheap-coder (CostFirst strategy)
        let resolved = bridge.resolve_client("Implement a function to parse JSON");
        assert_eq!(resolved.name(), "cheap");
    }

    #[test]
    fn test_bridge_falls_back_on_routing_failure() {
        let fallback: Arc<dyn LlmClient> = Arc::new(TestClient { name: "fallback" });

        // Create bridge with no providers
        let bridge =
            RouterBridgeLlmClient::new(Arc::clone(&fallback), MergedRouterConfig::default());

        // With no providers, should use fallback
        let resolved = bridge.resolve_client("Hello world");
        assert_eq!(resolved.name(), "fallback");
    }

    #[tokio::test]
    async fn test_list_models_aggregates() {
        struct ModelClient {
            name: &'static str,
            models: Vec<String>,
        }

        #[async_trait]
        impl LlmClient for ModelClient {
            fn name(&self) -> &'static str {
                self.name
            }
            async fn summarize(
                &self,
                _content: &str,
                _opts: SummarizeOptions,
            ) -> ServiceResult<String> {
                Ok(String::new())
            }
            async fn list_models(&self) -> ServiceResult<Vec<String>> {
                Ok(self.models.clone())
            }
        }

        let fallback: Arc<dyn LlmClient> = Arc::new(ModelClient {
            name: "fallback",
            models: vec![],
        });
        let client_a: Arc<dyn LlmClient> = Arc::new(ModelClient {
            name: "a",
            models: vec!["model-a1".to_string(), "model-a2".to_string()],
        });
        let client_b: Arc<dyn LlmClient> = Arc::new(ModelClient {
            name: "b",
            models: vec!["model-b1".to_string()],
        });

        let mut bridge = RouterBridgeLlmClient::new(fallback, MergedRouterConfig::default());
        bridge.register_provider(LlmProviderDescriptor {
            provider: Provider::new(
                "a",
                "A",
                ProviderType::Llm {
                    model_id: "a".to_string(),
                    api_endpoint: "http://a".to_string(),
                },
                vec![Capability::CodeGeneration],
            ),
            client: client_a,
        });
        bridge.register_provider(LlmProviderDescriptor {
            provider: Provider::new(
                "b",
                "B",
                ProviderType::Llm {
                    model_id: "b".to_string(),
                    api_endpoint: "http://b".to_string(),
                },
                vec![Capability::DeepThinking],
            ),
            client: client_b,
        });

        let models = bridge.list_models().await.unwrap();
        assert_eq!(models.len(), 3);
        assert!(models.contains(&"model-a1".to_string()));
        assert!(models.contains(&"model-a2".to_string()));
        assert!(models.contains(&"model-b1".to_string()));
    }
}
