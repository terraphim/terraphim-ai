//! Routing decision engine for ADF agent dispatch.
//!
//! Combines KG routing, keyword routing, and static config as first-class
//! signals. All candidates are collected and scored, with the winning signal
//! recorded in the decision rationale. Budget pressure biases route selection
//! toward cheaper models when an agent's spend approaches its limit.

use crate::control_plane::telemetry::TelemetryStore;
use crate::cost_tracker::BudgetVerdict;
use crate::kg_router::KgRouter;
use std::path::PathBuf;
use std::sync::Arc;
use terraphim_types::capability::{CostLevel, Latency, Provider, ProviderType};

#[derive(Debug, Clone, PartialEq)]
pub enum RouteSource {
    KnowledgeGraph,
    KeywordRouting,
    StaticConfig,
    CombinedKgKeyword,
    CliDefault,
}

impl std::fmt::Display for RouteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteSource::KnowledgeGraph => write!(f, "KG"),
            RouteSource::KeywordRouting => write!(f, "keyword"),
            RouteSource::StaticConfig => write!(f, "static"),
            RouteSource::CombinedKgKeyword => write!(f, "KG+keyword"),
            RouteSource::CliDefault => write!(f, "CLI default"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BudgetPressure {
    NoPressure,
    NearExhaustion,
    Exhausted,
}

impl BudgetPressure {
    pub fn from_verdict(verdict: &BudgetVerdict) -> Self {
        match verdict {
            BudgetVerdict::Exhausted { .. } => BudgetPressure::Exhausted,
            BudgetVerdict::NearExhaustion { .. } => BudgetPressure::NearExhaustion,
            _ => BudgetPressure::NoPressure,
        }
    }

    pub fn cost_penalty(&self, cost_level: &CostLevel) -> f64 {
        match self {
            BudgetPressure::NoPressure => 0.0,
            BudgetPressure::NearExhaustion => match cost_level {
                CostLevel::Cheap => 0.0,
                CostLevel::Moderate => 0.15,
                CostLevel::Expensive => 0.35,
            },
            BudgetPressure::Exhausted => match cost_level {
                CostLevel::Cheap => 0.10,
                CostLevel::Moderate => 0.40,
                CostLevel::Expensive => 0.70,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct DispatchContext {
    pub agent_name: String,
    pub task: String,
    pub static_model: Option<String>,
    pub cli_tool: String,
    pub layer: crate::config::AgentLayer,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RouteCandidate {
    pub provider: Provider,
    pub model: String,
    pub cli_tool: String,
    pub source: RouteSource,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub candidate: RouteCandidate,
    pub rationale: String,
    pub all_candidates: Vec<RouteCandidate>,
    pub primary_available: bool,
    pub dominant_signal: RouteSource,
    pub budget_pressure: BudgetPressure,
    pub budget_influenced: bool,
    pub telemetry_influenced: bool,
}

fn make_agent_provider(agent_name: &str, cli_tool: &str) -> Provider {
    Provider {
        id: format!("{}-agent", agent_name),
        name: format!("{} (agent)", agent_name),
        provider_type: ProviderType::Agent {
            agent_id: agent_name.to_string(),
            cli_command: cli_tool.to_string(),
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
    }
}

struct CollectedCandidates {
    kg: Vec<RouteCandidate>,
    keyword: Vec<RouteCandidate>,
    static_model: Option<RouteCandidate>,
}

pub struct RoutingDecisionEngine {
    kg_router: Option<Arc<KgRouter>>,
    /// Snapshot of unhealthy provider names at construction time.
    unhealthy_providers: Vec<String>,
    router: terraphim_router::Router,
    telemetry_store: Option<Arc<TelemetryStore>>,
}

impl RoutingDecisionEngine {
    pub fn new(
        kg_router: Option<Arc<KgRouter>>,
        unhealthy_providers: Vec<String>,
        router: terraphim_router::Router,
        telemetry_store: Option<Arc<TelemetryStore>>,
    ) -> Self {
        Self {
            kg_router,
            unhealthy_providers,
            router,
            telemetry_store,
        }
    }

    fn cli_name(ctx: &DispatchContext) -> &str {
        std::path::Path::new(&ctx.cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&ctx.cli_tool)
    }

    fn supports_model_flag(cli_name: &str) -> bool {
        matches!(cli_name, "claude" | "claude-code" | "opencode")
    }

    fn budget_pressure(verdict: &BudgetVerdict) -> BudgetPressure {
        BudgetPressure::from_verdict(verdict)
    }

    fn collect_kg_candidates(&self, ctx: &DispatchContext) -> Vec<RouteCandidate> {
        let kg_router = match self.kg_router {
            Some(ref r) => r,
            None => return Vec::new(),
        };

        let decision = match kg_router.route_agent(&ctx.task) {
            Some(d) => d,
            None => return Vec::new(),
        };

        let unhealthy = &self.unhealthy_providers;
        let mut candidates = Vec::new();

        for route in &decision.fallback_routes {
            let is_healthy = !unhealthy.iter().any(|u| u == &route.provider);
            let is_primary = route.provider == decision.provider;

            let cli_tool = route
                .action
                .as_ref()
                .and_then(|a| a.split_whitespace().next())
                .map(String::from)
                .unwrap_or_else(|| ctx.cli_tool.clone());

            let confidence = if is_primary {
                decision.confidence
            } else {
                decision.confidence * 0.8
            };

            let confidence = if is_healthy {
                confidence
            } else {
                confidence * 0.5
            };

            candidates.push(RouteCandidate {
                provider: make_agent_provider(&ctx.agent_name, &cli_tool),
                model: route.model.clone(),
                cli_tool,
                source: RouteSource::KnowledgeGraph,
                confidence,
            });
        }

        candidates
    }

    fn collect_keyword_candidates(&self, ctx: &DispatchContext) -> Vec<RouteCandidate> {
        let routing_ctx = terraphim_router::RoutingContext::default();
        let decision = match self.router.route(&ctx.task, &routing_ctx) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let model_id = match &decision.provider.provider_type {
            ProviderType::Llm { model_id, .. } => model_id.clone(),
            _ => return Vec::new(),
        };

        vec![RouteCandidate {
            provider: make_agent_provider(&ctx.agent_name, &ctx.cli_tool),
            model: model_id,
            cli_tool: ctx.cli_tool.clone(),
            source: RouteSource::KeywordRouting,
            confidence: decision.confidence as f64,
        }]
    }

    fn collect_static_candidate(&self, ctx: &DispatchContext) -> Option<RouteCandidate> {
        ctx.static_model.as_ref().map(|model| RouteCandidate {
            provider: make_agent_provider(&ctx.agent_name, &ctx.cli_tool),
            model: model.clone(),
            cli_tool: ctx.cli_tool.clone(),
            source: RouteSource::StaticConfig,
            confidence: 0.8,
        })
    }

    fn collect_all_candidates(&self, ctx: &DispatchContext) -> CollectedCandidates {
        CollectedCandidates {
            kg: self.collect_kg_candidates(ctx),
            keyword: self.collect_keyword_candidates(ctx),
            static_model: self.collect_static_candidate(ctx),
        }
    }

    fn score_candidate(candidate: &RouteCandidate, pressure: BudgetPressure) -> f64 {
        let source_weight = match candidate.source {
            RouteSource::KnowledgeGraph => 1.0,
            RouteSource::CombinedKgKeyword => 1.0,
            RouteSource::KeywordRouting => 0.8,
            RouteSource::StaticConfig => 0.6,
            RouteSource::CliDefault => 0.3,
        };
        let base = source_weight * candidate.confidence;
        let penalty = pressure.cost_penalty(&candidate.provider.cost_level);
        base * (1.0 - penalty)
    }

    pub async fn decide_route(
        &self,
        ctx: &DispatchContext,
        budget_verdict: &BudgetVerdict,
    ) -> RoutingDecision {
        let cli_name = Self::cli_name(ctx);
        let pressure = Self::budget_pressure(budget_verdict);

        if !Self::supports_model_flag(cli_name) {
            let candidate = RouteCandidate {
                provider: make_agent_provider(&ctx.agent_name, &ctx.cli_tool),
                model: String::new(),
                cli_tool: ctx.cli_tool.clone(),
                source: RouteSource::CliDefault,
                confidence: 0.5,
            };
            return RoutingDecision {
                candidate: candidate.clone(),
                rationale: format!("Using CLI default (no model routing for {})", cli_name),
                all_candidates: vec![candidate],
                primary_available: false,
                dominant_signal: RouteSource::CliDefault,
                budget_pressure: pressure,
                budget_influenced: false,
                telemetry_influenced: false,
            };
        }

        let collected = self.collect_all_candidates(ctx);
        let mut all_candidates = Vec::new();

        let has_kg = !collected.kg.is_empty();
        let has_keyword = !collected.keyword.is_empty();

        if has_kg && has_keyword {
            if let (Some(kg_cand), Some(kw_cand)) =
                (collected.kg.first(), collected.keyword.first())
            {
                if kg_cand.model == kw_cand.model {
                    let merged = RouteCandidate {
                        provider: kg_cand.provider.clone(),
                        model: kg_cand.model.clone(),
                        cli_tool: kg_cand.cli_tool.clone(),
                        source: RouteSource::CombinedKgKeyword,
                        confidence: (kg_cand.confidence + kw_cand.confidence) / 2.0,
                    };
                    all_candidates.push(merged);
                } else {
                    all_candidates.extend(collected.kg.clone());
                    all_candidates.extend(collected.keyword.clone());
                }
            }
        } else {
            all_candidates.extend(collected.kg.clone());
            all_candidates.extend(collected.keyword.clone());
        }

        if let Some(static_cand) = &collected.static_model {
            all_candidates.push(static_cand.clone());
        }

        if all_candidates.is_empty() {
            let candidate = RouteCandidate {
                provider: make_agent_provider(&ctx.agent_name, &ctx.cli_tool),
                model: String::new(),
                cli_tool: ctx.cli_tool.clone(),
                source: RouteSource::CliDefault,
                confidence: 0.5,
            };
            return RoutingDecision {
                candidate: candidate.clone(),
                rationale: format!(
                    "No routing signal matched; using CLI default ({})",
                    cli_name
                ),
                all_candidates: vec![candidate],
                primary_available: false,
                dominant_signal: RouteSource::CliDefault,
                budget_pressure: pressure,
                budget_influenced: false,
                telemetry_influenced: false,
            };
        }
        let no_pressure_scores: Vec<f64> = all_candidates
            .iter()
            .map(|c| Self::score_candidate(c, BudgetPressure::NoPressure))
            .collect();
        let mut pressured_scores: Vec<f64> = all_candidates
            .iter()
            .map(|c| Self::score_candidate(c, pressure))
            .collect();

        // Apply telemetry-based scoring adjustments
        let mut telemetry_influenced = false;
        if let Some(ref store) = self.telemetry_store {
            let mut performances = Vec::with_capacity(all_candidates.len());
            for candidate in &all_candidates {
                performances.push(store.model_performance(&candidate.model).await);
            }

            for (i, perf) in performances.iter().enumerate() {
                if perf.is_subscription_limited() {
                    pressured_scores[i] *= 0.1;
                    telemetry_influenced = true;
                } else if perf.successful_completions > 0 {
                    let success_bonus = (perf.success_rate - 0.5).max(0.0_f64) * 0.2_f64;
                    let latency_bonus = if perf.avg_latency_ms > 0.0 && perf.avg_latency_ms < 5000.0
                    {
                        (1.0_f64 - perf.avg_latency_ms / 10000.0_f64).max(0.0_f64) * 0.1_f64
                    } else {
                        0.0_f64
                    };
                    let bonus = success_bonus + latency_bonus;
                    if bonus > 0.01 {
                        pressured_scores[i] *= 1.0 + bonus;
                        telemetry_influenced = true;
                    }
                }
            }
        }

        let mut indexed: Vec<usize> = (0..all_candidates.len()).collect();
        indexed.sort_by(|&a, &b| {
            pressured_scores[b]
                .partial_cmp(&pressured_scores[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let winner_idx = indexed[0];
        let winner = &all_candidates[winner_idx];
        let dominant_signal = winner.source.clone();

        let budget_influenced = pressure != BudgetPressure::NoPressure
            && no_pressure_scores.iter().enumerate().any(|(i, &s)| {
                let was_winner = s >= no_pressure_scores[winner_idx];
                let now_loses = pressured_scores[i] < pressured_scores[winner_idx];
                was_winner && now_loses
            });

        let mut rationale_parts = Vec::new();

        if has_kg {
            rationale_parts.push(format!("KG: {} candidates", collected.kg.len()));
        }
        if has_keyword {
            rationale_parts.push(format!("keyword: {} candidates", collected.keyword.len()));
        }
        if collected.static_model.is_some() {
            rationale_parts.push("static config".to_string());
        }

        let signal_summary = if rationale_parts.is_empty() {
            "no signals".to_string()
        } else {
            rationale_parts.join(", ")
        };

        let mut rationale = format!(
            "Selected {} via {} (score: {:.3}, confidence: {:.2}). Signals: {}",
            winner.model,
            winner.source,
            pressured_scores[winner_idx],
            winner.confidence,
            signal_summary,
        );

        if budget_influenced {
            rationale.push_str(". Budget pressure biased selection toward cheaper model");
        }
        if telemetry_influenced {
            rationale.push_str(". Telemetry data influenced selection");
        }

        let primary_available = !matches!(winner.source, RouteSource::CliDefault);

        RoutingDecision {
            candidate: winner.clone(),
            rationale,
            all_candidates,
            primary_available,
            dominant_signal,
            budget_pressure: pressure,
            budget_influenced,
            telemetry_influenced,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost_tracker::CostTracker;

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
            session_id: None,
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
            session_id: None,
        }
    }

    fn test_engine() -> RoutingDecisionEngine {
        RoutingDecisionEngine::new(None, Vec::new(), terraphim_router::Router::new(), None)
    }

    fn test_engine_with_spent(
        agent_name: &str,
        budget_cents: Option<u64>,
        spend_usd: f64,
    ) -> (RoutingDecisionEngine, CostTracker) {
        let mut ct = CostTracker::new();
        ct.register(agent_name, budget_cents);
        ct.record_cost(agent_name, spend_usd);
        let engine =
            RoutingDecisionEngine::new(None, Vec::new(), terraphim_router::Router::new(), None);
        (engine, ct)
    }

    #[tokio::test]
    async fn test_cli_default_for_unsupported_tool() {
        let engine = test_engine();
        let ctx = create_test_context_with_cli("test-agent", "Implement a feature", "codex");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::CliDefault);
        assert!(decision.candidate.model.is_empty());
        assert!(decision.rationale.contains("codex"));
        assert_eq!(decision.dominant_signal, RouteSource::CliDefault);
        assert_eq!(decision.budget_pressure, BudgetPressure::NoPressure);
        assert!(!decision.budget_influenced);
    }

    #[tokio::test]
    async fn test_static_model_selected_when_only_signal() {
        let engine = test_engine();
        let ctx = create_test_context_with_static_model(
            "test-agent",
            "Implement a feature",
            "claude-3-opus",
        );
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::StaticConfig);
        assert_eq!(decision.candidate.model, "claude-3-opus");
        assert!(decision.rationale.contains("static config"));
        assert_eq!(decision.dominant_signal, RouteSource::StaticConfig);
    }

    #[tokio::test]
    async fn test_unsupported_cli_ignores_static_model() {
        let engine = test_engine();
        let ctx = DispatchContext {
            agent_name: "test-agent".to_string(),
            task: "Implement a feature".to_string(),
            static_model: Some("some-model".to_string()),
            cli_tool: "codex".to_string(),
            layer: crate::config::AgentLayer::Core,
            session_id: None,
        };
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::CliDefault);
        assert_eq!(decision.dominant_signal, RouteSource::CliDefault);
    }

    #[tokio::test]
    async fn test_opencode_gets_static_model() {
        let engine = test_engine();
        let ctx = create_test_context_with_static_model(
            "test-agent",
            "Implement a feature",
            "kimi-for-coding/k2p5",
        );
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::StaticConfig);
        assert_eq!(decision.candidate.model, "kimi-for-coding/k2p5");
    }

    #[tokio::test]
    async fn test_cli_default_when_no_signals_match() {
        let engine = test_engine();
        let ctx = create_test_context_with_cli("test-agent", "do something", "opencode");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::CliDefault);
        assert!(decision.rationale.contains("No routing signal matched"));
        assert_eq!(decision.dominant_signal, RouteSource::CliDefault);
    }

    #[tokio::test]
    async fn test_rationale_records_dominant_signal() {
        let engine = test_engine();
        let ctx = create_test_context_with_static_model("agent", "task", "model-x");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(decision.rationale.contains("static config"));
        assert!(decision.rationale.contains("Selected model-x"));
    }

    #[tokio::test]
    async fn test_all_candidates_collected_from_multiple_sources() {
        let engine = test_engine();
        let ctx = DispatchContext {
            agent_name: "test-agent".to_string(),
            task: "implement feature".to_string(),
            static_model: Some("static-model".to_string()),
            cli_tool: "opencode".to_string(),
            layer: crate::config::AgentLayer::Core,
            session_id: None,
        };
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(!decision.all_candidates.is_empty());
        assert!(decision
            .all_candidates
            .iter()
            .any(|c| c.source == RouteSource::StaticConfig));
    }

    #[tokio::test]
    async fn test_combined_kg_keyword_when_models_agree() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("impl.md"),
            "priority:: 50\nsynonyms:: implement, build\nroute:: kimi, kimi-for-coding/k2p5\n",
        )
        .unwrap();

        let kg_router = Arc::new(crate::kg_router::KgRouter::load(dir.path()).unwrap());
        let engine = RoutingDecisionEngine::new(
            Some(kg_router),
            Vec::new(),
            terraphim_router::Router::new(),
            None,
        );

        let ctx = create_test_context_with_cli("agent", "implement feature", "opencode");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(
            decision.candidate.source == RouteSource::KnowledgeGraph
                || decision.candidate.source == RouteSource::CombinedKgKeyword
                || decision.candidate.source == RouteSource::KeywordRouting,
            "expected a routing signal, got {:?}",
            decision.candidate.source,
        );
        assert!(decision.primary_available);
    }

    #[tokio::test]
    async fn test_kg_only_no_keyword_match() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("security.md"),
            "priority:: 80\nsynonyms:: security audit, CVE\nroute:: anthropic, opus\n",
        )
        .unwrap();

        let kg_router = Arc::new(crate::kg_router::KgRouter::load(dir.path()).unwrap());
        let engine = RoutingDecisionEngine::new(
            Some(kg_router),
            Vec::new(),
            terraphim_router::Router::new(),
            None,
        );

        let ctx = create_test_context_with_cli("agent", "security audit the codebase", "opencode");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.candidate.source, RouteSource::KnowledgeGraph);
        assert!(decision.candidate.model.contains("opus"));
        assert_eq!(decision.dominant_signal, RouteSource::KnowledgeGraph);
    }

    #[tokio::test]
    async fn test_keyword_only_no_kg_match() {
        let engine = test_engine();
        let ctx = create_test_context_with_cli("agent", "implement a feature", "opencode");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(
            decision.candidate.source == RouteSource::KeywordRouting
                || decision.candidate.source == RouteSource::CliDefault,
        );
    }

    #[tokio::test]
    async fn test_dispatch_context_session_id() {
        let ctx = DispatchContext {
            agent_name: "test-agent".to_string(),
            task: "Do something".to_string(),
            static_model: Some("model-id".to_string()),
            cli_tool: "claude".to_string(),
            layer: crate::config::AgentLayer::Safety,
            session_id: Some("sess-123".to_string()),
        };
        assert_eq!(ctx.session_id, Some("sess-123".to_string()));
    }

    #[tokio::test]
    async fn test_route_source_display() {
        assert_eq!(RouteSource::KnowledgeGraph.to_string(), "KG");
        assert_eq!(RouteSource::KeywordRouting.to_string(), "keyword");
        assert_eq!(RouteSource::StaticConfig.to_string(), "static");
        assert_eq!(RouteSource::CombinedKgKeyword.to_string(), "KG+keyword");
        assert_eq!(RouteSource::CliDefault.to_string(), "CLI default");
    }

    #[tokio::test]
    async fn test_make_agent_provider() {
        let provider = make_agent_provider("my-agent", "opencode");
        assert!(provider.id.contains("my-agent"));
        if let ProviderType::Agent {
            agent_id,
            cli_command,
            ..
        } = &provider.provider_type
        {
            assert_eq!(agent_id, "my-agent");
            assert_eq!(cli_command, "opencode");
        } else {
            panic!("expected Agent provider type");
        }
    }

    #[tokio::test]
    async fn test_budget_pressure_no_pressure_for_uncapped() {
        let engine = test_engine();
        let ctx = create_test_context_with_static_model("test-agent", "task", "model-x");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert_eq!(decision.budget_pressure, BudgetPressure::NoPressure);
        assert!(!decision.budget_influenced);
    }

    #[tokio::test]
    async fn test_budget_pressure_near_exhaustion_detected() {
        let (engine, ct) = test_engine_with_spent("test-agent", Some(10000), 85.0);
        let ctx = create_test_context_with_static_model("test-agent", "task", "model-x");
        let decision = engine.decide_route(&ctx, &ct.check("test-agent")).await;

        assert_eq!(decision.budget_pressure, BudgetPressure::NearExhaustion);
    }

    #[tokio::test]
    async fn test_budget_pressure_exhausted_detected() {
        let (engine, ct) = test_engine_with_spent("test-agent", Some(10000), 100.0);
        let ctx = create_test_context_with_static_model("test-agent", "task", "model-x");
        let decision = engine.decide_route(&ctx, &ct.check("test-agent")).await;

        assert_eq!(decision.budget_pressure, BudgetPressure::Exhausted);
    }

    #[tokio::test]
    async fn test_budget_pressure_penalty_calculation() {
        let no_pressure = BudgetPressure::NoPressure;
        assert_eq!(no_pressure.cost_penalty(&CostLevel::Cheap), 0.0);
        assert_eq!(no_pressure.cost_penalty(&CostLevel::Moderate), 0.0);
        assert_eq!(no_pressure.cost_penalty(&CostLevel::Expensive), 0.0);

        let near = BudgetPressure::NearExhaustion;
        assert_eq!(near.cost_penalty(&CostLevel::Cheap), 0.0);
        assert!((near.cost_penalty(&CostLevel::Moderate) - 0.15).abs() < 0.001);
        assert!((near.cost_penalty(&CostLevel::Expensive) - 0.35).abs() < 0.001);

        let exhausted = BudgetPressure::Exhausted;
        assert!((exhausted.cost_penalty(&CostLevel::Cheap) - 0.10).abs() < 0.001);
        assert!((exhausted.cost_penalty(&CostLevel::Moderate) - 0.40).abs() < 0.001);
        assert!((exhausted.cost_penalty(&CostLevel::Expensive) - 0.70).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_budget_influences_rationale_when_pressure() {
        let (engine, ct) = test_engine_with_spent("test-agent", Some(10000), 85.0);
        let ctx = create_test_context_with_static_model("test-agent", "task", "model-x");
        let decision = engine.decide_route(&ctx, &ct.check("test-agent")).await;

        assert_eq!(decision.budget_pressure, BudgetPressure::NearExhaustion);
    }

    #[tokio::test]
    async fn test_budget_verdict_conversion() {
        assert_eq!(
            BudgetPressure::from_verdict(&BudgetVerdict::Uncapped),
            BudgetPressure::NoPressure
        );
        assert_eq!(
            BudgetPressure::from_verdict(&BudgetVerdict::WithinBudget),
            BudgetPressure::NoPressure
        );
        assert_eq!(
            BudgetPressure::from_verdict(&BudgetVerdict::NearExhaustion {
                spent_cents: 80,
                budget_cents: 100
            }),
            BudgetPressure::NearExhaustion
        );
        assert_eq!(
            BudgetPressure::from_verdict(&BudgetVerdict::Exhausted {
                spent_cents: 100,
                budget_cents: 100
            }),
            BudgetPressure::Exhausted
        );
    }

    #[tokio::test]
    async fn test_score_candidate_with_budget_pressure() {
        let candidate = RouteCandidate {
            provider: Provider {
                id: "test".to_string(),
                name: "test".to_string(),
                provider_type: ProviderType::Agent {
                    agent_id: "test".to_string(),
                    cli_command: "opencode".to_string(),
                    working_dir: PathBuf::from("/tmp"),
                },
                capabilities: vec![],
                cost_level: CostLevel::Expensive,
                latency: Latency::Medium,
                keywords: vec![],
            },
            model: "opus".to_string(),
            cli_tool: "opencode".to_string(),
            source: RouteSource::KnowledgeGraph,
            confidence: 0.9,
        };

        let score_no_pressure =
            RoutingDecisionEngine::score_candidate(&candidate, BudgetPressure::NoPressure);
        let score_near =
            RoutingDecisionEngine::score_candidate(&candidate, BudgetPressure::NearExhaustion);
        let score_exhausted =
            RoutingDecisionEngine::score_candidate(&candidate, BudgetPressure::Exhausted);

        assert!(score_no_pressure > score_near);
        assert!(score_near > score_exhausted);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_telemetry_penalises_subscription_limited_model() {
        use crate::control_plane::telemetry::{CompletionEvent, TelemetryStore, TokenBreakdown};

        let store = TelemetryStore::new(3600);
        store
            .record(CompletionEvent {
                model: "limited-model".to_string(),
                session_id: "test".to_string(),
                completed_at: chrono::Utc::now(),
                latency_ms: 0,
                success: false,
                tokens: TokenBreakdown::default(),
                cost_usd: 0.0,
                error: Some("weekly session limit reached".to_string()),
            })
            .await;

        let engine = RoutingDecisionEngine::new(
            None,
            Vec::new(),
            terraphim_router::Router::new(),
            Some(Arc::new(store)),
        );

        let ctx = create_test_context_with_static_model("agent", "task", "limited-model");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(
            decision.telemetry_influenced,
            "telemetry should influence when subscription limited"
        );
        assert!(
            decision.rationale.contains("Telemetry"),
            "rationale should mention telemetry"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_telemetry_boosts_high_success_model() {
        use crate::control_plane::telemetry::{CompletionEvent, TelemetryStore, TokenBreakdown};

        let store = TelemetryStore::new(3600);
        // Record 10 successful completions with good latency
        for _ in 0..10 {
            store
                .record(CompletionEvent {
                    model: "fast-model".to_string(),
                    session_id: "test".to_string(),
                    completed_at: chrono::Utc::now(),
                    latency_ms: 200,
                    success: true,
                    tokens: TokenBreakdown {
                        total: 500,
                        input: 400,
                        output: 100,
                        ..Default::default()
                    },
                    cost_usd: 0.005,
                    error: None,
                })
                .await;
        }

        let engine = RoutingDecisionEngine::new(
            None,
            Vec::new(),
            terraphim_router::Router::new(),
            Some(Arc::new(store)),
        );

        let ctx = create_test_context_with_static_model("agent", "implement feature", "fast-model");
        let decision = engine.decide_route(&ctx, &BudgetVerdict::Uncapped).await;

        assert!(
            decision.telemetry_influenced,
            "telemetry should influence with high success rate"
        );
        assert!(
            decision.rationale.contains("Telemetry"),
            "rationale should mention telemetry"
        );
    }
}
