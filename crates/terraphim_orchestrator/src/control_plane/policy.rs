//! Routing policy for telemetry-aware model selection.
//!
//! Applies hard filters (health, subscription limits, staleness) and
//! soft ranking (throughput, latency, consumption preference) to produce
//! an ordered list of eligible route candidates with rationale.

use crate::control_plane::routing::{RouteCandidate, RouteSource};
use crate::control_plane::telemetry::{ModelPerformanceSnapshot, UsageSnapshot};

/// Maximum staleness in seconds before performance data is considered stale.
const DEFAULT_MAX_STALENESS_SECS: u64 = 600;

/// Result of applying routing policy to a set of candidates.
#[derive(Debug, Clone)]
pub struct PolicyResult {
    /// Eligible candidates after hard filters, ranked by policy.
    pub eligible: Vec<ScoredCandidate>,
    /// Candidates rejected by hard filters, with reasons.
    pub rejected: Vec<(RouteCandidate, String)>,
}

/// A candidate with its policy score.
#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub candidate: RouteCandidate,
    pub score: f64,
    pub rationale_breakdown: Vec<(String, f64)>,
}

/// Configuration for routing policy weights.
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// Weight for task fit (KG/keyword match).
    pub task_fit_weight: f64,
    /// Weight for throughput (successful completions / time window).
    pub throughput_weight: f64,
    /// Weight for latency (lower is better).
    pub latency_weight: f64,
    /// Weight for consumption preference (lower recent usage preferred).
    pub consumption_weight: f64,
    /// Maximum staleness in seconds before performance data is stale.
    pub max_staleness_secs: u64,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            task_fit_weight: 0.35,
            throughput_weight: 0.25,
            latency_weight: 0.20,
            consumption_weight: 0.20,
            max_staleness_secs: DEFAULT_MAX_STALENESS_SECS,
        }
    }
}

impl PolicyConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Apply routing policy to a set of candidates.
///
/// Hard filters:
/// - Subscription limit reached -> reject
/// - Unhealthy (from ProviderHealthMap) -> reject (handled upstream)
/// - Stale performance data -> reject unless it is a static/config fallback
///
/// Ranking (among eligible):
/// 1. Task fit (source confidence)
/// 2. Throughput
/// 3. Latency (inverted)
/// 4. Lower recent consumption
pub fn apply_policy(
    candidates: Vec<RouteCandidate>,
    performances: &[ModelPerformanceSnapshot],
    session_usage: Option<&UsageSnapshot>,
    config: &PolicyConfig,
) -> PolicyResult {
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();

    let perf_map: std::collections::HashMap<&str, &ModelPerformanceSnapshot> =
        performances.iter().map(|p| (p.model.as_str(), p)).collect();

    let _total_session_tokens = session_usage.map(|u| u.totals.total_tokens).unwrap_or(0);

    let session_max_tokens: u64 = 5_000_000;

    for candidate in candidates {
        let model = candidate.model.clone();
        let perf = perf_map.get(model.as_str());

        if let Some(perf) = perf {
            if perf.is_subscription_limited() {
                rejected.push((
                    candidate,
                    format!("subscription limit reached for {}", model),
                ));
                continue;
            }

            let is_fallback = matches!(
                candidate.source,
                RouteSource::StaticConfig | RouteSource::CliDefault
            );

            if !is_fallback && perf.is_stale(config.max_staleness_secs) {
                rejected.push((
                    candidate,
                    format!(
                        "stale performance data for {} (last event: {:?})",
                        model, perf.last_event_at
                    ),
                ));
                continue;
            }
        }

        let model_tokens = session_usage
            .and_then(|u| u.by_model.get(&candidate.model))
            .map(|mu| mu.total_tokens)
            .unwrap_or(0);

        let score = score_candidate(
            &candidate,
            perf.cloned(),
            model_tokens,
            session_max_tokens,
            config,
        );

        eligible.push(ScoredCandidate {
            candidate,
            score,
            rationale_breakdown: vec![],
        });
    }

    eligible.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    PolicyResult { eligible, rejected }
}

/// Score a single candidate based on policy weights.
fn score_candidate(
    candidate: &RouteCandidate,
    perf: Option<&ModelPerformanceSnapshot>,
    model_tokens: u64,
    session_max_tokens: u64,
    config: &PolicyConfig,
) -> f64 {
    let task_fit = candidate.confidence;

    let (throughput, latency, consumption) = match perf {
        Some(p) => {
            let tp = normalize_throughput(p.throughput);
            let lat = normalize_latency(p.avg_latency_ms);
            let cons = normalize_consumption(model_tokens, session_max_tokens);
            (tp, lat, cons)
        }
        None => {
            let is_fallback = matches!(
                candidate.source,
                RouteSource::StaticConfig | RouteSource::CliDefault
            );
            if is_fallback {
                (0.5, 0.5, 0.5)
            } else {
                (0.0, 0.0, 0.5)
            }
        }
    };

    config.task_fit_weight * task_fit
        + config.throughput_weight * throughput
        + config.latency_weight * latency
        + config.consumption_weight * consumption
}

/// Normalize throughput to 0.0-1.0 range.
/// 0 completions/sec -> 0.0, 1.0/sec -> 1.0, capped at 1.0.
fn normalize_throughput(throughput: f64) -> f64 {
    (throughput / 1.0).min(1.0)
}

/// Normalize latency to 0.0-1.0 range (inverted: lower latency = higher score).
/// 0ms -> 1.0, 5000ms -> 0.0, linear between.
fn normalize_latency(latency_ms: f64) -> f64 {
    if latency_ms <= 0.0 {
        return 1.0;
    }
    (1.0 - latency_ms / 5000.0).max(0.0)
}

/// Normalize consumption preference to 0.0-1.0.
/// Lower usage -> higher score (prefer less-used models under pressure).
/// 0% of budget -> 1.0, 100% of budget -> 0.0.
fn normalize_consumption(total_tokens: u64, max_tokens: u64) -> f64 {
    if max_tokens == 0 {
        return 0.5;
    }
    1.0 - (total_tokens as f64 / max_tokens as f64).min(1.0)
}

/// Generate a human-readable rationale for a policy result.
pub fn generate_rationale(result: &PolicyResult) -> String {
    match result.eligible.first() {
        Some(scored) => {
            let candidate = &scored.candidate;
            let rejected_count = result.rejected.len();

            let mut parts = vec![format!(
                "Selected {} (source: {:?}, score: {:.3})",
                candidate.model, candidate.source, scored.score
            )];

            if rejected_count > 0 {
                let reasons: Vec<String> = result
                    .rejected
                    .iter()
                    .map(|(c, r)| format!("{}: {}", c.model, r))
                    .collect();
                parts.push(format!(
                    "Rejected {}: {}",
                    rejected_count,
                    reasons.join("; ")
                ));
            }

            parts.join(". ")
        }
        None => {
            if result.rejected.is_empty() {
                "No candidates available".to_string()
            } else {
                let reasons: Vec<String> = result
                    .rejected
                    .iter()
                    .map(|(c, r)| format!("{}: {}", c.model, r))
                    .collect();
                format!("All candidates rejected: {}", reasons.join("; "))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control_plane::telemetry::ModelUsage;
    use std::path::PathBuf;
    use terraphim_types::capability::{CostLevel, Latency, Provider, ProviderType};

    fn test_candidate(model: &str, source: RouteSource, confidence: f64) -> RouteCandidate {
        RouteCandidate {
            provider: Provider {
                id: format!("test-{}", model),
                name: format!("Test {}", model),
                provider_type: ProviderType::Agent {
                    agent_id: "test".to_string(),
                    cli_command: "opencode".to_string(),
                    working_dir: PathBuf::from("/tmp"),
                },
                capabilities: vec![],
                cost_level: CostLevel::Moderate,
                latency: Latency::Medium,
                keywords: vec![],
            },
            model: model.to_string(),
            cli_tool: "opencode".to_string(),
            source,
            confidence,
        }
    }

    fn test_perf(
        model: &str,
        throughput: f64,
        latency_ms: f64,
        success_rate: f64,
    ) -> ModelPerformanceSnapshot {
        ModelPerformanceSnapshot {
            model: model.to_string(),
            successful_completions: (throughput * 3600.0) as u64,
            failed_completions: 0,
            window_secs: 3600,
            throughput,
            avg_latency_ms: latency_ms,
            success_rate,
            last_event_at: Some(chrono::Utc::now()),
            subscription_limit_reached: false,
            subscription_limit_expires_at: None,
        }
    }

    #[test]
    fn test_policy_ranks_higher_throughput() {
        let candidates = vec![
            test_candidate("model-slow", RouteSource::KnowledgeGraph, 0.9),
            test_candidate("model-fast", RouteSource::KnowledgeGraph, 0.9),
        ];
        let performances = vec![
            test_perf("model-slow", 0.1, 3000.0, 1.0),
            test_perf("model-fast", 0.8, 500.0, 1.0),
        ];

        let result = apply_policy(candidates, &performances, None, &PolicyConfig::default());

        assert_eq!(result.eligible.len(), 2);
        assert_eq!(result.eligible[0].candidate.model, "model-fast");
    }

    #[test]
    fn test_policy_rejects_subscription_limited() {
        let candidates = vec![
            test_candidate("model-limited", RouteSource::KnowledgeGraph, 0.9),
            test_candidate("model-ok", RouteSource::KnowledgeGraph, 0.8),
        ];
        let mut perf_limited = test_perf("model-limited", 0.5, 1000.0, 1.0);
        perf_limited.subscription_limit_reached = true;
        perf_limited.subscription_limit_expires_at =
            Some(chrono::Utc::now() + chrono::Duration::hours(1));

        let performances = vec![perf_limited, test_perf("model-ok", 0.5, 1000.0, 1.0)];

        let result = apply_policy(candidates, &performances, None, &PolicyConfig::default());

        assert_eq!(result.eligible.len(), 1);
        assert_eq!(result.eligible[0].candidate.model, "model-ok");
        assert_eq!(result.rejected.len(), 1);
        assert!(result.rejected[0].1.contains("subscription limit"));
    }

    #[test]
    fn test_policy_prefers_lower_consumption_under_pressure() {
        let candidates = vec![
            test_candidate("model-a", RouteSource::KnowledgeGraph, 0.9),
            test_candidate("model-b", RouteSource::KnowledgeGraph, 0.9),
        ];
        let performances = vec![
            test_perf("model-a", 0.5, 1000.0, 1.0),
            test_perf("model-b", 0.5, 1000.0, 1.0),
        ];

        let usage = UsageSnapshot {
            id: "sess-1".to_string(),
            window_start: chrono::Utc::now() - chrono::Duration::hours(1),
            window_end: chrono::Utc::now(),
            by_model: {
                let mut m = std::collections::HashMap::new();
                m.insert(
                    "model-a".to_string(),
                    ModelUsage {
                        input_tokens: 3_000_000,
                        output_tokens: 500_000,
                        total_tokens: 3_500_000,
                        cost_usd: 0.0,
                        message_count: 50,
                    },
                );
                m.insert(
                    "model-b".to_string(),
                    ModelUsage {
                        input_tokens: 500_000,
                        output_tokens: 100_000,
                        total_tokens: 600_000,
                        cost_usd: 0.0,
                        message_count: 10,
                    },
                );
                m
            },
            totals: ModelUsage {
                input_tokens: 3_500_000,
                output_tokens: 600_000,
                total_tokens: 4_100_000,
                cost_usd: 0.0,
                message_count: 60,
            },
        };

        let result = apply_policy(
            candidates,
            &performances,
            Some(&usage),
            &PolicyConfig::default(),
        );

        assert_eq!(result.eligible.len(), 2);
        assert_eq!(result.eligible[0].candidate.model, "model-b");
    }

    #[test]
    fn test_policy_allows_fallback_without_perf() {
        let candidates = vec![test_candidate(
            "model-unknown",
            RouteSource::CliDefault,
            0.3,
        )];

        let result = apply_policy(candidates, &[], None, &PolicyConfig::default());

        assert_eq!(result.eligible.len(), 1);
    }

    #[test]
    fn test_normalise_latency() {
        assert!((normalize_latency(0.0) - 1.0).abs() < 0.01);
        assert!((normalize_latency(2500.0) - 0.5).abs() < 0.01);
        assert!((normalize_latency(5000.0) - 0.0).abs() < 0.01);
        assert!((normalize_latency(10000.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_generate_rationale() {
        let candidates = vec![test_candidate("model-a", RouteSource::KnowledgeGraph, 0.9)];
        let performances = vec![test_perf("model-a", 0.5, 1000.0, 1.0)];

        let result = apply_policy(candidates, &performances, None, &PolicyConfig::default());
        let rationale = generate_rationale(&result);
        assert!(rationale.contains("model-a"));
        assert!(rationale.contains("KnowledgeGraph"));
    }
}
