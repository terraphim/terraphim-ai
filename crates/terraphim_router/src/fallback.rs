//! Fallback routing for when primary provider fails
//!
//! This module provides fallback logic to route to alternative providers
//! when the primary choice fails (e.g., agent spawn failure, LLM API error).

use crate::{Router, RoutingContext, RoutingDecision, RoutingError};
use terraphim_types::capability::{Provider, ProviderType};

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

        loop {
            // Get routing decision
            let decision = self.router.route(&current_prompt, context)?;
            let provider = decision.provider.clone();

            log::info!(
                "Attempt {}: Routing to {} ({})",
                attempts + 1,
                provider.name,
                provider.id
            );

            // Try to execute with the provider
            match execute(provider.clone()).await {
                Ok(()) => {
                    log::info!("Successfully executed with {}", provider.id);
                    return Ok(decision);
                }
                Err(error) => {
                    log::warn!("Provider {} failed: {}", provider.id, error);

                    attempts += 1;
                    if attempts >= self.max_fallbacks {
                        return Err(RoutingError::NoProviderFound(vec![]));
                    }

                    // Apply fallback strategy
                    match self.fallback_strategy {
                        FallbackStrategy::FailFast => {
                            return Err(RoutingError::NoProviderFound(vec![]));
                        }
                        FallbackStrategy::Retry { max_attempts } => {
                            if attempts >= max_attempts {
                                continue; // Will fail on next loop check
                            }
                            // Retry same provider
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        }
                        FallbackStrategy::NextBestProvider => {
                            // Exclude failed provider and retry
                            current_prompt = format!("{} [exclude:{}]", prompt, provider.id);
                        }
                        FallbackStrategy::LlmFallback => {
                            // If agent failed, try to find LLM
                            if matches!(provider.provider_type, ProviderType::Agent { .. }) {
                                current_prompt = format!("{} [prefer:llm]", prompt);
                            }
                        }
                    }
                }
            }
        }
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
