//! Fallback routing for when primary provider fails
//!
//! This module provides fallback logic to route to alternative providers
//! when the primary choice fails (e.g., agent spawn failure, LLM API error).

use crate::{Router, RoutingContext, RoutingDecision, RoutingError};
use terraphim_types::capability::{Provider, ProviderType};
use tracing::{info, info_span, warn, Instrument};

/// Fallback strategy when primary provider fails
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Try next best provider from routing
    NextBestProvider,
    /// Fall back to LLM if agent fails
    LlmFallback,
    /// Retry same provider
    Retry { max_attempts: u32 },
    /// Fail immediately
    FailFast,
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        FallbackStrategy::NextBestProvider
    }
}

/// Router with fallback capabilities
#[derive(Debug)]
pub struct FallbackRouter {
    router: Router,
    fallback_strategy: FallbackStrategy,
    max_fallbacks: u32,
}

impl FallbackRouter {
    /// Create a new fallback router
    pub fn new(router: Router) -> Self {
        Self {
            router,
            fallback_strategy: FallbackStrategy::default(),
            max_fallbacks: 3,
        }
    }

    /// Set fallback strategy
    pub fn with_strategy(mut self, strategy: FallbackStrategy) -> Self {
        self.fallback_strategy = strategy;
        self
    }

    /// Set max fallback attempts
    pub fn with_max_fallbacks(mut self, max: u32) -> Self {
        self.max_fallbacks = max;
        self
    }

    /// Route with fallback on failure
    ///
    /// The `execute` closure receives a cloned `Provider` to avoid
    /// lifetime issues with async closures.
    pub async fn route_with_fallback<F, Fut>(
        &self,
        prompt: &str,
        context: &RoutingContext,
        mut execute: F,
    ) -> Result<RoutingDecision, RoutingError>
    where
        F: FnMut(Provider) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let mut attempts = 0;
        let mut current_prompt = prompt.to_string();

        let fallback_span = info_span!(
            "router.route_with_fallback",
            prompt_len = prompt.len(),
            fallback_strategy = ?self.fallback_strategy,
            max_fallbacks = self.max_fallbacks,
            total_attempts = tracing::field::Empty,
            final_provider = tracing::field::Empty,
            outcome = tracing::field::Empty,
        );

        async {
            loop {
                let decision = self.router.route(&current_prompt, context)?;
                let provider = decision.provider.clone();

                let attempt_span = info_span!(
                    "router.fallback_attempt",
                    attempt_number = attempts + 1,
                    provider_id = provider.id.as_str(),
                    provider_type = ?provider.provider_type,
                    outcome = tracing::field::Empty,
                );

                let execute_result = async {
                    info!(
                        attempt = attempts + 1,
                        provider_id = provider.id.as_str(),
                        provider_name = provider.name.as_str(),
                        "Attempting provider execution"
                    );

                    match execute(provider.clone()).await {
                        Ok(()) => {
                            info!(
                                provider_id = provider.id.as_str(),
                                "Provider execution succeeded"
                            );
                            tracing::Span::current().record("outcome", "success");
                            Ok(decision.clone())
                        }
                        Err(error) => {
                            warn!(
                                provider_id = provider.id.as_str(),
                                error = error.as_str(),
                                "Provider execution failed"
                            );
                            tracing::Span::current().record("outcome", "failed");
                            Err(error)
                        }
                    }
                }
                .instrument(attempt_span)
                .await;

                match execute_result {
                    Ok(decision) => {
                        tracing::Span::current().record("total_attempts", attempts + 1);
                        tracing::Span::current()
                            .record("final_provider", decision.provider.id.as_str());
                        tracing::Span::current().record("outcome", "success");
                        return Ok(decision);
                    }
                    Err(_error) => {
                        attempts += 1;
                        if attempts >= self.max_fallbacks {
                            tracing::Span::current().record("total_attempts", attempts);
                            tracing::Span::current().record("outcome", "exhausted");
                            return Err(RoutingError::NoProviderFound(vec![]));
                        }

                        match self.fallback_strategy {
                            FallbackStrategy::FailFast => {
                                tracing::Span::current().record("outcome", "fail_fast");
                                return Err(RoutingError::NoProviderFound(vec![]));
                            }
                            FallbackStrategy::Retry { max_attempts } => {
                                if attempts >= max_attempts {
                                    continue;
                                }
                                info!(delay_ms = 1000, "Retrying same provider after delay");
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                            FallbackStrategy::NextBestProvider => {
                                info!("Excluding failed provider, trying next best");
                                current_prompt = format!("{} [exclude:{}]", prompt, provider.id);
                            }
                            FallbackStrategy::LlmFallback => {
                                if matches!(provider.provider_type, ProviderType::Agent { .. }) {
                                    info!("Agent failed, falling back to LLM preference");
                                    current_prompt = format!("{} [prefer:llm]", prompt);
                                }
                            }
                        }
                    }
                }
            }
        }
        .instrument(fallback_span)
        .await
    }

    /// Get inner router
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Get mutable inner router
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use terraphim_types::capability::{Capability, CostLevel, Latency};

    fn create_test_router() -> Router {
        let mut router = Router::new();

        // Add LLM provider
        router.add_provider(Provider::new(
            "gpt-4",
            "GPT-4",
            ProviderType::Llm {
                model_id: "gpt-4".to_string(),
                api_endpoint: "https://api.openai.com".to_string(),
            },
            vec![Capability::CodeGeneration],
        ));

        // Add agent provider
        router.add_provider(Provider::new(
            "@codex",
            "Codex",
            ProviderType::Agent {
                agent_id: "@codex".to_string(),
                cli_command: "opencode".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        ));

        router
    }

    #[tokio::test]
    async fn test_fallback_to_next_provider() {
        let router = create_test_router();
        let fallback_router = FallbackRouter::new(router)
            .with_strategy(FallbackStrategy::NextBestProvider)
            .with_max_fallbacks(2);

        let attempts = std::sync::atomic::AtomicU32::new(0);
        let result = fallback_router
            .route_with_fallback(
                "Implement a function",
                &RoutingContext::default(),
                |_provider| {
                    let n = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    async move {
                        // First attempt fails, second succeeds
                        if n == 1 {
                            Err("First provider failed".to_string())
                        } else {
                            Ok(())
                        }
                    }
                },
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_fail_fast() {
        let router = create_test_router();
        let fallback_router = FallbackRouter::new(router).with_strategy(FallbackStrategy::FailFast);

        let result = fallback_router
            .route_with_fallback(
                "Implement a function",
                &RoutingContext::default(),
                |_provider| async { Err("Always fails".to_string()) },
            )
            .await;

        assert!(result.is_err());
    }
}
