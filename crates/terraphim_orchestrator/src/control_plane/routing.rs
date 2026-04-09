//! Routing decision engine for ADF agent dispatch.
//!
//! Extracts and isolates the route selection logic from AgentOrchestrator::spawn_agent
//! into a testable, reusable component.

use crate::{cost_tracker::CostTracker, kg_router::KgRouter, provider_probe::ProviderHealthMap};
use std::path::PathBuf;
use std::sync::Arc;
use terraphim_types::capability::{CostLevel, Latency, Provider, ProviderType};

/// Context for a routing decision.
#[derive(Debug, Clone)]
pub struct DispatchContext {
    /// Agent name being dispatched.
    pub agent_name: String,
    /// Task description for routing.
    pub task: String,
    /// Static model from agent config, if any.
    pub static_model: Option<String>,
    /// CLI tool path from agent definition.
    pub cli_tool: String,
    /// Agent layer (Safety, Core, Growth).
    pub layer: crate::config::AgentLayer,
}

/// A candidate route option.
#[derive(Debug, Clone)]
pub struct RouteCandidate {
    /// Provider configuration.
    pub provider: Provider,
    /// Model identifier.
    pub model: String,
    /// CLI tool path to use.
    pub cli_tool: String,
    /// How this candidate was derived.
    pub source: RouteSource,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
}

/// Source of a route candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum RouteSource {
    /// From KG router with tier-based selection.
    KnowledgeGraph,
    /// From keyword/capability routing engine.
    KeywordRouting,
    /// From static agent config.
    StaticConfig,
    /// Fallback to CLI default.
    CliDefault,
}

/// The result of a routing decision.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Selected route candidate.
    pub candidate: RouteCandidate,
    /// Human-readable rationale.
    pub rationale: String,
    /// All candidates considered (for debugging/auditing).
    pub all_candidates: Vec<RouteCandidate>,
    /// Whether the primary choice was available.
    pub primary_available: bool,
}

/// Engine for making routing decisions.
pub struct RoutingDecisionEngine {
    /// KG router for semantic tier routing.
    kg_router: Option<Arc<KgRouter>>,
    /// Provider health status.
    provider_health: Arc<ProviderHealthMap>,
    /// Cost/budget tracking (reserved for budget-aware routing - issue #527).
    #[allow(dead_code)]
    cost_tracker: CostTracker,
    /// Keyword routing engine.
    router: terraphim_router::Router,
}

impl RoutingDecisionEngine {
    /// Create a new routing decision engine.
    pub fn new(
        kg_router: Option<Arc<KgRouter>>,
        provider_health: Arc<ProviderHealthMap>,
        cost_tracker: CostTracker,
        router: terraphim_router::Router,
    ) -> Self {
        Self {
            kg_router,
            provider_health,
            cost_tracker,
            router,
        }
    }

    /// Determine the best route for an agent dispatch.
    ///
    /// This method preserves the exact precedence from the original spawn_agent:
    /// 1. KG routing with health-aware fallback
    /// 2. Static model config
    /// 3. Keyword routing engine
    /// 4. CLI default (no model specified)
    #[allow(unused_assignments)]
    pub fn decide_route(&self, ctx: &DispatchContext) -> RoutingDecision {
        let cli_name = std::path::Path::new(&ctx.cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&ctx.cli_tool);

        let supports_model_flag = matches!(cli_name, "claude" | "claude-code" | "opencode");

        let mut all_candidates = Vec::new();
        let mut primary_available = false;

        if supports_model_flag {
            // 1. KG routing first
            if let Some(ref kg_router) = self.kg_router {
                let unhealthy = self.provider_health.unhealthy_providers();

                if let Some(decision) = kg_router.route_agent(&ctx.task) {
                    // Check if primary route is healthy
                    if !unhealthy.is_empty() {
                        if let Some(healthy_route) = decision.first_healthy_route(&unhealthy) {
                            // Use healthy fallback from KG
                            let cli_tool = healthy_route
                                .action
                                .as_ref()
                                .and_then(|a| a.split_whitespace().next())
                                .map(String::from)
                                .unwrap_or_else(|| ctx.cli_tool.clone());

                            let candidate = RouteCandidate {
                                provider: Provider {
                                    id: format!("{}-kg", ctx.agent_name),
                                    name: format!("{} (KG)", ctx.agent_name),
                                    provider_type: ProviderType::Agent {
                                        agent_id: ctx.agent_name.clone(),
                                        cli_command: cli_tool.clone(),
                                        working_dir: PathBuf::from(
                                            std::env::current_dir()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .to_string(),
                                        ),
                                    },
                                    capabilities: vec![],
                                    cost_level: CostLevel::Moderate,
                                    latency: Latency::Medium,
                                    keywords: vec![],
                                },
                                model: healthy_route.model.clone(),
                                cli_tool,
                                source: RouteSource::KnowledgeGraph,
                                confidence: decision.confidence * 0.9,
                            };

                            all_candidates.push(candidate.clone());

                            return RoutingDecision {
                                candidate,
                                rationale: format!(
                                    "KG routed to healthy fallback: {} (skipped unhealthy: {:?})",
                                    healthy_route.model, unhealthy
                                ),
                                all_candidates,
                                primary_available: false,
                            };
                        }
                    }

                    // Primary route is healthy
                    let cli_tool = decision
                        .action
                        .as_ref()
                        .and_then(|a| a.split_whitespace().next())
                        .map(String::from)
                        .unwrap_or_else(|| ctx.cli_tool.clone());

                    let candidate = RouteCandidate {
                        provider: Provider {
                            id: format!("{}-kg", ctx.agent_name),
                            name: format!("{} (KG)", ctx.agent_name),
                            provider_type: ProviderType::Agent {
                                agent_id: ctx.agent_name.clone(),
                                cli_command: cli_tool.clone(),
                                working_dir: PathBuf::from(
                                    std::env::current_dir()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            },
                            capabilities: vec![],
                            cost_level: CostLevel::Moderate,
                            latency: Latency::Medium,
                            keywords: vec![],
                        },
                        model: decision.model.clone(),
                        cli_tool,
                        source: RouteSource::KnowledgeGraph,
                        confidence: decision.confidence,
                    };

                    all_candidates.push(candidate.clone());

                    return RoutingDecision {
                        candidate,
                        rationale: format!(
                            "KG tier routing selected: {} (concept: {}, confidence: {})",
                            decision.model, decision.matched_concept, decision.confidence
                        ),
                        all_candidates,
                        primary_available: true,
                    };
                }
            }

            // 2. Static model fallback
            if let Some(ref model) = ctx.static_model {
                let candidate = RouteCandidate {
                    provider: Provider {
                        id: format!("{}-static", ctx.agent_name),
                        name: format!("{} (static)", ctx.agent_name),
                        provider_type: ProviderType::Agent {
                            agent_id: ctx.agent_name.clone(),
                            cli_command: ctx.cli_tool.clone(),
                            working_dir: PathBuf::from(
                                std::env::current_dir()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                        },
                        capabilities: vec![],
                        cost_level: CostLevel::Moderate,
                        latency: Latency::Medium,
                        keywords: vec![],
                    },
                    model: model.clone(),
                    cli_tool: ctx.cli_tool.clone(),
                    source: RouteSource::StaticConfig,
                    confidence: 0.8,
                };

                all_candidates.push(candidate.clone());

                return RoutingDecision {
                    candidate,
                    rationale: format!("Static model from config: {}", model),
                    all_candidates,
                    primary_available: true,
                };
            }

            // 3. Keyword routing engine
            let routing_ctx = terraphim_router::RoutingContext::default();
            if let Ok(decision) = self.router.route(&ctx.task, &routing_ctx) {
                if let ProviderType::Llm { model_id, .. } = &decision.provider.provider_type {
                    let candidate = RouteCandidate {
                        provider: Provider {
                            id: format!("{}-keyword", ctx.agent_name),
                            name: format!("{} (keyword)", ctx.agent_name),
                            provider_type: ProviderType::Agent {
                                agent_id: ctx.agent_name.clone(),
                                cli_command: ctx.cli_tool.clone(),
                                working_dir: PathBuf::from(
                                    std::env::current_dir()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                ),
                            },
                            capabilities: vec![],
                            cost_level: CostLevel::Moderate,
                            latency: Latency::Medium,
                            keywords: vec![],
                        },
                        model: model_id.clone(),
                        cli_tool: ctx.cli_tool.clone(),
                        source: RouteSource::KeywordRouting,
                        confidence: decision.confidence as f64,
                    };

                    all_candidates.push(candidate.clone());

                    return RoutingDecision {
                        candidate,
                        rationale: format!(
                            "Keyword routing selected: {} (confidence: {})",
                            model_id, decision.confidence
                        ),
                        all_candidates,
                        primary_available: true,
                    };
                }
            }

            primary_available = false;
        } else {
            // CLI doesn't support model flags
            primary_available = false;
        };

        // 4. CLI default (no model)
        let candidate = RouteCandidate {
            provider: Provider {
                id: format!("{}-default", ctx.agent_name),
                name: format!("{} (default)", ctx.agent_name),
                provider_type: ProviderType::Agent {
                    agent_id: ctx.agent_name.clone(),
                    cli_command: ctx.cli_tool.clone(),
                    working_dir: PathBuf::from(
                        std::env::current_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    ),
                },
                capabilities: vec![],
                cost_level: CostLevel::Moderate,
                latency: Latency::Medium,
                keywords: vec![],
            },
            model: String::new(),
            cli_tool: ctx.cli_tool.clone(),
            source: RouteSource::CliDefault,
            confidence: 0.5,
        };

        all_candidates.push(candidate.clone());

        RoutingDecision {
            candidate,
            rationale: format!("Using CLI default (no model routing for {})", cli_name),
            all_candidates,
            primary_available,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context_with_cli(
        agent_name: &str,
        task: &str,
        cli_tool: &str,
    ) -> DispatchContext {
        DispatchContext {
            agent_name: agent_name.to_string(),
            task: task.to_string(),
            static_model: None,
            cli_tool: cli_tool.to_string(),
            layer: crate::config::AgentLayer::Core,
        }
    }

    fn create_test_context_with_static_model(
        agent_name: &str,
        task: &str,
        static_model: &str,
    ) -> DispatchContext {
        DispatchContext {
            agent_name: agent_name.to_string(),
            task: task.to_string(),
            static_model: Some(static_model.to_string()),
            cli_tool: "opencode".to_string(),
            layer: crate::config::AgentLayer::Core,
        }
    }

    #[test]
    fn test_cli_default_for_unsupported_tool() {
        // Test that unsupported CLI tools (like codex) get CLI default routing
        let engine = RoutingDecisionEngine::new(
            None,
            Arc::new(crate::provider_probe::ProviderHealthMap::new(
                std::time::Duration::from_secs(300),
            )),
            crate::cost_tracker::CostTracker::new(),
            terraphim_router::Router::new(),
        );

        // Codex doesn't support --model flag
        let ctx = create_test_context_with_cli("test-agent", "Implement a feature", "codex");
        let decision = engine.decide_route(&ctx);

        assert_eq!(decision.candidate.source, RouteSource::CliDefault);
        assert!(decision.candidate.model.is_empty());
        assert!(decision.rationale.contains("codex"));
    }

    #[test]
    fn test_static_model_fallback_works() {
        // Test that static model config is used when no KG router is available
        let engine = RoutingDecisionEngine::new(
            None,
            Arc::new(crate::provider_probe::ProviderHealthMap::new(
                std::time::Duration::from_secs(300),
            )),
            crate::cost_tracker::CostTracker::new(),
            terraphim_router::Router::new(),
        );

        let ctx = create_test_context_with_static_model(
            "test-agent",
            "Implement a feature",
            "claude-3-opus",
        );
        let decision = engine.decide_route(&ctx);

        assert_eq!(decision.candidate.source, RouteSource::StaticConfig);
        assert_eq!(decision.candidate.model, "claude-3-opus");
        assert!(decision.rationale.contains("Static model"));
    }

    #[test]
    fn test_routing_precedence_cli_default_for_unsupported() {
        // Verify that unsupported CLI tools bypass all model routing
        // and go straight to CLI default
        let engine = RoutingDecisionEngine::new(
            None,
            Arc::new(crate::provider_probe::ProviderHealthMap::new(
                std::time::Duration::from_secs(300),
            )),
            crate::cost_tracker::CostTracker::new(),
            terraphim_router::Router::new(),
        );

        // Even with static model, unsupported CLI should use default
        let ctx = DispatchContext {
            agent_name: "test-agent".to_string(),
            task: "Implement a feature".to_string(),
            static_model: Some("some-model".to_string()),
            cli_tool: "codex".to_string(), // Unsupported
            layer: crate::config::AgentLayer::Core,
        };
        let decision = engine.decide_route(&ctx);

        assert_eq!(decision.candidate.source, RouteSource::CliDefault);
    }

    #[test]
    fn test_opencode_gets_static_model() {
        // Verify that opencode (supported CLI) uses static model when available
        let engine = RoutingDecisionEngine::new(
            None,
            Arc::new(crate::provider_probe::ProviderHealthMap::new(
                std::time::Duration::from_secs(300),
            )),
            crate::cost_tracker::CostTracker::new(),
            terraphim_router::Router::new(),
        );

        let ctx = create_test_context_with_static_model(
            "test-agent",
            "Implement a feature",
            "kimi-for-coding/k2p5",
        );
        let decision = engine.decide_route(&ctx);

        assert_eq!(decision.candidate.source, RouteSource::StaticConfig);
        assert_eq!(decision.candidate.model, "kimi-for-coding/k2p5");
    }

    #[test]
    fn test_route_candidate_structure() {
        // Verify the RouteCandidate struct is properly constructed
        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Agent {
                agent_id: "test-agent".to_string(),
                cli_command: "opencode".to_string(),
                working_dir: std::path::PathBuf::from("/tmp"),
            },
            capabilities: vec![],
            cost_level: CostLevel::Moderate,
            latency: Latency::Medium,
            keywords: vec![],
        };

        let candidate = RouteCandidate {
            provider,
            model: "test-model".to_string(),
            cli_tool: "opencode".to_string(),
            source: RouteSource::StaticConfig,
            confidence: 0.8,
        };

        assert_eq!(candidate.model, "test-model");
        assert_eq!(candidate.source, RouteSource::StaticConfig);
        assert_eq!(candidate.confidence, 0.8);
    }

    #[test]
    fn test_dispatch_context_creation() {
        let ctx = DispatchContext {
            agent_name: "test-agent".to_string(),
            task: "Do something".to_string(),
            static_model: Some("model-id".to_string()),
            cli_tool: "claude".to_string(),
            layer: crate::config::AgentLayer::Safety,
        };

        assert_eq!(ctx.agent_name, "test-agent");
        assert_eq!(ctx.task, "Do something");
        assert_eq!(ctx.static_model, Some("model-id".to_string()));
        assert_eq!(ctx.cli_tool, "claude");
    }

    #[test]
    fn test_routing_decision_structure() {
        let candidate = RouteCandidate {
            provider: Provider {
                id: "test-id".to_string(),
                name: "Test Provider".to_string(),
                provider_type: ProviderType::Agent {
                    agent_id: "test-agent".to_string(),
                    cli_command: "opencode".to_string(),
                    working_dir: std::path::PathBuf::from("/tmp"),
                },
                capabilities: vec![],
                cost_level: CostLevel::Cheap,
                latency: Latency::Fast,
                keywords: vec![],
            },
            model: "test-model".to_string(),
            cli_tool: "opencode".to_string(),
            source: RouteSource::CliDefault,
            confidence: 0.5,
        };

        let decision = RoutingDecision {
            candidate: candidate.clone(),
            rationale: "Test rationale".to_string(),
            all_candidates: vec![candidate],
            primary_available: false,
        };

        assert_eq!(decision.rationale, "Test rationale");
        assert!(!decision.primary_available);
        assert_eq!(decision.all_candidates.len(), 1);
    }
}
